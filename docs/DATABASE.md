# Database

OmniDev stores its data in a single SQLite file, `omnidev.db`, inside the platform data directory (or `$OMNIDEV_HOME/data` when that variable is set). The schema version is kept in SQLite's `user_version` pragma.

## Migrations

Migrations live in `migrations/` as numbered SQL files and are compiled into the binary. On startup OmniDev applies every migration newer than the database's version, each inside its own transaction. If the database is newer than the binary, OmniDev refuses to open it instead of risking damage.

## Tables

| Table | Purpose |
| --- | --- |
| `repositories` | Tracked repositories: name, canonical path, default flag, last head and sync time. Path is unique. A partial unique index allows at most one default. |
| `contributors` | One row per author email (lower-cased), holding the latest name seen. |
| `commits` | One row per commit per repository. Unique on `(repository_id, hash)`. Stores the local day, merge flag and line statistics. |
| `daily_activity` | Aggregate per repository and day, rebuilt after each sync. Feeds the heatmap and timeline. |
| `language_statistics` | Files and lines per language per repository, from the HEAD snapshot. |
| `goals` | One goal per kind. |
| `achievements` | Unlocked achievements and their unlock time. |
| `application_metadata` | Key-value settings such as the timezone mode and report counter. |

Foreign keys are enabled, and deleting a repository cascades to its commits, daily aggregates and language rows. Indexes cover `(repository_id, local_day)`, `local_day`, contributor and commit time.

## Integrity and recovery

- Each repository sync runs in one transaction. An interrupted sync rolls back.
- WAL mode and a five-second busy timeout let the dashboard read while a refresh writes.
- Commits already stored are never re-analysed, so a repeated refresh is cheap.
- `omnidev doctor` reports the schema version and whether it matches the binary.
