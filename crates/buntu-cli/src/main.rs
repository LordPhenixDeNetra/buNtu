//! Binaire `buntu` : serveur, validation de configuration, version.

mod config_cmd;
mod health_cmd;

use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use buntu_core::config::{ConfigHandle, ProcessEnv, RawConfig, spawn_file_watcher};
use buntu_gateway::admin::{AdminStore, load_base};
use buntu_gateway::server::ServerConfig;
use buntu_gateway::{GatewayState, Readiness, app_with_state, serve, shutdown};
use buntu_security::secrets::{DEFAULT_KEY_ENV, SecretCipher};

const USAGE: &str = "\
usage: buntu <commande>

commandes:
  serve [--config FICHIER] [--listen ADRESSE]   démarre la passerelle
  config validate FICHIER                       valide un fichier de configuration
  config schema                                 affiche le schéma JSON de la configuration
  health [ADRESSE]                              sonde GET /health (défaut 127.0.0.1:8080)
  --version                                     affiche la version
";

#[derive(Debug, Default)]
struct ServeOptions {
    listen: Option<SocketAddr>,
    config: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = arguments.first().cloned() else {
        println!("buntu {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    };
    arguments.remove(0);

    match command.as_str() {
        "--version" | "version" => {
            println!("buntu {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "serve" => serve_command(parse_serve(arguments)?),
        "config" => config_command(arguments),
        "health" => {
            let address: SocketAddr = arguments
                .first()
                .map_or(health_cmd::DEFAULT_ADDRESS, String::as_str)
                .parse()?;
            if health_cmd::probe(address) {
                Ok(())
            } else {
                Err(format!("passerelle injoignable ou non prête sur {address}").into())
            }
        }
        other => Err(format!("commande inconnue : {other}\n{USAGE}").into()),
    }
}

fn parse_serve(arguments: Vec<String>) -> Result<ServeOptions, Box<dyn Error>> {
    let mut options = ServeOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--listen" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or("--listen requiert une adresse")?;
                options.listen = Some(value.parse()?);
            }
            "--config" => {
                index += 1;
                let value = arguments.get(index).ok_or("--config requiert un fichier")?;
                options.config = Some(PathBuf::from(value));
            }
            other => return Err(format!("option inconnue : {other}\n{USAGE}").into()),
        }
        index += 1;
    }
    Ok(options)
}

fn serve_command(options: ServeOptions) -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        // Stockage (journal, jetons, couche base de la configuration), chemin réglable.
        let database = std::env::var("BUNTU_DB_PATH").unwrap_or_else(|_| "buntu.sqlite".to_owned());
        let store = match buntu_store::Store::open(&format!("sqlite://{database}")).await {
            Ok(store) => Some(store),
            Err(error) => {
                eprintln!("buNtu: stockage indisponible ({error})");
                None
            }
        };
        let cipher = master_key(&database, store.is_some())?;

        // Trois sources, un seul modèle (cahier §7.1, ADR-0008) : base < fichier < environnement.
        let base = match &store {
            Some(store) => load_base(store, cipher.as_ref()).await?,
            None => RawConfig::default(),
        };
        let file = match &options.config {
            Some(path) => buntu_core::config::file::read(path)?,
            None => RawConfig::default(),
        };
        let handle = ConfigHandle::from_layers(base, file, &ProcessEnv)?;
        if let Some(path) = &options.config {
            spawn_watchers(handle.clone(), path.clone())?;
        }
        let listen = options.listen.unwrap_or(handle.snapshot().server.listen);

        let listener = tokio::net::TcpListener::bind(listen).await?;
        let address = listener.local_addr()?;
        println!("buntu listening on {address}");
        // Exportateur Prometheus global (T1.19) : un échec n'empêche pas de servir.
        if let Err(error) = buntu_observe::metrics::install_global() {
            eprintln!("buNtu: métriques indisponibles ({error})");
        }
        let readiness = Readiness::new(true);
        let mut state = GatewayState::new(readiness.clone(), handle)
            .with_admin(Arc::new(AdminStore::new(cipher)));
        if let Some(store) = store {
            state = state.with_storage(Arc::new(store));
        }
        state = admin_token(state).await?;
        serve(
            listener,
            app_with_state(state),
            readiness,
            shutdown::signal(),
            ServerConfig::default(),
        )
        .await?;
        Ok::<(), Box<dyn Error>>(())
    })
}

/// Clé maître des secrets au repos (T1.21) : `BUNTU_MASTER_KEY` (base64), sinon le fichier
/// `BUNTU_MASTER_KEY_FILE`, sinon `<base>.key` à côté de la base, créé au premier démarrage
/// (0600). Elle n'est jamais écrite en base.
fn master_key(database: &str, has_store: bool) -> Result<Option<SecretCipher>, Box<dyn Error>> {
    if std::env::var_os(DEFAULT_KEY_ENV).is_some() {
        return Ok(Some(SecretCipher::from_env(DEFAULT_KEY_ENV)?));
    }
    if let Some(path) = std::env::var_os("BUNTU_MASTER_KEY_FILE") {
        return Ok(Some(SecretCipher::from_file(Path::new(&path))?));
    }
    if !has_store {
        return Ok(None);
    }
    let path = PathBuf::from(format!("{database}.key"));
    let (cipher, created) = SecretCipher::load_or_create(&path)?;
    if created {
        println!(
            "buNtu: clé maître créée dans {} (à sauvegarder, jamais à versionner)",
            path.display()
        );
    }
    Ok(Some(cipher))
}

/// Longueur minimale d'un jeton admin fourni par l'environnement.
const MIN_ADMIN_TOKEN_CHARS: usize = 32;

/// Jeton de l'API d'administration (revue 2026-11-05, point 1a).
///
/// `BUNTU_ADMIN_TOKEN` s'il est défini ; sinon, au premier démarrage (aucun jeton actif en
/// base), un jeton de 256 bits est généré, **affiché une seule fois** et stocké haché.
async fn admin_token(state: GatewayState) -> Result<GatewayState, Box<dyn Error>> {
    if let Ok(token) = std::env::var("BUNTU_ADMIN_TOKEN") {
        if token.chars().count() < MIN_ADMIN_TOKEN_CHARS {
            return Err(format!(
                "BUNTU_ADMIN_TOKEN trop court ({MIN_ADMIN_TOKEN_CHARS} caractères minimum)"
            )
            .into());
        }
        return Ok(state.with_admin_token(&token));
    }
    let Some(store) = state.store.clone() else {
        eprintln!("buNtu: API d'administration fermée (ni stockage ni BUNTU_ADMIN_TOKEN)");
        return Ok(state);
    };
    if !store.has_active_api_token().await? {
        let token = buntu_security::tokens::generate_token();
        let id = format!("tok-{}", token.hash.get(..16).unwrap_or(&token.hash));
        store
            .insert_api_token(&id, &token.hash, "jeton admin initial")
            .await?;
        println!(
            "buNtu: jeton d'administration (affiché une seule fois) : {}",
            token.token
        );
    }
    Ok(state)
}

fn spawn_watchers(handle: ConfigHandle, path: PathBuf) -> Result<(), Box<dyn Error>> {
    let _watcher = spawn_file_watcher(handle.clone(), path.clone(), Duration::from_millis(500))
        .map_err(|error| Box::new(error) as Box<dyn Error>)?;
    #[cfg(unix)]
    spawn_sighup(handle, path);
    Ok(())
}

#[cfg(unix)]
fn spawn_sighup(handle: ConfigHandle, path: PathBuf) {
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut stream) = signal(SignalKind::hangup()) else {
            eprintln!("buNtu: SIGHUP indisponible");
            return;
        };
        while stream.recv().await.is_some() {
            if let Err(error) = handle.reload(&path, &ProcessEnv) {
                eprintln!(
                    "buNtu: rechargement sur SIGHUP refusé ({error}) ; ancienne configuration conservée"
                );
            }
        }
    });
}

fn config_command(arguments: Vec<String>) -> Result<(), Box<dyn Error>> {
    match arguments.first().map(String::as_str) {
        Some("validate") => {
            let path = arguments
                .get(1)
                .ok_or("usage: buntu config validate <fichier>")?;
            let config = config_cmd::validate(Path::new(path))?;
            println!(
                "configuration valide : {} ({} fournisseur(s), {} route(s), écoute {})",
                path,
                config.providers.len(),
                config.routes.len(),
                config.server.listen
            );
            Ok(())
        }
        Some("schema") => {
            let schema = buntu_core::config::json_schema();
            let pretty = serde_json::to_string_pretty(&schema)
                .map_err(|error| format!("schéma non sérialisable : {error}"))?;
            println!("{pretty}");
            Ok(())
        }
        Some(other) => Err(format!(
            "sous-commande inconnue : {other}\nusage: buntu config validate <fichier> | buntu config schema"
        )
        .into()),
        None => Err("usage: buntu config validate <fichier> | buntu config schema".into()),
    }
}
