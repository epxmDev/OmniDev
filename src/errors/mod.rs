use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, OmniDevError>;

#[derive(Debug, Error)]
pub enum OmniDevError {
    #[error(
        "OmniDev is not initialized. Run `omnidev init` to create the configuration and database."
    )]
    NotInitialized,
    #[error("invalid configuration at {path}: {reason}")]
    InvalidConfig { path: PathBuf, reason: String },
    #[error("invalid configuration value for '{key}': {reason}")]
    InvalidSetting { key: String, reason: String },
    #[error("unknown configuration key '{0}'")]
    UnknownSetting(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("repository not found: {0}")]
    RepositoryNotFound(String),
    #[error("repository name '{0}' is ambiguous; use its path instead")]
    AmbiguousRepository(String),
    #[error("no default repository is set; use `omnidev projects default <name>`")]
    NoDefaultRepository,
    #[error("the database schema version {found} is newer than this build supports ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },
    #[error(
        "repository path {0} no longer exists; restore it or stop tracking it with `omnidev projects remove`"
    )]
    RepositoryMissing(PathBuf),
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] std::fmt::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    TomlDecode(#[from] toml::de::Error),
    #[error(transparent)]
    TomlEncode(#[from] toml::ser::Error),
    #[error("could not determine application directories for this platform")]
    NoProjectDirectories,
}
