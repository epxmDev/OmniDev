use chrono::Utc;

use crate::analytics::languages;
use crate::config::Settings;
use crate::config::settings::TimezoneMode;
use crate::core::activity::{format_timestamp, local_day};
use crate::core::model::CommitInfo;
use crate::database::repositories::{self, RepositoryRecord};
use crate::database::{Database, commits, goals, languages as language_store};
use crate::errors::{OmniDevError, Result};
use crate::git;

const TIMEZONE_METADATA_KEY: &str = "timezone_mode";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    pub new_commits: usize,
    pub total_commits: u64,
    pub head_commit: Option<String>,
}

pub fn timezone_label(mode: TimezoneMode) -> &'static str {
    match mode {
        TimezoneMode::Local => "local",
        TimezoneMode::Utc => "utc",
    }
}

pub fn timezone_mismatch(database: &Database, settings: &Settings) -> Result<bool> {
    let stored = goals::get_metadata(database.connection(), TIMEZONE_METADATA_KEY)?;
    Ok(matches!(stored, Some(value) if value != timezone_label(settings.general.timezone)))
}

pub fn apply_timezone_mode(database: &mut Database, settings: &Settings) -> Result<()> {
    let current = timezone_label(settings.general.timezone);
    let stored = goals::get_metadata(database.connection(), TIMEZONE_METADATA_KEY)?;
    if stored.as_deref() == Some(current) {
        return Ok(());
    }
    if stored.is_some() {
        let transaction = database.connection_mut().transaction()?;
        transaction.execute("DELETE FROM daily_activity", [])?;
        transaction.execute("DELETE FROM commits", [])?;
        goals::set_metadata(&transaction, TIMEZONE_METADATA_KEY, current)?;
        transaction.commit()?;
    } else {
        goals::set_metadata(database.connection(), TIMEZONE_METADATA_KEY, current)?;
    }
    Ok(())
}

pub fn reset_repository(database: &mut Database, repository_id: i64) -> Result<()> {
    let transaction = database.connection_mut().transaction()?;
    commits::clear_repository_commits(&transaction, repository_id)?;
    language_store::replace_repository_languages(&transaction, repository_id, &[])?;
    transaction.commit()?;
    Ok(())
}

fn commit_day(commit: &CommitInfo, mode: TimezoneMode) -> Result<chrono::NaiveDate> {
    let offset = match mode {
        TimezoneMode::Local => commit.utc_offset_minutes,
        TimezoneMode::Utc => 0,
    };
    local_day(commit.committed_at, offset)
}

pub fn sync_repository(
    database: &mut Database,
    record: &RepositoryRecord,
    settings: &Settings,
) -> Result<SyncOutcome> {
    if !record.path.is_dir() {
        return Err(OmniDevError::RepositoryMissing(record.path.clone()));
    }
    let repository = git::repository::open(&record.path)?;
    let mode = settings.general.timezone;
    let known = commits::known_hashes(database.connection(), record.id)?;
    let transaction = database.connection_mut().transaction()?;
    let new_commits = git::commits::walk_new_commits(&repository, &known, |commit| {
        let day = commit_day(&commit, mode)?;
        commits::insert_commit(&transaction, record.id, &commit, day)
    })?;
    let head = git::repository::head_hash(&repository)?;
    if new_commits > 0 || head != record.head_commit {
        commits::rebuild_daily_activity(&transaction, record.id)?;
        let tallies =
            languages::tally_repository(&repository, settings.analytics.exclude_generated)?;
        language_store::replace_repository_languages(&transaction, record.id, &tallies)?;
    }
    repositories::record_sync(
        &transaction,
        record.id,
        head.as_deref(),
        &format_timestamp(Utc::now()),
    )?;
    transaction.commit()?;
    let total_commits = commits::count_commits(
        database.connection(),
        &commits::CommitFilter {
            repository_id: Some(record.id),
            include_merges: true,
            ..commits::CommitFilter::default()
        },
    )?;
    Ok(SyncOutcome {
        new_commits,
        total_commits,
        head_commit: head,
    })
}
