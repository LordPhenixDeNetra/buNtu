use std::error::Error;
use std::net::SocketAddr;

use buntu_gateway::server::ServerConfig;
use buntu_gateway::{Readiness, app, serve, shutdown};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        None | Some("--version") | Some("version") => {
            println!("buntu {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("serve") => {
            let listen = parse_listen(arguments.collect())?;
            serve_command(listen)
        }
        Some(other) => Err(format!(
            "unknown command: {other}\nusage: buntu [serve [--listen ADDR] | --version]"
        )
        .into()),
    }
}

fn parse_listen(arguments: Vec<String>) -> Result<SocketAddr, Box<dyn Error>> {
    let mut listen = String::from("0.0.0.0:8080");
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--listen" => {
                index += 1;
                listen = arguments
                    .get(index)
                    .cloned()
                    .ok_or("--listen requires an address")?;
            }
            other => return Err(format!("unexpected argument: {other}").into()),
        }
        index += 1;
    }
    Ok(listen.parse()?)
}

fn serve_command(listen: SocketAddr) -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(listen).await?;
        let address = listener.local_addr()?;
        println!("buntu listening on {address}");
        let readiness = Readiness::new(true);
        serve(
            listener,
            app(readiness.clone()),
            readiness,
            shutdown::signal(),
            ServerConfig::default(),
        )
        .await?;
        Ok::<(), Box<dyn Error>>(())
    })
}
