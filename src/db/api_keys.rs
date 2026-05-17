use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::CloudError;

#[derive(Debug, sqlx::FromRow)]
pub struct ApiKeyRecord {
    pub key_id: Uuid,
    pub account_id: Uuid,
    pub key_prefix: String,
    pub label: Option<String>,
    pub is_revoked: bool,
}

pub fn hash_key(raw_key: &str) -> (Vec<u8>, String) {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let hash = hasher.finalize().to_vec();
    let prefix = raw_key.chars().take(8).collect::<String>();
    (hash, prefix)
}

pub async fn create_api_key(
    pool: &PgPool,
    account_id: Uuid,
    raw_key: &str,
    label: Option<&str>,
) -> Result<ApiKeyRecord, CloudError> {
    let (key_hash, key_prefix) = hash_key(raw_key);

    let record = sqlx::query_as::<_, ApiKeyRecord>(
        "INSERT INTO api_keys (account_id, key_hash, key_prefix, label)
         VALUES ($1, $2, $3, $4)
         RETURNING key_id, account_id, key_prefix, label, is_revoked",
    )
    .bind(account_id)
    .bind(&key_hash[..])
    .bind(&key_prefix)
    .bind(label)
    .fetch_one(pool)
    .await?;

    Ok(record)
}

pub async fn validate_api_key(
    pool: &PgPool,
    raw_key: &str,
) -> Result<Option<(Uuid, Uuid)>, CloudError> {
    let (key_hash, _) = hash_key(raw_key);

    #[derive(sqlx::FromRow)]
    struct ValidationRow {
        key_id: Uuid,
        account_id: Uuid,
        is_revoked: bool,
    }

    let row: Option<ValidationRow> = sqlx::query_as::<_, ValidationRow>(
        "SELECT key_id, account_id, is_revoked FROM api_keys WHERE key_hash = $1",
    )
    .bind(&key_hash[..])
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) if !r.is_revoked => {
            sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE key_id = $1")
                .bind(r.key_id)
                .execute(pool)
                .await?;

            Ok(Some((r.key_id, r.account_id)))
        }
        Some(_) => Ok(None),
        None => Ok(None),
    }
}
