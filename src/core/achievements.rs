use chrono::Utc;

use crate::core::activity::format_timestamp;
use crate::database::{Database, goals};
use crate::errors::Result;

pub const REPORTS_METADATA_KEY: &str = "reports_generated";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Achievement {
    FirstHundredCommits,
    SevenDayStreak,
    FiveRepositories,
    FirstReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementInput {
    pub total_commits: u64,
    pub longest_streak_days: u32,
    pub tracked_repositories: u64,
    pub reports_generated: u64,
}

impl Achievement {
    pub const ALL: [Achievement; 4] = [
        Achievement::FirstHundredCommits,
        Achievement::SevenDayStreak,
        Achievement::FiveRepositories,
        Achievement::FirstReport,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Achievement::FirstHundredCommits => "first-100-commits",
            Achievement::SevenDayStreak => "seven-day-streak",
            Achievement::FiveRepositories => "five-repositories",
            Achievement::FirstReport => "first-report",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Achievement::FirstHundredCommits => "Century",
            Achievement::SevenDayStreak => "Steady Hands",
            Achievement::FiveRepositories => "Portfolio",
            Achievement::FirstReport => "Analyst",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Achievement::FirstHundredCommits => "Reached 100 commits across tracked repositories",
            Achievement::SevenDayStreak => "Committed on seven consecutive days",
            Achievement::FiveRepositories => "Tracking five or more repositories",
            Achievement::FirstReport => "Generated your first report",
        }
    }

    pub fn is_earned(self, input: &AchievementInput) -> bool {
        match self {
            Achievement::FirstHundredCommits => input.total_commits >= 100,
            Achievement::SevenDayStreak => input.longest_streak_days >= 7,
            Achievement::FiveRepositories => input.tracked_repositories >= 5,
            Achievement::FirstReport => input.reports_generated >= 1,
        }
    }
}

pub fn evaluate(database: &Database, input: &AchievementInput) -> Result<Vec<Achievement>> {
    let timestamp = format_timestamp(Utc::now());
    let mut newly_unlocked = Vec::new();
    for achievement in Achievement::ALL {
        if achievement.is_earned(input)
            && goals::unlock_achievement(database.connection(), achievement.code(), &timestamp)?
        {
            newly_unlocked.push(achievement);
        }
    }
    Ok(newly_unlocked)
}

pub fn increment_reports(database: &Database) -> Result<u64> {
    let current = goals::get_metadata(database.connection(), REPORTS_METADATA_KEY)?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let next = current + 1;
    goals::set_metadata(
        database.connection(),
        REPORTS_METADATA_KEY,
        &next.to_string(),
    )?;
    Ok(next)
}

pub fn reports_generated(database: &Database) -> Result<u64> {
    Ok(
        goals::get_metadata(database.connection(), REPORTS_METADATA_KEY)?
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0),
    )
}
