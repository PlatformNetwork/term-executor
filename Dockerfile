# Build stage
FROM rust:1.83-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git curl wget \
    python3 python3-pip python3-venv \
    nodejs npm \
    build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install common test tools
RUN python3 -m pip install --break-system-packages pytest pytest-cov \
    && npm install -g tsx jest vitest

# Install Go
RUN curl -fsSL https://go.dev/dl/go1.22.5.linux-amd64.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

COPY --from=builder /build/target/release/term-executor /usr/local/bin/term-executor

RUN mkdir -p /tmp/sessions

ENV PORT=8080
ENV RUST_LOG=info,term_executor=debug

EXPOSE 8080

CMD ["term-executor"]
