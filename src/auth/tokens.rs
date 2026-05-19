use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a cryptographically random API key.
/// Format: bsk_{32_hex}_{32_hex} = 256 bits of entropy.
pub fn generate_api_key() -> String {
    let mut rng = rand::thread_rng();
    // 16 bytes per part → 32 hex chars each → 256 bits total entropy.
    let part1: String = (0..16)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect();
    let part2: String = (0..16)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect();
    format!("bsk_{}_{}", part1, part2)
}

/// Hash an API key for storage. Returns (hash_bytes, prefix_for_display).
/// SHA-256 is sufficient for API key hashing (not password hashing — no rainbow table risk).
pub fn hash_key(raw_key: &str) -> (Vec<u8>, String) {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let hash = hasher.finalize().to_vec();
    let prefix = raw_key.chars().take(12).collect::<String>();
    (hash, prefix)
}

/// Extract the Bearer token from an authorization header value.
/// Returns None if the header is missing, empty, or not Bearer format.
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    let auth = auth_header.trim();
    if auth.len() <= 7 || !auth[..7].eq_ignore_ascii_case("bearer ") {
        return None;
    }
    let token = auth[7..].trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_correct_format() {
        let key = generate_api_key();
        assert!(key.starts_with("bsk_"));
        assert_eq!(key.len(), 4 + 32 + 1 + 32); // bsk_ + 32hex + _ + 32hex = 69 chars
    }

    #[test]
    fn hash_produces_different_values() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        let (h1, _) = hash_key(&key1);
        let (h2, _) = hash_key(&key2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn extract_bearer_token_valid() {
        let token = extract_bearer_token("Bearer abc123def456");
        assert_eq!(token, Some("abc123def456"));
    }

    #[test]
    fn extract_bearer_token_case_insensitive() {
        let token = extract_bearer_token("bearer abc123def456");
        assert_eq!(token, Some("abc123def456"));
    }

    #[test]
    fn extract_bearer_token_missing() {
        assert_eq!(extract_bearer_token(""), None);
        assert_eq!(extract_bearer_token("NotBearer xyz"), None);
        assert_eq!(extract_bearer_token("Bearer "), None);
    }
}
