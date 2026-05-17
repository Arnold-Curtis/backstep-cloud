use backstep_cloud::{
    auth::RateLimiter,
    config::ServerConfig,
    db::pool,
    service::{AppState, SyncServiceImpl},
    storage,
    CloudError,
};

use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), CloudError> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = ServerConfig::from_env()?;
    config.validate()?;

    tracing::info!(
        listen_addr = %config.listen_addr,
        "backstep-cloud starting"
    );

    let db_pool = pool::init_pool(&config.database_url, config.db_max_connections).await?;
    pool::run_migrations(&db_pool).await?;

    let r2_client = storage::init_client(&config).await?;

    let rate_limiter = RateLimiter::new(50);

    let state = AppState {
        pool: db_pool,
        r2_client,
        r2_bucket: config.r2_bucket.clone(),
        rate_limiter,
        max_pack_bytes: config.max_pack_bytes,
        max_pull_operations: config.max_pull_operations,
    };

    let svc = SyncServiceImpl::new(state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .map_err(|e| CloudError::Internal(format!("bind failed: {}", e)))?;

    tracing::info!(addr = %config.listen_addr, "listening");

    Server::builder()
        .add_service(backstep_cloud::service::sync::sync_service_server::SyncServiceServer::new(svc))
        .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
        .await
        .map_err(|e| CloudError::Internal(format!("server error: {}", e)))?;

    Ok(())
}
