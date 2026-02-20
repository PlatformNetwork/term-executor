# [2.1.0](https://github.com/PlatformNetwork/term-executor/compare/v2.0.0...v2.1.0) (2026-02-20)


### Features

* integrate HuggingFace dataset handler with task/evaluation system ([db3ba95](https://github.com/PlatformNetwork/term-executor/commit/db3ba957c0f15cac899197e2e0455a8cf9ea39f9))

# [2.0.0](https://github.com/PlatformNetwork/term-executor/compare/v1.2.0...v2.0.0) (2026-02-18)


### Features

* **auth:** replace static hotkey/API-key auth with Bittensor validator whitelisting and 50% consensus ([#5](https://github.com/PlatformNetwork/term-executor/issues/5)) ([a573ad0](https://github.com/PlatformNetwork/term-executor/commit/a573ad04df1157843b8a825d24ed5c4df06f0f90))


### BREAKING CHANGES

* **auth:** WORKER_API_KEY env var and X-Api-Key header no longer required.
All validators on Bittensor netuid 100 with sufficient stake are auto-whitelisted.

* ci: trigger CI run

* fix(security): address auth bypass, input validation, and config issues

- Move nonce consumption AFTER signature verification in verify_request()
  to prevent attackers from burning legitimate nonces via invalid signatures
- Fix TOCTOU race in NonceStore::check_and_insert() using atomic DashMap
  entry API instead of separate contains_key + insert
- Add input length limits for auth headers (hotkey 128B, nonce 256B,
  signature 256B) to prevent memory exhaustion via oversized values
- Add consensus_threshold validation in Config::from_env() — must be
  in range (0.0, 1.0], panics at startup if invalid
- Add saturating conversion for consensus required calculation to prevent
  integer overflow on f64→usize cast
- Add tests for all security fixes

* fix(dead-code): remove orphaned default_concurrent fn and unnecessary allow(dead_code)

* fix: code quality issues in bittensor validator consensus

- Extract magic number 100 to configurable MAX_PENDING_CONSENSUS
- Restore #[allow(dead_code)] on DEFAULT_MAX_OUTPUT_BYTES constant
- Use anyhow::Context instead of map_err(anyhow::anyhow!) in validator_whitelist

* fix(security): address race condition, config panic, SS58 checksum, and container security

- consensus.rs: Fix TOCTOU race condition in record_vote by using
  DashMap entry API (remove_entry) to atomically check votes and remove
  entry while holding the shard lock, preventing concurrent threads from
  inserting votes between drop and remove
- config.rs: Replace assert! with proper Result<Self, String> return
  from Config::from_env() to avoid panicking in production on invalid
  CONSENSUS_THRESHOLD values
- main.rs: Update Config::from_env() call to handle Result with expect
- auth.rs: Add SS58 checksum verification using Blake2b-512 (correct
  Substrate algorithm) in ss58_to_public_key_bytes to reject addresses
  with corrupted checksums; previously only decoded base58 without
  validating the 2-byte checksum suffix
- Dockerfile: Add non-root executor user for container runtime security

* fix(dead-code): remove unused max_output_bytes config field and constant

Remove DEFAULT_MAX_OUTPUT_BYTES constant and max_output_bytes Config field
that were defined and populated from env but never read anywhere outside
config.rs. Both had #[allow(dead_code)] annotations suppressing warnings.

* fix(quality): replace expect/unwrap with proper error handling, extract magic numbers to constants

- main.rs: Replace .expect() on Config::from_env() with match + tracing::error! + process::exit(1)
- validator_whitelist.rs: Extract retry count (3) and backoff base (2) to named constants
- validator_whitelist.rs: Replace unwrap_or_else on Option with if-let pattern
- consensus.rs: Extract reaper interval (30s) to REAPER_INTERVAL_SECS constant

* fix(security): address multiple security vulnerabilities in PR files

- consensus.rs: Remove archive_data storage from PendingConsensus to
  prevent memory exhaustion (up to 50GB with 100 pending × 500MB each).
  Callers now use their own archive bytes since all votes for the same
  hash have identical data.

- handlers.rs: Stream multipart upload with per-chunk size enforcement
  instead of buffering entire archive before checking size limit.
  Sanitize error messages to not leak internal details (file paths,
  extraction errors) to clients; log details server-side instead.

- auth.rs: Add nonce format validation requiring non-empty printable
  ASCII characters (defense-in-depth against log injection and empty
  nonce edge cases).

- main.rs: Replace .unwrap() on TcpListener::bind and axum::serve with
  proper error logging and process::exit per AGENTS.md rules.

- ws.rs: Replace .unwrap() on serde_json::to_string with
  unwrap_or_default() to comply with AGENTS.md no-unwrap rule.

* fix(dead-code): rename misleading underscore-prefixed variable in consensus

* fix(quality): replace unwrap/expect with proper error handling in production code

- main.rs:21: Replace .parse().unwrap() on tracing directive with
  unwrap_or_else fallback to INFO level directive
- main.rs:36: Replace .expect() on workspace dir creation with
  error log + process::exit(1) pattern
- main.rs:110: Replace .expect() on ctrl_c handler with if-let-Err
  that logs and returns gracefully
- executor.rs:189: Replace semaphore.acquire().unwrap() with match
  that handles closed semaphore by creating a failed TaskResult

All changes follow AGENTS.md rule: no .unwrap()/.expect() in
production code paths. Test code is unchanged.

* docs: refresh AGENTS.md

# [1.2.0](https://github.com/PlatformNetwork/term-executor/compare/v1.1.0...v1.2.0) (2026-02-17)


### Features

* **auth:** add sr25519 signature + nonce verification ([dc8d8d4](https://github.com/PlatformNetwork/term-executor/commit/dc8d8d405e5e6d08100d900b8e94e29ced0b5417))
* **auth:** require API key alongside whitelisted hotkey ([#3](https://github.com/PlatformNetwork/term-executor/issues/3)) ([887f72b](https://github.com/PlatformNetwork/term-executor/commit/887f72bac8021e073e10d65e385ecb3205b55010))

# [1.1.0](https://github.com/PlatformNetwork/term-executor/compare/v1.0.0...v1.1.0) (2026-02-17)


### Features

* **executor:** add SWE-bench batch evaluation with hotkey auth and WebSocket streaming ([#2](https://github.com/PlatformNetwork/term-executor/issues/2)) ([8bfa8ee](https://github.com/PlatformNetwork/term-executor/commit/8bfa8eea464fc19b10eda23a834b3b019582d624))

# 1.0.0 (2026-02-17)


### Bug Fixes

* bump Rust Docker image to 1.85 for edition2024 support ([209f460](https://github.com/PlatformNetwork/term-executor/commit/209f460eb34788ff60604d2dc9c54c7c548be806))
* lowercase GHCR image tags for Docker push ([89449f9](https://github.com/PlatformNetwork/term-executor/commit/89449f992311c3712c5caa7a8d520dba09937866))
* remove target-cpu=native to avoid SIGILL on Blacksmith runners ([22bcb85](https://github.com/PlatformNetwork/term-executor/commit/22bcb85a7de818a03dcb43e01437316fd0ad0a0f))
* use rust:1.93-bookworm Docker image ([ddd1a24](https://github.com/PlatformNetwork/term-executor/commit/ddd1a2450e73bc41348a756b86dcb231d976acbd))


### Features

* initial term-executor — remote evaluation server for Basilica ([18f4f67](https://github.com/PlatformNetwork/term-executor/commit/18f4f673d213dc07522034346fdda656bd016352))
* production-ready implementation with Basilica integration ([5797025](https://github.com/PlatformNetwork/term-executor/commit/57970256c3a3201f6749f99933617a5e16fdd5cd))


### Performance Improvements

* minimal Docker image - remove all language runtimes from executor ([38058e8](https://github.com/PlatformNetwork/term-executor/commit/38058e8a848c0a945b411ea955eb56f0a9a5272a))
