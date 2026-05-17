use crate::CloudError;

pub fn validate_device_id(s: &str) -> Result<(), CloudError> {
    if s.is_empty() {
        return Err(CloudError::Validation("device_id must be non-empty".into()));
    }
    if s.len() > 256 {
        return Err(CloudError::Validation("device_id exceeds 256 characters".into()));
    }
    Ok(())
}

pub fn validate_pack_size(bytes: u64, max: u64) -> Result<(), CloudError> {
    if bytes > max {
        return Err(CloudError::Validation(format!(
            "pack size {} exceeds maximum {}",
            bytes, max
        )));
    }
    Ok(())
}

pub fn validate_since_clock(since: u64) -> Result<(), CloudError> {
    if since > i64::MAX as u64 {
        return Err(CloudError::Validation("since_clock exceeds server maximum".into()));
    }
    Ok(())
}

pub fn validate_entity_type(s: &str) -> Result<(), CloudError> {
    match s {
        "version" | "chunk" | "tombstone" => Ok(()),
        _ => Err(CloudError::Validation(format!(
            "invalid entity_type: {}",
            s
        ))),
    }
}

pub fn validate_operation(s: &str) -> Result<(), CloudError> {
    match s {
        "create" | "delete" => Ok(()),
        _ => Err(CloudError::Validation(format!(
            "invalid operation: {}",
            s
        ))),
    }
}
