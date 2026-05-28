use backstep_cloud::{
    auth::RateLimiter,
    config::ServerConfig,
    db::pool,
    metrics,
    service::{AppState, SyncServiceImpl},
    storage, CloudError,
};

use std::time::Duration;
use tonic::transport::server::ServerTlsConfig;
use tonic::transport::{Identity, Server};
use tonic_health::ServingStatus;

/// Waits for SIGINT, then returns so the server can drain in-flight RPCs.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c signal handler");
    tracing::info!("SIGINT received — initiating graceful shutdown, draining in-flight RPCs");
}

fn redact_database_url(url: &str) -> String {
    match url.find("://") {
        Some(scheme_end) => {
            let authority_start = scheme_end + 3;
            match url[authority_start..].find('@') {
                Some(relative_at) => {
                    let at_index = authority_start + relative_at;
                    match url[authority_start..at_index].rfind(':') {
                        Some(relative_colon) => {
                            let password_start = authority_start + relative_colon + 1;
                            format!("{}****{}", &url[..password_start], &url[at_index..])
                        }
                        None => url.to_string(),
                    }
                }
                None => url.to_string(),
            }
        }
        None => url.to_string(),
    }
}

async fn verify_database_health(pool: &sqlx::PgPool) -> Result<(), CloudError> {
    sqlx::query("SELECT 1").execute(pool).await.map_err(|e| {
        tracing::error!(error = %e, "database health check failed");
        CloudError::Database(e)
    })?;

    tracing::info!("database health check passed");
    Ok(())
}

fn has_ensure_database_flag() -> bool {
    std::env::args().any(|arg| arg == "--ensure-database")
}

#[tokio::main]
async fn main() -> Result<(), CloudError> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = ServerConfig::from_env()?;
    config.validate()?;

    tracing::info!(
        listen_addr = %config.listen_addr,
        "backstep-cloud starting"
    );
    tracing::info!(
        database_url = %redact_database_url(&config.database_url),
        "database target"
    );

    if has_ensure_database_flag() {
        pool::ensure_database(&config.database_url).await?;
    }

    let db_pool = pool::init_pool(&config.database_url, config.db_max_connections).await?;
    pool::run_migrations(&db_pool).await?;
    verify_database_health(&db_pool).await?;

    let pool_for_shutdown = db_pool.clone();

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

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status("backstep.sync.v1.SyncService", ServingStatus::Serving)
        .await;
    tracing::info!("health check endpoint registered: grpc.health.v1.Health/Check");

    let _metrics_handle = metrics::start_metrics_server(config.metrics_port);

    let max_message_bytes = config.max_pack_bytes as usize;

    let tls_config = match (&config.tls_cert_path, &config.tls_key_path) {
        (Some(cert_path), Some(key_path)) => {
            let cert_pem = std::fs::read_to_string(cert_path).map_err(|e| {
                CloudError::Config(format!("failed to read TLS certificate: {}", e))
            })?;
            let key_pem = std::fs::read_to_string(key_path)
                .map_err(|e| CloudError::Config(format!("failed to read TLS key: {}", e)))?;
            let identity = Identity::from_pem(cert_pem, key_pem);
            let tls = ServerTlsConfig::new().identity(identity);

            tracing::info!(
                cert_path = %cert_path,
                key_path = %key_path,
                "TLS configured"
            );
            Some(tls)
        }
        _ => {
            tracing::warn!("TLS not configured — running with plaintext transport");
            None
        }
    };

    let transport_label = if tls_config.is_some() {
        "TLS"
    } else {
        "plaintext"
    };
    tracing::info!(
        addr = %config.listen_addr,
        max_message_bytes,
        transport = transport_label,
        timeout_secs = 60,
        "listening"
    );

    let mut server = Server::builder().timeout(Duration::from_secs(60));

    if let Some(tls) = tls_config {
        server = server
            .tls_config(tls)
            .map_err(|e| CloudError::Internal(format!("TLS configuration error: {}", e)))?;
    }

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .map_err(|e| CloudError::Internal(format!("bind failed: {}", e)))?;

    server
        .add_service(health_service)
        .add_service(
            backstep_cloud::service::sync::sync_service_server::SyncServiceServer::new(svc)
                .max_decoding_message_size(max_message_bytes)
                .max_encoding_message_size(max_message_bytes),
        )
        .serve_with_incoming_shutdown(
            tokio_stream::wrappers::TcpListenerStream::new(listener),
            shutdown_signal(),
        )
        .await
        .map_err(|e| CloudError::Internal(format!("server error: {}", e)))?;

    tracing::info!("gRPC server stopped, closing database pool");
    pool_for_shutdown.close().await;
    tracing::info!("database pool closed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::redact_database_url;

    #[test]
    fn redact_database_url_hides_password() {
        let redacted =
            redact_database_url("postgres://backstep:backstep_dev@localhost:5433/backstep_cloud");

        assert_eq!(
            redacted,
            "postgres://backstep:****@localhost:5433/backstep_cloud"
        );
    }

    #[test]
    fn redact_database_url_leaves_passwordless_url_unchanged() {
        let url = "postgres://localhost:5433/backstep_cloud";

        assert_eq!(redact_database_url(url), url);
    }
}
