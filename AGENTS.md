# AGENTS.md — term-executor

## Project Purpose

**term-executor** is a remote evaluation executor for the [term-challenge](https://github.com/PlatformNetwork/term-challenge) platform. It runs as a containerized Rust service on [Basilica](https://basilica.ai) that receives batch task archives via multipart upload, executes agent code against cloned task repositories, runs validation test scripts, and reports pass/fail results with aggregate rewards. It is the core compute backend that evaluates AI agent coding challenges.

## Architecture Overview

This is a **single-crate Rust binary** (`term-executor`) built with Axum. There are no sub-crates or workspaces.

### Data Flow

```
Validator → POST /submit (multipart archive) → term-executor
  1. Authenticate via X-Hotkey, X-Nonce, X-Signature headers
  2. Verify hotkey is in the dynamic validator whitelist (Bittensor netuid 100, >10k TAO stake)
  3. Compute SHA-256 hash of archive bytes
  4. Record vote in ConsensusManager
  5. If <50% of whitelisted validators have voted for this hash:
     → Return 202 Accepted with pending_consensus status
  6. If ≥50% consensus reached:
     a. Extract uploaded archive (zip/tar.gz) containing tasks/ and agent_code/
     b. Parse each task: workspace.yaml, prompt.md, tests/
     c. For each task (concurrently, up to limit):
        i.   git clone the target repository at base_commit
        ii.  Run install commands (pip install, etc.)
        iii. Write & execute agent code in the repo
        iv.  Write test source files into the repo
        v.   Run test scripts (bash), collect exit codes
     d. Aggregate results (reward per task, aggregate reward)
     e. Stream progress via WebSocket (GET /ws?batch_id=...)
     f. Return results via GET /batch/{id}
```

### Background Tasks

```
ValidatorWhitelist refresh loop (every 5 minutes):
  1. Connect to Bittensor subtensor via BittensorClient::with_failover()
  2. Sync metagraph for netuid 100
  3. Filter validators: validator_permit && active && stake >= 10,000 TAO
  4. Atomically replace whitelist with new set of SS58 hotkeys
  5. On failure: retry up to 3 times with exponential backoff, keep cached whitelist

ConsensusManager reaper loop (every 30 seconds):
  1. Remove pending consensus entries older than TTL (default 60s)
```

### Module Map

| File | Responsibility |
|---|---|
| `src/main.rs` | Entry point — bootstraps config, session manager, executor, validator whitelist, consensus manager, Axum server, background tasks |
| `src/config.rs` | `Config` struct loaded from environment variables with defaults; Bittensor and consensus configuration |
| `src/handlers.rs` | Axum route handlers: `/health`, `/status`, `/metrics`, `/submit`, `/batch/{id}`, `/batch/{id}/tasks`, `/batch/{id}/task/{task_id}`, `/batches` |
| `src/auth.rs` | Authentication: `extract_auth_headers()`, `verify_request()` (whitelist-based), `validate_ss58()`, sr25519 signature verification via `verify_sr25519_signature()`, `NonceStore` for replay protection, `AuthHeaders`/`AuthError` types |
| `src/validator_whitelist.rs` | Dynamic validator whitelist — fetches validators from Bittensor netuid 100 every 5 minutes, filters by stake ≥10k TAO, stores SS58 hotkeys in `parking_lot::RwLock<HashSet>` |
| `src/consensus.rs` | 50% consensus manager — tracks pending votes per archive hash in `DashMap`, triggers evaluation when ≥50% of whitelisted validators submit same payload, TTL reaper for expired entries |
| `src/executor.rs` | Core evaluation engine — spawns batch tasks that clone repos, run agents, run tests concurrently |
| `src/session.rs` | `SessionManager` with `DashMap`, `Batch`, `BatchResult`, `TaskResult`, `BatchStatus`, `TaskStatus`, `WsEvent` types |
| `src/task.rs` | Archive extraction (zip/tar.gz), task directory parsing, agent code loading, language detection |
| `src/metrics.rs` | Atomic counter-based Prometheus metrics (batches total/active/completed, tasks passed/failed, duration) |
| `src/cleanup.rs` | Work directory removal, stale session reaping, process group killing |
| `src/ws.rs` | WebSocket handler for real-time batch progress streaming |

### Key Shared State (via `Arc`)

- `AppState` (in `handlers.rs`) holds `Config`, `SessionManager`, `Metrics`, `Executor`, `NonceStore`, `started_at`, `ValidatorWhitelist`, `ConsensusManager`
- `SessionManager` uses `DashMap<String, Arc<Batch>>` for lock-free concurrent access
- `ValidatorWhitelist` uses `parking_lot::RwLock<HashSet<String>>` for concurrent read access with rare writes
- `ConsensusManager` uses `DashMap<String, PendingConsensus>` for lock-free concurrent vote tracking
- Per-batch `Semaphore` in `executor.rs` controls concurrent tasks within a batch (configurable, default: 8)
- `broadcast::Sender<WsEvent>` per batch for WebSocket event streaming

## Tech Stack

- **Language**: Rust (edition 2021, nightly toolchain for fmt/clippy)
- **Async Runtime**: Tokio (full features + process), `tokio-stream`, `futures`
- **Web Framework**: Axum 0.7 (json, ws, multipart) with Tower middleware, `tower-http` (cors, trace)
- **HTTP Client**: reqwest 0.12 (json, stream) for downloading task archives
- **Serialization**: serde + serde_json + serde_yaml
- **Concurrency**: `DashMap` 6, `parking_lot` 0.12, `tokio::sync::Semaphore`, `tokio::sync::broadcast`
- **Archive Handling**: `flate2` + `tar` (tar.gz), `zip` 2 (zip)
- **Error Handling**: `anyhow` 1 + `thiserror` 2
- **Logging**: `tracing` + `tracing-subscriber` with env-filter
- **Crypto/Identity**: `sha2`, `hex`, `base64`, `bs58` (SS58 address validation), `schnorrkel` 0.11 (sr25519 signature verification), `rand_core` 0.6, `uuid` v4
- **Blockchain**: `bittensor-rs` (git dependency) for Bittensor validator whitelisting via subtensor RPC
- **Time**: `chrono` with serde support
- **Build Tooling**: `mold` linker via `.cargo/config.toml`, `clang` as linker driver
- **Container**: Multi-stage Dockerfile — `rust:1.93-slim-bookworm` builder → `debian:bookworm-slim` runtime (includes python3, pip, venv, build-essential, git, curl)
- **CI**: GitHub Actions on `blacksmith-32vcpu-ubuntu-2404` runners, nightly Rust

## CRITICAL RULES

1. **Always use `cargo +nightly fmt --all` before committing.** The CI enforces `--check` and will reject unformatted code. The project uses the nightly formatter exclusively.

2. **All clippy warnings are errors.** Run `cargo +nightly clippy --all-targets -- -D warnings` locally. CI runs the same command and will fail on any warning.

3. **Never expose secrets in logs or responses.** Auth failures log only the rejection, never the submitted hotkey value. Follow this pattern for any new secrets.

4. **All process execution MUST have timeouts.** Every call to `run_cmd`/`run_shell` in `src/executor.rs` takes a `Duration` timeout. Never spawn a child process without a timeout — agent code is untrusted and may hang forever.

5. **Output MUST be truncated.** The `truncate_output()` function in `src/executor.rs` caps output at `MAX_OUTPUT` (1MB). Any new command output capture must use this function to prevent memory exhaustion from malicious agent output.

6. **Shared state must use `Arc` + lock-free structures.** `SessionManager` uses `DashMap` (not `Mutex<HashMap>`). Metrics use `AtomicU64`. `ValidatorWhitelist` uses `parking_lot::RwLock`. `ConsensusManager` uses `DashMap`. New shared state should follow these patterns — never use `std::sync::Mutex` for hot-path data.

7. **Semaphore must gate task concurrency.** The per-batch `Semaphore` in `executor.rs` limits concurrent tasks within a batch. The `SessionManager::has_active_batch()` check prevents multiple batches from running simultaneously.

8. **Session cleanup is mandatory.** Every task must clean up its work directory in `src/executor.rs`. The stale session reaper in `src/cleanup.rs` is a safety net, not a primary mechanism.

9. **Error handling: use `anyhow::Result` for internal logic, `(StatusCode, Json<Value>)` for HTTP responses.** Handler functions in `src/handlers.rs` return `Result<impl IntoResponse, (StatusCode, Json<Value>)>`. Internal executor/task functions return `anyhow::Result<T>`.

10. **All new fields on serialized structs must use `#[serde(default)]` or `Option<T>`.** The `WorkspaceConfig`, `BatchResult`, and `TaskResult` structs are deserialized from external input or stored results. Missing fields must not break deserialization.

## DO / DO NOT

### DO
- Write unit tests for all new public functions (see existing `#[cfg(test)]` modules in every file)
- Use `tracing::info!`/`warn!`/`error!` for logging (not `println!`)
- Add new routes in `src/handlers.rs` via the `router()` function
- Use `tokio::fs` for async file I/O in the executor pipeline
- Use conventional commits (`feat:`, `fix:`, `perf:`, `chore:`, etc.)

### DO NOT
- Do NOT add `unsafe` code — there is none in this project and it should stay that way
- Do NOT add synchronous blocking I/O in async functions — use `tokio::task::spawn_blocking` for CPU-heavy work (see `extract_archive_bytes` in `src/task.rs`)
- Do NOT store large data (agent output, test output) in memory without truncation
- Do NOT add new dependencies without justification — the binary must stay small for container deployment
- Do NOT use `unwrap()` in production code paths — use `?` or `context()` from anyhow. `unwrap()` is only acceptable in tests and infallible cases (like parsing a known-good string)
- Do NOT modify `.cargo/config.toml` — it configures the mold linker for fast builds

## Build & Test Commands

```bash
# Build (debug)
cargo build

# Build (release, matches CI)
cargo +nightly build --release -j $(nproc)

# Run tests
cargo test

# Run tests (release, matches CI)
cargo +nightly test --release -j $(nproc) -- --test-threads=$(nproc)

# Format (required before commit)
cargo +nightly fmt --all

# Format check (what CI runs)
cargo +nightly fmt --all -- --check

# Lint (required before commit)
cargo +nightly clippy --all-targets -- -D warnings

# Run locally
PORT=8080 cargo run

# Docker build
docker build -t term-executor .
```

## Git Hooks

The `.githooks/` directory contains automated quality gates:

### pre-commit
- Runs `cargo +nightly fmt --all -- --check` to enforce formatting
- Runs `cargo +nightly clippy --all-targets -- -D warnings` to enforce lint
- Skip with `SKIP_GIT_HOOKS=1 git commit ...`

### pre-push
- Runs format check, clippy, full test suite, and release build
- This is the full quality gate matching CI
- Skip with `SKIP_GIT_HOOKS=1 git push ...`

Both hooks are activated via `git config core.hooksPath .githooks`.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | HTTP listen port |
| `SESSION_TTL_SECS` | `7200` | Max batch lifetime before reaping |
| `MAX_CONCURRENT_TASKS` | `8` | Maximum parallel tasks per batch |
| `CLONE_TIMEOUT_SECS` | `180` | Git clone timeout |
| `AGENT_TIMEOUT_SECS` | `600` | Agent execution timeout |
| `TEST_TIMEOUT_SECS` | `300` | Test suite timeout |
| `MAX_ARCHIVE_BYTES` | `524288000` | Max uploaded archive size (500MB) |
| `MAX_OUTPUT_BYTES` | `1048576` | Max captured output per command (1MB) |
| `WORKSPACE_BASE` | `/tmp/sessions` | Base directory for session workspaces |
| `BITTENSOR_NETUID` | `100` | Bittensor subnet ID for validator lookup |
| `MIN_VALIDATOR_STAKE_TAO` | `10000` | Minimum TAO stake for validator whitelisting |
| `VALIDATOR_REFRESH_SECS` | `300` | Interval for refreshing validator whitelist (seconds) |
| `CONSENSUS_THRESHOLD` | `0.5` | Fraction of validators required for consensus (0.0–1.0) |
| `CONSENSUS_TTL_SECS` | `60` | TTL for pending consensus entries (seconds) |

## Authentication

Authentication requires three HTTP headers: `X-Hotkey` (SS58 address), `X-Nonce` (unique per-request), and `X-Signature` (sr25519 hex signature of `hotkey + nonce`). The authorized hotkeys are dynamically loaded from the Bittensor blockchain — all validators on netuid 100 with ≥10,000 TAO stake and an active validator permit are whitelisted. The whitelist refreshes every 5 minutes. Verification steps: hotkey must be in the validator whitelist, SS58 format must be valid, nonce must not have been seen before (replay protection via `NonceStore` in `src/auth.rs` with 5-minute TTL), and the sr25519 signature must verify against the hotkey's public key using the Substrate signing context. Only requests passing all checks can submit batches via `POST /submit`. Evaluations are only triggered when ≥50% of whitelisted validators have submitted the same archive payload (identified by SHA-256 hash). All other endpoints are open.
