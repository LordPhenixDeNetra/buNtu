//! Déclencheur d'arrêt : SIGTERM (Unix) ou Ctrl-C.

use std::future::Future;
use std::pin::Pin;

/// Future qui se résout quand le processus reçoit un signal d'arrêt.
///
/// Sur Unix, SIGTERM et SIGINT (Ctrl-C) sont écoutés ; sur Windows, Ctrl-C seulement. En cas
/// d'échec d'enregistrement de SIGTERM, on retombe sur Ctrl-C plutôt que de rester sourd.
pub fn signal() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            if let Ok(mut terminate) = signal(SignalKind::terminate()) {
                tokio::select! {
                    _ = terminate.recv() => return,
                    _ = tokio::signal::ctrl_c() => return,
                }
            }
        }
        let _ = tokio::signal::ctrl_c().await;
    })
}
