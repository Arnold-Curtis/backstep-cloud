# Contributing

## Development Setup

**Prerequisites:** Rust 1.77+, PostgreSQL 16+, Cloudflare R2 bucket (or MinIO for local dev).

```sh
git clone https://github.com/Arnold-Curtis/backstep-cloud.git
cd backstep-cloud
cp .env.example .env
# Edit .env with your DATABASE_URL, R2_* credentials
```

## Code Standards

`backstep-cloud` enforces Principal Rust Engineer standards. All code must be:

- **Idiomatic:** Use `?` operator, pattern matching, `impl` blocks. No monolithic functions. No `unwrap()`/`expect()` without a mathematical safety comment.
- **Secure:** No raw tokens in logs. Input validation at every gRPC boundary. SQL via parameterized binds only.
- **Terse:** Comments document WHY, not WHAT. No conversational filler. No emoji.
- **Audited:** Every mutation emits a structured audit log. Every handler authenticates before processing.

## Build & Test

```sh
# Check compilation
cargo check

# Lint (zero warnings)
cargo clippy -- -D warnings

# Format
cargo fmt --check

# Tests (requires DATABASE_URL pointing to a running PostgreSQL)
cargo test
```

## Pull Request Process

1. Create a feature branch from `main`: `git checkout -b feature/description`
2. Implement with tests. All existing tests must continue passing.
3. Run `cargo clippy -- -D warnings && cargo fmt --check`. Both must pass.
4. Run `cargo test`. All tests must pass.
5. Commit using conventional commits: `feat(scope): description`
6. Open a PR against `main`. Title format: `[scope] type: description`
7. Ensure CI passes (GitHub Actions runs `check`, `clippy`, `fmt`).

## Commit Convention

```
type(scope): imperative description

Types: feat, fix, refactor, test, chore, docs, perf
Scope: component name (auth, storage, service, db, config)
```

## Architecture Decision Records

Significant design decisions are documented as ADRs in the Backstep monorepo's `_MEMORY/DECISIONS.md`. When contributing a feature that changes server behavior, include an ADR in your PR or reference an existing one.

## Questions

Open a GitHub Discussion for design questions. Open an Issue for bugs or feature requests.
