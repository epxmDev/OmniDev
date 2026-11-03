CREATE TABLE repositories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    head_commit TEXT,
    added_at TEXT NOT NULL,
    last_synced_at TEXT
);

CREATE UNIQUE INDEX idx_repositories_single_default ON repositories(is_default) WHERE is_default = 1;

CREATE TABLE contributors (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL
);

CREATE TABLE commits (
    id INTEGER PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    hash TEXT NOT NULL,
    contributor_id INTEGER NOT NULL REFERENCES contributors(id),
    author_name TEXT NOT NULL,
    committed_at INTEGER NOT NULL,
    utc_offset_minutes INTEGER NOT NULL,
    local_day TEXT NOT NULL,
    summary TEXT NOT NULL,
    is_merge INTEGER NOT NULL CHECK (is_merge IN (0, 1)),
    insertions INTEGER NOT NULL,
    deletions INTEGER NOT NULL,
    files_changed INTEGER NOT NULL,
    UNIQUE (repository_id, hash)
);

CREATE INDEX idx_commits_repository_day ON commits(repository_id, local_day);
CREATE INDEX idx_commits_day ON commits(local_day);
CREATE INDEX idx_commits_contributor ON commits(contributor_id);
CREATE INDEX idx_commits_committed_at ON commits(committed_at);

CREATE TABLE daily_activity (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    day TEXT NOT NULL,
    commits INTEGER NOT NULL,
    merge_commits INTEGER NOT NULL,
    insertions INTEGER NOT NULL,
    deletions INTEGER NOT NULL,
    PRIMARY KEY (repository_id, day)
);

CREATE INDEX idx_daily_activity_day ON daily_activity(day);

CREATE TABLE language_statistics (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    language TEXT NOT NULL,
    files INTEGER NOT NULL,
    lines INTEGER NOT NULL,
    PRIMARY KEY (repository_id, language)
);

CREATE TABLE goals (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL UNIQUE,
    target INTEGER NOT NULL CHECK (target > 0),
    created_at TEXT NOT NULL
);

CREATE TABLE achievements (
    code TEXT PRIMARY KEY,
    unlocked_at TEXT NOT NULL
);

CREATE TABLE application_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
