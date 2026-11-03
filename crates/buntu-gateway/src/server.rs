//! Boucle d'acceptation et arrêt gracieux.

use std::future::Future;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::JoinSet;

use crate::health::Readiness;

/// Délai de lecture des en-têtes avant fermeture de la connexion.
pub const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Délai maximal laissé aux connexions en cours pendant l'arrêt.
pub const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(30);
/// Nombre maximal de connexions simultanées.
pub const MAX_CONNECTIONS: usize = 10_000;

#[derive(Debug, Clone, Copy)]
pub struct ServerConfig {
    pub header_read_timeout: Duration,
    pub shutdown_deadline: Duration,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            header_read_timeout: HEADER_READ_TIMEOUT,
            shutdown_deadline: SHUTDOWN_DEADLINE,
            max_connections: MAX_CONNECTIONS,
        }
    }
}

/// Sert `router` sur `listener` jusqu'à ce que `shutdown` se résolve.
///
/// À l'arrêt : `/ready` passe à 503, plus aucune connexion n'est acceptée, les connexions en
/// cours reçoivent l'ordre de terminer leur réponse, et le serveur attend au plus
/// `shutdown_deadline` avant de rendre la main.
pub async fn serve<F>(
    listener: TcpListener,
    router: Router,
    readiness: Readiness,
    shutdown: F,
    config: ServerConfig,
) -> io::Result<()>
where
    F: Future<Output = ()> + Send,
{
    let (close_tx, _close_rx) = watch::channel(false);
    let slots = Arc::new(Semaphore::new(config.max_connections));
    let mut connections = JoinSet::new();
    let mut shutdown = std::pin::pin!(shutdown);

    loop {
        let slot = tokio::select! {
            biased;
            _ = &mut shutdown => break,
            slot = Arc::clone(&slots).acquire_owned() => match slot {
                Ok(slot) => slot,
                Err(_) => break,
            },
        };

        let accepted = tokio::select! {
            biased;
            _ = &mut shutdown => break,
            accepted = listener.accept() => accepted,
        };

        let (stream, _peer) = match accepted {
            Ok(value) => value,
            Err(error) => {
                eprintln!("buNtu: accept failed: {error}");
                continue;
            }
        };

        let service = TowerToHyperService::new(router.clone());
        let mut close = close_tx.subscribe();
        let header_read_timeout = config.header_read_timeout;

        connections.spawn(async move {
            let mut builder = http1::Builder::new();
            builder
                .header_read_timeout(header_read_timeout)
                .timer(TokioTimer::new());
            let connection = builder.serve_connection(TokioIo::new(stream), service);
            let mut connection = std::pin::pin!(connection);

            tokio::select! {
                result = connection.as_mut() => {
                    if let Err(error) = result {
                        eprintln!("buNtu: connection failed: {error}");
                    }
                }
                changed = close.changed() => {
                    if changed.is_ok() {
                        connection.as_mut().graceful_shutdown();
                    }
                    if let Err(error) = connection.await {
                        eprintln!("buNtu: connection failed: {error}");
                    }
                }
            }

            drop(slot);
        });
    }

    readiness.set(false);
    let _ = close_tx.send(true);

    let deadline = tokio::time::sleep(config.shutdown_deadline);
    let mut deadline = std::pin::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => break,
            joined = connections.join_next() => {
                if joined.is_none() {
                    break;
                }
            }
        }
    }

    Ok(())
}
