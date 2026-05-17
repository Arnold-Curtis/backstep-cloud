# backstep-cloud

Zero-knowledge, end-to-end encrypted cloud synchronization engine for Backstep — a continuous local file versioning system. This server routes encrypted `.pack` files and encrypted CRDT metadata envelopes between devices. It never reads, inspects, or derives meaning from payload contents.

## Architecture

`backstep-cloud` is a blind message broker. Clients encrypt all data locally (XChaCha20-Poly1305) before transmission. The server indexes only `device_id`, `lamport_clock`, and metadata type strings — everything else is opaque bytes.

```
Device A (Laptop)                  backstep-cloud                  Device B (Desktop)
     │                                   │                               │
     │ gRPC (TLS) ──────────────────────►│                               │
     │ PushMetadata(encrypted)           │  PostgreSQL event_log         │
     │ PushPack(encrypted .pack)         │  Cloudflare R2 object store   │
     │                                   │                               │
     │                                   │ gRPC (TLS) ──────────────────►│
     │                                   │ PullMetadata(since_clock=N)   │
     │                                   │ PullPack(pack_id=7)           │
```

| Layer | Technology |
|-------|-----------|
| Transport | gRPC via tonic (HTTP/2, TLS) |
| Database | PostgreSQL via sqlx (parameterized, compile-checked) |
| Object Store | Cloudflare R2 (S3-compatible) via aws-sdk-s3 |
| Auth | SHA-256 hashed Bearer tokens — raw keys never stored or logged |
| CRDT Ordering | Server-authoritative Lamport clock — `GREATEST(server, client) + 1` |

## Security

- **Zero-knowledge:** The server never sees file paths, content hashes, or plaintext metadata. All `encrypted_metadata` fields are treated as opaque byte arrays. They are logged as `[encrypted N bytes]` only.
- **Multi-tenant isolation:** Every database query filters by `account_id`. Cross-tenant access is impossible by construction. The client never provides `account_id` — it is derived from the validated Bearer token.
- **Authentication:** API keys are 256-bit random values. Only SHA-256 hashes are stored in the database. Raw keys appear in no log, error message, or HTTP response.
- **Input validation:** All gRPC request fields are validated before touching the database or object store. `device_id` must be non-empty. `total_bytes` must not exceed the configured maximum. `since_clock` must be non-negative.
- **SQL injection:** All queries use `$1, $2` parameterized binds. User input is never interpolated into SQL strings.
- **TLS:** Mandatory in production. The server rejects non-TLS connections when not running in debug mode.

## Schema

Six PostgreSQL tables. No tables from the local SQLite schema — the server is fully blind.

| Table | Purpose |
|-------|---------|
| `accounts` | Multi-tenant boundary |
| `devices` | Auto-registered on first Handshake |
| `account_state` | Per-account Lamport clock |
| `event_log` | CRDT operation log (encrypted payloads) |
| `packs` | Pack → R2 object mapping |
| `api_keys` | SHA-256 hashed authentication tokens |

## Quick Start

### Prerequisites
- Rust 1.77+
- PostgreSQL 16+
- Cloudflare R2 bucket (or MinIO for local development)

### Setup

```sh
cp .env.example .env
# Edit .env with your DATABASE_URL, R2_ENDPOINT, R2_ACCESS_KEY_ID, etc.
```

### Run

```sh
cargo run
```

The server starts on `0.0.0.0:50051` (configurable via `LISTEN_ADDR`).

### Health Check

```
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

## Development

```sh
# Compile
cargo check

# Lint (zero-warning policy)
cargo clippy -- -D warnings

# Format
cargo fmt --check

# Run all tests (requires DATABASE_URL)
cargo test
```

## API

The gRPC service is defined in `proto/sync.proto`. Five RPCs:

| RPC | Type | Description |
|-----|------|-------------|
| `Handshake` | Unary | Device registration, clock synchronization |
| `PushMetadata` | Unary | Push one CRDT operation (encrypted) |
| `PullMetadata` | Unary | Pull operations since a given clock |
| `PushPack` | Client-streaming | Upload an encrypted `.pack` file |
| `PullPack` | Server-streaming | Download a `.pack` file |

## License

MIT
