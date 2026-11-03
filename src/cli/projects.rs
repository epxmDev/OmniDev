use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use comfy_table::Cell;

use crate::cli::arguments::{ProjectAction, ProjectsArgs};
use crate::cli::context::Context;
use crate::core::activity::format_timestamp;
use crate::database::commits::{self, CommitFilter};
use crate::database::repositories;
use crate::git;
use crate::output::formatting::grouped;
use crate::output::tables::new_table;

const SKIPPED_DIRECTORIES: &[&str] = &["node_modules", "target", "vendor", "build", "dist"];

pub fn run(args: &ProjectsArgs) -> Result<()> {
    match args.action.as_ref().unwrap_or(&ProjectAction::List) {
        ProjectAction::List => list(),
        ProjectAction::Add { path, name } => add(path, name.as_deref()),
        ProjectAction::Remove { selector } => remove(selector),
        ProjectAction::Default { selector } => set_default(selector),
        ProjectAction::Scan { dir, depth } => scan(dir, *depth),
    }
}

fn list() -> Result<()> {
    let context = Context::open()?;
    let records = repositories::list(context.database.connection())?;
    let theme = context.theme;
    println!("{}", theme.heading("Tracked repositories"));
    if records.is_empty() {
        println!("\nNone yet. Add one with `omnidev projects add <path>`.");
        return Ok(());
    }
    let mut table = new_table(
        &["Name", "Default", "Commits", "Head", "Last synced", "Path"],
        theme.ascii,
    );
    for record in &records {
        let total = commits::count_commits(
            context.database.connection(),
            &CommitFilter {
                repository_id: Some(record.id),
                include_merges: true,
                ..CommitFilter::default()
            },
        )?;
        table.add_row(vec![
            Cell::new(&record.name),
            Cell::new(if record.is_default { "yes" } else { "" }),
            Cell::new(grouped(total as i64)),
            Cell::new(
                record
                    .head_commit
                    .as_deref()
                    .map(|hash| hash.chars().take(10).collect::<String>())
                    .unwrap_or_else(|| "-".to_owned()),
            ),
            Cell::new(record.last_synced_at.as_deref().unwrap_or("never")),
            Cell::new(record.path.display().to_string()),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn add(path: &Path, name: Option<&str>) -> Result<()> {
    let mut context = Context::open()?;
    let repository = git::repository::discover(path)
        .with_context(|| format!("{} is not a Git repository", path.display()))?;
    let root = fs::canonicalize(git::repository::root_path(&repository))?;
    if repositories::find_by_path(context.database.connection(), &root)?.is_some() {
        println!("{} is already tracked.", root.display());
        return Ok(());
    }
    let display = name
        .map(str::to_owned)
        .unwrap_or_else(|| git::repository::display_name(&root));
    let id = repositories::insert(
        context.database.connection(),
        &display,
        &root,
        &format_timestamp(chrono::Utc::now()),
    )?;
    if repositories::default_repository(context.database.connection())?.is_none() {
        repositories::set_default(context.database.connection_mut(), id)?;
    }
    println!(
        "{} Added {} ({})",
        context.theme.success(context.theme.symbols().success),
        display,
        root.display()
    );
    println!(
        "Run `omnidev refresh` or `omnidev analyze --path {}` to read its history.",
        root.display()
    );
    Ok(())
}

fn remove(selector: &str) -> Result<()> {
    let context = Context::open()?;
    let record = repositories::find(context.database.connection(), selector)?;
    repositories::remove(context.database.connection(), record.id)?;
    println!(
        "Stopped tracking {}. Its files at {} were not modified or deleted.",
        record.name,
        record.path.display()
    );
    Ok(())
}

fn set_default(selector: &str) -> Result<()> {
    let mut context = Context::open()?;
    let record = repositories::find(context.database.connection(), selector)?;
    repositories::set_default(context.database.connection_mut(), record.id)?;
    println!("Default repository set to {}.", record.name);
    Ok(())
}

fn scan(directories: &[PathBuf], depth: usize) -> Result<()> {
    let mut context = Context::open()?;
    let roots: Vec<PathBuf> = if directories.is_empty() {
        context.settings.scan.directories.clone()
    } else {
        directories.to_vec()
    };
    if roots.is_empty() {
        bail!(
            "no directories to scan; pass --dir <DIR> or set scan.directories in the configuration"
        );
    }
    let mut found = Vec::new();
    for root in &roots {
        if !root.is_dir() {
            println!(
                "{} skipped {} (not a directory)",
                context.theme.symbols().warning,
                root.display()
            );
            continue;
        }
        find_repositories(root, depth, &mut found)?;
    }
    let mut added = 0_usize;
    for path in found {
        let canonical = fs::canonicalize(&path)?;
        if repositories::find_by_path(context.database.connection(), &canonical)?.is_some() {
            continue;
        }
        let name = git::repository::display_name(&canonical);
        match git::repository::open(&canonical) {
            Ok(_) => {
                let id = repositories::insert(
                    context.database.connection(),
                    &name,
                    &canonical,
                    &format_timestamp(chrono::Utc::now()),
                )?;
                if repositories::default_repository(context.database.connection())?.is_none() {
                    repositories::set_default(context.database.connection_mut(), id)?;
                }
                added += 1;
                println!("Added {} ({})", name, canonical.display());
            }
            Err(error) => println!("Skipped {}: {}", canonical.display(), error),
        }
    }
    println!("\nScan complete: {added} new repository(ies) registered.");
    Ok(())
}

fn find_repositories(root: &Path, depth: usize, found: &mut Vec<PathBuf>) -> Result<()> {
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    while let Some((directory, level)) = pending.pop() {
        if directory.join(".git").exists() {
            found.push(directory);
            continue;
        }
        if level >= depth {
            continue;
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            pending.push((entry.path(), level + 1));
        }
    }
    Ok(())
}
