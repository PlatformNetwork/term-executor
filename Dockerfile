# ── Build stage ──
FROM rust:1.93-slim-bookworm AS builder
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release && strip target/release/term-executor

# ── Runtime stage ──
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/term-executor /usr/local/bin/
RUN mkdir -p /tmp/sessions
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
CMD ["term-executor"]
