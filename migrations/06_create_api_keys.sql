-- V006__create_api_keys.sql
-- API key storage — only SHA-256 hashes stored, never raw keys.
-- key_prefix (first 8 chars) for UI display only.
-- Unique constraint on (account_id, key_hash) prevents duplicate keys.

CREATE TABLE api_keys (
    key_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id    UUID NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    key_hash      BYTEA NOT NULL,
    key_prefix    TEXT NOT NULL,
    label         TEXT,
    last_used_at  TIMESTAMPTZ,
    is_revoked    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_account ON api_keys(account_id) WHERE is_revoked = FALSE;
