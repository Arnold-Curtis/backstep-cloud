use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use std::str::FromStr;
use std::time::Duration;

const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(60);
const ENSURE_DATABASE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn ensure_database(database_url: &str) -> Result<(), crate::CloudError> {
    let database_name = database_name_from_url(database_url)?;
    validate_database_name(&database_name)?;

    let maintenance_options = PgConnectOptions::from_str(database_url)
        .map_err(|e| crate::CloudError::Config(format!("invalid DATABASE_URL: {}", e)))?
        .database("postgres");

    tracing::info!(
        database_name = %database_name,
        connect_timeout_s = ENSURE_DATABASE_CONNECT_TIMEOUT.as_secs(),
        "ensuring PostgreSQL database exists"
    );

    let mut connection = tokio::time::timeout(
        ENSURE_DATABASE_CONNECT_TIMEOUT,
        PgConnection::connect_with(&maintenance_options),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "timed out connecting to PostgreSQL maintenance database");
        crate::CloudError::Internal(format!(
            "timed out connecting to PostgreSQL maintenance database after {} seconds",
            ENSURE_DATABASE_CONNECT_TIMEOUT.as_secs()
        ))
    })?
    .map_err(|e| {
        tracing::error!(error = %e, "failed to connect to PostgreSQL maintenance database");
        crate::CloudError::Database(e)
    })?;

    let exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(&database_name)
        .fetch_optional(&mut connection)
        .await?;

    if exists.is_some() {
        tracing::info!(database_name = %database_name, "database already exists");
        return Ok(());
    }

    let statement = format!("CREATE DATABASE {}", quote_identifier(&database_name));
    sqlx::query(&statement).execute(&mut connection).await?;
    tracing::info!(database_name = %database_name, "database created");
    Ok(())
}

pub async fn init_pool(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, crate::CloudError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                pool_acquire_timeout_s = POOL_ACQUIRE_TIMEOUT.as_secs(),
                "database connection failed; verify DATABASE_URL and PostgreSQL availability"
            );
            crate::CloudError::Database(e)
        })?;

    tracing::info!(
        max_connections,
        pool_acquire_timeout_s = POOL_ACQUIRE_TIMEOUT.as_secs(),
        "database pool initialized"
    );
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

fn database_name_from_url(database_url: &str) -> Result<String, crate::CloudError> {
    let scheme_end = database_url.find("://").ok_or_else(|| {
        crate::CloudError::Config("DATABASE_URL must include a URL scheme".into())
    })?;
    let authority_start = scheme_end + 3;
    let path_start = database_url[authority_start..]
        .find('/')
        .map(|offset| authority_start + offset + 1)
        .ok_or_else(|| {
            crate::CloudError::Config("DATABASE_URL must include a database name".into())
        })?;
    let path_end = database_url[path_start..]
        .find(['?', '#'])
        .map(|offset| path_start + offset)
        .unwrap_or(database_url.len());
    let database_name = &database_url[path_start..path_end];

    if database_name.is_empty() {
        return Err(crate::CloudError::Config(
            "DATABASE_URL must include a database name".into(),
        ));
    }

    Ok(database_name.to_string())
}

fn validate_database_name(database_name: &str) -> Result<(), crate::CloudError> {
    let valid = database_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(crate::CloudError::Config(
            "DATABASE_URL database name may only contain ASCII letters, digits, and underscores"
                .into(),
        ))
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::{database_name_from_url, quote_identifier, validate_database_name};

    #[test]
    fn extracts_database_name_from_postgres_url() {
        let name = database_name_from_url(
            "postgres://backstep:backstep_dev@localhost:5433/backstep_cloud?sslmode=disable",
        );

        let database_name = match name {
            Ok(database_name) => database_name,
            Err(error) => panic!("unexpected database name parse error: {}", error),
        };

        assert_eq!(database_name, "backstep_cloud");
    }

    #[test]
    fn rejects_database_name_with_unsafe_characters() {
        let result = validate_database_name("backstep-cloud");

        assert!(result.is_err());
    }

    #[test]
    fn quotes_valid_database_identifier() {
        assert_eq!(quote_identifier("backstep_cloud"), "\"backstep_cloud\"");
    }
}
