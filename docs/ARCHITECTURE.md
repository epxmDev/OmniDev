# Architecture

OmniDev is one Rust crate with a library (`src/lib.rs`) and a thin binary (`src/main.rs`). The binary only calls `omnidev::run()`, so the whole application is testable as a library.

## Layers

```
cli ──────► core ──────► database
 │           │              ▲
 │           ▼              │
 │        git ─────────────┘ (sync writes through database)
 │           │
 ├──► analytics (languages, heatmap, reports)
 ├──► dashboard (Ratatui UI over core snapshots)
 ├──► output (theme, symbols, tables, formatting)
 ├──► config (TOML settings, paths)
 └──► errors
```

- **cli**: clap argument definitions, command handlers, and the `Context` that opens configuration and the database.
- **core**: domain logic that does not depend on terminal output. `statistics` is a pure function over activity records; `activity` handles date ranges, streaks helpers and local-day conversion; `analyzer` synchronizes Git history into the database; `goals`, `achievements` and `diagnostics` build on stored data.
- **git**: thin wrappers over `git2`. Revision walking, diff statistics, branch summaries and changed-file lists. Nothing here writes to a repository.
- **database**: SQLite access. `connection` configures pragmas, `migrations` applies numbered SQL files from `migrations/`, and the other modules hold typed queries.
- **analytics**: language detection from HEAD trees, the heatmap grid, and report rendering in JSON, Markdown and CSV.
- **dashboard**: `app` holds state and key handling, `ui` renders, and `mod` runs the event loop, restores the terminal on exit and runs refreshes on a background thread.
- **output**: colour and ASCII fallbacks, comfy-table construction, and number formatting.

## Data flow for `omnidev refresh`

1. `Context::open` reads `config.toml` and opens the database, applying migrations.
2. `analyzer::apply_timezone_mode` checks whether the stored timezone mode still matches configuration; a change clears commit data so days are recomputed.
3. For each repository, `sync_repository`:
   - loads the hashes already stored for that repository;
   - walks every local and remote branch with a revision walk, skipping known hashes;
   - computes diff statistics only for new commits (merge commits contribute no line counts);
   - writes commits, refreshes `daily_activity`, recomputes language tallies when the HEAD changed, and records the sync time, all in one transaction.
4. Achievements are evaluated and newly unlocked ones are reported.

## Key decisions

- **Local days**: each commit stores the author's recorded UTC offset, and the day is derived from it (or UTC when `general.timezone = "utc"`). Commit timestamps are not working hours, and the statistics output says so.
- **Merges**: merge commits are counted as commits but add no insertions or deletions, because their changes are already attributed to the branch commits.
- **Incremental sync**: a stored commit is never re-diffed. Rewritten history leaves unreachable commits in the database until `omnidev refresh --rebuild` is run.
- **Dashboard concurrency**: refreshes open their own SQLite connection in a worker thread. WAL mode allows the UI to keep reading while the worker writes.
- **Terminal safety**: a guard restores raw mode and the alternate screen on drop, which covers normal exit, errors and panics.
