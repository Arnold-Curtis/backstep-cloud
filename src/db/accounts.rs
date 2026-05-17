use sqlx::PgPool;
use uuid::Uuid;

use crate::CloudError;

#[derive(Debug, sqlx::FromRow)]
pub struct Account {
    pub account_id: Uuid,
    pub display_name: String,
    pub plan_tier: String,
    pub is_active: bool,
}

pub async fn create_account(pool: &PgPool, display_name: &str) -> Result<Account, CloudError> {
    let account = sqlx::query_as::<_, Account>(
        "INSERT INTO accounts (display_name) VALUES ($1)
         RETURNING account_id, display_name, plan_tier, is_active",
    )
    .bind(display_name)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        account_id = %account.account_id,
        display_name = %account.display_name,
        "account created"
    );

    Ok(account)
}

pub async fn lookup_account(pool: &PgPool, account_id: Uuid) -> Result<Option<Account>, CloudError> {
    let account = sqlx::query_as::<_, Account>(
        "SELECT account_id, display_name, plan_tier, is_active FROM accounts WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?;

    Ok(account)
}
