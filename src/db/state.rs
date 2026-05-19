use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::CloudError;

/// Returns the current Lamport clock for an account without locking.
pub async fn get_clock(pool: &PgPool, account_id: Uuid) -> Result<i64, CloudError> {
    let result: Option<(i64,)> =
        sqlx::query_as("SELECT lamport_clock FROM account_state WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(pool)
            .await?;

    Ok(result.map(|r| r.0).unwrap_or(0))
}

/// Atomically increments the Lamport clock within a transaction.
/// Uses GREATEST(server_clock, client_clock) + 1 per Lamport clock semantics.
/// Caller must manage the transaction boundary (BEGIN/COMMIT).
pub async fn increment_clock(
    tx: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
    client_clock: i64,
) -> Result<i64, CloudError> {
    // Lock the row to serialize per-account clock updates.
    let current: Option<(i64,)> =
        sqlx::query_as("SELECT lamport_clock FROM account_state WHERE account_id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut **tx)
            .await?;

    let current_clock = current.map(|r| r.0).unwrap_or(0);

    // Lamport clock: max(server, client) + 1 ensures monotonicity
    // regardless of clock skew from offline devices.
    let new_clock = std::cmp::max(current_clock, client_clock) + 1;

    sqlx::query(
        "INSERT INTO account_state (account_id, lamport_clock, updated_at)
         VALUES ($1, $2, now())
         ON CONFLICT (account_id) DO UPDATE SET
           lamport_clock = EXCLUDED.lamport_clock,
           updated_at = now()",
    )
    .bind(account_id)
    .bind(new_clock)
    .execute(&mut **tx)
    .await?;

    Ok(new_clock)
}
