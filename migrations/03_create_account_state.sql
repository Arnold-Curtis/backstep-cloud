-- V003__create_account_state.sql
-- Per-account Lamport clock — the server is the clock authority.
-- Serialized via SELECT FOR UPDATE during PushMetadata.

CREATE TABLE account_state (
    account_id      UUID PRIMARY KEY REFERENCES accounts(account_id) ON DELETE CASCADE,
    lamport_clock   BIGINT NOT NULL DEFAULT 0,
    last_handshake_at TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
