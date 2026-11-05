//! Chiffrement des secrets au repos : AES-256-GCM, nonce aléatoire par valeur (T1.21).
//!
//! La clé maître fait 32 octets : lue depuis un fichier (permissions 0600 vérifiées sous Unix)
//! ou une variable d'environnement encodée en base64. Elle n'est **jamais** écrite en base.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use base64::Engine;
use rand::RngCore;
use thiserror::Error;

/// Taille de la clé maître (AES-256).
pub const MASTER_KEY_BYTES: usize = 32;
/// Taille d'un nonce GCM.
pub const NONCE_BYTES: usize = 12;
/// Nom de la variable d'environnement par défaut.
pub const DEFAULT_KEY_ENV: &str = "BUNTU_MASTER_KEY";

/// Erreur de chiffrement.
#[derive(Debug, Error)]
pub enum SecretError {
    #[error("clé maître absente ou invalide : {0}")]
    Key(String),
    #[error("chiffrement : {0}")]
    Encrypt(String),
    #[error("déchiffrement refusé (clé ou données altérées)")]
    Decrypt,
}

/// Chiffreur symétrique des secrets.
#[derive(Clone)]
pub struct SecretCipher {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for SecretCipher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretCipher([clé masquée])")
    }
}

impl SecretCipher {
    /// Construit le chiffreur à partir d'une clé maître de 32 octets.
    pub fn from_master_key(key: &[u8]) -> Result<Self, SecretError> {
        if key.len() != MASTER_KEY_BYTES {
            return Err(SecretError::Key(format!(
                "32 octets attendus, {} reçus",
                key.len()
            )));
        }
        Ok(Self {
            cipher: Aes256Gcm::new_from_slice(key)
                .map_err(|_| SecretError::Key("clé AES-256 invalide".to_owned()))?,
        })
    }

    /// Lit la clé maître depuis une variable d'environnement (base64 standard ou URL).
    pub fn from_env(variable: &str) -> Result<Self, SecretError> {
        let value = std::env::var(variable)
            .map_err(|_| SecretError::Key(format!("variable `{variable}` absente")))?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(value.trim()))
            .map_err(|_| SecretError::Key(format!("`{variable}` n'est pas du base64")))?;
        Self::from_master_key(&decoded)
    }

    /// Lit la clé maître depuis un fichier ; sous Unix, exige des permissions 0600.
    pub fn from_file(path: &std::path::Path) -> Result<Self, SecretError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata =
                std::fs::metadata(path).map_err(|error| SecretError::Key(error.to_string()))?;
            let mode = metadata.permissions().mode() & 0o777;
            if mode & 0o077 != 0 {
                return Err(SecretError::Key(format!(
                    "permissions trop ouvertes sur {} ({mode:o}) ; attendu 0600",
                    path.display()
                )));
            }
        }
        let bytes = std::fs::read(path).map_err(|error| SecretError::Key(error.to_string()))?;
        let content = bytes.trim_ascii();
        // 32 octets bruts, ou leur encodage base64 (format écrit par `load_or_create`).
        if content.len() == MASTER_KEY_BYTES {
            return Self::from_master_key(content);
        }
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err(|_| SecretError::Key("ni 32 octets bruts ni base64".to_owned()))?;
        Self::from_master_key(&decoded)
    }

    /// Lit la clé maître du fichier, ou la crée (aléatoire, base64, 0600) s'il n'existe pas.
    /// Rend `true` quand la clé vient d'être créée.
    pub fn load_or_create(path: &std::path::Path) -> Result<(Self, bool), SecretError> {
        if path.exists() {
            return Ok((Self::from_file(path)?, false));
        }
        let key = Self::random_master_key();
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(path)
            .map_err(|error| SecretError::Key(format!("{} : {error}", path.display())))?;
        std::io::Write::write_all(&mut file, format!("{encoded}\n").as_bytes())
            .map_err(|error| SecretError::Key(error.to_string()))?;
        Ok((Self::from_master_key(&key)?, true))
    }

    /// Chiffre une valeur : `nonce || ciphertext` avec un nonce aléatoire par appel.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, SecretError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|error| SecretError::Encrypt(error.to_string()))?;
        let mut blob = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
        blob.extend_from_slice(nonce.as_slice());
        blob.extend_from_slice(&ciphertext);
        Ok(blob)
    }

    /// Déchiffre une valeur produite par [`SecretCipher::encrypt`].
    pub fn decrypt(&self, blob: &[u8]) -> Result<Vec<u8>, SecretError> {
        if blob.len() <= NONCE_BYTES {
            return Err(SecretError::Decrypt);
        }
        let (nonce, ciphertext) = blob.split_at(NONCE_BYTES);
        let nonce = Nonce::from_slice(nonce);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| SecretError::Decrypt)
    }

    /// Rotation : déchiffre avec l'ancienne clé maître, réchiffre avec la nouvelle.
    pub fn rotate(&self, new: &SecretCipher, blob: &[u8]) -> Result<Vec<u8>, SecretError> {
        let plaintext = self.decrypt(blob)?;
        new.encrypt(&plaintext)
    }

    /// Génère une clé maître aléatoire de 32 octets (premier démarrage).
    pub fn random_master_key() -> [u8; MASTER_KEY_BYTES] {
        let mut key = [0_u8; MASTER_KEY_BYTES];
        rand::rngs::OsRng.fill_bytes(&mut key);
        key
    }
}
