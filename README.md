# backstep-cloud

> Zero-knowledge, end-to-end encrypted cloud synchronization engine for Backstep. A blind message broker that routes encrypted pack files and CRDT metadata between devices without ever reading their contents.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)
[![CI](https://github.com/Arnold-Curtis/backstep-cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/Arnold-Curtis/backstep-cloud/actions/workflows/ci.yml)

---

## Table of Contents

- [Architecture](#architecture)
- [Security Model](#security-model)
- [Schema](#schema)
- [API](#api)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

---

## Architecture

`backstep-cloud` is a **blind message broker**. Clients encrypt all data locally using XChaCha20-Poly1305 with BLAKE3-derived keys (Message-Locked Encryption) before transmission. The server indexes only structural routing fields — `device_id`, `lamport_clock`, `entity_type`, and `operation`. All payload columns (`encrypted_metadata`, `.pack` files) are opaque byte sequences.

```
  Device A                             backstep-cloud                            Device B
  ┌──────────┐                         ┌──────────────┐                         ┌──────────┐
  │ Encrypt  │── PushMetadata ────────►│ PostgreSQL   │◄─── PullMetadata ───────│ Decrypt  │
  │ locally  │── PushPack    ────────►│   + R2       │◄─── PullPack    ───────│ locally  │
  │          │                         │              │                         │          │
  │ XChaCha20│    gRPC (TLS 1.3)       │ Blind routing│    gRPC (TLS 1.3)       │ XChaCha20│
  └──────────┘                         └──────────────┘                         └──────────┘
```

### Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Transport** | tonic (gRPC over HTTP/2) | Client-server communication with TLS 1.3 |
| **Database** | PostgreSQL via sqlx | Multi-tenant metadata, CRDT event log, API key storage |
| **Object Store** | Cloudflare R2 via aws-sdk-s3 | Encrypted `.pack` file storage (4-8 MB immutable blobs) |
| **Auth** | SHA-256 hashed Bearer tokens | Per-device API keys, raw keys never persisted |
| **CRDT Ordering** | Server-authoritative Lamport clock | `GREATEST(server_clock, client_clock) + 1` per mutation |
| **Observability** | tracing (JSON structured logs) | Audit trail with `audit=true` discriminator, per-request latency |

---

## Security Model

`backstep-cloud` operates under a **hostile-infrastructure** threat model: the server, database, and object store are all presumed untrusted. Security is achieved through client-side encryption and server-side discipline.

### Principles

| Principle | Implementation |
|-----------|---------------|
| **Zero-knowledge** | All `encrypted_metadata` columns are `BYTEA`. Server logs them as `[encrypted N bytes]` — never hex-dumped, never deserialized. No plaintext file paths, content hashes, or version metadata exist on the server. |
| **Multi-tenant isolation** | Every query filters by `account_id`. The `account_id` is derived from the validated Bearer token — clients cannot specify it. Cross-tenant access is structurally impossible. |
| **Token safety** | API keys are 256-bit random values (`bsk_` prefix). Only SHA-256 hashes are stored in `api_keys.key_hash`. Raw keys appear in zero logs, error messages, or HTTP responses. |
| **Input validation** | All gRPC fields are validated at the handler boundary: `device_id` length, `total_bytes` vs configured maximum, `since_clock` range, `entity_type` enum membership. Invalid input returns `INVALID_ARGUMENT` before any database or R2 access. |
| **SQL injection prevention** | All queries use `$1, $2` parameterized binds via `sqlx::query()`. User-provided strings are never interpolated into SQL. |
| **TLS enforcement** | Production deployments (`not(debug_assertions)`) require `TLS_CERT_PATH` and `TLS_KEY_PATH`. Plaintext gRPC is allowed only in local development. |
| **Audit trail** | Every mutating operation emits a structured JSON log with `account_id`, `device_id`, `server_clock`, `entity_type`, `operation`. These are `tracing::info!` events with `audit = true`. |

### What the Server Knows

| Field | Visible? | Purpose |
|-------|----------|---------|
| `device_id` | Yes | Device routing |
| `lamport_clock` | Yes | CRDT causal ordering |
| `entity_type` | Yes | `version`, `chunk`, or `tombstone` |
| `operation` | Yes | `create` or `delete` |
| `entity_id` / `entity_sub_id` | Yes | Opaque routing identifiers |
| `encrypted_metadata` | **No** | XChaCha20-Poly1305 encrypted CRDT payload |
| `.pack` file contents | **No** | XChaCha20-Poly1305 encrypted BPKP binary |

### Reporting Vulnerabilities

See [SECURITY.md](SECURITY.md). Do not open public issues for security vulnerabilities.

---

## Schema

Six PostgreSQL tables. No tables from the local Backstep SQLite schema are replicated — the server is fully blind to file-level metadata.

| Table | Primary Key | Purpose |
|-------|------------|---------|
| `accounts` | `account_id` (UUID) | Multi-tenant boundary |
| `devices` | `device_id` (TEXT) | Auto-registered on first Handshake |
| `account_state` | `account_id` (UUID) | Per-account Lamport clock authority |
| `event_log` | `(account_id, event_id)` | Immutable CRDT operation log |
| `packs` | `(account_id, pack_id)` | Pack → R2 object key mapping |
| `api_keys` | `key_id` (UUID) | SHA-256 hashed authentication tokens |

Full DDL is in [migrations/](migrations/).

---

## API

The gRPC service is defined in [proto/sync.proto](proto/sync.proto). Five RPCs:

| RPC | Type | Description |
|-----|------|-------------|
| `Handshake` | Unary | Device registration, clock synchronization |
| `PushMetadata` | Unary | Push one CRDT operation (encrypted metadata envelope) |
| `PullMetadata` | Unary | Pull remote operations since a given Lamport clock |
| `PushPack` | Client-streaming | Upload an encrypted `.pack` file (4-8 MB) |
| `PullPack` | Server-streaming | Download a `.pack` file, streamed in 64 KB chunks |

### Example: PushMetadata

```protobuf
message MetadataPushRequest {
  string device_id = 1;
  uint64 lamport_clock = 2;
  EntityType entity_type = 3;
  SyncOperation operation = 4;
  uint64 entity_id = 5;
  uint64 entity_sub_id = 6;
  string timestamp = 7;
  bytes encrypted_metadata = 8;  // XChaCha20-Poly1305 encrypted
}

message MetadataPushResponse {
  uint32 accepted_count = 1;
  uint64 server_clock = 2;       // Server-assigned Lamport clock
  string status = 3;
}
```

Server assigns `server_clock = max(current_server_clock, request.lamport_clock) + 1` within a transaction that atomically inserts the event and updates the account's clock. This guarantees monotonic, gap-free ordering even when offline devices have clock skew.

---

## Quick Start

### Prerequisites

- Rust 1.77+
- PostgreSQL 16+
- Cloudflare R2 bucket (or MinIO for local development)

### Setup

```sh
git clone https://github.com/Arnold-Curtis/backstep-cloud.git
cd backstep-cloud
cp .env.example .env
# Edit .env with your DATABASE_URL, R2_ENDPOINT, R2_ACCESS_KEY_ID, etc.
```

### Run

```sh
cargo run
```

Server listens on `0.0.0.0:50051` (configurable via `LISTEN_ADDR`).

### Health Check

```sh
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

---

## Configuration

All configuration via environment variables (or `.env`):

| Variable | Default | Description |
|----------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:50051` | gRPC listen address |
| `DATABASE_URL` | **required** | PostgreSQL connection string |
| `DB_MAX_CONNECTIONS` | `10` | sqlx pool size |
| `R2_ENDPOINT` | **required** | R2 endpoint URL |
| `R2_BUCKET` | `backstep-packs` | R2 bucket name |
| `R2_ACCESS_KEY_ID` | **required** | R2 access key |
| `R2_SECRET_ACCESS_KEY` | **required** | R2 secret key |
| `R2_REGION` | `auto` | R2 region |
| `MAX_PACK_BYTES` | `8388608` | Maximum pack file size |
| `MAX_PULL_OPERATIONS` | `100` | Max operations per PullMetadata response |
| `TLS_CERT_PATH` | *(optional)* | TLS certificate path (required in production) |
| `TLS_KEY_PATH` | *(optional)* | TLS key path (required in production) |
| `LOG_LEVEL` | `info` | tracing log level |

---

## Development

```sh
cargo check                           # Compile
cargo clippy -- -D warnings           # Lint (zero-warning policy)
cargo fmt --check                     # Format
cargo test                            # Tests (requires DATABASE_URL)
```

CI runs `check`, `clippy`, `fmt`, and `test` on every push. See [.github/workflows/ci.yml](.github/workflows/ci.yml).

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). TL;DR: conventional commits, no `unwrap()`, zero warnings, PRs require CI green.

---

## License

MIT. See [LICENSE](LICENSE).
