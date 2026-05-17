use sqlx::PgPool;
use uuid::Uuid;

use crate::CloudError;

#[derive(Debug, sqlx::FromRow)]
pub struct PackRecord {
    pub pack_id: i64,
    pub account_id: Uuid,
    pub device_id: String,
    pub file_name: String,
    pub chunk_count: i32,
    pub total_bytes: i64,
    pub r2_key: String,
    pub r2_etag: Option<String>,
    pub state: String,
}

pub async fn insert_pack(pool: &PgPool, record: &PackRecord) -> Result<(), CloudError> {
    sqlx::query(
        "INSERT INTO packs
         (pack_id, account_id, device_id, file_name, chunk_count, total_bytes, r2_key, r2_etag, state, uploaded_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())",
    )
    .bind(record.pack_id)
    .bind(record.account_id)
    .bind(&record.device_id)
    .bind(&record.file_name)
    .bind(record.chunk_count)
    .bind(record.total_bytes)
    .bind(&record.r2_key)
    .bind(&record.r2_etag)
    .bind(&record.state)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn lookup_pack(
    pool: &PgPool,
    account_id: Uuid,
    pack_id: i64,
) -> Result<Option<PackRecord>, CloudError> {
    sqlx::query_as::<_, PackRecord>(
        "SELECT pack_id, account_id, device_id, file_name, chunk_count, total_bytes,
                r2_key, r2_etag, state
         FROM packs WHERE account_id = $1 AND pack_id = $2",
    )
    .bind(account_id)
    .bind(pack_id)
    .fetch_optional(pool)
    .await
    .map_err(CloudError::from)
}

pub async fn update_pack_ready(
    pool: &PgPool,
    account_id: Uuid,
    pack_id: i64,
    etag: &str,
) -> Result<(), CloudError> {
    sqlx::query(
        "UPDATE packs SET state = 'ready', r2_etag = $1, uploaded_at = now()
         WHERE account_id = $2 AND pack_id = $3",
    )
    .bind(etag)
    .bind(account_id)
    .bind(pack_id)
    .execute(pool)
    .await?;

    Ok(())
}
