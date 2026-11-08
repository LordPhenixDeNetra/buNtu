//! `buntu health` : sonde de vie pour l'image distroless (ni shell ni `curl`).
//!
//! Interroge `GET /health` en HTTP/1.1 brut avec `std::net`, sans dépendance : code de sortie
//! 0 si la passerelle répond 200, 1 sinon. Utilisé par le `HEALTHCHECK` de `deploy/Dockerfile`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Adresse sondée par défaut : celle de l'image (`--listen 0.0.0.0:8080`), vue de l'intérieur.
pub const DEFAULT_ADDRESS: &str = "127.0.0.1:8080";
/// Délai maximal de connexion, d'écriture et de lecture.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// `true` si `GET /health` répond 200 à `address` dans le délai.
pub fn probe(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, TIMEOUT) else {
        return false;
    };
    if stream.set_read_timeout(Some(TIMEOUT)).is_err()
        || stream.set_write_timeout(Some(TIMEOUT)).is_err()
    {
        return false;
    }
    let request = format!("GET /health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    // La ligne de statut suffit : « HTTP/1.1 200 OK ».
    let mut head = [0_u8; 32];
    let Ok(read) = stream.read(&mut head) else {
        return false;
    };
    let status_line = String::from_utf8_lossy(&head[..read]);
    status_line
        .split_whitespace()
        .nth(1)
        .is_some_and(|code| code == "200")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::net::TcpListener;

    /// Serveur d'une seule réponse, sur un port libre.
    fn serve_once(response: &'static str) -> Result<SocketAddr, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 512];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Ok(address)
    }

    #[test]
    fn healthy_gateway_answers_200() -> Result<(), Box<dyn Error>> {
        let address =
            serve_once("HTTP/1.1 200 OK\r\ncontent-length: 15\r\n\r\n{\"status\":\"ok\"}")?;
        assert!(probe(address));
        Ok(())
    }

    #[test]
    fn non_200_status_is_unhealthy() -> Result<(), Box<dyn Error>> {
        let address = serve_once("HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\r\n")?;
        assert!(!probe(address));
        Ok(())
    }

    #[test]
    fn nothing_listening_is_unhealthy() -> Result<(), Box<dyn Error>> {
        // Port obtenu puis libéré : plus personne n'écoute dessus.
        let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?;
        assert!(!probe(address));
        Ok(())
    }
}
