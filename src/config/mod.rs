pub mod settings;

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::errors::{OmniDevError, Result};

pub use settings::{SETTING_KEYS, Settings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        if let Some(root) = std::env::var_os("OMNIDEV_HOME") {
            return Ok(Self::from_root(Path::new(&root)));
        }
        let dirs = ProjectDirs::from("dev", "omnidev", "omnidev")
            .ok_or(OmniDevError::NoProjectDirectories)?;
        Ok(Self {
            config_file: dirs.config_dir().join("config.toml"),
            data_dir: dirs.data_dir().to_path_buf(),
        })
    }

    pub fn from_root(root: &Path) -> Self {
        Self {
            config_file: root.join("config.toml"),
            data_dir: root.join("data"),
        }
    }

    pub fn database_path(&self, settings: &Settings) -> PathBuf {
        self.data_dir.join(&settings.storage.database_name)
    }
}

pub fn read_settings(paths: &AppPaths) -> Result<Option<Settings>> {
    if !paths.config_file.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&paths.config_file)?;
    let settings: Settings = toml::from_str(&raw).map_err(|error| OmniDevError::InvalidConfig {
        path: paths.config_file.clone(),
        reason: error.message().to_owned(),
    })?;
    settings
        .validate()
        .map_err(|error| OmniDevError::InvalidConfig {
            path: paths.config_file.clone(),
            reason: error.to_string(),
        })?;
    Ok(Some(settings))
}

pub fn require_settings(paths: &AppPaths) -> Result<Settings> {
    read_settings(paths)?.ok_or(OmniDevError::NotInitialized)
}

pub fn write_settings(paths: &AppPaths, settings: &Settings) -> Result<()> {
    settings.validate()?;
    if let Some(parent) = paths.config_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = toml::to_string_pretty(settings)?;
    let temporary = paths.config_file.with_extension("toml.tmp");
    fs::write(&temporary, rendered)?;
    fs::rename(&temporary, &paths.config_file)?;
    Ok(())
}
