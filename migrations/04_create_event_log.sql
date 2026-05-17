-- V004__create_event_log.sql
-- CRDT operation log — the definitive ordered event stream per account.
-- server_clock is monotonically increasing per account (no gaps, no duplicates).
-- encrypted_metadata is opaque to the server (zero-knowledge constraint).
-- Composite PK (account_id, event_id) co-locates data per tenant.

CREATE TABLE event_log (
    event_id          BIGSERIAL NOT NULL,
    account_id        UUID NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    origin_device_id  TEXT NOT NULL,
    server_clock      BIGINT NOT NULL,
    entity_type       TEXT NOT NULL
                        CHECK (entity_type IN ('version', 'chunk', 'tombstone')),
    operation         TEXT NOT NULL
                        CHECK (operation IN ('create', 'delete')),
    entity_id         BIGINT NOT NULL,
    entity_sub_id     BIGINT NOT NULL DEFAULT 0,
    event_timestamp   TEXT NOT NULL,
    encrypted_metadata BYTEA NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (account_id, event_id),
    UNIQUE (account_id, server_clock)
);

-- PullMetadata: "give me events since clock X"
CREATE INDEX idx_event_log_since ON event_log (account_id, server_clock);

-- Per-device queries for analytics
CREATE INDEX idx_event_log_device ON event_log (account_id, origin_device_id);
