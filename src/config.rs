use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub r2_endpoint: String,
    pub r2_bucket: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_region: String,
    pub max_pack_bytes: u64,
    pub max_pull_operations: u32,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub log_level: String,

    /// Port for the Prometheus /metrics HTTP endpoint.
    /// Default: 9090.
    pub metrics_port: u16,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, crate::CloudError> {
        Ok(Self {
            listen_addr: env_or("LISTEN_ADDR", "0.0.0.0:50051"),
            database_url: env_required("DATABASE_URL")?,
            db_max_connections: env_or("DB_MAX_CONNECTIONS", "10").parse().unwrap_or(10),
            r2_endpoint: env_required("R2_ENDPOINT")?,
            r2_bucket: env_required("R2_BUCKET")?,
            r2_access_key_id: env_required("R2_ACCESS_KEY_ID")?,
            r2_secret_access_key: env_required("R2_SECRET_ACCESS_KEY")?,
            r2_region: env_or("R2_REGION", "auto"),
            max_pack_bytes: env_or("MAX_PACK_BYTES", "8388608")
                .parse()
                .unwrap_or(8_388_608),
            max_pull_operations: env_or("MAX_PULL_OPERATIONS", "100").parse().unwrap_or(100),
            tls_cert_path: std::env::var("TLS_CERT_PATH").ok(),
            tls_key_path: std::env::var("TLS_KEY_PATH").ok(),
            log_level: env_or("LOG_LEVEL", "info"),
            metrics_port: env_or("METRICS_PORT", "9090").parse().unwrap_or(9090),
        })
    }

    pub fn validate(&self) -> Result<(), crate::CloudError> {
        if cfg!(not(debug_assertions))
            && (self.tls_cert_path.is_none() || self.tls_key_path.is_none())
        {
            return Err(crate::CloudError::Config(
                "TLS_CERT_PATH and TLS_KEY_PATH required for production".into(),
            ));
        }
        Ok(())
    }
}

fn env_required(key: &str) -> Result<String, crate::CloudError> {
    std::env::var(key).map_err(|_| crate::CloudError::Config(format!("{} is required", key)))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
