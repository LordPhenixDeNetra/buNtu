//! Binaire `buntu` : serveur, validation de configuration, version.

mod config_cmd;

use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use buntu_core::config::{ConfigHandle, ProcessEnv, spawn_file_watcher};
use buntu_gateway::admin::AdminStore;
use buntu_gateway::server::ServerConfig;
use buntu_gateway::{GatewayState, Readiness, app_with_state, serve, shutdown};

const USAGE: &str = "\
usage: buntu <commande>

commandes:
  serve [--config FICHIER] [--listen ADRESSE]   démarre la passerelle
  config validate FICHIER                       valide un fichier de configuration
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
        let (listen, handle, raw_config) = match &options.config {
            Some(path) => {
                let config = buntu_core::config::load(path, &ProcessEnv)?;
                let raw = buntu_core::config::file::read(path)?;
                let listen = options.listen.unwrap_or(config.server.listen);
                let handle = ConfigHandle::new(config);
                spawn_watchers(handle.clone(), path.clone())?;
                (listen, handle, raw)
            }
            None => (
                options
                    .listen
                    .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080))),
                ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
                buntu_core::config::RawConfig::default(),
            ),
        };

        let listener = tokio::net::TcpListener::bind(listen).await?;
        let address = listener.local_addr()?;
        println!("buntu listening on {address}");
        let readiness = Readiness::new(true);
        let state = GatewayState::new(readiness.clone(), handle)
            .with_admin(Arc::new(AdminStore::new(raw_config)));
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
        Some(other) => Err(format!(
            "sous-commande inconnue : {other}\nusage: buntu config validate <fichier>"
        )
        .into()),
        None => Err("usage: buntu config validate <fichier>".into()),
    }
}
