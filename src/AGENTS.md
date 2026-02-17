# AGENTS.md — src/ (term-executor core)

This is a single-crate binary. All source files live in `src/` with no sub-modules or nested directories.

## Module Dependency Graph

```
main.rs
  ├── config.rs      (Config::from_env, AUTHORIZED_HOTKEY)
  ├── auth.rs        (NonceStore created in main, reaper_loop spawned from main)
  ├── handlers.rs     (Axum router + AppState)
  │     ├── auth.rs   (extract_auth_headers + verify_request for /submit)
  │     ├── executor.rs (spawned from submit handler)
  │     │     ├── task.rs (extract, parse, load tasks)
  │     │     ├── session.rs (BatchResult/TaskResult mutation)
  │     │     └── cleanup.rs (work dir removal)
  │     ├── metrics.rs (Prometheus rendering)
  │     ├── session.rs (SessionManager CRUD)
  │     └── ws.rs     (WebSocket handler)
  ├── session.rs      (reaper_loop spawned from main)
  └── cleanup.rs      (reap_stale_sessions spawned from main)
```

## File-by-File Guide

### `main.rs`
- Entry point. Initializes tracing, config, session manager, metrics, executor.
- Creates `AppState`, builds Axum router, spawns background tasks (session reaper, nonce reaper, stale dir reaper).
- Binds to `0.0.0.0:{PORT}` with graceful shutdown on CTRL+C.
- **Convention**: Background tasks are spawned with `tokio::spawn` and run indefinitely.

### `config.rs`
- `Config` struct with all environment-driven settings.
- `Config::from_env()` reads env vars with `env_parse()` helper (returns default on missing/invalid).
- `Config::print_banner()` logs a formatted startup banner.
- `AUTHORIZED_HOTKEY` — hardcoded SS58 hotkey constant for authentication.
- **Convention**: Add new config fields here, with a `DEFAULT_*` constant and an env var name. Always provide a sensible default.

### `handlers.rs`
- Defines `AppState` struct (`config`, `sessions`, `metrics`, `executor`, `nonce_store`, `started_at`).
- `router()` builds the Axum `Router` with all routes and shared state.
- Route handlers: `health`, `status`, `metrics`, `submit_batch`, `get_batch`, `get_batch_tasks`, `get_task`, `list_batches`.
- Routes: `GET /health`, `GET /status`, `GET /metrics`, `POST /submit`, `GET /batch/{id}`, `GET /batch/{id}/tasks`, `GET /batch/{id}/task/{task_id}`, `GET /batches`, `GET /ws`.
- `submit_batch` handler does: auth header extraction → `verify_request` (hotkey + API key + nonce + signature) → busy check → multipart upload → archive extraction → batch creation → executor spawn.
- **Convention**: Return `Result<impl IntoResponse, (StatusCode, Json<Value>)>` from handlers that can fail. Use `Json(serde_json::json!({...}))` for responses.

### `auth.rs`
- `NonceStore` — `DashMap`-backed nonce tracker with 5-minute TTL and background reaper loop for replay protection.
- `AuthHeaders` — struct holding `hotkey`, `nonce`, `signature`, `api_key` extracted from request headers.
- `extract_auth_headers(headers)` — reads `X-Hotkey`, `X-Nonce`, `X-Signature`, `X-Api-Key` headers from request.
- `verify_request(auth, nonce_store, expected_api_key)` — full auth pipeline: hotkey match → API key match → SS58 validation → nonce replay check → sr25519 signature verification.
- `validate_ss58(address)` — validates SS58 address format using `bs58`.
- `verify_sr25519_signature(ss58_hotkey, message, signature_hex)` — verifies an sr25519 signature using `schnorrkel` with the Substrate signing context.
- `AuthError` — enum with `UnauthorizedHotkey`, `InvalidHotkey`, `InvalidApiKey`, `NonceReused`, `InvalidSignature` variants, each with `.code()` and `.message()` methods.
- **Convention**: Auth is mandatory — `POST /submit` requires all four headers (`X-Hotkey`, `X-Nonce`, `X-Signature`, `X-Api-Key`). The signed message is `hotkey + nonce`.

### `executor.rs`
- `Executor::spawn_batch(batch, archive, concurrent_limit)` — spawns a tokio task that runs all tasks in the batch.
- `run_batch(config, batch, archive, concurrent_limit)` — orchestrates concurrent task execution with a per-batch `Semaphore`.
- `run_single_task(config, task, agent_code, agent_language, cancel_rx)` — runs one task: clone → install → agent → tests → cleanup.
- `run_cmd(argv, cwd, timeout, env)` / `run_shell(shell_cmd, cwd, timeout, env)` — process execution with timeout.
- `truncate_output(raw)` — caps output at 1MB.
- `agent_extension(language)` / `agent_runner(language, script_path)` — maps language strings to file extensions and runner commands.
- **Convention**: Every phase checks `cancel_rx` for cancellation. Every process has a timeout. Output is always truncated.

### `session.rs`
- `BatchStatus` (enum: Pending, Extracting, Running, Completed, Failed), `TaskStatus` (enum: Queued, CloningRepo, InstallingDeps, RunningAgent, RunningTests, Completed, Failed).
- `TaskTestResult`, `TaskResult`, `BatchResult` — core result data types.
- `WsEvent` — WebSocket event struct with `event`, `batch_id`, `task_id`, `data`.
- `Batch` — holds id, created_at, result (`Arc<Mutex<BatchResult>>`), events_tx (`broadcast::Sender<WsEvent>`), cancel channel.
- `SessionStats` — atomic counters for created/active/completed/failed batches.
- `BatchSummary` — lightweight struct for `list_batches()` output.
- `SessionManager` — `DashMap`-backed batch store with `SessionStats`, create/get/list/has_active_batch/mark_completed/mark_failed operations.
- `reaper_loop()` — runs every 60s, removes batches older than TTL, sends cancel signal.
- **Convention**: All enums use `#[serde(rename_all = "snake_case")]`. Batch IDs are UUID v4 strings.

### `task.rs`
- `extract_uploaded_archive(data, dest)` — extracts uploaded bytes (zip or tar.gz) in a blocking task, then parses contents.
- `extract_archive_bytes(data, dest)` — synchronous zip/tar.gz extraction.
- `find_archive_root(base)` — locates `tasks/` or `agent_code/` in extracted archive (direct or one level nested).
- `load_agent_code(root)` — reads all files from `agent_code/` directory.
- `detect_agent_language(root)` — infers language from file extensions in `agent_code/`.
- `load_tasks(root)` — iterates `tasks/` subdirectories, parses each into `SweForgeTask`.
- `parse_task(task_dir)` — reads `workspace.yaml`, `prompt.md`, `tests/` directory, `checks.txt`.
- `WorkspaceConfig` — deserialized from `workspace.yaml` (repo, version, base_commit, install, language).
- `SweForgeTask` — parsed task with workspace config, prompt text, test scripts, test source files.
- `ExtractedArchive` — contains all parsed tasks plus agent code and language.
- **Convention**: `.sh` files in `tests/` are test scripts (executed); all other files are source files (written to repo). Archive size capped at 500MB.

### `metrics.rs`
- `Metrics` — atomic counters for batches total/active/completed, tasks total/passed/failed, duration_sum_ms.
- `start_batch()` / `finish_batch(all_passed, duration_ms)` / `record_task_result(passed)` — counter operations.
- `render_prometheus()` — formats counters as Prometheus text exposition format.
- **Convention**: All counters are `AtomicU64` with `Ordering::Relaxed`. Metrics are exposed at `GET /metrics`.

### `cleanup.rs`
- `remove_work_dir(path)` — async directory removal (logs warning on failure, never panics).
- `kill_process_group(pgid)` — best-effort `kill -9` on a process group.
- `reap_stale_sessions(base, max_age_secs)` — scans workspace base, removes dirs older than TTL.
- **Convention**: Cleanup functions are fire-and-forget. They log but never return errors.

### `ws.rs`
- `ws_handler(ws, state, query)` — Axum WebSocket upgrade handler, requires `batch_id` query parameter.
- `handle_ws(socket, state, batch_id)` — manages WebSocket connection: sends initial snapshot, then streams `WsEvent`s from the batch's `broadcast` channel.
- Handles connection lifecycle: ping, close, lagged messages, stream closed.
- **Convention**: WebSocket URL is `GET /ws?batch_id={id}`. Events are JSON-serialized `WsEvent` structs.

## Testing

Every module has a `#[cfg(test)] mod tests` block. Tests use:
- `#[test]` for sync unit tests
- `#[tokio::test]` for async tests
- `tempfile::tempdir()` for filesystem tests
- No external test fixtures or mock servers needed

Run all tests: `cargo test` or `cargo +nightly test --release`
