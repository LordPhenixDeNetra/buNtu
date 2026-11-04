//! Jetons API : 256 bits aléatoires, stockés **hachés**, comparaison en temps constant (T1.20).
//!
//! Le jeton en clair n'existe qu'entre sa création et son affichage unique à l'administrateur ;
//! seul son SHA-256 est persisté.

use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;

/// Entropie d'un jeton (octets) : 256 bits.
pub const TOKEN_BYTES: usize = 32;
/// Préfixe lisible des jetons buNtu.
pub const TOKEN_PREFIX: &str = "buntu_";

/// Erreur de jeton.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("jeton invalide")]
    Invalid,
}

/// Jeton fraîchement généré : le clair est à afficher une seule fois, le hash est persisté.
#[derive(Debug, Clone)]
pub struct ApiToken {
    pub token: String,
    pub hash: String,
}

/// Calcule l'empreinte SHA-256 d'un jeton, en hexadécimal.
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Génère un jeton aléatoire de 256 bits et son empreinte.
pub fn generate_token() -> ApiToken {
    let mut bytes = [0_u8; TOKEN_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = format!("{TOKEN_PREFIX}{}", base64_url(&bytes));
    let hash = hash_token(&token);
    ApiToken { token, hash }
}

/// Vérifie un jeton contre l'empreinte persistée, en temps constant.
pub fn verify_token(expected_hash: &str, token: &str) -> Result<bool, TokenError> {
    if expected_hash.is_empty() {
        return Err(TokenError::Invalid);
    }
    let actual = hash_token(token);
    Ok(actual.as_bytes().ct_eq(expected_hash.as_bytes()).into())
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
