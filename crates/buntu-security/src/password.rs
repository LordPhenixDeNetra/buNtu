//! Hachage de mot de passe admin : Argon2id avec les paramètres OWASP (T1.20).
//!
//! Référence : <https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html>
//! (consultée le 2026-11-04) : Argon2id, m = 19 456 Kio, t = 2, p = 1.

use argon2::password_hash::{
    PasswordHash as PhcHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use thiserror::Error;

/// Coût mémoire OWASP (Kio).
pub const MEMORY_COST_KIB: u32 = 19_456;
/// Itérations OWASP.
pub const TIME_COST: u32 = 2;
/// Parallélisme OWASP.
pub const PARALLELISM: u32 = 1;

/// Erreur de hachage ou de vérification.
#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("paramètres Argon2id invalides : {0}")]
    Params(String),
    #[error("hachage Argon2id : {0}")]
    Hash(String),
    #[error("empreinte de mot de passe illisible")]
    InvalidHash,
}

/// Empreinte PHC d'un mot de passe admin (jamais le mot de passe en clair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn argon2() -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(MEMORY_COST_KIB, TIME_COST, PARALLELISM, None)
        .map_err(|error| PasswordError::Params(error.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hache un mot de passe admin avec Argon2id et un sel aléatoire.
pub fn hash_password(password: &str) -> Result<PasswordHash, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| PasswordError::Hash(error.to_string()))?;
    Ok(PasswordHash(hash.to_string()))
}

/// Vérifie un mot de passe contre son empreinte ; ne panique jamais sur une entrée invalide.
pub fn verify_password(hash: &PasswordHash, password: &str) -> Result<bool, PasswordError> {
    let parsed = PhcHash::new(hash.as_str()).map_err(|_| PasswordError::InvalidHash)?;
    Ok(argon2()?
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
