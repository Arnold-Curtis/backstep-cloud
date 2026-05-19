use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use uuid::Uuid;

use crate::CloudError;

/// R2 object key template: accounts/{account_id}/packs/{pack_id}.pack
fn pack_key(account_id: Uuid, pack_id: i64) -> String {
    format!("accounts/{}/packs/{}.pack", account_id, pack_id)
}

/// S3-compatible client (Cloudflare R2).
/// Initialized with the configured endpoint, credentials, and region.
pub async fn init_client(config: &crate::config::ServerConfig) -> Result<Client, CloudError> {
    let credentials = aws_credential_types::Credentials::new(
        &config.r2_access_key_id,
        &config.r2_secret_access_key,
        None,
        None,
        "backstep-cloud",
    );

    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .endpoint_url(&config.r2_endpoint)
        .region(aws_config::Region::new(config.r2_region.clone()))
        .credentials_provider(credentials)
        .load()
        .await;

    let client = Client::new(&sdk_config);

    tracing::info!(
        endpoint = %config.r2_endpoint,
        bucket = %config.r2_bucket,
        "R2 client initialized"
    );

    Ok(client)
}

/// Upload pack bytes to R2.
/// Uses PutObject with the deterministic key scheme.
pub async fn upload_pack(
    client: &Client,
    bucket: &str,
    account_id: Uuid,
    pack_id: i64,
    bytes: Vec<u8>,
) -> Result<String, CloudError> {
    let key = pack_key(account_id, pack_id);
    let len = bytes.len();

    let body = ByteStream::from(bytes);

    let output = client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(body)
        .send()
        .await
        .map_err(|e| CloudError::Storage(format!("R2 PutObject failed: {}", e)))?;

    let etag = output.e_tag().map(|s| s.to_string()).unwrap_or_default();

    tracing::info!(
        key = %key,
        bytes = len,
        etag = %etag,
        "R2 pack uploaded"
    );

    Ok(etag)
}

/// Download pack bytes from R2.
/// Returns the full pack file as Vec<u8>.
pub async fn download_pack(
    client: &Client,
    bucket: &str,
    account_id: Uuid,
    pack_id: i64,
) -> Result<Vec<u8>, CloudError> {
    let key = pack_key(account_id, pack_id);

    let output = client
        .get_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| CloudError::Storage(format!("R2 GetObject failed: {}", e)))?;

    let bytes = output
        .body
        .collect()
        .await
        .map_err(|e| CloudError::Storage(format!("R2 stream read failed: {}", e)))?
        .into_bytes()
        .to_vec();

    tracing::info!(
        key = %key,
        bytes = bytes.len(),
        "R2 pack downloaded"
    );

    Ok(bytes)
}

/// Delete a pack from R2.
pub async fn delete_pack(
    client: &Client,
    bucket: &str,
    account_id: Uuid,
    pack_id: i64,
) -> Result<(), CloudError> {
    let key = pack_key(account_id, pack_id);

    client
        .delete_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| CloudError::Storage(format!("R2 DeleteObject failed: {}", e)))?;

    tracing::info!(key = %key, "R2 pack deleted");
    Ok(())
}
