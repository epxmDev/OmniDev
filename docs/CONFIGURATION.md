# Configuration

OmniDev reads `config.toml` from the platform configuration directory, or from `$OMNIDEV_HOME/config.toml` when that variable is set. Run `omnidev config path` to see the exact location.

Repositories are not stored in the configuration file. Register them with `omnidev projects add`.

## Example

```toml
[general]
theme = "dark"
timezone = "local"
auto_refresh = true

[analytics]
default_range = "365d"
include_merges = true
exclude_generated = true

[display]
show_colors = true
show_ascii_art = true
compact_mode = false

[storage]
database_name = "omnidev.db"

[scan]
directories = ["/home/you/code"]
```

## Keys

| Key | Values | Meaning |
| --- | --- | --- |
| `general.theme` | `dark` | Only the dark theme exists. |
| `general.timezone` | `local`, `utc` | `local` uses each commit's recorded author offset. `utc` uses UTC. Changing it rebuilds commit days on the next refresh. |
| `general.auto_refresh` | boolean | Reserved for background refresh in the dashboard. |
| `analytics.default_range` | `Nd`, `Nw`, `Nm`, `Ny`, `all` | Range used by `stats`, `commits` and `report` when none is given. |
| `analytics.include_merges` | boolean | Whether merge commits count as commits in statistics. |
| `analytics.exclude_generated` | boolean | Skip lock files, minified bundles and generated sources in language statistics. |
| `display.show_colors` | boolean | Enable terminal colours. `NO_COLOR` also disables them. |
| `display.show_ascii_art` | boolean | Show the logo on the welcome screen. |
| `display.compact_mode` | boolean | Reserved for denser layouts. |
| `storage.database_name` | plain file name | Name of the SQLite file inside the data directory. |
| `scan.directories` | list of paths | Directories `omnidev projects scan` searches by default. |

Edit values with `omnidev config set <key> <value>`. Invalid values are rejected and the file is left unchanged. Writes use a temporary file followed by a rename, so an interrupted write cannot corrupt the configuration.

## Environment variables

| Variable | Effect |
| --- | --- |
| `OMNIDEV_HOME` | Root directory for `config.toml` and `data/`. Useful for isolated testing. |
| `OMNIDEV_ASCII` | Force ASCII symbols instead of Unicode. |
| `OMNIDEV_LOG` | Log filter for diagnostics, for example `debug`. Defaults to `warn`. |
| `NO_COLOR` | Disable colours. |
