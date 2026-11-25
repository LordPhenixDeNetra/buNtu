//! `buntu secrets rotate` (T1.21) : rotation de la clé maître des secrets au repos.
//!
//! La clé actuelle est résolue comme pour `serve` (`BUNTU_MASTER_KEY`, `BUNTU_MASTER_KEY_FILE`
//! ou `<base>.key`). La nouvelle est lue dans le fichier donné, ou créée (aléatoire, 0600) s'il
//! n'existe pas. Toutes les clés fournisseur chiffrées sont réchiffrées dans une seule
//! transaction : une erreur ne laisse jamais la base à moitié tournée.

use std::error::Error;
use std::path::Path;

use buntu_security::secrets::SecretCipher;
use buntu_store::{Store, StoreError};

pub const USAGE: &str = "usage: buntu secrets rotate <FICHIER_NOUVELLE_CLE>";

pub fn command(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    match arguments {
        [action, new_key] if action == "rotate" => rotate(Path::new(new_key)),
        _ => Err(USAGE.into()),
    }
}

fn rotate(new_key: &Path) -> Result<(), Box<dyn Error>> {
    let database = crate::database_path();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let store = Store::open(&format!("sqlite://{database}")).await?;
        let current = crate::master_key(&database, true)?.ok_or("clé maître actuelle absente")?;
        let (next, created) = SecretCipher::load_or_create(new_key)?;
        let count = reencrypt(&store, &current, &next).await?;
        if created {
            println!(
                "buNtu: nouvelle clé maître créée dans {}",
                new_key.display()
            );
        }
        println!(
            "buNtu: {count} clé(s) réchiffrée(s). Avant le prochain démarrage, pointez \
             BUNTU_MASTER_KEY_FILE vers {} (ou remplacez {database}.key par ce fichier).",
            new_key.display()
        );
        Ok::<(), Box<dyn Error>>(())
    })
}

/// Réchiffre toutes les clés de `store` de `current` vers `next`.
pub async fn reencrypt(
    store: &Store,
    current: &SecretCipher,
    next: &SecretCipher,
) -> Result<usize, StoreError> {
    store
        .reencrypt_keys(|blob| {
            current
                .rotate(next, blob)
                .map_err(|error| error.to_string())
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use buntu_store::{ProviderRecord, StoredKey, StoredSecret};

    fn cipher() -> Result<SecretCipher, Box<dyn Error>> {
        Ok(SecretCipher::from_master_key(
            &SecretCipher::random_master_key(),
        )?)
    }

    #[tokio::test]
    async fn rotation_reencrypts_every_key_and_keeps_references() -> Result<(), Box<dyn Error>> {
        let store = Store::open_in_memory().await?;
        let (old, new) = (cipher()?, cipher()?);
        let provider = ProviderRecord {
            id: "openai".to_owned(),
            kind: "openai".to_owned(),
            base_url: None,
        };
        let keys = [
            StoredKey {
                secret: StoredSecret::Encrypted(old.encrypt(b"sk-one")?),
                weight: 1,
            },
            StoredKey {
                secret: StoredSecret::Reference("env:OPENAI_KEY".to_owned()),
                weight: 1,
            },
        ];
        store.save_provider(&provider, &keys).await?;

        assert_eq!(reencrypt(&store, &old, &new).await?, 1);
        let stored = store.stored_keys("openai").await?;
        let StoredSecret::Encrypted(blob) = &stored[0].secret else {
            return Err("clé chiffrée attendue".into());
        };
        assert_eq!(new.decrypt(blob)?, b"sk-one");
        assert!(
            old.decrypt(blob).is_err(),
            "l'ancienne clé ne doit plus servir"
        );
        assert_eq!(
            stored[1].secret,
            StoredSecret::Reference("env:OPENAI_KEY".to_owned())
        );
        Ok(())
    }

    #[tokio::test]
    async fn wrong_current_key_changes_nothing() -> Result<(), Box<dyn Error>> {
        let store = Store::open_in_memory().await?;
        let (owner, stranger, new) = (cipher()?, cipher()?, cipher()?);
        let provider = ProviderRecord {
            id: "openai".to_owned(),
            kind: "openai".to_owned(),
            base_url: None,
        };
        let keys = [
            StoredKey {
                secret: StoredSecret::Encrypted(owner.encrypt(b"sk-one")?),
                weight: 1,
            },
            StoredKey {
                secret: StoredSecret::Encrypted(owner.encrypt(b"sk-two")?),
                weight: 1,
            },
        ];
        store.save_provider(&provider, &keys).await?;

        assert!(reencrypt(&store, &stranger, &new).await.is_err());
        for key in store.stored_keys("openai").await? {
            let StoredSecret::Encrypted(blob) = key.secret else {
                return Err("clé chiffrée attendue".into());
            };
            assert!(owner.decrypt(&blob).is_ok(), "rien ne doit avoir changé");
        }
        Ok(())
    }
}
