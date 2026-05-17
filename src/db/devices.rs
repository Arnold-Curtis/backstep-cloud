use sqlx::PgPool;
use uuid::Uuid;

use crate::CloudError;

#[derive(Debug, sqlx::FromRow)]
pub struct Device {
    pub device_id: String,
    pub account_id: Uuid,
    pub device_name: Option<String>,
    pub engine_version: Option<String>,
    pub is_active: bool,
}

/// Auto-register or update device on Handshake.
/// Uses ON CONFLICT to upsert — idempotent for repeated handshakes.
pub async fn upsert_device(
    pool: &PgPool,
    device_id: &str,
    account_id: Uuid,
    engine_version: Option<&str>,
) -> Result<(), CloudError> {
    sqlx::query(
        "INSERT INTO devices (device_id, account_id, engine_version, last_handshake_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (device_id) DO UPDATE SET
           engine_version = EXCLUDED.engine_version,
           last_handshake_at = now()",
    )
    .bind(device_id)
    .bind(account_id)
    .bind(engine_version)
    .execute(pool)
    .await?;

    tracing::info!(
        account_id = %account_id,
        device_id = %device_id,
        "device registered"
    );

    Ok(())
}

pub async fn lookup_device(
    pool: &PgPool,
    account_id: Uuid,
    device_id: &str,
) -> Result<Option<Device>, CloudError> {
    sqlx::query_as::<_, Device>(
        "SELECT device_id, account_id, device_name, engine_version, is_active
         FROM devices WHERE account_id = $1 AND device_id = $2",
    )
    .bind(account_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await
    .map_err(CloudError::from)
}
