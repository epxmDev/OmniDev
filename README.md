<div align="center">

```
   ____  __  __ _   _ ___ ____  _______     __
  / __ \|  \/  | \ | |_ _|  _ \| ____\ \   / /
 | |  | | |\/| |  \| || || | | |  _|  \ \ / /
 | |__| | |  | | |\  || || |_| | |___  \ V /
  \____/|_|  |_|_| \_|___|____/|_____|  \_/
```

**Track your code. Understand your growth.**

[![CI](https://github.com/YOUR_GITHUB_USER/omnidev/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_GITHUB_USER/omnidev/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/YOUR_GITHUB_USER/omnidev?include_prereleases)](https://github.com/YOUR_GITHUB_USER/omnidev/releases)

</div>

OmniDev is a local-first developer activity analytics platform for the terminal. It reads your Git history, computes statistics, language distribution, contribution heatmaps and reports, and shows them in an interactive dashboard. Everything runs on your machine: there is no telemetry, no account and no network access.

> Replace `YOUR_GITHUB_USER` in the badges above with your repository owner once the repository exists.

## Features

- **Repository analysis** over every local and remote branch, with incremental synchronization.
- **Developer statistics**: commits, contributors, insertions and deletions, streaks, averages, weekday and monthly activity, and repository distribution.
- **Contribution heatmap** for the last 365 days, in the terminal and in the dashboard.
- **Interactive dashboard** with responsive layouts, keyboard navigation, a help overlay and background refresh.
- **Commit explorer** with author, date, text and repository filters, pagination and changed-file lists.
- **Language distribution** estimated from line counts, excluding vendored, build, generated and binary files.
- **Reports** in JSON, Markdown and CSV.
- **Goals and achievements** based only on real data.
- **Repository management**: register, scan directories, set a default, and stop tracking without touching files.

## Screenshots

Screenshots of the dashboard and heatmap belong in `assets/screenshots/`. Capture them on your own terminal with `omnidev dashboard` and `omnidev heatmap`, then reference them here.

## Installation

### From a release

Download the archive for your platform from the [Releases page](https://github.com/YOUR_GITHUB_USER/omnidev/releases), verify it against `SHA256SUMS.txt`, and place the `omnidev` binary on your `PATH`.

### From source

You need Rust 1.85 or newer and a C compiler (SQLite is bundled).

```bash
git clone https://github.com/YOUR_GITHUB_USER/omnidev.git
cd omnidev
cargo install --path .
```

## Quick start

```bash
omnidev init                      # create configuration and database
cd ~/code/my-project
omnidev analyze                   # read this repository's history
omnidev stats --range 90d         # statistics for the last 90 days
omnidev heatmap                   # contribution heatmap
omnidev dashboard                 # interactive dashboard (press H for help)
```

Track several repositories:

```bash
omnidev projects add ~/code/api --name api
omnidev projects scan --dir ~/code
omnidev refresh                   # incremental update of everything tracked
```

## Command reference

| Command | Description |
| --- | --- |
| `init` | Create configuration and database |
| `analyze` | Analyze a repository and register it |
| `stats` | Developer statistics for a range |
| `heatmap` | Contribution heatmap |
| `dashboard` | Interactive terminal dashboard |
| `projects` | List, add, remove, set default and scan repositories |
| `commits` | Explore commit history |
| `languages` | Language distribution |
| `report` | Export JSON, Markdown or CSV |
| `goals` | Goals and achievements |
| `config` | Show or change configuration |
| `refresh` | Synchronize all tracked repositories |
| `doctor` | Check installation and repository access |
| `version` | Build and dependency versions |

The full reference, with every option, is in [docs/COMMANDS.md](docs/COMMANDS.md).

## Configuration

OmniDev reads `config.toml` from your platform configuration directory. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for every key. Repositories are stored in the database, not the configuration file.

## Examples

```bash
omnidev commits --author "Ada" --range 30d --files
omnidev report --format csv --output activity.csv
omnidev report --format json --range 1y --repo api --output api-year.json
omnidev goals set weekly-commits 5
omnidev stats --from 2026-01-01 --to 2026-03-31
```

A small library example that prints statistics for one path is in [`examples/`](examples/).

## Architecture overview

OmniDev is a single crate with a library and a thin binary. Git access goes through `git2`, storage through `rusqlite` with bundled SQLite, the interface through `ratatui` and `crossterm`. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for module boundaries and data flow, and [docs/DATABASE.md](docs/DATABASE.md) for the schema.

## Development

```bash
cargo build
cargo run -- --help
```

## Testing

```bash
cargo test --all-targets
```

The suite has three layers:

- **Unit tests** cover statistics, streaks, heatmap levels, language detection, date ranges, formatting and report rendering.
- **Integration tests** (`tests/sync_and_store.rs`) build Git repositories in temporary directories with explicit timestamps and time zones, then check synchronization, merges, empty and missing repositories, corruption handling, rebuilds and timezone behaviour.
- **CLI tests** (`tests/cli.rs`) run the compiled binary against an isolated `OMNIDEV_HOME`.

Tests never read your own repositories.

## Roadmap

- Background auto-refresh in the dashboard (the setting exists; the scheduler does not yet).
- Screenshot and asciinema demonstrations.
- Per-file ownership and churn analysis.
- Configurable language map.
- Packaged builds for Homebrew and winget.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for the code rules (no comments in source, no `unwrap` in production code) and the checks to run before opening a pull request.

## Security

See [SECURITY.md](SECURITY.md) for the security model and how to report a vulnerability.

## License

MIT. See [LICENSE](LICENSE).
