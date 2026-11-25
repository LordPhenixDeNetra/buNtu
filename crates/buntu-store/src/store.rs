//! Stockage SQLite (T1.17) : pool partagé, WAL, `busy_timeout` et migrations versionnées.
//!
//! Le schéma vit dans `migrations/` à la racine du dépôt ; les migrations sont **en avant
//! seulement** (jamais de retour arrière destructif).
//!
//! Les requêtes passent par les macros `sqlx::query!` : elles sont vérifiées à la compilation
//! contre le schéma. Sans base, la compilation lit le cache hors ligne `.sqlx/` de la crate,
//! régénéré par `scripts/sqlx-prepare.sh` et contrôlé par la CI. Deux requêtes restent
//! dynamiques : le filtre du journal (construit à la volée) et la version de schéma (table
//! interne de sqlx).
//!
//! Référence : <https://docs.rs/sqlx/latest/sqlx/macro.query.html> (consultée le 2026-11-25).

use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

/// Migration embarquée à la compilation (chemin relatif au manifeste de la crate).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Erreur de stockage.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("unknown entity")]
    NotFound,
    #[error("key rotation: {0}")]
    Rotation(String),
}

/// Un fournisseur persisté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRecord {
    pub id: String,
    pub kind: String,
    pub base_url: Option<String>,
}

/// Une clé fournisseur persistée : seul le chiffré est stocké.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRecord {
    pub id: i64,
    pub provider_id: String,
    pub secret_ciphertext: Vec<u8>,
    pub weight: u32,
}

/// Une route persistée (chaîne de repli sérialisée en JSON).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRecord {
    pub alias: String,
    pub fallbacks: Vec<String>,
}

/// Secret d'une clé de la couche base : chiffré, ou référence `env:VAR` (pas un secret).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredSecret {
    Encrypted(Vec<u8>),
    Reference(String),
}

/// Une clé de la couche base, dans l'ordre de déclaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredKey {
    pub secret: StoredSecret,
    pub weight: u32,
}

/// Une route de la couche base ; `timeouts` est le JSON des surcharges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRoute {
    pub alias: String,
    pub fallbacks: Vec<String>,
    pub timeouts: String,
}

/// Une ligne du journal des requêtes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLogRecord {
    pub id: String,
    pub model: String,
    pub provider: String,
    pub status: u16,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub created_at: i64,
    pub attempts: u32,
    pub rowid: i64,
}

/// Agrégats du journal sur une fenêtre de temps, calculés par SQLite sur toutes ses lignes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestStats {
    pub requests: i64,
    pub errors: i64,
    pub latency_p50_ms: Option<i64>,
    pub latency_p95_ms: Option<i64>,
}

/// Filtres du journal, paginés par curseur (rowid).
#[derive(Debug, Clone, Default)]
pub struct RequestLogQuery {
    pub cursor: Option<i64>,
    pub limit: i64,
    pub provider: Option<String>,
    pub status: Option<u16>,
    pub model: Option<String>,
}

/// Pool SQLite partagé : à cloner, jamais à recréer par requête.
#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

/// Erreur de décodage d'une valeur stockée (JSON, entier hors bornes).
fn decode_error(error: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Database(sqlx::Error::Decode(Box::new(error)))
}

/// Poids stocké (INTEGER, `CHECK (weight > 0)`) vers `u32`.
fn weight(value: i64) -> Result<u32, StoreError> {
    u32::try_from(value).map_err(decode_error)
}

impl Store {
    /// Ouvre (ou crée) une base SQLite et applique les migrations.
    ///
    /// `path` accepte `sqlite::memory:` pour les tests.
    pub async fn open(path: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(path)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// Base en mémoire, une seule connexion (les migrations restent visibles).
    pub async fn open_in_memory() -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .journal_mode(SqliteJournalMode::Memory)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// Accès au pool (pour les futures requêtes de l'API d'administration).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Version de schéma appliquée par la dernière migration.
    ///
    /// Requête dynamique : `_sqlx_migrations` est créée par sqlx à l'exécution, pas par les
    /// migrations du dépôt, et n'existe donc pas dans la base qui sert à vérifier les macros.
    pub async fn schema_version(&self) -> Result<i64, StoreError> {
        let row = sqlx::query("SELECT COALESCE(MAX(version), 0) AS version FROM _sqlx_migrations")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("version")?)
    }

    pub async fn insert_provider(&self, provider: &ProviderRecord) -> Result<(), StoreError> {
        sqlx::query!(
            "INSERT INTO providers (id, kind, base_url) VALUES (?, ?, ?)",
            provider.id,
            provider.kind,
            provider.base_url,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn provider(&self, id: &str) -> Result<ProviderRecord, StoreError> {
        sqlx::query_as!(
            ProviderRecord,
            "SELECT id, kind, base_url FROM providers WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)
    }

    pub async fn providers(&self) -> Result<Vec<ProviderRecord>, StoreError> {
        Ok(sqlx::query_as!(
            ProviderRecord,
            "SELECT id, kind, base_url FROM providers ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Insère une clé : le chiffré seul est stocké (T1.21).
    pub async fn insert_key(
        &self,
        provider_id: &str,
        secret_ciphertext: &[u8],
        weight: u32,
    ) -> Result<i64, StoreError> {
        let result = sqlx::query!(
            "INSERT INTO provider_keys (provider_id, secret_ciphertext, weight) VALUES (?, ?, ?)",
            provider_id,
            secret_ciphertext,
            weight,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn keys(&self, provider_id: &str) -> Result<Vec<KeyRecord>, StoreError> {
        let rows = sqlx::query!(
            "SELECT id, provider_id, secret_ciphertext, weight FROM provider_keys \
             WHERE provider_id = ? ORDER BY id",
            provider_id
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(KeyRecord {
                    id: row.id,
                    provider_id: row.provider_id,
                    secret_ciphertext: row.secret_ciphertext,
                    weight: weight(row.weight)?,
                })
            })
            .collect()
    }

    pub async fn insert_route(&self, alias: &str, fallbacks: &[String]) -> Result<(), StoreError> {
        let fallbacks = serde_json::to_string(fallbacks).map_err(decode_error)?;
        sqlx::query!(
            "INSERT INTO routes (alias, fallbacks) VALUES (?, ?)",
            alias,
            fallbacks
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn route(&self, alias: &str) -> Result<RouteRecord, StoreError> {
        let row = sqlx::query!("SELECT alias, fallbacks FROM routes WHERE alias = ?", alias)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::NotFound)?;
        Ok(RouteRecord {
            alias: row.alias,
            fallbacks: serde_json::from_str(&row.fallbacks).unwrap_or_default(),
        })
    }

    // --- Couche base de la configuration (cahier §7.1) -------------------------------------

    /// Crée ou remplace un fournisseur et **toutes** ses clés, dans une transaction.
    pub async fn save_provider(
        &self,
        provider: &ProviderRecord,
        keys: &[StoredKey],
    ) -> Result<(), StoreError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query!(
            "INSERT INTO providers (id, kind, base_url) VALUES (?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, base_url = excluded.base_url",
            provider.id,
            provider.kind,
            provider.base_url,
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query!(
            "DELETE FROM provider_keys WHERE provider_id = ?",
            provider.id
        )
        .execute(&mut *transaction)
        .await?;
        for key in keys {
            let (ciphertext, reference): (&[u8], Option<&str>) = match &key.secret {
                StoredSecret::Encrypted(blob) => (blob, None),
                StoredSecret::Reference(reference) => (&[], Some(reference)),
            };
            sqlx::query!(
                "INSERT INTO provider_keys (provider_id, secret_ciphertext, reference, weight) \
                 VALUES (?, ?, ?, ?)",
                provider.id,
                ciphertext,
                reference,
                key.weight,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Supprime un fournisseur (ses clés suivent) ; `false` s'il n'existe pas.
    pub async fn delete_provider(&self, id: &str) -> Result<bool, StoreError> {
        let result = sqlx::query!("DELETE FROM providers WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Clés d'un fournisseur, dans l'ordre d'insertion.
    pub async fn stored_keys(&self, provider_id: &str) -> Result<Vec<StoredKey>, StoreError> {
        let rows = sqlx::query!(
            "SELECT secret_ciphertext, reference, weight FROM provider_keys \
             WHERE provider_id = ? ORDER BY id",
            provider_id
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let secret = match row.reference {
                    Some(reference) => StoredSecret::Reference(reference),
                    None => StoredSecret::Encrypted(row.secret_ciphertext),
                };
                Ok(StoredKey {
                    secret,
                    weight: weight(row.weight)?,
                })
            })
            .collect()
    }

    /// Réchiffre toutes les clés chiffrées (pas les références `env:`) dans une seule
    /// transaction : rotation de la clé maître (T1.21). Tout ou rien ; rend le nombre de clés.
    pub async fn reencrypt_keys<F>(&self, reencrypt: F) -> Result<usize, StoreError>
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, String>,
    {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query!(
            "SELECT id, secret_ciphertext FROM provider_keys WHERE reference IS NULL ORDER BY id"
        )
        .fetch_all(&mut *transaction)
        .await?;
        for row in &rows {
            let fresh = reencrypt(&row.secret_ciphertext).map_err(StoreError::Rotation)?;
            sqlx::query!(
                "UPDATE provider_keys SET secret_ciphertext = ? WHERE id = ?",
                fresh,
                row.id
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(rows.len())
    }

    /// Crée ou remplace une route.
    pub async fn save_route(&self, route: &StoredRoute) -> Result<(), StoreError> {
        let fallbacks = serde_json::to_string(&route.fallbacks).map_err(decode_error)?;
        sqlx::query!(
            "INSERT INTO routes (alias, fallbacks, timeouts) VALUES (?, ?, ?) \
             ON CONFLICT(alias) DO UPDATE SET fallbacks = excluded.fallbacks, \
             timeouts = excluded.timeouts",
            route.alias,
            fallbacks,
            route.timeouts,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Supprime une route ; `false` si elle n'existe pas.
    pub async fn delete_route(&self, alias: &str) -> Result<bool, StoreError> {
        let result = sqlx::query!("DELETE FROM routes WHERE alias = ?", alias)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Toutes les routes de la couche base, triées par alias.
    pub async fn stored_routes(&self) -> Result<Vec<StoredRoute>, StoreError> {
        let rows = sqlx::query!("SELECT alias, fallbacks, timeouts FROM routes ORDER BY alias")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(StoredRoute {
                    alias: row.alias,
                    fallbacks: serde_json::from_str(&row.fallbacks).map_err(decode_error)?,
                    timeouts: row.timeouts,
                })
            })
            .collect()
    }

    pub async fn insert_request_log(&self, record: &RequestLogRecord) -> Result<(), StoreError> {
        self.insert_request_logs(std::slice::from_ref(record)).await
    }

    /// Écrit un lot du journal dans une seule transaction : tout ou rien.
    pub async fn insert_request_logs(
        &self,
        records: &[RequestLogRecord],
    ) -> Result<(), StoreError> {
        let mut transaction = self.pool.begin().await?;
        for record in records {
            let latency_ms = i64::try_from(record.latency_ms).unwrap_or(i64::MAX);
            sqlx::query!(
                "INSERT INTO request_log \
                 (id, model, provider, status, latency_ms, input_tokens, output_tokens, created_at, attempts) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                record.id,
                record.model,
                record.provider,
                record.status,
                latency_ms,
                record.input_tokens,
                record.output_tokens,
                record.created_at,
                record.attempts,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Journal filtré, trié du plus récent au plus ancien, paginé par curseur (rowid).
    ///
    /// Requête dynamique (`QueryBuilder`) : les filtres sont optionnels et combinables, ce
    /// qu'une macro vérifiée ne sait pas exprimer sans une requête par combinaison.
    pub async fn query_request_logs(
        &self,
        query: &RequestLogQuery,
    ) -> Result<Vec<RequestLogRecord>, StoreError> {
        let mut builder = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT rowid, id, model, provider, status, latency_ms, input_tokens, output_tokens, \
             created_at, attempts FROM request_log WHERE 1 = 1",
        );
        if let Some(cursor) = query.cursor {
            builder.push(" AND rowid < ").push_bind(cursor);
        }
        if let Some(provider) = &query.provider {
            builder.push(" AND provider = ").push_bind(provider.clone());
        }
        if let Some(status) = query.status {
            builder.push(" AND status = ").push_bind(i64::from(status));
        }
        if let Some(model) = &query.model {
            builder.push(" AND model = ").push_bind(model.clone());
        }
        builder
            .push(" ORDER BY rowid DESC LIMIT ")
            .push_bind(query.limit.clamp(1, 100));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(log_from_row).collect()
    }

    /// Détail d'une requête par identifiant.
    pub async fn request_log(&self, id: &str) -> Result<RequestLogRecord, StoreError> {
        let row = sqlx::query!(
            r#"SELECT rowid AS "rowid!: i64", id, model, provider, status, latency_ms,
               input_tokens, output_tokens, created_at, attempts
               FROM request_log WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        Ok(RequestLogRecord {
            id: row.id,
            model: row.model,
            provider: row.provider,
            status: u16::try_from(row.status).map_err(decode_error)?,
            latency_ms: u64::try_from(row.latency_ms).map_err(decode_error)?,
            input_tokens: u32::try_from(row.input_tokens).map_err(decode_error)?,
            output_tokens: u32::try_from(row.output_tokens).map_err(decode_error)?,
            created_at: row.created_at,
            attempts: u32::try_from(row.attempts).map_err(decode_error)?,
            rowid: row.rowid,
        })
    }

    /// Agrégats des requêtes reçues depuis `since` (secondes Unix) : nombre, erreurs (statut
    /// ≥ 400) et latences p50/p95 (rang le plus proche).
    pub async fn request_stats(&self, since: i64) -> Result<RequestStats, StoreError> {
        let counts = sqlx::query!(
            r#"SELECT COUNT(*) AS "requests!: i64", COALESCE(SUM(status >= 400), 0) AS "errors!: i64"
               FROM request_log WHERE created_at >= ?"#,
            since
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(RequestStats {
            requests: counts.requests,
            errors: counts.errors,
            latency_p50_ms: self.latency_rank(since, counts.requests, 0.50).await?,
            latency_p95_ms: self.latency_rank(since, counts.requests, 0.95).await?,
        })
    }

    /// Latence au quantile `quantile` (rang le plus proche : ⌈q × n⌉).
    async fn latency_rank(
        &self,
        since: i64,
        count: i64,
        quantile: f64,
    ) -> Result<Option<i64>, StoreError> {
        if count == 0 {
            return Ok(None);
        }
        let offset = ((quantile * count as f64).ceil() as i64 - 1).max(0);
        Ok(sqlx::query_scalar!(
            "SELECT latency_ms FROM request_log WHERE created_at >= ? \n             ORDER BY latency_ms LIMIT 1 OFFSET ?",
            since,
            offset
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn recent_request_logs(
        &self,
        limit: i64,
    ) -> Result<Vec<RequestLogRecord>, StoreError> {
        self.query_request_logs(&RequestLogQuery {
            limit,
            ..RequestLogQuery::default()
        })
        .await
    }

    /// Persiste un jeton API : seul son empreinte est stockée (T1.20).
    pub async fn insert_api_token(
        &self,
        id: &str,
        token_hash: &str,
        label: &str,
    ) -> Result<(), StoreError> {
        sqlx::query!(
            "INSERT INTO api_tokens (id, token_hash, label) VALUES (?, ?, ?)",
            id,
            token_hash,
            label
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Révoque un jeton ; `false` s'il n'existe pas.
    pub async fn revoke_api_token(&self, id: &str) -> Result<bool, StoreError> {
        let result = sqlx::query!("UPDATE api_tokens SET revoked = 1 WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Au moins un jeton API non révoqué existe (amorçage de l'authentification admin).
    pub async fn has_active_api_token(&self) -> Result<bool, StoreError> {
        let present = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM api_tokens WHERE revoked = 0) AS "present!: i64""#
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(present == 1)
    }

    /// Un jeton est actif s'il existe et n'est pas révoqué.
    pub async fn api_token_is_active(&self, token_hash: &str) -> Result<bool, StoreError> {
        let revoked = sqlx::query_scalar!(
            "SELECT revoked FROM api_tokens WHERE token_hash = ?",
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        Ok(revoked == 0)
    }
}

fn log_from_row(row: sqlx::sqlite::SqliteRow) -> Result<RequestLogRecord, StoreError> {
    Ok(RequestLogRecord {
        id: row.try_get("id")?,
        model: row.try_get("model")?,
        provider: row.try_get("provider")?,
        status: row.try_get::<i64, _>("status")? as u16,
        latency_ms: row.try_get::<i64, _>("latency_ms")? as u64,
        input_tokens: row.try_get::<i64, _>("input_tokens")? as u32,
        output_tokens: row.try_get::<i64, _>("output_tokens")? as u32,
        created_at: row.try_get("created_at")?,
        attempts: row.try_get::<i64, _>("attempts")? as u32,
        rowid: row.try_get("rowid")?,
    })
}
