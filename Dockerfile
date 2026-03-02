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

# Pre-install all common runtimes and tools at build time (as root)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl wget unzip libssl3 libssl-dev pkg-config sudo \
    python3 python3-pip python3-venv python3-dev \
    build-essential gcc g++ make cmake autoconf automake libtool \
    default-jdk maven gradle \
    ruby ruby-dev \
    libffi-dev libxml2-dev libxslt1-dev zlib1g-dev libyaml-dev \
    libreadline-dev libncurses-dev libgdbm-dev libdb-dev \
    sqlite3 libsqlite3-dev postgresql-client libpq-dev \
    imagemagick libmagickwand-dev \
    jq \
    && ln -sf /usr/bin/python3 /usr/bin/python \
    && rm -rf /var/lib/apt/lists/*

# Install Go 1.23 (Debian bookworm ships 1.19 which is too old for most projects)
RUN curl -fsSL https://go.dev/dl/go1.23.6.linux-amd64.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

# Install Node.js 20 LTS via NodeSource (Debian bookworm ships Node 18)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g corepack yarn pnpm \
    && corepack enable \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create non-root 'agent' user to run the executor and all agent code.
# Basilica containers have no_new_privs, so sudo is unavailable at runtime.
# All system deps must be pre-installed above (as root during build).
RUN useradd -m -s /bin/bash agent \
    && cp -r /root/.cargo /home/agent/.cargo \
    && chown -R agent:agent /home/agent/.cargo \
    && mkdir -p /home/agent/.local/bin \
    && chown -R agent:agent /home/agent

COPY --from=builder /build/target/release/term-executor /usr/local/bin/
RUN mkdir -p /tmp/sessions && chown agent:agent /tmp/sessions

USER agent
ENV HOME=/home/agent
ENV PATH="/home/agent/.cargo/bin:/home/agent/.local/bin:/usr/local/go/bin:${PATH}"
ENV GOPATH="/home/agent/go"
ENV IMAGE_NAME=platformnetwork/term-executor
ENV IMAGE_DIGEST=""
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1
ENTRYPOINT ["/usr/local/bin/term-executor"]
