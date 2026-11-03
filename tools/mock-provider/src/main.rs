use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listen = std::env::var("MOCK_LISTEN").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    println!("mock-provider listening on {listen}");
    axum::serve(listener, buntu_mock_provider::app()).await?;
    Ok(())
}
