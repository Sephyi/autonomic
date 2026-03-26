# Rust Coding Standards — Autonomic

## Language

- **Edition 2024**, MSRV 1.94, toolchain pinned via `rust-toolchain.toml`
- Use Rust 1.94 features freely (let chains, async closures, precise capturing, trait upcasting, LazyLock::get). See `.claude/specs/rust-1.94-features.md` for catalog.

## Safety

- `#![forbid(unsafe_code)]` in all crates except where FFI requires it
- If `unsafe` is needed: `#![deny(unsafe_code)]` with per-block `#[allow]` and `// SAFETY:` comment
- `overflow-checks = true` in release profile (already in workspace Cargo.toml)
- `checked_add`, `try_into()` for arithmetic on external input — no unchecked `as` casts

## Async

- **tokio** as the only async runtime. Never mix runtimes.
- **Never block the async runtime.** Use `tokio::task::spawn_blocking` for blocking operations (gix, heavy computation).
- **Bounded channels everywhere.** `tokio::sync::mpsc::channel(cap)`, never unbounded.
- **`tokio::time::timeout`** over `select!` with sleep for simple timeouts.
- **`JoinSet`** for structured concurrency — tasks auto-aborted on drop.
- See `.claude/specs/async-concurrency.md` for detailed patterns.

## Error Handling

- `thiserror` 2.x for all library crate errors. One error enum per crate.
- `#[from]` for error conversion between crates.
- Never `anyhow` in library crates. `anyhow` only in `autonomic-cli` if needed.
- Never `Box<dyn Error>`. Never `.unwrap()` in library code (`.expect("reason")` acceptable for provably-safe cases).

## Naming

- `snake_case` for variables, functions, methods, modules
- `UpperCamelCase` for types, traits, enums
- `SCREAMING_SNAKE_CASE` for constants
- Descriptive names. Short names only in scope <= 10 lines.

## Dependencies

- Workspace dependencies via `{ workspace = true }` — never inline versions in crate Cargo.toml
- `sqlx` for database (compile-time checked queries via `sqlx::query!`)
- `gix` for git (pure Rust, not git2)
- `figment` for config, `clap` for CLI, `croner` for cron

## Testing

- `proptest` for property-based invariant testing
- `insta` for serialization snapshot testing
- `#[tokio::test]` for async tests
- Unit tests in same file (`#[cfg(test)]` module)
- Integration tests in `tests/` directory per crate
- TDD: write failing test first when possible

## Formatting and Linting

- `cargo fmt` enforced by PostToolUse hook (automatic)
- `cargo clippy -D warnings` enforced by PostToolUse hook (automatic)
- `clippy.toml` disallows: `std::thread::sleep`, `std::process::Command::new`, `dbg!`, `println!`, `eprintln!`
- Use `tracing` macros instead of print macros

## Commits

- Conventional commits: `feat`, `fix`, `docs`, `test`, `build`, `ci`, `refactor`, `perf`, `chore`
- Scope = crate name: `feat(core): ...`, `fix(session): ...`
- One concern per commit
- Pre-commit hook enforces fmt + clippy
