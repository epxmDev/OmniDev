mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use common::{commit_file, day_to_timestamp, init_repository};

const BIN: &str = env!("CARGO_BIN_EXE_omnidev");

fn run(home: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(BIN)
        .args(args)
        .env("OMNIDEV_HOME", home)
        .env("NO_COLOR", "1")
        .env("OMNIDEV_ASCII", "1")
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn seed_repository(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(path)?;
    let repository = init_repository(path)?;
    commit_file(
        &repository,
        "main.rs",
        "fn main() {}\n",
        "feat: start",
        ("Ada", "ada@example.com"),
        (day_to_timestamp("2026-10-01", 10, 0), 0),
    )?;
    commit_file(
        &repository,
        "lib.rs",
        "pub fn a() {}\n",
        "feat: lib",
        ("Ada", "ada@example.com"),
        (day_to_timestamp("2026-10-02", 10, 0), 0),
    )?;
    Ok(())
}

#[test]
fn commands_refuse_to_run_before_init() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let output = run(&temp.path().join("home"), &["stats"])?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not initialized"));
    Ok(())
}

#[test]
fn init_analyze_and_export_full_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let repo = temp.path().join("demo");
    seed_repository(&repo)?;

    assert!(run(&home, &["init"])?.status.success());
    let analyze = run(
        &home,
        &["analyze", "--path", repo.to_str().unwrap_or_default()],
    )?;
    assert!(
        analyze.status.success(),
        "{}",
        String::from_utf8_lossy(&analyze.stderr)
    );
    assert!(stdout(&analyze).contains("New commits: 2"));

    let json_path = temp.path().join("report.json");
    let report = run(
        &home,
        &[
            "report",
            "--format",
            "json",
            "--output",
            json_path.to_str().unwrap_or_default(),
        ],
    )?;
    assert!(report.status.success());
    let parsed: serde_json::Value = serde_json::from_str(&fs::read_to_string(&json_path)?)?;
    assert_eq!(parsed["statistics"]["total_commits"], 2);
    assert_eq!(parsed["repositories"][0]["name"], "demo");

    let csv_path = temp.path().join("report.csv");
    assert!(
        run(
            &home,
            &[
                "report",
                "--format",
                "csv",
                "-o",
                csv_path.to_str().unwrap_or_default()
            ]
        )?
        .status
        .success()
    );
    let csv = fs::read_to_string(&csv_path)?;
    assert!(csv.starts_with("section,name,metric,value\n"));
    assert!(csv.contains("summary,,commits,2"));

    let md_path = temp.path().join("report.md");
    assert!(
        run(
            &home,
            &["report", "-o", md_path.to_str().unwrap_or_default()]
        )?
        .status
        .success()
    );
    assert!(fs::read_to_string(&md_path)?.contains("## Global metrics"));

    let overwrite = run(
        &home,
        &["report", "-o", md_path.to_str().unwrap_or_default()],
    )?;
    assert!(!overwrite.status.success());
    assert!(String::from_utf8_lossy(&overwrite.stderr).contains("--force"));
    Ok(())
}

#[test]
fn configuration_round_trips_and_rejects_bad_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    assert!(run(&home, &["init"])?.status.success());
    assert!(
        run(
            &home,
            &["config", "set", "analytics.include_merges", "false"]
        )?
        .status
        .success()
    );
    assert_eq!(
        stdout(&run(&home, &["config", "get", "analytics.include_merges"])?).trim(),
        "false"
    );
    let rejected = run(&home, &["config", "set", "analytics.default_range", "soon"])?;
    assert!(!rejected.status.success());
    assert_eq!(
        stdout(&run(&home, &["config", "get", "analytics.default_range"])?).trim(),
        "365d"
    );
    Ok(())
}

#[test]
fn goals_and_projects_work_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    let repo = temp.path().join("tracked");
    seed_repository(&repo)?;
    assert!(run(&home, &["init"])?.status.success());
    assert!(
        run(
            &home,
            &["projects", "add", repo.to_str().unwrap_or_default()]
        )?
        .status
        .success()
    );
    assert!(
        run(&home, &["goals", "set", "weekly-commits", "3"])?
            .status
            .success()
    );
    let goals = stdout(&run(&home, &["goals"])?);
    assert!(goals.contains("weekly-commits"));
    let bad = run(&home, &["goals", "set", "unknown-kind", "3"])?;
    assert!(!bad.status.success());
    assert!(
        run(&home, &["projects", "remove", "tracked"])?
            .status
            .success()
    );
    assert!(repo.join("main.rs").exists());
    Ok(())
}

#[test]
fn version_and_doctor_report_build_information() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let home = temp.path().join("home");
    assert!(stdout(&run(&home, &["version"])?).contains("omnidev 0.1.0"));
    assert!(run(&home, &["init"])?.status.success());
    let doctor = run(&home, &["doctor"])?;
    assert!(doctor.status.success());
    assert!(stdout(&doctor).contains("database schema"));
    Ok(())
}
