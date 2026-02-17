# AGENTS.md — term-executor

## Project Purpose

**term-executor** is a remote evaluation executor for the [term-challenge](https://github.com/PlatformNetwork/term-challenge) platform. It runs as a containerized Rust service on [Basilica](https://basilica.ai) that receives agent code submissions, executes them against a cloned task repository, runs validation test scripts, and reports pass/fail results. It is the core compute backend that evaluates AI agent coding challenges.

## Architecture Overview

This is a **single-crate Rust binary** (`term-executor`) built with Axum. There are no sub-crates or workspaces.

### Data Flow

```
Platform Server → POST /evaluate → term-executor
  1. Download task archive (.tar.gz / .zip) from task_url
  2. Parse workspace.yaml, prompt.md, tests/
  3. git clone the target repository at base_commit
  4. Run install commands (pip install, etc.)
  5. Write & execute agent code in the repo
  6. Write test source files into the repo
  7. Run test scripts (bash), collect exit codes
  8. Return results via GET /evaluate/{id}
```

### Module Map

| File | Responsibility |
|---|---|
| `src/main.rs` | Entry point — bootstraps config, session manager, executor, Axum server, reaper tasks |
| `src/config.rs` | `Config` struct loaded from environment variables with defaults |
| `src/handlers.rs` | Axum route handlers: `/health`, `/status`, `/metrics`, `/evaluate`, `/evaluate/{id}`, `/evaluations` |
| `src/auth.rs` | Bearer token authentication middleware and `check_token()` helper |
| `src/executor.rs` | Core evaluation engine — spawns async tasks that clone repos, run agents, run tests |
| `src/session.rs` | `SessionManager` with `DashMap`, `Session`, `EvalResult`, `EvalStatus`, `EvalStep` types |
| `src/task.rs` | Task archive download/extraction (zip/tar.gz), `workspace.yaml` parsing, test file loading |
| `src/metrics.rs` | Atomic counter-based Prometheus metrics (total, passed, failed, active, duration) |
| `src/cleanup.rs` | Work directory removal, stale session reaping, process group killing |

### Key Shared State (via `Arc`)

- `AppState` (in `handlers.rs`) holds `Config`, `SessionManager`, `Metrics`, `Executor`, `Semaphore`
- `SessionManager` uses `DashMap<String, Arc<Session>>` for lock-free concurrent access
- `Semaphore` controls max concurrent evaluations (default: 4)

## Tech Stack

- **Language**: Rust (edition 2021, nightly toolchain for fmt/clippy)
- **Async Runtime**: Tokio (full features + process)
- **Web Framework**: Axum 0.7 with Tower middleware
- **HTTP Client**: reqwest 0.12 (for downloading task archives)
- **Serialization**: serde + serde_json + serde_yaml
- **Concurrency**: `DashMap` 6, `parking_lot` 0.12, `tokio::sync::Semaphore`
- **Archive Handling**: `flate2` + `tar` (tar.gz), `zip` 2 (zip)
- **Error Handling**: `anyhow` 1 + `thiserror` 2
- **Logging**: `tracing` + `tracing-subscriber` with env-filter
- **Build Tooling**: `mold` linker via `.cargo/config.toml`, `clang` as linker driver
- **Container**: Multi-stage Dockerfile — `rust:1.93-slim-bookworm` builder → `debian:bookworm-slim` runtime
- **CI**: GitHub Actions on Blacksmith runners (4/32 vCPU), nightly Rust

## CRITICAL RULES

1. **Always use `cargo +nightly fmt --all` before committing.** The CI enforces `--check` and will reject unformatted code. The project uses the nightly formatter exclusively.

2. **All clippy warnings are errors.** Run `cargo +nightly clippy --all-targets -- -D warnings` locally. CI runs the same command and will fail on any warning.

3. **Never expose secrets in logs or responses.** The `AUTH_TOKEN` environment variable is sensitive. Auth failures log only the `x-forwarded-for` header, never the token value. Follow this pattern for any new secrets.

4. **All process execution MUST have timeouts.** Every call to `run_cmd`/`run_shell` in `src/executor.rs` takes a `Duration` timeout. Never spawn a child process without a timeout — agent code is untrusted and may hang forever.

5. **Output MUST be truncated.** The `truncate_output()` function in `src/executor.rs` caps output at `MAX_OUTPUT` (1MB). Any new command output capture must use this function to prevent memory exhaustion from malicious agent output.

6. **Shared state must use `Arc` + lock-free structures.** `SessionManager` uses `DashMap` (not `Mutex<HashMap>`). Metrics use `AtomicU64`. New shared state should follow these patterns — never use `std::sync::Mutex` for hot-path data.

7. **Semaphore must gate evaluation capacity.** The `Semaphore` in `AppState` limits concurrent evaluations to `MAX_CONCURRENT_EVALS`. Any new evaluation path must acquire a permit before spawning work.

8. **Session cleanup is mandatory.** Every evaluation must clean up its work directory in `src/executor.rs` (the `Cleanup` step). The stale session reaper in `src/cleanup.rs` is a safety net, not a primary mechanism.

9. **Error handling: use `anyhow::Result` for internal logic, `(StatusCode, String)` for HTTP responses.** Handler functions in `src/handlers.rs` return `Result<impl IntoResponse, (StatusCode, String)>`. Internal executor/task functions return `anyhow::Result<T>`.

10. **All new fields on serialized structs must use `#[serde(default)]` or `Option<T>`.** The `EvalRequest`, `EvalResult`, and `WorkspaceConfig` structs are deserialized from external input. Missing fields must not break deserialization.

## DO / DO NOT

### DO
- Write unit tests for all new public functions (see existing `#[cfg(test)]` modules in every file)
- Use `tracing::info!`/`warn!`/`error!` for logging (not `println!`)
- Add new routes in `src/handlers.rs` via the `router()` function
- Use `tokio::fs` for async file I/O in the executor pipeline
- Keep the Dockerfile minimal — runtime image has no compilers or language runtimes
- Use conventional commits (`feat:`, `fix:`, `perf:`, `chore:`, etc.)

### DO NOT
- Do NOT add `unsafe` code — there is none in this project and it should stay that way
- Do NOT add synchronous blocking I/O in async functions — use `tokio::task::spawn_blocking` for CPU-heavy work (see `extract_archive` in `src/task.rs`)
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
AUTH_TOKEN=test PORT=8080 cargo run

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
| `AUTH_TOKEN` | *(none)* | Bearer token for `/evaluate`. If unset, auth is disabled |
| `SESSION_TTL_SECS` | `1800` | Max session lifetime before reaping |
| `MAX_CONCURRENT_EVALS` | `4` | Maximum parallel evaluations |
| `CLONE_TIMEOUT_SECS` | `120` | Git clone timeout |
| `AGENT_TIMEOUT_SECS` | `600` | Agent execution timeout |
| `TEST_TIMEOUT_SECS` | `300` | Test suite timeout |
| `MAX_AGENT_CODE_BYTES` | `5242880` | Max agent code payload (5MB) |
| `MAX_OUTPUT_BYTES` | `1048576` | Max captured output per command (1MB) |
| `WORKSPACE_BASE` | `/tmp/sessions` | Base directory for session workspaces |
