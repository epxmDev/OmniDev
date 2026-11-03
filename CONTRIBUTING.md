# Contributing to OmniDev

Thank you for helping improve OmniDev.

## Ground rules

- Source files contain no comments of any kind. Put explanations in Markdown under `docs/` or in this repository's documentation.
- No `unwrap()` or `expect()` in production code, no `unsafe` without a written justification, and no swallowed errors.
- Keep modules focused and avoid duplicated business logic.
- Do not add dependencies without a clear need.

## Development setup

You need a Rust toolchain that supports edition 2024 (Rust 1.85 or newer) and a C compiler for the bundled SQLite.

```bash
git clone <your fork>
cd omnidev
cargo build
cargo test --all-targets
```

## Before opening a pull request

Run the same gates CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
```

## Dependency pinning note

`comfy-table` is pinned to 7.1.x in `Cargo.lock` because 7.2 requires a newer compiler than the declared MSRV. Do not run `cargo update -p comfy-table` without verifying the MSRV still builds.

## Commits and pull requests

- Use Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, `test:` or `chore:`.
- Describe behaviour changes in `CHANGELOG.md` under *Unreleased*.
- Add tests for new behaviour. Tests must create their own repositories in temporary directories and never depend on a developer's personal repositories.
