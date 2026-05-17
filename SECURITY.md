# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in `backstep-cloud`, do **not** open a public issue. Send a detailed report to the Backstep security team at:

**security@backstep.cloud**

Include:
- A description of the vulnerability and its impact
- Steps to reproduce, including environment details (OS, Rust version, PostgreSQL version)
- If possible, a proof-of-concept or patch

Response timeline:
- **48 hours:** Acknowledgment of receipt
- **7 days:** Initial assessment and severity classification
- **30 days:** Patch released, with coordination for downstream deployments

We follow responsible disclosure. We request you not publicly disclose the vulnerability until a patch has been available for at least 48 hours.

## Threat Model

`backstep-cloud` is designed under the following adversarial assumptions:

| Assumption | Implication |
|-----------|-------------|
| **The server is untrusted.** Clients encrypt all data (XChaCha20-Poly1305) before transmission. The server never possesses plaintext keys or data. | Even full server compromise reveals no file contents, paths, or metadata. |
| **The database is untrusted.** All `encrypted_metadata` columns are opaque byte arrays. The server indexes only structural fields (`device_id`, `lamport_clock`, `entity_type`). | A database dump reveals only encrypted blobs and routing information. |
| **The object store is untrusted.** `.pack` files stored in R2 are encrypted BPKP binaries. The server never reads pack contents beyond counting bytes for integrity checks. | An R2 breach reveals only encrypted binary blobs. |
| **TLS is the transport baseline.** All gRPC traffic must be encrypted. Plaintext is allowed only in local development (`debug_assertions`). | Man-in-the-middle attacks are mitigated by standard TLS 1.3. |
| **API keys are bearer credentials.** They must be treated as secrets. Raw keys are never stored (SHA-256 hash only) and never logged. | Key compromise requires key rotation — create new key, revoke old. |

## Security Properties

- **Zero-knowledge:** The server cannot derive plaintext from `encrypted_metadata` or `.pack` files without the client's encryption keys. Encryption is Message-Locked (MLE) via XChaCha20-Poly1305 with BLAKE3-derived keys.
- **Multi-tenant isolation:** Database queries filter by `account_id` on every operation. The `account_id` is derived from the authenticated Bearer token — the client cannot specify it.
- **SQL injection prevention:** All queries use `$1, $2` parameterized binds. User-provided strings are never interpolated into SQL.
- **Input validation:** All gRPC request fields are validated before touching the database or object store. Field length limits, type checks, and range constraints are enforced at the handler boundary.
- **Audit trail:** Every mutating operation (PushMetadata, PushPack) emits a structured audit log entry with `account_id`, `device_id`, `server_clock`, `entity_type`, and `operation`. These are JSON-formatted `tracing::info!` events with `audit = true` discriminator.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Dependencies

We monitor dependencies for known vulnerabilities. A `cargo audit` run is part of CI. Dependencies are pinned by minor version in `Cargo.toml`.
