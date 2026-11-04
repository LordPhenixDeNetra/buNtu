//! Preuves de T1.21 : AES-256-GCM, nonce par valeur, rotation et clé maître.

use std::error::Error;

use buntu_security::secrets::{MASTER_KEY_BYTES, SecretCipher};

fn cipher(seed: u8) -> Result<SecretCipher, Box<dyn Error>> {
    Ok(SecretCipher::from_master_key(&[seed; MASTER_KEY_BYTES])?)
}

#[test]
fn encrypt_decrypt_roundtrip_with_random_nonce() -> Result<(), Box<dyn Error>> {
    let cipher = cipher(1)?;
    let first = cipher.encrypt(b"sk-fournisseur-secret")?;
    let second = cipher.encrypt(b"sk-fournisseur-secret")?;
    assert_ne!(first, second, "le nonce doit changer à chaque chiffrement");
    assert_eq!(cipher.decrypt(&first)?, b"sk-fournisseur-secret");
    assert_eq!(cipher.decrypt(&second)?, b"sk-fournisseur-secret");
    Ok(())
}

#[test]
fn ciphertext_does_not_contain_the_plaintext() -> Result<(), Box<dyn Error>> {
    let cipher = cipher(2)?;
    let plaintext = b"sk-supersecret-fournisseur";
    let blob = cipher.encrypt(plaintext)?;
    assert!(
        !blob
            .windows(plaintext.len())
            .any(|window| window == plaintext)
    );
    Ok(())
}

#[test]
fn wrong_key_is_rejected() -> Result<(), Box<dyn Error>> {
    let blob = cipher(3)?.encrypt(b"secret")?;
    assert!(cipher(4)?.decrypt(&blob).is_err());
    Ok(())
}

#[test]
fn master_key_rotation_reencrypts_the_value() -> Result<(), Box<dyn Error>> {
    let old = cipher(5)?;
    let new = cipher(6)?;
    let blob = old.encrypt(b"sk-a-rotation")?;
    let rotated = old.rotate(&new, &blob)?;
    assert!(old.decrypt(&rotated).is_err());
    assert_eq!(new.decrypt(&rotated)?, b"sk-a-rotation");
    Ok(())
}

#[test]
fn master_key_file_must_contain_32_bytes() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("buntu-key-{}.bin", std::process::id()));
    std::fs::write(&path, [0x42_u8; MASTER_KEY_BYTES])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // La CI Linux crée les fichiers en 0644 : la clé doit être en 0600 pour être acceptée.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    let cipher = SecretCipher::from_file(&path)?;
    let blob = cipher.encrypt(b"valeur")?;
    assert_eq!(cipher.decrypt(&blob)?, b"valeur");
    std::fs::write(&path, [0x42_u8; 16])?;
    assert!(SecretCipher::from_file(&path).is_err());
    let _ = std::fs::remove_file(&path);
    assert!(SecretCipher::from_env("BUNTU_MASTER_KEY_ABSENTE").is_err());
    Ok(())
}
