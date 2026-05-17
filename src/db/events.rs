use sqlx::PgPool;
use uuid::Uuid;

use crate::CloudError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EventRecord {
    pub origin_device_id: String,
    pub server_clock: i64,
    pub entity_type: String,
    pub operation: String,
    pub entity_id: i64,
    pub entity_sub_id: i64,
    pub event_timestamp: String,
    pub encrypted_metadata: Vec<u8>,
}

/// Insert event within an active transaction.
/// Caller manages the transaction for atomicity with clock increment.
pub async fn insert_event<'e, E>(
    executor: E,
    account_id: Uuid,
    record: &EventRecord,
) -> Result<(), CloudError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO event_log
         (account_id, origin_device_id, server_clock, entity_type, operation,
          entity_id, entity_sub_id, event_timestamp, encrypted_metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(account_id)
    .bind(&record.origin_device_id)
    .bind(record.server_clock)
    .bind(&record.entity_type)
    .bind(&record.operation)
    .bind(record.entity_id)
    .bind(record.entity_sub_id)
    .bind(&record.event_timestamp)
    .bind(&record.encrypted_metadata[..])
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn query_since_clock(
    pool: &PgPool,
    account_id: Uuid,
    since_clock: i64,
    limit: i64,
) -> Result<(Vec<EventRecord>, bool), CloudError> {
    // Fetch limit+1 to detect has_more without a count query.
    let rows: Vec<EventRecord> = sqlx::query_as::<_, EventRecord>(
        "SELECT origin_device_id, server_clock, entity_type, operation,
                entity_id, entity_sub_id, event_timestamp, encrypted_metadata
         FROM event_log
         WHERE account_id = $1 AND server_clock > $2
         ORDER BY server_clock ASC
         LIMIT $3",
    )
    .bind(account_id)
    .bind(since_clock)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let records = if has_more {
        rows[..limit as usize].to_vec()
    } else {
        rows
    };

    Ok((records, has_more))
}
