# Contributing to OmniDev

Thank you for your interest in contributing!

## Code rules

- No comments in source code
- No `unwrap` / `expect` in production code (use proper error handling)
- Prefer `anyhow` / `thiserror` for errors
- Keep the public API small and documented when needed

## Development workflow

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Run the checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

5. Open a pull request with a clear description

## Pull request guidelines

- Keep PRs focused (one feature or fix per PR)
- Update the README or docs if you change user-facing behaviour
- Add tests when it makes sense

## Reporting bugs

Use the GitHub issue templates. Include:

- OmniDev version (`omnidev version`)
- Operating system
- Steps to reproduce
- Expected vs actual behaviour

## Feature requests

Open an issue with the feature request template. Explain the use case clearly.

## Security

Please do **not** open public issues for security vulnerabilities.
See [SECURITY.md](SECURITY.md) for the private reporting process.

## Questions

Feel free to open a discussion or an issue if something is unclear.
