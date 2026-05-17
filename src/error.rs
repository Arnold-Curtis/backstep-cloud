use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloudError {
    #[error("config: {0}")]
    Config(String),

    #[error("auth: {0}")]
    Auth(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("database: {0}")]
    Database(#[from] sqlx::Error),

    #[error("storage: {0}")]
    Storage(String),

    #[error("grpc: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("internal: {0}")]
    Internal(String),
}

impl From<CloudError> for tonic::Status {
    fn from(err: CloudError) -> Self {
        match &err {
            CloudError::Auth(_) => tonic::Status::unauthenticated(err.to_string()),
            CloudError::Validation(_) => tonic::Status::invalid_argument(err.to_string()),
            CloudError::Config(_) | CloudError::Internal(_) => {
                tonic::Status::internal(err.to_string())
            }
            CloudError::Database(_) => tonic::Status::internal("database error"),
            CloudError::Storage(_) => tonic::Status::internal("storage error"),
            CloudError::Grpc(status) => status.clone(),
        }
    }
}
