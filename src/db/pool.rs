use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn init_pool(database_url: &str, max_connections: u32) -> Result<PgPool, crate::CloudError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;

    tracing::info!(max_connections, "database pool initialized");
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), crate::CloudError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| crate::CloudError::Internal(format!("migration failed: {}", e)))?;

    tracing::info!("database migrations complete");
    Ok(())
}
