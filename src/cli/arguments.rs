use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "omnidev",
    version,
    about = "Track your code. Understand your growth.",
    long_about = "OmniDev is a local-first developer activity analytics platform for the terminal. \
                  All analysis happens on this machine; nothing is uploaded."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Create the configuration file and the local database")]
    Init(InitArgs),
    #[command(about = "Analyze a Git repository and store its history")]
    Analyze(AnalyzeArgs),
    #[command(about = "Display developer statistics")]
    Stats(StatsArgs),
    #[command(about = "Launch the interactive dashboard")]
    Dashboard,
    #[command(about = "Display the contribution heatmap")]
    Heatmap(HeatmapArgs),
    #[command(about = "List, register, scan and remove tracked repositories")]
    Projects(ProjectsArgs),
    #[command(about = "Explore commit history")]
    Commits(CommitsArgs),
    #[command(about = "Display language statistics")]
    Languages(LanguagesArgs),
    #[command(about = "Generate a report in JSON, Markdown or CSV")]
    Report(ReportArgs),
    #[command(about = "Manage developer goals and view achievements")]
    Goals(GoalsArgs),
    #[command(about = "Display and manage the configuration")]
    Config(ConfigArgs),
    #[command(about = "Refresh every tracked repository incrementally")]
    Refresh(RefreshArgs),
    #[command(about = "Display version and build information")]
    Version,
    #[command(about = "Check installation, configuration, database and repository access")]
    Doctor,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(
        long,
        help = "Recreate the configuration file after confirmation (the database is kept)"
    )]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct RefreshArgs {
    #[arg(
        long,
        help = "Discard stored history for every repository and read it again"
    )]
    pub rebuild: bool,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to the repository to analyze (defaults to the current directory)"
    )]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct PeriodArgs {
    #[arg(
        long,
        value_name = "RANGE",
        conflicts_with_all = ["from", "to"],
        help = "Relative range such as 7d, 12w, 6m, 1y or all"
    )]
    pub range: Option<String>,
    #[arg(long, value_name = "YYYY-MM-DD", help = "Start date")]
    pub from: Option<String>,
    #[arg(long, value_name = "YYYY-MM-DD", help = "End date (defaults to today)")]
    pub to: Option<String>,
}

#[derive(Debug, Args)]
pub struct StatsArgs {
    #[command(flatten)]
    pub period: PeriodArgs,
    #[arg(
        long,
        value_name = "REPO",
        help = "Limit statistics to one repository (name or path)"
    )]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct HeatmapArgs {
    #[command(flatten)]
    pub period: PeriodArgs,
    #[arg(
        long,
        value_name = "REPO",
        help = "Limit the heatmap to one repository (name or path)"
    )]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProjectsArgs {
    #[command(subcommand)]
    pub action: Option<ProjectAction>,
}

#[derive(Debug, Subcommand)]
pub enum ProjectAction {
    #[command(about = "List tracked repositories")]
    List,
    #[command(about = "Register a repository")]
    Add {
        #[arg(help = "Path to the repository")]
        path: PathBuf,
        #[arg(long, help = "Display name (defaults to the directory name)")]
        name: Option<String>,
    },
    #[command(about = "Stop tracking a repository (its files are never deleted)")]
    Remove {
        #[arg(help = "Repository name or path")]
        selector: String,
    },
    #[command(about = "Set the default repository")]
    Default {
        #[arg(help = "Repository name or path")]
        selector: String,
    },
    #[command(about = "Register Git repositories found in the configured or given directories")]
    Scan {
        #[arg(
            long,
            value_name = "DIR",
            help = "Directory to scan (repeatable); defaults to scan.directories in the configuration"
        )]
        dir: Vec<PathBuf>,
        #[arg(
            long,
            default_value_t = 3,
            help = "Maximum directory depth to descend into"
        )]
        depth: usize,
    },
}

#[derive(Debug, Args)]
pub struct CommitsArgs {
    #[command(flatten)]
    pub period: PeriodArgs,
    #[arg(
        long,
        value_name = "NAME",
        help = "Only commits whose author name or email contains this text"
    )]
    pub author: Option<String>,
    #[arg(
        long,
        value_name = "TEXT",
        help = "Only commits whose summary contains this text"
    )]
    pub search: Option<String>,
    #[arg(
        long,
        value_name = "REPO",
        help = "Limit to one repository (name or path)"
    )]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 1, help = "Page number, starting at 1")]
    pub page: u64,
    #[arg(long, default_value_t = 20, help = "Commits per page (1 to 200)")]
    pub per_page: u64,
    #[arg(long, help = "List the files changed by each commit")]
    pub files: bool,
}

#[derive(Debug, Args)]
pub struct LanguagesArgs {
    #[arg(
        long,
        value_name = "REPO",
        help = "Limit to one repository (name or path)"
    )]
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Json,
    Markdown,
    Csv,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    #[command(flatten)]
    pub period: PeriodArgs,
    #[arg(
        long,
        value_name = "REPO",
        help = "Limit the report to one repository (name or path)"
    )]
    pub repo: Option<String>,
    #[arg(long, value_enum, default_value_t = FormatArg::Markdown, help = "Output format")]
    pub format: FormatArg,
    #[arg(
        long,
        short,
        value_name = "FILE",
        help = "Write the report to this file instead of standard output"
    )]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Overwrite the output file if it already exists")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct GoalsArgs {
    #[command(subcommand)]
    pub action: Option<GoalAction>,
}

#[derive(Debug, Subcommand)]
pub enum GoalAction {
    #[command(about = "Show goal progress and achievements")]
    List,
    #[command(about = "Create or update a goal")]
    Set {
        #[arg(help = "weekly-commits, monthly-commits, languages or repository-maintenance")]
        kind: String,
        #[arg(help = "Target value (greater than zero)")]
        target: u64,
    },
    #[command(about = "Remove a goal")]
    Remove {
        #[arg(help = "Goal identifier")]
        kind: String,
    },
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: Option<ConfigAction>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show the current configuration")]
    Show,
    #[command(about = "Print one configuration value")]
    Get {
        #[arg(help = "Dotted key such as analytics.include_merges")]
        key: String,
    },
    #[command(about = "Change one configuration value")]
    Set {
        #[arg(help = "Dotted key such as analytics.include_merges")]
        key: String,
        #[arg(help = "New value")]
        value: String,
    },
    #[command(about = "Print the configuration file location")]
    Path,
}
