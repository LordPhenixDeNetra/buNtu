//! Preuve T1.21 : une clé fournisseur chiffrée n'apparaît pas en clair dans le fichier SQLite.

use std::error::Error;

use buntu_security::secrets::{MASTER_KEY_BYTES, SecretCipher};
use buntu_store::{ProviderRecord, Store};

#[tokio::test]
async fn provider_key_is_not_stored_in_plaintext() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "buntu-secret-at-rest-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let url = format!("sqlite://{}", path.display().to_string().replace('\\', "/"));
    let plaintext = b"sk-supersecret-fournisseur-0123456789";
    let cipher = SecretCipher::from_master_key(&[0x11_u8; MASTER_KEY_BYTES])?;
    let blob = cipher.encrypt(plaintext)?;

    let store = Store::open(&url).await?;
    store
        .insert_provider(&ProviderRecord {
            id: "p".to_owned(),
            kind: "openai".to_owned(),
            base_url: None,
        })
        .await?;
    store.insert_key("p", &blob, 1).await?;
    let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(store.pool())
        .await;
    drop(store);

    for candidate in [&path, &path.with_extension("sqlite-wal")] {
        if let Ok(bytes) = std::fs::read(candidate) {
            assert!(
                !bytes
                    .windows(plaintext.len())
                    .any(|window| window == plaintext),
                "clé en clair trouvée dans {}",
                candidate.display()
            );
        }
    }

    let reopened = Store::open(&url).await?;
    let keys = reopened.keys("p").await?;
    assert_eq!(keys.len(), 1);
    assert_eq!(cipher.decrypt(&keys[0].secret_ciphertext)?, plaintext);
    drop(reopened);
    let _ = std::fs::remove_file(&path);
    Ok(())
}
