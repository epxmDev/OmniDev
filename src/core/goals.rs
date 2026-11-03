use chrono::{Datelike, Days, NaiveDate, TimeDelta, Utc};

use crate::core::activity::{DateRange, format_timestamp};
use crate::database::commits::{self, CommitFilter};
use crate::database::{Database, goals};
use crate::errors::{OmniDevError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalKind {
    WeeklyCommits,
    MonthlyCommits,
    Languages,
    RepositoryMaintenance,
}

impl GoalKind {
    pub const ALL: [GoalKind; 4] = [
        GoalKind::WeeklyCommits,
        GoalKind::MonthlyCommits,
        GoalKind::Languages,
        GoalKind::RepositoryMaintenance,
    ];

    pub fn key(self) -> &'static str {
        match self {
            GoalKind::WeeklyCommits => "weekly-commits",
            GoalKind::MonthlyCommits => "monthly-commits",
            GoalKind::Languages => "languages",
            GoalKind::RepositoryMaintenance => "repository-maintenance",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            GoalKind::WeeklyCommits => "commits this week (Monday to today)",
            GoalKind::MonthlyCommits => "commits this month",
            GoalKind::Languages => "languages detected across tracked repositories",
            GoalKind::RepositoryMaintenance => "repositories synced in the last 7 days",
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.key() == input)
            .ok_or_else(|| {
                let valid = Self::ALL.map(GoalKind::key).join(", ");
                OmniDevError::InvalidInput(format!(
                    "unknown goal '{input}'; expected one of: {valid}"
                ))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalContext {
    pub commits_this_week: u64,
    pub commits_this_month: u64,
    pub languages: u64,
    pub repositories_synced_recently: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalProgress {
    pub kind: GoalKind,
    pub target: u64,
    pub current: u64,
}

impl GoalProgress {
    pub fn percent(&self) -> f64 {
        if self.target == 0 {
            return 0.0;
        }
        (self.current as f64 / self.target as f64 * 100.0).min(100.0)
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.target
    }
}

pub fn progress(kind: GoalKind, target: u64, context: &GoalContext) -> GoalProgress {
    let current = match kind {
        GoalKind::WeeklyCommits => context.commits_this_week,
        GoalKind::MonthlyCommits => context.commits_this_month,
        GoalKind::Languages => context.languages,
        GoalKind::RepositoryMaintenance => context.repositories_synced_recently,
    };
    GoalProgress {
        kind,
        target,
        current,
    }
}

pub fn week_start(today: NaiveDate) -> NaiveDate {
    today - Days::new(u64::from(today.weekday().num_days_from_monday()))
}

pub fn month_start(today: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today)
}

pub fn build_context(
    database: &Database,
    today: NaiveDate,
    include_merges: bool,
) -> Result<GoalContext> {
    let week = DateRange::new(week_start(today), today)?;
    let month = DateRange::new(month_start(today), today)?;
    let commits_this_week = commits::count_commits(
        database.connection(),
        &CommitFilter {
            range: Some(week),
            include_merges,
            ..CommitFilter::default()
        },
    )?;
    let commits_this_month = commits::count_commits(
        database.connection(),
        &CommitFilter {
            range: Some(month),
            include_merges,
            ..CommitFilter::default()
        },
    )?;
    let languages =
        crate::database::languages::aggregate_languages(database.connection(), None)?.len() as u64;
    let cutoff = format_timestamp(Utc::now() - TimeDelta::days(7));
    let repositories_synced_recently = crate::database::repositories::list(database.connection())?
        .iter()
        .filter(|record| {
            record
                .last_synced_at
                .as_deref()
                .is_some_and(|synced| synced >= cutoff.as_str())
        })
        .count() as u64;
    Ok(GoalContext {
        commits_this_week,
        commits_this_month,
        languages,
        repositories_synced_recently,
    })
}

pub fn set_goal(database: &Database, kind: GoalKind, target: u64) -> Result<()> {
    if target == 0 {
        return Err(OmniDevError::InvalidInput(
            "goal target must be greater than zero".to_owned(),
        ));
    }
    goals::upsert_goal(
        database.connection(),
        kind.key(),
        target,
        &format_timestamp(Utc::now()),
    )
}
