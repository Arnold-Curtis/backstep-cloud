use sqlx::PgPool;
use tonic::metadata::MetadataMap;
use uuid::Uuid;

use crate::auth::tokens::extract_bearer_token;
use crate::db::api_keys;
use crate::CloudError;

/// Authenticated request context injected into handler logic.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub account_id: Uuid,
    pub key_id: Uuid,
}

/// Validate the Bearer token from gRPC metadata and return RequestContext.
/// Called at the start of every gRPC handler.
/// Returns CloudError::Auth if the token is missing, invalid, or revoked.
pub async fn authenticate(
    metadata: &MetadataMap,
    pool: &PgPool,
) -> Result<RequestContext, CloudError> {
    let token = metadata
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_bearer_token)
        .ok_or_else(|| CloudError::Auth("missing or malformed authorization header".into()))?;

    let (key_id, account_id) = api_keys::validate_api_key(pool, token)
        .await?
        .ok_or_else(|| CloudError::Auth("invalid or revoked API key".into()))?;

    Ok(RequestContext { account_id, key_id })
}
