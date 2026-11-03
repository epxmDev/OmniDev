# Command reference

Every command accepts `--help`. Run `omnidev --version` for the version.

| Command | Purpose |
| --- | --- |
| `omnidev` | Welcome screen with commands and initialization status. |
| `omnidev init [--force]` | Create configuration and database. `--force` asks before replacing configuration; the database is kept. |
| `omnidev analyze [--path PATH]` | Analyze a repository (defaults to the current directory). It is registered for tracking if it is not already. |
| `omnidev stats [--range R \| --from D --to D] [--repo REPO]` | Developer statistics for a period. |
| `omnidev heatmap [--range R \| --from D --to D] [--repo REPO]` | Contribution heatmap, 365 days by default. |
| `omnidev dashboard` | Interactive dashboard. |
| `omnidev projects [list]` | List tracked repositories. |
| `omnidev projects add PATH [--name NAME]` | Register a repository. |
| `omnidev projects remove SELECTOR` | Stop tracking a repository. Files are never touched. |
| `omnidev projects default SELECTOR` | Set the default repository shown in the dashboard. |
| `omnidev projects scan [--dir DIR]... [--depth N]` | Register repositories found under directories. Defaults to `scan.directories`. Hidden directories and common build folders are skipped. |
| `omnidev commits [--author TEXT] [--search TEXT] [--repo REPO] [--range R] [--page N] [--per-page N] [--files]` | Browse commit history. |
| `omnidev languages [--repo REPO]` | Language distribution, estimated from line counts. |
| `omnidev report [--format json\|markdown\|csv] [--output FILE] [--force] [--range R] [--repo REPO]` | Generate a report. Writing to a file that exists needs `--force`. |
| `omnidev goals [list]` | Goal progress and achievements. |
| `omnidev goals set KIND TARGET` | Create or update a goal. Kinds: `weekly-commits`, `monthly-commits`, `languages`, `repository-maintenance`. |
| `omnidev goals remove KIND` | Remove a goal. |
| `omnidev config [show]` | Print configuration. |
| `omnidev config get KEY` / `set KEY VALUE` / `path` | Read, change or locate configuration values. |
| `omnidev refresh [--rebuild]` | Incrementally synchronize every tracked repository. `--rebuild` re-reads all history. |
| `omnidev version` | Version, platform, libgit2 and SQLite versions. |
| `omnidev doctor` | Check configuration, data directory, schema, default repository and every repository path. Exits non-zero when a check fails. |

## Ranges

`--range` accepts `7d`, `12w`, `6m`, `1y`, or `all`. `--from` and `--to` take `YYYY-MM-DD` dates and are inclusive. `--to` defaults to today.

## Dashboard keys

| Key | Action |
| --- | --- |
| Tab / Shift+Tab | Move focus between panels |
| ↑ / ↓ | Move the selection in the recent commits and repository lists |
| ← / → | Shift the heatmap back or forward by one week when it has focus; otherwise move focus |
| Enter | Expand or collapse the focused panel |
| R | Refresh all repositories in the background |
| H or ? | Show or hide help |
| Q or Esc | Quit; Esc first collapses an expanded panel |
| Ctrl+C | Quit |

## Exit codes

`0` on success. `1` when a command fails, including uninitialized use, invalid input, or a refresh where some repositories failed.
