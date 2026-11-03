use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::activity::parse_range;
use crate::errors::{OmniDevError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimezoneMode {
    #[default]
    Local,
    Utc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeneralSettings {
    pub theme: Theme,
    pub timezone: TimezoneMode,
    pub auto_refresh: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            timezone: TimezoneMode::Local,
            auto_refresh: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalyticsSettings {
    pub default_range: String,
    pub include_merges: bool,
    pub exclude_generated: bool,
}

impl Default for AnalyticsSettings {
    fn default() -> Self {
        Self {
            default_range: "365d".to_owned(),
            include_merges: true,
            exclude_generated: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DisplaySettings {
    pub show_colors: bool,
    pub show_ascii_art: bool,
    pub compact_mode: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            show_colors: true,
            show_ascii_art: true,
            compact_mode: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageSettings {
    pub database_name: String,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            database_name: "omnidev.db".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScanSettings {
    pub directories: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub general: GeneralSettings,
    pub analytics: AnalyticsSettings,
    pub display: DisplaySettings,
    pub storage: StorageSettings,
    pub scan: ScanSettings,
}

pub const SETTING_KEYS: &[&str] = &[
    "general.theme",
    "general.timezone",
    "general.auto_refresh",
    "analytics.default_range",
    "analytics.include_merges",
    "analytics.exclude_generated",
    "display.show_colors",
    "display.show_ascii_art",
    "display.compact_mode",
    "storage.database_name",
];

impl Settings {
    pub fn validate(&self) -> Result<()> {
        parse_range(&self.analytics.default_range).map_err(|_| OmniDevError::InvalidSetting {
            key: "analytics.default_range".to_owned(),
            reason: "expected a value such as 30d, 12w, 6m, 1y or all".to_owned(),
        })?;
        let name = &self.storage.database_name;
        if name.trim().is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(OmniDevError::InvalidSetting {
                key: "storage.database_name".to_owned(),
                reason: "must be a plain file name without path separators".to_owned(),
            });
        }
        Ok(())
    }

    pub fn set_value(&mut self, key: &str, raw: &str) -> Result<()> {
        let invalid = |reason: &str| OmniDevError::InvalidSetting {
            key: key.to_owned(),
            reason: reason.to_owned(),
        };
        match key {
            "general.theme" => match raw {
                "dark" => self.general.theme = Theme::Dark,
                _ => return Err(invalid("only the 'dark' theme is available")),
            },
            "general.timezone" => {
                self.general.timezone = match raw {
                    "local" => TimezoneMode::Local,
                    "utc" => TimezoneMode::Utc,
                    _ => return Err(invalid("expected 'local' or 'utc'")),
                };
            }
            "general.auto_refresh" => self.general.auto_refresh = parse_bool(key, raw)?,
            "analytics.default_range" => {
                parse_range(raw)
                    .map_err(|_| invalid("expected a value such as 30d, 12w, 6m, 1y or all"))?;
                raw.clone_into(&mut self.analytics.default_range);
            }
            "analytics.include_merges" => self.analytics.include_merges = parse_bool(key, raw)?,
            "analytics.exclude_generated" => {
                self.analytics.exclude_generated = parse_bool(key, raw)?
            }
            "display.show_colors" => self.display.show_colors = parse_bool(key, raw)?,
            "display.show_ascii_art" => self.display.show_ascii_art = parse_bool(key, raw)?,
            "display.compact_mode" => self.display.compact_mode = parse_bool(key, raw)?,
            "storage.database_name" => {
                let previous = std::mem::replace(&mut self.storage.database_name, raw.to_owned());
                if let Err(error) = self.validate() {
                    self.storage.database_name = previous;
                    return Err(error);
                }
            }
            _ => return Err(OmniDevError::UnknownSetting(key.to_owned())),
        }
        Ok(())
    }

    pub fn get_value(&self, key: &str) -> Result<String> {
        let value = match key {
            "general.theme" => "dark".to_owned(),
            "general.timezone" => match self.general.timezone {
                TimezoneMode::Local => "local".to_owned(),
                TimezoneMode::Utc => "utc".to_owned(),
            },
            "general.auto_refresh" => self.general.auto_refresh.to_string(),
            "analytics.default_range" => self.analytics.default_range.clone(),
            "analytics.include_merges" => self.analytics.include_merges.to_string(),
            "analytics.exclude_generated" => self.analytics.exclude_generated.to_string(),
            "display.show_colors" => self.display.show_colors.to_string(),
            "display.show_ascii_art" => self.display.show_ascii_art.to_string(),
            "display.compact_mode" => self.display.compact_mode.to_string(),
            "storage.database_name" => self.storage.database_name.clone(),
            _ => return Err(OmniDevError::UnknownSetting(key.to_owned())),
        };
        Ok(value)
    }
}

fn parse_bool(key: &str, raw: &str) -> Result<bool> {
    match raw {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(OmniDevError::InvalidSetting {
            key: key.to_owned(),
            reason: "expected true or false".to_owned(),
        }),
    }
}
