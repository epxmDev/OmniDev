# Security Policy

## Supported versions

Security fixes are provided for the latest released minor version.

## Reporting a vulnerability

Please do **not** open a public issue for security problems.

Report them privately through:

- GitHub's "Report a vulnerability" feature on the repository's Security tab
- Or contact the maintainers through the address listed in the repository profile

Include:

- OmniDev version (`omnidev version`)
- Your operating system
- Reproduction steps
- Expected impact

You can expect an acknowledgement within a week.

## Security model

OmniDev is local-first:

- It makes no network requests and sends no telemetry.
- It reads repository history only for the repositories you register or pass with `--path`.
- It never runs scripts from a repository and never modifies repository contents.
- Git operations go through the libgit2 API. No shell is invoked, so repository paths and commit data are never interpolated into commands.
- Removing a repository from OmniDev deletes only OmniDev's database rows.
- Report and configuration writes refuse to overwrite existing files unless you pass `--force` or confirm the prompt.
