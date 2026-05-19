# ── Builder Stage ─────────────────────────────────────────────
FROM rust:1.77-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ proto/
COPY src/ src/
COPY migrations/ migrations/

RUN cargo build --release --bin backstep-cloud

# ── Runtime Stage ─────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backstep-cloud /usr/local/bin/backstep-cloud
COPY migrations/ /etc/backstep-cloud/migrations/

EXPOSE 50051

ENTRYPOINT ["/usr/local/bin/backstep-cloud"]
