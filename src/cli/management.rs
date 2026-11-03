use std::fs;

use anyhow::{Result, bail};
use comfy_table::Cell;

use crate::analytics::reports::{self, ReportFormat, ReportRequest};
use crate::cli::arguments::{
    ConfigAction, ConfigArgs, FormatArg, GoalAction, GoalsArgs, ReportArgs,
};
use crate::cli::commands::print_new_achievements;
use crate::cli::context::{Context, resolve_period, today};
use crate::config::{AppPaths, read_settings, require_settings, write_settings};
use crate::core::achievements::{self, Achievement};
use crate::core::diagnostics::{self, Level};
use crate::core::goals::{self, GoalKind};
use crate::database::goals as goal_store;
use crate::output::colors::{LOGO, Theme};
use crate::output::formatting::{bar, percent};
use crate::output::tables::new_table;

pub fn welcome() -> Result<()> {
    let paths = AppPaths::resolve()?;
    let settings = read_settings(&paths)?;
    let show_art = settings
        .as_ref()
        .is_none_or(|value| value.display.show_ascii_art);
    let theme = Theme::detect(
        settings
            .as_ref()
            .is_none_or(|value| value.display.show_colors),
    );
    let symbols = theme.symbols();
    if show_art && !theme.ascii {
        println!("{}", theme.accent(LOGO.trim_end_matches('\n')));
    } else {
        println!("{}", theme.accent("OmniDev"));
    }
    println!(
        "{} {}  ·  {}",
        theme.heading("OmniDev"),
        env!("CARGO_PKG_VERSION"),
        theme.muted("Track your code. Understand your growth.")
    );
    println!("\nLocal-first developer analytics for the terminal. Nothing leaves this machine.\n");
    let mut table = new_table(&["Command", "What it does"], theme.ascii);
    for (command, description) in COMMANDS {
        table.add_row(vec![Cell::new(command), Cell::new(description)]);
    }
    println!("{table}");
    println!();
    match settings {
        Some(_) => println!(
            "{} Initialized. Configuration: {}",
            theme.success(symbols.success),
            paths.config_file.display()
        ),
        None => println!(
            "{} Not initialized yet. Run `omnidev init` to create the configuration and database.",
            theme.warning(symbols.warning)
        ),
    }
    println!("\nRun `omnidev --help` for details on any command.");
    Ok(())
}

const COMMANDS: &[(&str, &str)] = &[
    ("init", "Create configuration and local database"),
    ("analyze", "Analyze the current or a given repository"),
    ("stats", "Developer statistics for a time range"),
    ("heatmap", "365-day contribution heatmap"),
    ("dashboard", "Interactive terminal dashboard"),
    ("projects", "List, add, remove, scan tracked repositories"),
    ("commits", "Browse commit history with filters"),
    ("languages", "Language distribution (estimated)"),
    ("report", "Export JSON, Markdown or CSV reports"),
    ("goals", "Goals and achievements"),
    ("config", "Show or change configuration"),
    ("refresh", "Incrementally refresh all repositories"),
    ("doctor", "Check installation and repository access"),
    ("version", "Version and build information"),
];

pub fn goals(args: &GoalsArgs) -> Result<()> {
    match args.action.as_ref() {
        None | Some(GoalAction::List) => list_goals(),
        Some(GoalAction::Set { kind, target }) => {
            let context = Context::open()?;
            let parsed = GoalKind::parse(kind)?;
            goals::set_goal(&context.database, parsed, *target)?;
            println!(
                "Goal set: {} → {} ({})",
                parsed.key(),
                target,
                parsed.description()
            );
            Ok(())
        }
        Some(GoalAction::Remove { kind }) => {
            let context = Context::open()?;
            let parsed = GoalKind::parse(kind)?;
            if goal_store::delete_goal(context.database.connection(), parsed.key())? {
                println!("Removed goal {}.", parsed.key());
            } else {
                println!("No goal named {} is set.", parsed.key());
            }
            Ok(())
        }
    }
}

fn list_goals() -> Result<()> {
    let context = Context::open()?;
    let theme = context.theme;
    let today = today();
    let stored = goal_store::list_goals(context.database.connection())?;
    let goal_context = goals::build_context(&context.database, today, context.include_merges())?;
    println!("{}", theme.heading("Goals"));
    if stored.is_empty() {
        println!("No goals set. Example: `omnidev goals set weekly-commits 5`.");
    } else {
        let mut table = new_table(&["Goal", "Progress", "Done", "Status"], theme.ascii);
        for goal in &stored {
            let kind = GoalKind::parse(&goal.kind)?;
            let progress = goals::progress(kind, goal.target, &goal_context);
            let status = if progress.is_complete() {
                theme.success(theme.symbols().success)
            } else {
                theme.muted("in progress")
            };
            table.add_row(vec![
                Cell::new(format!("{} ({})", kind.key(), kind.description())),
                Cell::new(format!(
                    "{} {}",
                    bar(progress.percent() / 100.0, 16, theme.ascii),
                    percent(progress.percent())
                )),
                Cell::new(format!("{}/{}", progress.current, progress.target)),
                Cell::new(status),
            ]);
        }
        println!("{table}");
    }
    println!("\n{}", theme.heading("Achievements"));
    let unlocked = goal_store::unlocked_achievements(context.database.connection())?;
    let mut table = new_table(&["Achievement", "Status", "Description"], theme.ascii);
    for achievement in Achievement::ALL {
        let date = unlocked
            .iter()
            .find(|(code, _)| code == achievement.code())
            .map(|(_, at)| at.clone());
        let status = match date {
            Some(at) => format!("unlocked {}", at.chars().take(10).collect::<String>()),
            None => "locked".to_owned(),
        };
        table.add_row(vec![
            Cell::new(achievement.title()),
            Cell::new(status),
            Cell::new(achievement.description()),
        ]);
    }
    println!("{table}");
    print_new_achievements(&context)?;
    Ok(())
}

pub fn report(args: &ReportArgs) -> Result<()> {
    let context = Context::open()?;
    let today = today();
    let (range, label) = resolve_period(
        &args.period,
        Some(&context.settings.analytics.default_range),
        today,
    )?;
    let repository = context.resolve_repository(args.repo.as_deref())?;
    let format = match args.format {
        FormatArg::Json => ReportFormat::Json,
        FormatArg::Markdown => ReportFormat::Markdown,
        FormatArg::Csv => ReportFormat::Csv,
    };
    if let Some(path) = &args.output {
        reports::ensure_writable_target(path, args.force)?;
    }
    let report = reports::build(&ReportRequest {
        range,
        range_label: label,
        repository_id: repository.as_ref().map(|record| record.id),
        include_merges: context.include_merges(),
        today,
        database: &context.database,
    })?;
    let rendered = reports::render(&report, format)?;
    match &args.output {
        Some(path) => {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, rendered)?;
            achievements::increment_reports(&context.database)?;
            println!(
                "{} Report written to {} ({})",
                context.theme.success(context.theme.symbols().success),
                path.display(),
                format.extension()
            );
            print_new_achievements(&context)?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
}

pub fn config(args: &ConfigArgs) -> Result<()> {
    let paths = AppPaths::resolve()?;
    match args.action.as_ref().unwrap_or(&ConfigAction::Show) {
        ConfigAction::Show => {
            let settings = require_settings(&paths)?;
            println!("# {}", paths.config_file.display());
            print!("{}", toml::to_string_pretty(&settings)?);
            println!("# database: {}", paths.database_path(&settings).display());
            Ok(())
        }
        ConfigAction::Get { key } => {
            let settings = require_settings(&paths)?;
            println!("{}", settings.get_value(key)?);
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut settings = require_settings(&paths)?;
            let previous = settings.get_value(key)?;
            settings.set_value(key, value)?;
            write_settings(&paths, &settings)?;
            println!("{key}: {previous} → {}", settings.get_value(key)?);
            if key == "general.timezone" {
                println!("Run `omnidev refresh` to rebuild commit days with the new timezone.");
            }
            Ok(())
        }
        ConfigAction::Path => {
            println!("{}", paths.config_file.display());
            Ok(())
        }
    }
}

pub fn version() -> Result<()> {
    let libgit2 = git2::Version::get();
    let (major, minor, patch) = libgit2.libgit2_version();
    println!("omnidev {}", env!("CARGO_PKG_VERSION"));
    println!(
        "platform:  {} / {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("libgit2:   {major}.{minor}.{patch}");
    println!("sqlite:    {}", rusqlite::version());
    println!(
        "schema:    {}",
        crate::database::migrations::latest_version()
    );
    Ok(())
}

pub fn doctor() -> Result<()> {
    let paths = AppPaths::resolve()?;
    let checks = diagnostics::run(&paths);
    let theme = Theme::detect(true);
    let symbols = theme.symbols();
    let mut table = new_table(&["Check", "Result", "Details"], theme.ascii);
    let mut failures = 0_usize;
    for check in &checks {
        let (marker, label) = match check.level {
            Level::Pass => (theme.success(symbols.success), "pass"),
            Level::Warn => (theme.warning(symbols.warning), "warn"),
            Level::Fail => {
                failures += 1;
                (theme.error(symbols.failure), "fail")
            }
        };
        table.add_row(vec![
            Cell::new(&check.name),
            Cell::new(format!("{marker} {label}")),
            Cell::new(&check.detail),
        ]);
    }
    println!("{}", theme.heading("OmniDev doctor"));
    println!("{table}");
    if failures > 0 {
        bail!("{failures} check(s) failed");
    }
    Ok(())
}
