pub mod analytics;
pub mod cli;
pub mod config;
pub mod core;
pub mod dashboard;
pub mod database;
pub mod errors;
pub mod git;
pub mod output;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::arguments::{Cli, Command};
use crate::cli::{commands, management, projects};

pub fn run() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let theme = crate::output::colors::Theme::detect(true);
            eprintln!("{} {}", theme.error(theme.symbols().failure), error);
            for cause in error.chain().skip(1) {
                eprintln!("  caused by: {cause}");
            }
            ExitCode::FAILURE
        }
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("OMNIDEV_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .init();
}

fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        None => management::welcome(),
        Some(Command::Init(args)) => commands::init(&args),
        Some(Command::Analyze(args)) => commands::analyze(&args),
        Some(Command::Stats(args)) => commands::stats(&args),
        Some(Command::Dashboard) => {
            let context = cli::context::Context::open()?;
            dashboard::run(context)?;
            Ok(())
        }
        Some(Command::Heatmap(args)) => commands::heatmap(&args),
        Some(Command::Projects(args)) => projects::run(&args),
        Some(Command::Commits(args)) => commands::commits(&args),
        Some(Command::Languages(args)) => commands::languages(&args),
        Some(Command::Report(args)) => management::report(&args),
        Some(Command::Goals(args)) => management::goals(&args),
        Some(Command::Config(args)) => management::config(&args),
        Some(Command::Refresh(args)) => commands::refresh(&args),
        Some(Command::Version) => management::version(),
        Some(Command::Doctor) => management::doctor(),
    }
}
