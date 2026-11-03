use std::fs;

use crate::config::{AppPaths, read_settings};
use crate::core::analyzer;
use crate::database::{Database, migrations, repositories};
use crate::git;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub level: Level,
    pub detail: String,
}

fn check(name: &str, level: Level, detail: impl Into<String>) -> Check {
    Check {
        name: name.to_owned(),
        level,
        detail: detail.into(),
    }
}

pub fn run(paths: &AppPaths) -> Vec<Check> {
    let mut checks = Vec::new();
    let libgit2 = git2::Version::get().libgit2_version();
    checks.push(check(
        "git engine",
        Level::Pass,
        format!("libgit2 {}.{}.{}", libgit2.0, libgit2.1, libgit2.2),
    ));

    let settings = match read_settings(paths) {
        Ok(Some(settings)) => {
            checks.push(check(
                "configuration",
                Level::Pass,
                paths.config_file.display().to_string(),
            ));
            Some(settings)
        }
        Ok(None) => {
            checks.push(check(
                "configuration",
                Level::Warn,
                "not found; run `omnidev init`",
            ));
            None
        }
        Err(error) => {
            checks.push(check("configuration", Level::Fail, error.to_string()));
            None
        }
    };

    let data_check = match fs::create_dir_all(&paths.data_dir) {
        Ok(()) => {
            let probe = paths.data_dir.join(".omnidev-write-probe");
            match fs::write(&probe, b"ok").and_then(|()| fs::remove_file(&probe)) {
                Ok(()) => check(
                    "data directory",
                    Level::Pass,
                    paths.data_dir.display().to_string(),
                ),
                Err(error) => check(
                    "data directory",
                    Level::Fail,
                    format!("not writable: {error}"),
                ),
            }
        }
        Err(error) => check("data directory", Level::Fail, error.to_string()),
    };
    checks.push(data_check);

    let Some(settings) = settings else {
        return checks;
    };

    let database_path = paths.database_path(&settings);
    match Database::open(&database_path) {
        Ok(database) => {
            let version = database.schema_version().unwrap_or(-1);
            let latest = migrations::latest_version();
            let level = if version == latest {
                Level::Pass
            } else {
                Level::Fail
            };
            checks.push(check(
                "database schema",
                level,
                format!(
                    "version {version} (expected {latest}) at {}",
                    database_path.display()
                ),
            ));
            push_repository_checks(&database, &settings, &mut checks);
        }
        Err(error) => checks.push(check("database", Level::Fail, error.to_string())),
    }
    checks
}

fn push_repository_checks(
    database: &Database,
    settings: &crate::config::Settings,
    checks: &mut Vec<Check>,
) {
    let records = match repositories::list(database.connection()) {
        Ok(records) => records,
        Err(error) => {
            checks.push(check("repositories", Level::Fail, error.to_string()));
            return;
        }
    };
    if records.is_empty() {
        checks.push(check("repositories", Level::Warn, "none tracked yet"));
        return;
    }
    let defaults = records.iter().filter(|record| record.is_default).count();
    let default_level = if defaults == 1 {
        Level::Pass
    } else {
        Level::Warn
    };
    checks.push(check(
        "default repository",
        default_level,
        if defaults == 1 {
            "one default is set".to_owned()
        } else {
            "no default repository set".to_owned()
        },
    ));
    for record in &records {
        match git::repository::open(&record.path) {
            Ok(_) => checks.push(check(
                &format!("repository {}", record.name),
                Level::Pass,
                record.path.display().to_string(),
            )),
            Err(error) => checks.push(check(
                &format!("repository {}", record.name),
                Level::Fail,
                format!("{} ({error})", record.path.display()),
            )),
        }
    }
    if analyzer::timezone_mismatch(database, settings).unwrap_or(false) {
        checks.push(check(
            "timezone setting",
            Level::Warn,
            "changed since last analysis; run `omnidev refresh`",
        ));
    }
}
