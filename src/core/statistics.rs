use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{Datelike, NaiveDate, Weekday};
use serde::Serialize;

use crate::core::activity::DateRange;
use crate::core::model::ActivityRecord;

#[derive(Debug, Clone, Copy)]
pub struct StatisticsOptions {
    pub include_merges: bool,
    pub period: Option<DateRange>,
    pub today: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayActivity {
    pub day: NaiveDate,
    pub commits: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeekdayActivity {
    pub weekday: String,
    pub commits: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MonthActivity {
    pub month: String,
    pub commits: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RepositoryShare {
    pub name: String,
    pub commits: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeveloperStatistics {
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub observed_days: u64,
    pub total_commits: u64,
    pub total_repositories: u64,
    pub total_contributors: u64,
    pub insertions: u64,
    pub deletions: u64,
    pub net_lines: i64,
    pub active_days: u64,
    pub most_active_day: Option<DayActivity>,
    pub most_active_weekday: Option<WeekdayActivity>,
    pub longest_streak_days: u32,
    pub current_streak_days: u32,
    pub average_commits_per_day: f64,
    pub average_commits_per_week: f64,
    pub contribution_frequency: f64,
    pub monthly_activity: Vec<MonthActivity>,
    pub repository_distribution: Vec<RepositoryShare>,
}

pub fn compute(records: &[ActivityRecord], options: StatisticsOptions) -> DeveloperStatistics {
    let counted: Vec<&ActivityRecord> = records
        .iter()
        .filter(|record| options.include_merges || !record.is_merge)
        .filter(|record| {
            options
                .period
                .is_none_or(|range| range.contains(record.day))
        })
        .collect();

    let observed_days = match options.period {
        Some(range) => range.day_count().max(0) as u64,
        None => observed_span(&counted),
    };

    let total_commits = counted.len() as u64;
    let insertions: u64 = counted.iter().map(|record| record.insertions).sum();
    let deletions: u64 = counted.iter().map(|record| record.deletions).sum();

    let mut per_day: BTreeMap<NaiveDate, u64> = BTreeMap::new();
    let mut per_month: BTreeMap<String, u64> = BTreeMap::new();
    let mut per_weekday = [0_u64; 7];
    let mut per_repository: HashMap<i64, (String, u64)> = HashMap::new();
    let mut contributors: HashSet<&str> = HashSet::new();
    for record in &counted {
        *per_day.entry(record.day).or_insert(0) += 1;
        *per_month
            .entry(record.day.format("%Y-%m").to_string())
            .or_insert(0) += 1;
        per_weekday[record.day.weekday().num_days_from_monday() as usize] += 1;
        let entry = per_repository
            .entry(record.repository_id)
            .or_insert_with(|| (record.repository_name.clone(), 0));
        entry.1 += 1;
        contributors.insert(record.author_email.as_str());
    }

    let active_days: BTreeSet<NaiveDate> = per_day.keys().copied().collect();
    let most_active_day = per_day
        .iter()
        .max_by(|left, right| left.1.cmp(right.1).then(right.0.cmp(left.0)))
        .map(|(day, commits)| DayActivity {
            day: *day,
            commits: *commits,
        });
    let most_active_weekday = per_weekday
        .iter()
        .enumerate()
        .filter(|(_, commits)| **commits > 0)
        .max_by(|left, right| left.1.cmp(right.1).then(right.0.cmp(&left.0)))
        .map(|(index, commits)| WeekdayActivity {
            weekday: weekday_name(index),
            commits: *commits,
        });

    let longest_streak_days = longest_streak(&active_days);
    let current_streak_days = current_streak(&active_days, options.today);

    let average_commits_per_day = ratio(total_commits as f64, observed_days as f64);
    let average_commits_per_week = average_commits_per_day * 7.0;
    let contribution_frequency = ratio(active_days.len() as f64, observed_days as f64);

    let mut repository_distribution: Vec<RepositoryShare> = per_repository
        .into_values()
        .map(|(name, commits)| RepositoryShare {
            name,
            commits,
            percent: ratio(commits as f64, total_commits as f64) * 100.0,
        })
        .collect();
    repository_distribution.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then(left.name.cmp(&right.name))
    });

    let total_repositories = repository_distribution.len() as u64;
    let monthly_activity = per_month
        .into_iter()
        .map(|(month, commits)| MonthActivity { month, commits })
        .collect();

    DeveloperStatistics {
        period_start: options
            .period
            .map(|range| range.start)
            .or_else(|| per_day.keys().next().copied()),
        period_end: options
            .period
            .map(|range| range.end)
            .or_else(|| per_day.keys().last().copied()),
        observed_days,
        total_commits,
        total_repositories,
        total_contributors: contributors.len() as u64,
        insertions,
        deletions,
        net_lines: insertions as i64 - deletions as i64,
        active_days: active_days.len() as u64,
        most_active_day,
        most_active_weekday,
        longest_streak_days,
        current_streak_days,
        average_commits_per_day,
        average_commits_per_week,
        contribution_frequency,
        monthly_activity,
        repository_distribution,
    }
}

fn observed_span(records: &[&ActivityRecord]) -> u64 {
    let first = records.iter().map(|record| record.day).min();
    let last = records.iter().map(|record| record.day).max();
    match (first, last) {
        (Some(first), Some(last)) => ((last - first).num_days() + 1).max(0) as u64,
        _ => 0,
    }
}

fn ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

pub fn longest_streak(active_days: &BTreeSet<NaiveDate>) -> u32 {
    let mut longest = 0_u32;
    let mut current = 0_u32;
    let mut previous: Option<NaiveDate> = None;
    for day in active_days {
        current = match previous {
            Some(last) if (*day - last).num_days() == 1 => current + 1,
            _ => 1,
        };
        longest = longest.max(current);
        previous = Some(*day);
    }
    longest
}

pub fn current_streak(active_days: &BTreeSet<NaiveDate>, today: NaiveDate) -> u32 {
    let Some(last) = active_days.last().copied() else {
        return 0;
    };
    if (today - last).num_days() > 1 {
        return 0;
    }
    let mut streak = 0_u32;
    let mut cursor = last;
    while active_days.contains(&cursor) {
        streak += 1;
        match cursor.pred_opt() {
            Some(previous) => cursor = previous,
            None => break,
        }
    }
    streak
}

fn weekday_name(index: usize) -> String {
    let weekday = match index {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    };
    weekday.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap_or(NaiveDate::MIN)
    }

    fn record(
        repository_id: i64,
        name: &str,
        text: &str,
        email: &str,
        merge: bool,
    ) -> ActivityRecord {
        ActivityRecord {
            repository_id,
            repository_name: name.to_owned(),
            day: day(text),
            author_email: email.to_owned(),
            insertions: 10,
            deletions: 4,
            is_merge: merge,
        }
    }

    fn options(include_merges: bool) -> StatisticsOptions {
        StatisticsOptions {
            include_merges,
            period: None,
            today: day("2026-06-30"),
        }
    }

    #[test]
    fn longest_streak_counts_consecutive_days_only() {
        let days: BTreeSet<NaiveDate> = [
            "2026-01-01",
            "2026-01-02",
            "2026-01-03",
            "2026-01-07",
            "2026-01-08",
        ]
        .iter()
        .map(|text| day(text))
        .collect();
        assert_eq!(longest_streak(&days), 3);
    }

    #[test]
    fn current_streak_survives_until_yesterday() {
        let days: BTreeSet<NaiveDate> = ["2026-03-01", "2026-03-02", "2026-03-03"]
            .iter()
            .map(|text| day(text))
            .collect();
        assert_eq!(current_streak(&days, day("2026-03-03")), 3);
        assert_eq!(current_streak(&days, day("2026-03-04")), 3);
        assert_eq!(current_streak(&days, day("2026-03-05")), 0);
    }

    #[test]
    fn streak_crosses_month_and_year_boundaries() {
        let days: BTreeSet<NaiveDate> = ["2025-12-31", "2026-01-01", "2026-01-02"]
            .iter()
            .map(|text| day(text))
            .collect();
        assert_eq!(longest_streak(&days), 3);
    }

    #[test]
    fn merges_are_excluded_when_requested() {
        let records = vec![
            record(1, "api", "2026-02-02", "a@example.com", false),
            record(1, "api", "2026-02-02", "a@example.com", true),
        ];
        assert_eq!(compute(&records, options(true)).total_commits, 2);
        assert_eq!(compute(&records, options(false)).total_commits, 1);
    }

    #[test]
    fn aggregates_totals_contributors_and_repositories() {
        let records = vec![
            record(1, "api", "2026-02-02", "a@example.com", false),
            record(1, "api", "2026-02-03", "b@example.com", false),
            record(2, "web", "2026-02-03", "a@example.com", false),
        ];
        let statistics = compute(&records, options(true));
        assert_eq!(statistics.total_commits, 3);
        assert_eq!(statistics.total_repositories, 2);
        assert_eq!(statistics.total_contributors, 2);
        assert_eq!(statistics.insertions, 30);
        assert_eq!(statistics.deletions, 12);
        assert_eq!(statistics.net_lines, 18);
        assert_eq!(statistics.repository_distribution[0].name, "api");
        assert_eq!(statistics.observed_days, 2);
        assert!((statistics.average_commits_per_day - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn period_filter_restricts_records_and_averages() {
        let records = vec![
            record(1, "api", "2026-01-10", "a@example.com", false),
            record(1, "api", "2026-02-10", "a@example.com", false),
        ];
        let range = DateRange::new(day("2026-02-01"), day("2026-02-28")).ok();
        let statistics = compute(
            &records,
            StatisticsOptions {
                include_merges: true,
                period: range,
                today: day("2026-02-28"),
            },
        );
        assert_eq!(statistics.total_commits, 1);
        assert_eq!(statistics.observed_days, 28);
    }

    #[test]
    fn most_active_day_prefers_earliest_on_tie() {
        let records = vec![
            record(1, "api", "2026-05-04", "a@example.com", false),
            record(1, "api", "2026-05-06", "a@example.com", false),
        ];
        let statistics = compute(&records, options(true));
        assert_eq!(
            statistics.most_active_day.map(|activity| activity.day),
            Some(day("2026-05-04"))
        );
    }

    #[test]
    fn weekday_totals_use_monday_first_ordering() {
        let records = vec![record(1, "api", "2026-11-02", "a@example.com", false)];
        let statistics = compute(&records, options(true));
        assert_eq!(
            statistics
                .most_active_weekday
                .map(|activity| activity.weekday),
            Some("Mon".to_owned())
        );
    }

    #[test]
    fn current_streak_uses_real_today_not_last_activity() {
        let records = vec![record(1, "api", "2026-06-01", "a@example.com", false)];
        assert_eq!(compute(&records, options(true)).current_streak_days, 0);
    }

    #[test]
    fn empty_input_produces_zeroed_statistics() {
        let statistics = compute(&[], options(true));
        assert_eq!(statistics.total_commits, 0);
        assert_eq!(statistics.average_commits_per_day, 0.0);
        assert!(statistics.most_active_day.is_none());
        assert_eq!(statistics.longest_streak_days, 0);
    }
}
