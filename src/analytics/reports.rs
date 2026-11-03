use std::collections::BTreeMap;
use std::fmt::Write;

use chrono::{NaiveDate, Utc};
use serde::Serialize;

use crate::core::activity::{DateRange, format_timestamp};
use crate::core::model::ActivityRecord;
use crate::core::statistics::{DeveloperStatistics, StatisticsOptions, compute};
use crate::database::Database;
use crate::database::commits::{self, CommitFilter};
use crate::database::languages::{self, LanguageTally};
use crate::database::repositories;
use crate::errors::{OmniDevError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Markdown,
    Csv,
}

impl ReportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Json => "json",
            ReportFormat::Markdown => "md",
            ReportFormat::Csv => "csv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RepositoryMetric {
    pub name: String,
    pub path: String,
    pub commits: u64,
    pub insertions: u64,
    pub deletions: u64,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LanguageMetric {
    pub language: String,
    pub files: u64,
    pub lines: u64,
    pub percent_of_lines: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub generated_at: String,
    pub requested_range: String,
    pub include_merges: bool,
    pub statistics: DeveloperStatistics,
    pub repositories: Vec<RepositoryMetric>,
    pub languages: Vec<LanguageMetric>,
}

pub struct ReportRequest<'a> {
    pub range: Option<DateRange>,
    pub range_label: String,
    pub repository_id: Option<i64>,
    pub include_merges: bool,
    pub today: NaiveDate,
    pub database: &'a Database,
}

pub fn build(request: &ReportRequest<'_>) -> Result<Report> {
    let filter = CommitFilter {
        repository_id: request.repository_id,
        range: request.range,
        include_merges: true,
        ..CommitFilter::default()
    };
    let records: Vec<ActivityRecord> =
        commits::load_activity_records(request.database.connection(), &filter)?;
    let statistics = compute(
        &records,
        StatisticsOptions {
            include_merges: request.include_merges,
            period: request.range,
            today: request.today,
        },
    );
    let repositories = repository_metrics(&records, request.database, request.include_merges)?;
    let language_tallies =
        languages::aggregate_languages(request.database.connection(), request.repository_id)?;
    Ok(Report {
        generated_at: format_timestamp(Utc::now()),
        requested_range: request.range_label.clone(),
        include_merges: request.include_merges,
        statistics,
        repositories,
        languages: language_metrics(&language_tallies),
    })
}

fn repository_metrics(
    records: &[ActivityRecord],
    database: &Database,
    include_merges: bool,
) -> Result<Vec<RepositoryMetric>> {
    let mut totals: BTreeMap<i64, (u64, u64, u64)> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| include_merges || !record.is_merge)
    {
        let entry = totals.entry(record.repository_id).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += record.insertions;
        entry.2 += record.deletions;
    }
    let mut metrics = Vec::new();
    for record in repositories::list(database.connection())? {
        if let Some((count, insertions, deletions)) = totals.get(&record.id) {
            metrics.push(RepositoryMetric {
                name: record.name,
                path: record.path.to_string_lossy().into_owned(),
                commits: *count,
                insertions: *insertions,
                deletions: *deletions,
                last_synced_at: record.last_synced_at,
            });
        }
    }
    metrics.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then(left.name.cmp(&right.name))
    });
    Ok(metrics)
}

fn language_metrics(tallies: &[LanguageTally]) -> Vec<LanguageMetric> {
    let total_lines: u64 = tallies.iter().map(|tally| tally.lines).sum();
    tallies
        .iter()
        .map(|tally| LanguageMetric {
            language: tally.language.clone(),
            files: tally.files,
            lines: tally.lines,
            percent_of_lines: if total_lines == 0 {
                0.0
            } else {
                tally.lines as f64 / total_lines as f64 * 100.0
            },
        })
        .collect()
}

pub fn render(report: &Report, format: ReportFormat) -> Result<String> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)? + "\n"),
        ReportFormat::Markdown => render_markdown(report),
        ReportFormat::Csv => Ok(render_csv(report)),
    }
}

fn render_markdown(report: &Report) -> Result<String> {
    let statistics = &report.statistics;
    let mut output = String::new();
    writeln!(output, "# OmniDev Report\n")?;
    writeln!(output, "- Generated: {}", report.generated_at)?;
    writeln!(output, "- Requested range: {}", report.requested_range)?;
    writeln!(
        output,
        "- Date span: {} to {}",
        display_day(statistics.period_start),
        display_day(statistics.period_end)
    )?;
    writeln!(
        output,
        "- Merge commits included: {}\n",
        if report.include_merges { "yes" } else { "no" }
    )?;
    writeln!(output, "## Global metrics\n")?;
    writeln!(output, "| Metric | Value |")?;
    writeln!(output, "| --- | --- |")?;
    writeln!(output, "| Commits | {} |", statistics.total_commits)?;
    writeln!(
        output,
        "| Repositories | {} |",
        statistics.total_repositories
    )?;
    writeln!(
        output,
        "| Contributors | {} |",
        statistics.total_contributors
    )?;
    writeln!(output, "| Insertions | {} |", statistics.insertions)?;
    writeln!(output, "| Deletions | {} |", statistics.deletions)?;
    writeln!(output, "| Net lines | {} |", statistics.net_lines)?;
    writeln!(output, "| Active days | {} |", statistics.active_days)?;
    writeln!(
        output,
        "| Average commits per day | {:.2} |",
        statistics.average_commits_per_day
    )?;
    writeln!(
        output,
        "| Average commits per week | {:.2} |",
        statistics.average_commits_per_week
    )?;
    writeln!(
        output,
        "| Contribution frequency | {:.1}% |",
        statistics.contribution_frequency * 100.0
    )?;
    writeln!(
        output,
        "| Longest streak (days) | {} |",
        statistics.longest_streak_days
    )?;
    writeln!(
        output,
        "| Current streak (days) | {} |",
        statistics.current_streak_days
    )?;
    if let Some(day) = &statistics.most_active_day {
        writeln!(
            output,
            "| Most active day | {} ({} commits) |",
            day.day, day.commits
        )?;
    }
    if let Some(weekday) = &statistics.most_active_weekday {
        writeln!(
            output,
            "| Most active weekday | {} ({} commits) |",
            weekday.weekday, weekday.commits
        )?;
    }
    writeln!(output, "\n## Repositories\n")?;
    writeln!(
        output,
        "| Repository | Commits | Insertions | Deletions | Last synced |"
    )?;
    writeln!(output, "| --- | --- | --- | --- | --- |")?;
    for repository in &report.repositories {
        writeln!(
            output,
            "| {} | {} | {} | {} | {} |",
            repository.name,
            repository.commits,
            repository.insertions,
            repository.deletions,
            repository.last_synced_at.as_deref().unwrap_or("never")
        )?;
    }
    writeln!(output, "\n## Monthly activity\n")?;
    writeln!(output, "| Month | Commits |")?;
    writeln!(output, "| --- | --- |")?;
    for month in &statistics.monthly_activity {
        writeln!(output, "| {} | {} |", month.month, month.commits)?;
    }
    writeln!(output, "\n## Language statistics\n")?;
    writeln!(
        output,
        "Estimated from the HEAD snapshot of each repository; percentages are by line count.\n"
    )?;
    writeln!(output, "| Language | Files | Lines | Share of lines |")?;
    writeln!(output, "| --- | --- | --- | --- |")?;
    for language in &report.languages {
        writeln!(
            output,
            "| {} | {} | {} | {:.1}% |",
            language.language, language.files, language.lines, language.percent_of_lines
        )?;
    }
    Ok(output)
}

fn render_csv(report: &Report) -> String {
    let statistics = &report.statistics;
    let mut rows: Vec<[String; 4]> = vec![row("section", "name", "metric", "value")];
    rows.push(row("summary", "", "generated_at", &report.generated_at));
    rows.push(row(
        "summary",
        "",
        "requested_range",
        &report.requested_range,
    ));
    rows.push(row(
        "summary",
        "",
        "period_start",
        &display_day(statistics.period_start),
    ));
    rows.push(row(
        "summary",
        "",
        "period_end",
        &display_day(statistics.period_end),
    ));
    rows.push(row(
        "summary",
        "",
        "commits",
        &statistics.total_commits.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "repositories",
        &statistics.total_repositories.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "contributors",
        &statistics.total_contributors.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "insertions",
        &statistics.insertions.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "deletions",
        &statistics.deletions.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "net_lines",
        &statistics.net_lines.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "active_days",
        &statistics.active_days.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "longest_streak_days",
        &statistics.longest_streak_days.to_string(),
    ));
    rows.push(row(
        "summary",
        "",
        "current_streak_days",
        &statistics.current_streak_days.to_string(),
    ));
    for repository in &report.repositories {
        rows.push(row(
            "repository",
            &repository.name,
            "commits",
            &repository.commits.to_string(),
        ));
        rows.push(row(
            "repository",
            &repository.name,
            "insertions",
            &repository.insertions.to_string(),
        ));
        rows.push(row(
            "repository",
            &repository.name,
            "deletions",
            &repository.deletions.to_string(),
        ));
    }
    for month in &statistics.monthly_activity {
        rows.push(row(
            "month",
            &month.month,
            "commits",
            &month.commits.to_string(),
        ));
    }
    for language in &report.languages {
        rows.push(row(
            "language",
            &language.language,
            "files",
            &language.files.to_string(),
        ));
        rows.push(row(
            "language",
            &language.language,
            "lines",
            &language.lines.to_string(),
        ));
    }
    rows.iter()
        .map(|fields| {
            fields
                .iter()
                .map(|field| escape_csv(field))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn row(section: &str, name: &str, metric: &str, value: &str) -> [String; 4] {
    [
        section.to_owned(),
        name.to_owned(),
        metric.to_owned(),
        value.to_owned(),
    ]
}

fn escape_csv(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

fn display_day(day: Option<NaiveDate>) -> String {
    day.map(|day| day.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "n/a".to_owned())
}

pub fn ensure_writable_target(path: &std::path::Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(OmniDevError::InvalidInput(format!(
            "{} already exists; pass --force to overwrite it",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> Report {
        Report {
            generated_at: "2026-01-01T00:00:00Z".to_owned(),
            requested_range: "30d".to_owned(),
            include_merges: true,
            statistics: compute(
                &[],
                StatisticsOptions {
                    include_merges: true,
                    period: None,
                    today: NaiveDate::MIN,
                },
            ),
            repositories: vec![RepositoryMetric {
                name: "api, core".to_owned(),
                path: "/tmp/api".to_owned(),
                commits: 3,
                insertions: 10,
                deletions: 2,
                last_synced_at: None,
            }],
            languages: vec![LanguageMetric {
                language: "Rust".to_owned(),
                files: 2,
                lines: 40,
                percent_of_lines: 100.0,
            }],
        }
    }

    #[test]
    fn csv_quotes_fields_containing_separators() {
        let csv = render(&sample_report(), ReportFormat::Csv).unwrap_or_default();
        assert!(csv.starts_with("section,name,metric,value\n"));
        assert!(csv.contains("repository,\"api, core\",commits,3"));
    }

    #[test]
    fn json_contains_statistics_and_repositories() {
        let json = render(&sample_report(), ReportFormat::Json).unwrap_or_default();
        assert!(json.contains("\"total_commits\": 0"));
        assert!(json.contains("\"repositories\""));
    }

    #[test]
    fn markdown_contains_all_sections() {
        let markdown = render(&sample_report(), ReportFormat::Markdown).unwrap_or_default();
        for heading in [
            "## Global metrics",
            "## Repositories",
            "## Monthly activity",
            "## Language statistics",
        ] {
            assert!(markdown.contains(heading), "missing {heading}");
        }
    }
}
