# AGENTS.md — term-executor

## Project Purpose

**term-executor** is a remote evaluation executor for the [term-challenge](https://github.com/PlatformNetwork/term-challenge) platform. It runs as a containerized Rust service on [Basilica](https://basilica.ai) that receives batch task archives via multipart upload, executes agent code against cloned task repositories, runs validation test scripts, and reports pass/fail results with aggregate rewards. It is the core compute backend that evaluates AI agent coding challenges.

## Architecture Overview

This is a **single-crate Rust binary** (`term-executor`) built with Axum. There are no sub-crates or workspaces.

### Data Flow

```
Client → POST /submit (multipart archive) → term-executor
  1. Authenticate via X-Hotkey header (SS58 hotkey)
  2. Extract uploaded archive (zip/tar.gz) containing tasks/ and agent_code/
  3. Parse each task: workspace.yaml, prompt.md, tests/
  4. For each task (concurrently, up to limit):
     a. git clone the target repository at base_commit
     b. Run install commands (pip install, etc.)
     c. Write & execute agent code in the repo
     d. Write test source files into the repo
     e. Run test scripts (bash), collect exit codes
  5. Aggregate results (reward per task, aggregate reward)
  6. Stream progress via WebSocket (GET /ws?batch_id=...)
  7. Return results via GET /batch/{id}
```

### Module Map

| File | Responsibility |
|---|---|
| `src/main.rs` | Entry point — bootstraps config, session manager, executor, Axum server, reaper tasks |
| `src/config.rs` | `Config` struct loaded from environment variables with defaults; `AUTHORIZED_HOTKEY` constant |
| `src/handlers.rs` | Axum route handlers: `/health`, `/status`, `/metrics`, `/submit`, `/batch/{id}`, `/batch/{id}/tasks`, `/batch/{id}/task/{task_id}`, `/batches` |
| `src/auth.rs` | Hotkey authentication: `extract_hotkey()`, `verify_hotkey()`, `validate_ss58()` |
| `src/executor.rs` | Core evaluation engine — spawns batch tasks that clone repos, run agents, run tests concurrently |
| `src/session.rs` | `SessionManager` with `DashMap`, `Batch`, `BatchResult`, `TaskResult`, `BatchStatus`, `TaskStatus`, `WsEvent` types |
| `src/task.rs` | Archive extraction (zip/tar.gz), task directory parsing, agent code loading, language detection |
| `src/metrics.rs` | Atomic counter-based Prometheus metrics (batches total/active/completed, tasks passed/failed, duration) |
| `src/cleanup.rs` | Work directory removal, stale session reaping, process group killing |
| `src/ws.rs` | WebSocket handler for real-time batch progress streaming |

### Key Shared State (via `Arc`)

- `AppState` (in `handlers.rs`) holds `Config`, `SessionManager`, `Metrics`, `Executor`, `started_at`
- `SessionManager` uses `DashMap<String, Arc<Batch>>` for lock-free concurrent access
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
- **Crypto/Identity**: `sha2`, `hex`, `base64`, `bs58` (SS58 address validation), `uuid` v4
- **Time**: `chrono` with serde support
- **Build Tooling**: `mold` linker via `.cargo/config.toml`, `clang` as linker driver
- **Container**: Multi-stage Dockerfile — `rust:1.93-slim-bookworm` builder → `debian:bookworm-slim` runtime (includes python3, pip, venv, build-essential, git, curl)
- **CI**: GitHub Actions on `blacksmith-32vcpu-ubuntu-2404` runners, nightly Rust

## CRITICAL RULES

1. **Always use `cargo +nightly fmt --all` before committing.** The CI enforces `--check` and will reject unformatted code. The project uses the nightly formatter exclusively.

2. **All clippy warnings are errors.** Run `cargo +nightly clippy --all-targets -- -D warnings` locally. CI runs the same command and will fail on any warning.

3. **Never expose secrets in logs or responses.** The `AUTHORIZED_HOTKEY` in `src/config.rs` is the only authorized SS58 hotkey. Auth failures log only the rejection, never the submitted hotkey value. Follow this pattern for any new secrets.

4. **All process execution MUST have timeouts.** Every call to `run_cmd`/`run_shell` in `src/executor.rs` takes a `Duration` timeout. Never spawn a child process without a timeout — agent code is untrusted and may hang forever.

5. **Output MUST be truncated.** The `truncate_output()` function in `src/executor.rs` caps output at `MAX_OUTPUT` (1MB). Any new command output capture must use this function to prevent memory exhaustion from malicious agent output.

6. **Shared state must use `Arc` + lock-free structures.** `SessionManager` uses `DashMap` (not `Mutex<HashMap>`). Metrics use `AtomicU64`. New shared state should follow these patterns — never use `std::sync::Mutex` for hot-path data.

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

## Authentication

Authentication uses SS58 hotkey validation via the `X-Hotkey` HTTP header. The authorized hotkey is hardcoded as `AUTHORIZED_HOTKEY` in `src/config.rs`. Only requests with a matching hotkey can submit batches via `POST /submit`. All other endpoints are open.
