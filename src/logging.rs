use uuid::Uuid;

/// Audit log for mutating operations.
/// These are NOT debug logs — they are the authoritative audit trail.
/// Every mutation (PushMetadata, PushPack) must emit an audit event.

pub fn audit_mutation(
    account_id: Uuid,
    device_id: &str,
    operation: &str,
    entity_type: &str,
    entity_id: i64,
    server_clock: i64,
) {
    tracing::info!(
        audit = true,
        event = operation,
        account_id = %account_id,
        device_id = %device_id,
        entity_type = %entity_type,
        entity_id = entity_id,
        server_clock = server_clock,
        "audit: mutation"
    );
}

pub fn audit_access(account_id: Uuid, device_id: &str, operation: &str) {
    tracing::info!(
        audit = true,
        event = operation,
        account_id = %account_id,
        device_id = %device_id,
        "audit: access"
    );
}

/// Log encrypted payload size only — never the bytes themselves.
/// Zero-knowledge constraint: the server must remain blind to content.
pub fn encrypted_payload_size(bytes: &[u8]) -> String {
    format!("[encrypted {} bytes]", bytes.len())
}
