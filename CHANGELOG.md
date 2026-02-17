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
