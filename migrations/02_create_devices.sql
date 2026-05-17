-- V002__create_devices.sql
-- Devices auto-register on first Handshake.
-- device_id is the UUID v4 from the engine's device_info table.
-- One device_id can only belong to one account.

CREATE TABLE devices (
    device_id    TEXT PRIMARY KEY,
    account_id   UUID NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    device_name  TEXT,
    engine_version TEXT,
    last_handshake_at TIMESTAMPTZ,
    last_sync_at TIMESTAMPTZ,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_devices_account ON devices(account_id);
