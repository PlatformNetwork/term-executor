# AGENTS.md — src/ (term-executor core)

This is a single-crate binary. All source files live in `src/` with no sub-modules or nested directories.

## Module Dependency Graph

```
main.rs
  ├── config.rs      (Config::from_env)
  ├── handlers.rs     (Axum router + AppState)
  │     ├── auth.rs   (check_token for /evaluate)
  │     ├── executor.rs (spawned from evaluate handler)
  │     │     ├── task.rs (download, extract, parse)
  │     │     ├── session.rs (EvalResult mutation)
  │     │     └── cleanup.rs (work dir removal)
  │     ├── metrics.rs (Prometheus rendering)
  │     └── session.rs (SessionManager CRUD)
  ├── session.rs      (reaper_loop spawned from main)
  └── cleanup.rs      (reap_stale_sessions spawned from main)
```

## File-by-File Guide

### `main.rs`
- Entry point. Initializes tracing, config, session manager, metrics, executor, semaphore.
- Creates `AppState`, builds Axum router, spawns background tasks (session reaper, stale dir reaper).
- Binds to `0.0.0.0:{PORT}` with graceful shutdown on SIGTERM/CTRL+C.
- **Convention**: Background tasks are spawned with `tokio::spawn` and run indefinitely.

### `config.rs`
- `Config` struct with all environment-driven settings.
- `Config::from_env()` reads env vars with `env_parse()` helper (returns default on missing/invalid).
- `Config::print_banner()` logs a formatted startup banner.
- **Convention**: Add new config fields here, with a `DEFAULT_*` constant and an env var name. Always provide a sensible default.

### `handlers.rs`
- Defines `AppState` struct (all fields `Arc`-wrapped for sharing).
- `router()` builds the Axum `Router` with all routes and shared state.
- Route handlers: `health`, `status`, `metrics`, `evaluate`, `get_eval`, `list_evals`.
- `evaluate` handler does: auth check → payload validation → capacity check → session creation → executor spawn.
- **Convention**: Return `Result<impl IntoResponse, (StatusCode, String)>` from handlers that can fail. Use `Json(serde_json::json!({...}))` for responses.

### `auth.rs`
- `auth_middleware` — Axum middleware (currently unused in router, auth is inline in `evaluate`).
- `check_token(auth_header, expected)` — simple Bearer token comparison used by `evaluate` handler.
- `inject_request_id` — adds `x-request-id` UUID header to responses.
- **Convention**: Auth is optional — if `AUTH_TOKEN` env var is unset, `/evaluate` is open.

### `executor.rs`
- `Executor::spawn_eval(session)` — spawns a tokio task that runs the full evaluation pipeline.
- `run_eval(config, session, cancel_rx)` — orchestrates: download → clone → install → agent → tests → cleanup.
- `run_cmd(argv, cwd, timeout, env)` / `run_shell(shell_cmd, cwd, timeout, env)` — process execution with timeout.
- `truncate_output(raw)` — caps output at 1MB.
- `agent_extension(language)` / `agent_runner(language, script_path)` — maps language strings to file extensions and runner commands.
- **Convention**: Every phase checks `cancel_rx` for cancellation. Every process has a timeout. Output is always truncated.

### `session.rs`
- `EvalRequest`, `EvalStatus` (enum), `EvalStep` (enum), `TaskTestResult`, `EvalResult` — core data types.
- `Session` — holds id, request, result (`Arc<Mutex<EvalResult>>`), created_at, cancel channel.
- `SessionManager` — `DashMap`-backed session store with create/get/remove/list/mark operations.
- `reaper_loop()` — runs every 60s, removes sessions older than TTL, sends cancel signal.
- **Convention**: All enums use `#[serde(rename_all = "snake_case")]`. Session IDs are UUID v4 strings.

### `task.rs`
- `download_and_extract(url, dest)` — HTTP GET → bytes → extract (zip or tar.gz) in a blocking task.
- `parse_task(task_dir)` — reads `workspace.yaml`, `prompt.md`, `tests/` directory, `checks.txt`.
- `find_task_root(base)` — locates `workspace.yaml` in extracted archive (direct or one level nested).
- `WorkspaceConfig` — deserialized from `workspace.yaml` (repo, version, base_commit, install, language).
- `SweForgeTask` — parsed task with workspace config, prompt text, test scripts, test source files.
- **Convention**: `.sh` files in `tests/` are test scripts (executed); all other files are source files (written to repo). Archive size capped at 100MB.

### `metrics.rs`
- `Metrics` — atomic counters for evals total/passed/failed/cancelled/active/duration_sum.
- `start_eval()` / `finish_eval(passed, duration_ms)` / `cancel_eval()` — counter operations.
- `render_prometheus()` — formats counters as Prometheus text exposition format.
- **Convention**: All counters are `AtomicU64` with `Ordering::Relaxed`. Metrics are exposed at `GET /metrics`.

### `cleanup.rs`
- `remove_work_dir(path)` — async directory removal (logs warning on failure, never panics).
- `kill_process_group(pgid)` — best-effort `kill -9` on a process group.
- `reap_stale_sessions(base, max_age_secs)` — scans workspace base, removes dirs older than TTL.
- **Convention**: Cleanup functions are fire-and-forget. They log but never return errors.

## Testing

Every module has a `#[cfg(test)] mod tests` block. Tests use:
- `#[test]` for sync unit tests
- `#[tokio::test]` for async tests
- `tempfile::tempdir()` for filesystem tests
- No external test fixtures or mock servers needed

Run all tests: `cargo test` or `cargo +nightly test --release`
