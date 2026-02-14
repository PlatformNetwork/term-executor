FROM rust:1.93-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl wget procps \
    python3 python3-pip python3-venv \
    nodejs npm \
    build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN python3 -m pip install --break-system-packages \
    pytest pytest-cov pytest-xdist unittest2 nose2 tox

RUN npm install -g tsx jest vitest mocha

RUN curl -fsSL https://go.dev/dl/go1.22.5.linux-amd64.tar.gz | tar -C /usr/local -xz

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/usr/local/go/bin:/root/.cargo/bin:${PATH}"

COPY --from=builder /build/target/release/term-executor /usr/local/bin/

RUN mkdir -p /tmp/sessions

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["term-executor"]
