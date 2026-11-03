use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, Result, bail};
use comfy_table::Cell;
use indicatif::ProgressBar;
use std::time::Duration;

use crate::analytics::heatmap::{self, WEEKDAY_LABELS};
use crate::cli::arguments::{
    AnalyzeArgs, CommitsArgs, HeatmapArgs, InitArgs, LanguagesArgs, RefreshArgs, StatsArgs,
};
use crate::cli::context::{Context, heatmap_period, resolve_period, today};
use crate::config::{AppPaths, Settings, write_settings};
use crate::core::achievements::{self, AchievementInput};
use crate::core::analyzer;
use crate::core::statistics::{DeveloperStatistics, StatisticsOptions, compute};
use crate::database::Database;
use crate::database::commits::{self, CommitFilter};
use crate::database::languages::{self, LanguageTally};
use crate::database::repositories;
use crate::git;
use crate::output::formatting::{bar, grouped, percent, truncate};
use crate::output::tables::{new_table, right_align};

pub fn init(args: &InitArgs) -> Result<()> {
    let paths = AppPaths::resolve()?;
    let theme = crate::output::colors::Theme::detect(true);
    let symbols = theme.symbols();
    let exists = paths.config_file.exists();
    if exists && !args.force {
        println!(
            "{} OmniDev is already initialized.\n  Configuration: {}\n  Use `omnidev config show` to review it.",
            symbols.success,
            paths.config_file.display()
        );
        return Ok(());
    }
    if exists
        && !confirm(&format!(
            "Overwrite the configuration at {}? [y/N] ",
            paths.config_file.display()
        ))?
    {
        println!("Configuration left unchanged.");
        return Ok(());
    }
    let settings = Settings::default();
    write_settings(&paths, &settings)?;
    let database = Database::open(&paths.database_path(&settings))?;
    println!("{} OmniDev initialized.", theme.success(symbols.success));
    println!("  Configuration: {}", paths.config_file.display());
    println!(
        "  Database:      {}",
        paths.database_path(&settings).display()
    );
    println!("  Schema:        version {}", database.schema_version()?);
    println!(
        "\nNext: `omnidev analyze` inside a Git repository, or `omnidev projects add <path>`."
    );
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{BufRead, Write};
    print!("{prompt}");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

pub fn analyze(args: &AnalyzeArgs) -> Result<()> {
    let mut context = Context::open()?;
    let start = match &args.path {
        Some(path) => path.clone(),
        None => std::env::current_dir()?,
    };
    let repository = git::repository::discover(&start)
        .with_context(|| format!("{} is not inside a Git repository", start.display()))?;
    let root = fs::canonicalize(git::repository::root_path(&repository))?;
    let record = match repositories::find_by_path(context.database.connection(), &root)? {
        Some(record) => record,
        None => {
            let name = git::repository::display_name(&root);
            let id = repositories::insert(
                context.database.connection(),
                &name,
                &root,
                &crate::core::activity::format_timestamp(chrono::Utc::now()),
            )?;
            println!(
                "Registered {} for tracking (use `omnidev projects remove` to stop).",
                name
            );
            repositories::find_by_id(context.database.connection(), id)?
                .context("repository vanished after insert")?
        }
    };
    analyzer::apply_timezone_mode(&mut context.database, &context.settings)?;
    let spinner = ProgressBar::new_spinner();
    spinner.set_message(format!("Reading history of {}", record.name));
    spinner.enable_steady_tick(Duration::from_millis(120));
    let outcome = analyzer::sync_repository(&mut context.database, &record, &context.settings);
    spinner.finish_and_clear();
    let outcome = outcome?;
    let theme = context.theme;
    let symbols = theme.symbols();
    println!(
        "{} {}",
        theme.accent(symbols.repository),
        theme.heading(&record.name)
    );
    println!("  Path:        {}", record.path.display());
    println!(
        "  Head:        {}",
        outcome
            .head_commit
            .as_deref()
            .map(|hash| &hash[..hash.len().min(12)])
            .unwrap_or("(no commits yet)")
    );
    println!("  New commits: {}", grouped(outcome.new_commits as i64));
    println!("  Total:       {}", grouped(outcome.total_commits as i64));
    let branches = git::branches::summarize(&repository)?;
    println!(
        "  Branches:    {} local, {} remote",
        branches.local, branches.remote
    );
    print_language_summary(&context, Some(record.id))?;
    print_new_achievements(&context)?;
    Ok(())
}

pub fn print_language_summary(context: &Context, repository_id: Option<i64>) -> Result<()> {
    let tallies = languages::aggregate_languages(context.database.connection(), repository_id)?;
    if tallies.is_empty() {
        println!("\nNo recognized source files were found at HEAD.");
        return Ok(());
    }
    println!(
        "\n{}",
        context.theme.heading("Top languages (estimated, by lines)")
    );
    let table = language_table(&tallies, context.theme.ascii, 5);
    println!("{table}");
    Ok(())
}

pub fn language_table(tallies: &[LanguageTally], ascii: bool, limit: usize) -> comfy_table::Table {
    let total_lines: u64 = tallies.iter().map(|tally| tally.lines).sum();
    let mut table = new_table(&["Language", "Files", "Lines", "Share", ""], ascii);
    for tally in tallies.iter().take(limit) {
        let share = if total_lines == 0 {
            0.0
        } else {
            tally.lines as f64 / total_lines as f64
        };
        table.add_row(vec![
            Cell::new(&tally.language),
            right_align(&grouped(tally.files as i64)),
            right_align(&grouped(tally.lines as i64)),
            right_align(&percent(share * 100.0)),
            Cell::new(bar(share, 20, ascii)),
        ]);
    }
    table
}

pub fn print_new_achievements(context: &Context) -> Result<()> {
    let statistics = compute(
        &commits::load_activity_records(
            context.database.connection(),
            &CommitFilter {
                include_merges: true,
                ..CommitFilter::default()
            },
        )?,
        StatisticsOptions {
            include_merges: true,
            period: None,
            today: today(),
        },
    );
    let input = AchievementInput {
        total_commits: statistics.total_commits,
        longest_streak_days: statistics.longest_streak_days,
        tracked_repositories: repositories::list(context.database.connection())?.len() as u64,
        reports_generated: achievements::reports_generated(&context.database)?,
    };
    for achievement in achievements::evaluate(&context.database, &input)? {
        println!(
            "{} Achievement unlocked: {} — {}",
            context.theme.success(context.theme.symbols().trophy),
            achievement.title(),
            achievement.description()
        );
    }
    Ok(())
}

pub fn refresh(args: &RefreshArgs) -> Result<()> {
    let mut context = Context::open()?;
    analyzer::apply_timezone_mode(&mut context.database, &context.settings)?;
    let records = repositories::list(context.database.connection())?;
    if args.rebuild {
        for record in &records {
            analyzer::reset_repository(&mut context.database, record.id)?;
        }
    }
    let theme = context.theme;
    let symbols = theme.symbols();
    if records.is_empty() {
        println!(
            "No repositories are tracked yet. Use `omnidev projects add <path>` or `omnidev analyze`."
        );
        return Ok(());
    }
    let mut failures = 0_usize;
    for record in &records {
        match analyzer::sync_repository(&mut context.database, record, &context.settings) {
            Ok(outcome) => println!(
                "{} {}: {} new commit(s), {} total",
                theme.success(symbols.success),
                record.name,
                outcome.new_commits,
                outcome.total_commits
            ),
            Err(error) => {
                failures += 1;
                println!(
                    "{} {}: {}",
                    theme.warning(symbols.warning),
                    record.name,
                    error
                );
            }
        }
    }
    print_new_achievements(&context)?;
    if failures > 0 {
        bail!(
            "{failures} of {} repositories could not be refreshed",
            records.len()
        );
    }
    Ok(())
}

pub fn stats(args: &StatsArgs) -> Result<()> {
    let context = Context::open()?;
    let today = today();
    let (period, label) = resolve_period(
        &args.period,
        Some(&context.settings.analytics.default_range),
        today,
    )?;
    let repository = context.resolve_repository(args.repo.as_deref())?;
    let records = commits::load_activity_records(
        context.database.connection(),
        &CommitFilter {
            repository_id: repository.as_ref().map(|record| record.id),
            range: period,
            include_merges: true,
            ..CommitFilter::default()
        },
    )?;
    let statistics = compute(
        &records,
        StatisticsOptions {
            include_merges: context.include_merges(),
            period,
            today,
        },
    );
    let theme = context.theme;
    let symbols = theme.symbols();
    println!("{}", theme.heading("Developer statistics"));
    println!(
        "Range: {} · Repository: {} · Merge commits: {}",
        label,
        repository
            .as_ref()
            .map_or("all tracked", |record| record.name.as_str()),
        if context.include_merges() {
            "included"
        } else {
            "excluded"
        }
    );
    if analyzer::timezone_mismatch(&context.database, &context.settings)? {
        println!(
            "{} The timezone setting changed. Run `omnidev refresh` to rebuild commit days.",
            theme.warning(symbols.warning)
        );
    }
    println!();
    print_summary_table(&statistics, &context);
    print_streaks(&statistics);
    print_distribution(&statistics, &context);
    print_monthly(&statistics, &context);
    print_language_summary(&context, repository.as_ref().map(|record| record.id))?;
    println!(
        "\n{}",
        theme.muted("Note: commit timestamps show when work was committed, not how long it took.")
    );
    Ok(())
}

fn print_summary_table(statistics: &DeveloperStatistics, context: &Context) {
    let ascii = context.theme.ascii;
    let mut table = new_table(&["Metric", "Value"], ascii);
    table.add_row(vec![
        Cell::new("Commits"),
        right_align(&grouped(statistics.total_commits as i64)),
    ]);
    table.add_row(vec![
        Cell::new("Repositories"),
        right_align(&statistics.total_repositories.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Contributors"),
        right_align(&statistics.total_contributors.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Insertions"),
        right_align(&grouped(statistics.insertions as i64)),
    ]);
    table.add_row(vec![
        Cell::new("Deletions"),
        right_align(&grouped(statistics.deletions as i64)),
    ]);
    table.add_row(vec![
        Cell::new("Net lines"),
        right_align(&grouped(statistics.net_lines)),
    ]);
    table.add_row(vec![
        Cell::new("Active days"),
        right_align(&format!(
            "{} of {}",
            statistics.active_days, statistics.observed_days
        )),
    ]);
    table.add_row(vec![
        Cell::new("Avg commits / day"),
        right_align(&format!("{:.2}", statistics.average_commits_per_day)),
    ]);
    table.add_row(vec![
        Cell::new("Avg commits / week"),
        right_align(&format!("{:.2}", statistics.average_commits_per_week)),
    ]);
    table.add_row(vec![
        Cell::new("Contribution frequency"),
        right_align(&percent(statistics.contribution_frequency * 100.0)),
    ]);
    if let Some(day) = &statistics.most_active_day {
        table.add_row(vec![
            Cell::new("Most active day"),
            right_align(&format!("{} ({} commits)", day.day, day.commits)),
        ]);
    }
    if let Some(weekday) = &statistics.most_active_weekday {
        table.add_row(vec![
            Cell::new("Most active weekday"),
            right_align(&format!(
                "{} ({} commits)",
                weekday.weekday, weekday.commits
            )),
        ]);
    }
    println!("{table}");
}

fn print_streaks(statistics: &DeveloperStatistics) {
    println!(
        "\nStreaks: longest {} day(s), current {} day(s)",
        statistics.longest_streak_days, statistics.current_streak_days
    );
}

fn print_distribution(statistics: &DeveloperStatistics, context: &Context) {
    if statistics.repository_distribution.is_empty() {
        return;
    }
    println!("\n{}", context.theme.heading("Repository activity"));
    let mut table = new_table(&["Repository", "Commits", "Share", ""], context.theme.ascii);
    for share in statistics.repository_distribution.iter().take(10) {
        table.add_row(vec![
            Cell::new(truncate(&share.name, 32)),
            right_align(&grouped(share.commits as i64)),
            right_align(&percent(share.percent)),
            Cell::new(bar(share.percent / 100.0, 20, context.theme.ascii)),
        ]);
    }
    println!("{table}");
}

fn print_monthly(statistics: &DeveloperStatistics, context: &Context) {
    if statistics.monthly_activity.is_empty() {
        return;
    }
    println!("\n{}", context.theme.heading("Monthly activity"));
    let peak = statistics
        .monthly_activity
        .iter()
        .map(|month| month.commits)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut table = new_table(&["Month", "Commits", ""], context.theme.ascii);
    let skip = statistics.monthly_activity.len().saturating_sub(12);
    for month in statistics.monthly_activity.iter().skip(skip) {
        table.add_row(vec![
            Cell::new(&month.month),
            right_align(&grouped(month.commits as i64)),
            Cell::new(bar(
                month.commits as f64 / peak as f64,
                24,
                context.theme.ascii,
            )),
        ]);
    }
    println!("{table}");
}

pub fn heatmap(args: &HeatmapArgs) -> Result<()> {
    let context = Context::open()?;
    let (range, _) = heatmap_period(&args.period, today())?;
    let repository = context.resolve_repository(args.repo.as_deref())?;
    let totals = commits::daily_totals(
        context.database.connection(),
        repository.as_ref().map(|record| record.id),
        context.include_merges(),
    )?;
    let counts: BTreeMap<_, _> = totals.into_iter().collect();
    let grid = heatmap::build(range, &counts);
    let theme = context.theme;
    println!("{}", theme.heading("Contribution heatmap"));
    println!(
        "Range: {} to {} · Repository: {}",
        range.start,
        range.end,
        repository
            .as_ref()
            .map_or("all tracked", |record| record.name.as_str())
    );
    println!();
    let mut label_line = String::from("    ");
    let mut cursor = 0_usize;
    for (column, month) in grid.month_labels() {
        let position = column * 2;
        if position >= cursor {
            label_line.push_str(&" ".repeat(position - cursor));
            label_line.push_str(&month);
            cursor = position + month.len();
        }
    }
    println!("{}", theme.muted(&label_line));
    for (row, weekday) in WEEKDAY_LABELS.iter().enumerate() {
        let mut line = format!("{weekday:<4}");
        for column in 0..grid.columns {
            match grid.cells[row][column] {
                Some(cell) => {
                    line.push_str(&theme.heat(cell.level, heatmap::glyph(cell.level, theme.ascii)));
                    line.push(' ');
                }
                None => line.push_str("  "),
            }
        }
        println!("{line}");
    }
    println!(
        "\nLegend: {} none  {} low  {} medium  {} high  {} peak",
        theme.heat(0, heatmap::glyph(0, theme.ascii)),
        theme.heat(1, heatmap::glyph(1, theme.ascii)),
        theme.heat(2, heatmap::glyph(2, theme.ascii)),
        theme.heat(3, heatmap::glyph(3, theme.ascii)),
        theme.heat(4, heatmap::glyph(4, theme.ascii)),
    );
    println!(
        "{} commits on {} active day(s) in this range",
        grouped(grid.total_commits() as i64),
        grid.active_days()
    );
    Ok(())
}

pub fn languages(args: &LanguagesArgs) -> Result<()> {
    let context = Context::open()?;
    let repository = context.resolve_repository(args.repo.as_deref())?;
    let tallies = languages::aggregate_languages(
        context.database.connection(),
        repository.as_ref().map(|record| record.id),
    )?;
    println!("{}", context.theme.heading("Language statistics"));
    println!(
        "Repository: {} · Source: HEAD snapshot of each repository",
        repository
            .as_ref()
            .map_or("all tracked", |record| record.name.as_str())
    );
    if tallies.is_empty() {
        println!(
            "\nNo recognized source files found. Run `omnidev analyze` inside a repository first."
        );
        return Ok(());
    }
    println!();
    println!(
        "{}",
        language_table(&tallies, context.theme.ascii, usize::MAX)
    );
    if let Some(top) = tallies.first() {
        println!(
            "\nMost used language: {} (estimated from line counts)",
            top.language
        );
    }
    println!(
        "{}",
        context
            .theme
            .muted("Estimates exclude vendored, build, generated and binary files.")
    );
    Ok(())
}

pub fn commits(args: &CommitsArgs) -> Result<()> {
    let context = Context::open()?;
    let today = today();
    let (period, label) = resolve_period(&args.period, None, today)?;
    let repository = context.resolve_repository(args.repo.as_deref())?;
    if !(1..=200).contains(&args.per_page) {
        bail!("--per-page must be between 1 and 200");
    }
    if args.page == 0 {
        bail!("--page starts at 1");
    }
    let filter = CommitFilter {
        repository_id: repository.as_ref().map(|record| record.id),
        author: args.author.clone(),
        range: period,
        search: args.search.clone(),
        include_merges: context.include_merges(),
    };
    let total = commits::count_commits(context.database.connection(), &filter)?;
    let pages = total.div_ceil(args.per_page).max(1);
    let offset = (args.page - 1).saturating_mul(args.per_page);
    let listings = commits::list_commits(
        context.database.connection(),
        &filter,
        args.per_page,
        offset,
    )?;
    let theme = context.theme;
    println!("{}", theme.heading("Commit history"));
    println!(
        "Range: {} · Showing page {} of {} ({} matching commits)",
        label,
        args.page.min(pages),
        pages,
        grouped(total as i64)
    );
    if listings.is_empty() {
        println!("\nNo commits match these filters.");
        return Ok(());
    }
    let mut table = new_table(
        &["Hash", "Date", "Author", "Message", "+", "-", "Files"],
        theme.ascii,
    );
    for listing in &listings {
        table.add_row(vec![
            Cell::new(listing.hash.chars().take(10).collect::<String>()),
            Cell::new(listing.day.to_string()),
            Cell::new(truncate(&listing.author_name, 20)),
            Cell::new(truncate(&listing.summary, 48)),
            right_align(&grouped(listing.insertions as i64)),
            right_align(&grouped(listing.deletions as i64)),
            right_align(&listing.files_changed.to_string()),
        ]);
    }
    println!("{table}");
    if args.files {
        for listing in &listings {
            print_changed_files(listing)?;
        }
    }
    Ok(())
}

fn print_changed_files(listing: &commits::CommitListing) -> Result<()> {
    let repository = git::repository::open(&PathBuf::from(&listing.repository_path))?;
    let files = git::diff::changed_files(&repository, &listing.hash)?;
    println!(
        "\n{} {} ({})",
        listing.repository_name,
        listing.hash.chars().take(10).collect::<String>(),
        listing.author_email
    );
    if files.is_empty() {
        println!("  (no file changes)");
    }
    for file in files {
        println!("  {} {}", file.status, file.path);
    }
    Ok(())
}
