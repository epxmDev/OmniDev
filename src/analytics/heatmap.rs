use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};

use crate::core::activity::DateRange;

pub const DEFAULT_HEATMAP_DAYS: i64 = 365;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatCell {
    pub day: NaiveDate,
    pub commits: u64,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heatmap {
    pub range: DateRange,
    pub columns: usize,
    pub cells: Vec<Vec<Option<HeatCell>>>,
}

pub const WEEKDAY_LABELS: [&str; 7] = ["Mon", "", "Wed", "", "Fri", "", "Sun"];

pub fn build(range: DateRange, counts: &BTreeMap<NaiveDate, u64>) -> Heatmap {
    let first_monday =
        range.start - chrono::Days::new(u64::from(range.start.weekday().num_days_from_monday()));
    let columns = ((range.end - first_monday).num_days() / 7 + 1).max(1) as usize;
    let peak = counts.values().copied().max().unwrap_or(0);
    let mut cells: Vec<Vec<Option<HeatCell>>> = vec![vec![None; columns]; 7];
    let mut day = range.start;
    while day <= range.end {
        let column = ((day - first_monday).num_days() / 7) as usize;
        let row = day.weekday().num_days_from_monday() as usize;
        let commits = counts.get(&day).copied().unwrap_or(0);
        cells[row][column] = Some(HeatCell {
            day,
            commits,
            level: level_for(commits, peak),
        });
        match day.succ_opt() {
            Some(next) => day = next,
            None => break,
        }
    }
    Heatmap {
        range,
        columns,
        cells,
    }
}

pub fn level_for(commits: u64, peak: u64) -> u8 {
    if commits == 0 || peak == 0 {
        return 0;
    }
    let scaled = (commits * 4).div_ceil(peak);
    scaled.clamp(1, 4) as u8
}

pub fn glyph(level: u8, ascii: bool) -> char {
    match (level, ascii) {
        (0, _) => '·',
        (_, true) => '#',
        (_, false) => '■',
    }
}

impl Heatmap {
    pub fn month_labels(&self) -> Vec<(usize, String)> {
        let mut labels = Vec::new();
        let mut last_month: Option<u32> = None;
        for column in 0..self.columns {
            let first_day = self
                .cells
                .iter()
                .filter_map(|row| row.get(column).copied().flatten())
                .map(|cell| cell.day)
                .min();
            if let Some(day) = first_day {
                if last_month != Some(day.month()) {
                    labels.push((column, month_abbreviation(day.month())));
                    last_month = Some(day.month());
                }
            }
        }
        labels
    }

    pub fn total_commits(&self) -> u64 {
        self.cells
            .iter()
            .flatten()
            .flatten()
            .map(|cell| cell.commits)
            .sum()
    }

    pub fn active_days(&self) -> u64 {
        self.cells
            .iter()
            .flatten()
            .flatten()
            .filter(|cell| cell.commits > 0)
            .count() as u64
    }
}

fn month_abbreviation(month: u32) -> String {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    NAMES
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("???")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap_or(NaiveDate::MIN)
    }

    #[test]
    fn places_days_on_monday_first_grid() {
        let range = DateRange::new(day("2026-11-02"), day("2026-11-08")).ok();
        let Some(range) = range else { return };
        let heatmap = build(range, &BTreeMap::new());
        assert_eq!(heatmap.columns, 1);
        assert!(heatmap.cells[0][0].is_some());
        assert!(heatmap.cells[6][0].is_some());
    }

    #[test]
    fn days_without_activity_have_zero_level() {
        let range = DateRange::new(day("2026-01-01"), day("2026-01-10")).ok();
        let Some(range) = range else { return };
        let mut counts = BTreeMap::new();
        counts.insert(day("2026-01-05"), 4);
        let heatmap = build(range, &counts);
        assert_eq!(heatmap.total_commits(), 4);
        assert_eq!(heatmap.active_days(), 1);
        let idle = heatmap
            .cells
            .iter()
            .flatten()
            .flatten()
            .filter(|cell| cell.commits == 0);
        assert!(idle.clone().all(|cell| cell.level == 0));
    }

    #[test]
    fn levels_scale_relative_to_peak() {
        assert_eq!(level_for(0, 10), 0);
        assert_eq!(level_for(1, 10), 1);
        assert_eq!(level_for(5, 10), 2);
        assert_eq!(level_for(7, 10), 3);
        assert_eq!(level_for(10, 10), 4);
    }

    #[test]
    fn spans_a_full_year_in_about_53_columns() {
        let range = DateRange::new(day("2025-11-04"), day("2026-11-03")).ok();
        let Some(range) = range else { return };
        let heatmap = build(range, &BTreeMap::new());
        assert!((52..=54).contains(&heatmap.columns));
        let present = heatmap.cells.iter().flatten().flatten().count() as i64;
        assert_eq!(present, range.day_count());
    }

    #[test]
    fn month_labels_appear_once_per_month() {
        let range = DateRange::new(day("2026-01-01"), day("2026-03-31")).ok();
        let Some(range) = range else { return };
        let heatmap = build(range, &BTreeMap::new());
        let labels: Vec<String> = heatmap
            .month_labels()
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        assert_eq!(labels, vec!["Jan", "Feb", "Mar"]);
    }
}
