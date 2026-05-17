-- V005__create_packs.sql
-- Pack registry tracking R2 object storage.
-- pack_id is from engine's packs.pack_id (SQLite auto-increment).
-- r2_key is the deterministic R2 object key: accounts/{account_id}/packs/{pack_id}.pack
-- state lifecycle: uploading -> ready | deleted

CREATE TABLE packs (
    pack_id      BIGINT NOT NULL,
    account_id   UUID NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    device_id    TEXT NOT NULL,
    file_name    TEXT NOT NULL,
    chunk_count  INTEGER NOT NULL,
    total_bytes  BIGINT NOT NULL,
    r2_key       TEXT NOT NULL,
    r2_etag      TEXT,
    state        TEXT NOT NULL DEFAULT 'uploading'
                   CHECK (state IN ('uploading', 'ready', 'deleted')),
    uploaded_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (account_id, pack_id)
);

CREATE INDEX idx_packs_account ON packs(account_id);
CREATE INDEX idx_packs_state ON packs(account_id, state);
