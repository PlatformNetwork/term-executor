# ── Build stage ──
FROM rust:1.93-slim-bookworm AS builder
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    pkg-config libssl-dev protobuf-compiler cmake clang mold && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release && strip target/release/term-executor

# ── Runtime stage ──
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl unzip libssl3 libssl-dev pkg-config \
    python3 python3-pip python3-venv \
    build-essential nodejs npm \
    golang-go \
    default-jdk maven \
    && ln -sf /usr/bin/python3 /usr/bin/python \
    && npm install -g corepack yarn pnpm \
    && corepack enable \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && rm -rf /var/lib/apt/lists/*
ENV PATH="/root/.cargo/bin:${PATH}"
COPY --from=builder /build/target/release/term-executor /usr/local/bin/
RUN mkdir -p /tmp/sessions
ENV IMAGE_NAME=platformnetwork/term-executor
ENV IMAGE_DIGEST=""
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
ENTRYPOINT ["/usr/local/bin/term-executor"]
