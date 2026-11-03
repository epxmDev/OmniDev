use std::path::PathBuf;

use omnidev::config::Settings;
use omnidev::core::activity::{format_day, parse_range, resolve_range};
use omnidev::core::analyzer;
use omnidev::core::statistics::{StatisticsOptions, compute};
use omnidev::database::Database;
use omnidev::database::commits::{self, CommitFilter};
use omnidev::database::repositories;
use omnidev::errors::Result;
use omnidev::git;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let repository = git::repository::discover(&path)?;
    let root = git::repository::root_path(&repository);
    let mut database = Database::open_in_memory()?;
    let id = repositories::insert(
        database.connection(),
        &git::repository::display_name(&root),
        &root,
        "1970-01-01T00:00:00Z",
    )?;
    let record = repositories::find_by_id(database.connection(), id)?.ok_or_else(|| {
        omnidev::errors::OmniDevError::RepositoryNotFound(root.display().to_string())
    })?;
    let settings = Settings::default();
    analyzer::sync_repository(&mut database, &record, &settings)?;
    let today = chrono::Local::now().date_naive();
    let period = resolve_range(parse_range("365d")?, today)?;
    let records = commits::load_activity_records(
        database.connection(),
        &CommitFilter {
            range: period,
            include_merges: true,
            ..CommitFilter::default()
        },
    )?;
    let statistics = compute(
        &records,
        StatisticsOptions {
            include_merges: true,
            period,
            today,
        },
    );
    println!(
        "{}: {} commits by {} contributor(s)",
        record.name, statistics.total_commits, statistics.total_contributors
    );
    if let Some(day) = statistics.most_active_day {
        println!(
            "most active day: {} ({} commits)",
            format_day(day.day),
            day.commits
        );
    }
    Ok(())
}
