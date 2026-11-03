mod common;

use std::fs;

use omnidev::analytics::languages::tally_repository;
use omnidev::config::Settings;
use omnidev::config::settings::TimezoneMode;
use omnidev::core::activity::DateRange;
use omnidev::core::analyzer;
use omnidev::core::statistics::{StatisticsOptions, compute};
use omnidev::database::Database;
use omnidev::database::commits::{self, CommitFilter};
use omnidev::database::repositories;
use omnidev::errors::OmniDevError;
use omnidev::git;

use common::{commit_file, day_to_timestamp, init_repository, signature};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ADA: (&str, &str) = ("Ada Lovelace", "ada@example.com");
const GRACE: (&str, &str) = ("Grace Hopper", "grace@example.com");

fn build_history(path: &std::path::Path) -> Result<(), git2::Error> {
    let repository = init_repository(path)?;
    commit_file(
        &repository,
        "main.rs",
        "fn main() {\n}\n",
        "feat: start",
        ADA,
        (day_to_timestamp("2026-10-01", 10, 120), 120),
    )?;
    commit_file(
        &repository,
        "lib.rs",
        "pub fn a() {}\n",
        "feat: lib",
        GRACE,
        (day_to_timestamp("2026-10-02", 11, 120), 120),
    )?;
    let main_head = repository.head()?.peel_to_commit()?;
    let feature_time = (day_to_timestamp("2026-10-03", 9, 120), 120);
    let feature_signature = signature(ADA, feature_time)?;
    let feature_tree = {
        std::fs::write(path.join("feature.rs"), "pub fn f() -> u8 {\n    1\n}\n")
            .map_err(|error| git2::Error::from_str(&error.to_string()))?;
        let mut index = repository.index()?;
        index.add_path(std::path::Path::new("feature.rs"))?;
        index.write()?;
        index.write_tree()?
    };
    let tree = repository.find_tree(feature_tree)?;
    let feature_id = repository.commit(
        None,
        &feature_signature,
        &feature_signature,
        "feat: feature",
        &tree,
        &[&main_head],
    )?;
    repository.reference("refs/heads/feature", feature_id, true, "feature branch")?;
    let feature_commit = repository.find_commit(feature_id)?;
    let merge_signature = signature(ADA, (day_to_timestamp("2026-10-04", 8, 120), 120))?;
    repository.commit(
        Some("HEAD"),
        &merge_signature,
        &merge_signature,
        "Merge feature",
        &feature_commit.tree()?,
        &[&main_head, &feature_commit],
    )?;
    Ok(())
}

fn register(
    database: &Database,
    path: &std::path::Path,
) -> Result<repositories::RepositoryRecord, OmniDevError> {
    let id = repositories::insert(database.connection(), "demo", path, "2026-11-01T00:00:00Z")?;
    repositories::find_by_id(database.connection(), id)?
        .ok_or(OmniDevError::RepositoryNotFound("demo".into()))
}

#[test]
fn analyzes_history_with_authors_merges_and_line_counts() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("demo");
    fs::create_dir_all(&repo_path)?;
    build_history(&repo_path)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    let settings = Settings::default();

    let outcome = analyzer::sync_repository(&mut database, &record, &settings)?;
    assert_eq!(outcome.new_commits, 4);
    assert_eq!(outcome.total_commits, 4);

    let records = commits::load_activity_records(
        database.connection(),
        &CommitFilter {
            include_merges: true,
            ..CommitFilter::default()
        },
    )?;
    let statistics = compute(
        &records,
        StatisticsOptions {
            include_merges: true,
            period: None,
            today: day("2026-11-03"),
        },
    );
    assert_eq!(statistics.total_commits, 4);
    assert_eq!(statistics.total_contributors, 2);
    assert_eq!(statistics.insertions, 2 + 1 + 3);
    assert_eq!(statistics.longest_streak_days, 4);

    let without_merges = compute(
        &records,
        StatisticsOptions {
            include_merges: false,
            period: None,
            today: day("2026-11-03"),
        },
    );
    assert_eq!(without_merges.total_commits, 3);
    Ok(())
}

#[test]
fn incremental_sync_reads_only_new_commits() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("demo");
    fs::create_dir_all(&repo_path)?;
    let repository = init_repository(&repo_path)?;
    commit_file(
        &repository,
        "a.txt",
        "one\n",
        "first",
        ADA,
        (day_to_timestamp("2026-10-01", 9, 0), 0),
    )?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    let settings = Settings::default();
    assert_eq!(
        analyzer::sync_repository(&mut database, &record, &settings)?.new_commits,
        1
    );

    commit_file(
        &repository,
        "b.txt",
        "two\n",
        "second",
        ADA,
        (day_to_timestamp("2026-10-02", 9, 0), 0),
    )?;
    let second = analyzer::sync_repository(&mut database, &record, &settings)?;
    assert_eq!(second.new_commits, 1);
    assert_eq!(second.total_commits, 2);

    let idle = analyzer::sync_repository(&mut database, &record, &settings)?;
    assert_eq!(idle.new_commits, 0);
    Ok(())
}

#[test]
fn empty_repository_syncs_without_errors() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("empty");
    fs::create_dir_all(&repo_path)?;
    init_repository(&repo_path)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    let outcome = analyzer::sync_repository(&mut database, &record, &Settings::default())?;
    assert_eq!(outcome.total_commits, 0);
    assert!(outcome.head_commit.is_none());
    let repository = git::repository::open(&repo_path)?;
    assert!(tally_repository(&repository, true)?.is_empty());
    Ok(())
}

#[test]
fn missing_repository_is_reported_not_panicked() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("gone");
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    match analyzer::sync_repository(&mut database, &record, &Settings::default()) {
        Err(OmniDevError::RepositoryMissing(path)) => assert_eq!(path, repo_path),
        other => panic!("expected RepositoryMissing, got {other:?}"),
    }
    Ok(())
}

#[test]
fn corrupted_git_directory_is_an_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("broken");
    fs::create_dir_all(repo_path.join(".git"))?;
    fs::write(repo_path.join(".git").join("HEAD"), "garbage")?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    assert!(analyzer::sync_repository(&mut database, &record, &Settings::default()).is_err());
    Ok(())
}

#[test]
fn timezone_mode_changes_the_recorded_day() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("tz");
    fs::create_dir_all(&repo_path)?;
    let repository = init_repository(&repo_path)?;
    let late_evening_west = (day_to_timestamp("2026-10-10", 23, -300) + 30 * 60, -300);
    commit_file(&repository, "t.txt", "t\n", "late", ADA, late_evening_west)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    let mut settings = Settings::default();
    analyzer::apply_timezone_mode(&mut database, &settings)?;
    analyzer::sync_repository(&mut database, &record, &settings)?;
    let local = commits::daily_totals(database.connection(), None, true)?;
    assert_eq!(local[0].0, day("2026-10-10"));

    settings.general.timezone = TimezoneMode::Utc;
    analyzer::apply_timezone_mode(&mut database, &settings)?;
    assert!(!analyzer::timezone_mismatch(&database, &settings)?);
    analyzer::sync_repository(&mut database, &record, &settings)?;
    let utc = commits::daily_totals(database.connection(), None, true)?;
    assert_eq!(utc[0].0, day("2026-10-11"));
    Ok(())
}

#[test]
fn rebuild_reads_history_again() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("demo");
    fs::create_dir_all(&repo_path)?;
    build_history(&repo_path)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    let settings = Settings::default();
    analyzer::sync_repository(&mut database, &record, &settings)?;
    analyzer::reset_repository(&mut database, record.id)?;
    assert_eq!(
        commits::count_commits(
            database.connection(),
            &CommitFilter {
                include_merges: true,
                ..CommitFilter::default()
            }
        )?,
        0
    );
    let outcome = analyzer::sync_repository(&mut database, &record, &settings)?;
    assert_eq!(outcome.total_commits, 4);
    Ok(())
}

#[test]
fn removing_a_repository_keeps_its_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("keep");
    fs::create_dir_all(&repo_path)?;
    build_history(&repo_path)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    analyzer::sync_repository(&mut database, &record, &Settings::default())?;
    repositories::remove(database.connection(), record.id)?;
    assert_eq!(
        commits::count_commits(
            database.connection(),
            &CommitFilter {
                include_merges: true,
                ..CommitFilter::default()
            }
        )?,
        0
    );
    assert!(repo_path.join("main.rs").exists());
    assert!(repo_path.join(".git").exists());
    Ok(())
}

#[test]
fn period_filter_limits_statistics() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("demo");
    fs::create_dir_all(&repo_path)?;
    build_history(&repo_path)?;
    let mut database = Database::open_in_memory()?;
    let record = register(&database, &repo_path)?;
    analyzer::sync_repository(&mut database, &record, &Settings::default())?;
    let range = DateRange::new(day("2026-10-02"), day("2026-10-03"))?;
    let records = commits::load_activity_records(
        database.connection(),
        &CommitFilter {
            range: Some(range),
            include_merges: true,
            ..CommitFilter::default()
        },
    )?;
    let statistics = compute(
        &records,
        StatisticsOptions {
            include_merges: true,
            period: Some(range),
            today: day("2026-10-03"),
        },
    );
    assert_eq!(statistics.total_commits, 2);
    assert_eq!(statistics.observed_days, 2);
    Ok(())
}

#[test]
fn language_tally_skips_vendor_and_binary_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_path = temp.path().join("langs");
    fs::create_dir_all(&repo_path)?;
    let repository = init_repository(&repo_path)?;
    commit_file(
        &repository,
        "src.rs",
        "a\nb\n",
        "rust",
        ADA,
        (day_to_timestamp("2026-10-01", 9, 0), 0),
    )?;
    commit_file(
        &repository,
        "vendor/lib.py",
        "x\n",
        "vendored",
        ADA,
        (day_to_timestamp("2026-10-01", 9, 0), 0),
    )?;
    let tallies = tally_repository(&repository, true)?;
    assert_eq!(tallies.len(), 1);
    assert_eq!(tallies[0].language, "Rust");
    assert_eq!(tallies[0].lines, 2);
    Ok(())
}

fn day(text: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap_or(chrono::NaiveDate::MIN)
}
