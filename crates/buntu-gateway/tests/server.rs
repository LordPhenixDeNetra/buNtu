//! Tests d'intégration du serveur : santé, limites, arrêt gracieux.

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use async_stream::stream;
use axum::Router;
use axum::body::Body;
use axum::routing::get;
use buntu_gateway::server::ServerConfig;
use buntu_gateway::{MAX_BODY_BYTES, Readiness, app, serve};
use bytes::Bytes;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

type Spawned = (SocketAddr, oneshot::Sender<()>, JoinHandle<()>);

async fn spawn(router: Router, readiness: Readiness) -> Result<Spawned, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = ServerConfig {
        header_read_timeout: Duration::from_millis(200),
        shutdown_deadline: Duration::from_secs(3),
        max_connections: 16,
    };
    let handle = tokio::spawn(async move {
        let _ = serve(
            listener,
            router,
            readiness,
            async move {
                let _ = shutdown_rx.await;
            },
            config,
        )
        .await;
    });
    Ok((address, shutdown_tx, handle))
}

/// Route lente : cinq morceaux espacés de 100 ms (500 ms au total).
fn slow_router() -> Router {
    Router::new().route(
        "/slow",
        get(|| async {
            Body::from_stream(stream! {
                for chunk in ["one", "two", "three", "four", "five"] {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    yield Ok::<Bytes, std::io::Error>(Bytes::from_static(chunk.as_bytes()));
                }
            })
        }),
    )
}

#[tokio::test]
async fn health_is_always_up_but_ready_depends_on_state() -> Result<(), Box<dyn Error>> {
    let readiness = Readiness::new(true);
    let (address, shutdown, handle) = spawn(app(readiness.clone()), readiness.clone()).await?;
    let client = reqwest::Client::new();

    assert_eq!(
        client
            .get(format!("http://{address}/health"))
            .send()
            .await?
            .status(),
        200
    );
    assert_eq!(
        client
            .get(format!("http://{address}/ready"))
            .send()
            .await?
            .status(),
        200
    );

    readiness.set(false);
    assert_eq!(
        client
            .get(format!("http://{address}/ready"))
            .send()
            .await?
            .status(),
        503
    );
    assert_eq!(
        client
            .get(format!("http://{address}/health"))
            .send()
            .await?
            .status(),
        200
    );

    let _ = shutdown.send(());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn body_larger_than_ten_megabytes_is_rejected() -> Result<(), Box<dyn Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let readiness = Readiness::new(true);
    let (address, shutdown, handle) = spawn(app(readiness.clone()), readiness).await?;

    // On annonce un corps trop gros sans l'envoyer : le serveur doit refuser d'après l'en-tête
    // `Content-Length`, avant de lire quoi que ce soit.
    let mut stream = tokio::net::TcpStream::connect(address).await?;
    let request = format!(
        "POST /health HTTP/1.1\r\nHost: buNtu\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
        MAX_BODY_BYTES + 1
    );
    stream.write_all(request.as_bytes()).await?;

    let mut buffer = vec![0_u8; 512];
    let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buffer)).await??;
    let response = String::from_utf8_lossy(&buffer[..read]).to_string();
    assert!(
        response.starts_with("HTTP/1.1 413"),
        "réponse inattendue : {response}"
    );

    let _ = shutdown.send(());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn partial_headers_are_timed_out() -> Result<(), Box<dyn Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let readiness = Readiness::new(true);
    let (address, shutdown, handle) = spawn(app(readiness.clone()), readiness).await?;

    let mut stream = tokio::net::TcpStream::connect(address).await?;
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: buNtu\r\n")
        .await?;

    let started = std::time::Instant::now();
    let mut buffer = [0_u8; 64];
    let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buffer)).await;
    assert!(
        read.is_ok(),
        "la connexion n'a pas été fermée par le timeout"
    );
    assert!(started.elapsed() < Duration::from_secs(2));

    let _ = shutdown.send(());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .is_ok()
    );
    Ok(())
}

#[tokio::test]
async fn graceful_shutdown_lets_a_stream_finish() -> Result<(), Box<dyn Error>> {
    let readiness = Readiness::new(true);
    let (address, shutdown, handle) = spawn(slow_router(), readiness).await?;

    let reader = tokio::spawn(async move {
        let response = reqwest::Client::new()
            .get(format!("http://{address}/slow"))
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        Ok::<_, reqwest::Error>((status, body))
    });

    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = shutdown.send(());

    let joined = tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .map_err(|_| "le flux n'a pas été terminé")??;
    let (status, body) = joined?;
    assert_eq!(status, 200);
    assert_eq!(body, "onetwothreefourfive");

    assert!(
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .is_ok(),
        "le serveur ne s'est pas arrêté dans le délai"
    );
    Ok(())
}

#[tokio::test]
async fn no_new_connection_is_accepted_after_shutdown() -> Result<(), Box<dyn Error>> {
    let readiness = Readiness::new(true);
    let (address, shutdown, handle) = spawn(app(readiness.clone()), readiness).await?;

    let _ = shutdown.send(());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .is_ok()
    );

    assert!(
        tokio::net::TcpStream::connect(address).await.is_err(),
        "une connexion a été acceptée après l'arrêt"
    );
    Ok(())
}
