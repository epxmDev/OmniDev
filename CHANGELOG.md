# Changelog

All notable changes to OmniDev are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - Initial release

### Added

- Git repository analysis over all local and remote branches, with incremental synchronization.
- Developer statistics: commits, contributors, line changes, streaks, weekday and monthly activity, repository distribution and contribution frequency.
- 365-day contribution heatmap, available as a command and inside the dashboard.
- Interactive Ratatui dashboard with responsive layouts, a help overlay and background refresh.
- Repository management: register, list, remove, set default, and scan configured directories.
- Commit explorer with author, date, text and repository filters, pagination and changed-file listings.
- Language distribution estimated from HEAD line counts, excluding vendored, build, generated and binary files.
- JSON, Markdown and CSV reports.
- Goals and achievements computed from stored data.
- `doctor` diagnostics, `version`, and `config` management.
