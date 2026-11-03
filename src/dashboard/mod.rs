pub mod app;
pub mod ui;

use std::io::{Stdout, stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::cli::context::{Context, today};
use crate::config::{AppPaths, Settings};
use crate::core::analyzer;
use crate::database::Database;
use crate::database::repositories;
use crate::errors::{OmniDevError, Result};
use app::{Action, App, Snapshot};

const POLL_INTERVAL: Duration = Duration::from_millis(200);

enum RefreshMessage {
    Finished(String),
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, Hide) {
            restore_terminal_best_effort();
            return Err(error.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal_best_effort();
    }
}

fn restore_terminal_best_effort() {
    if let Err(error) = disable_raw_mode() {
        tracing::warn!("could not disable raw mode: {error}");
    }
    if let Err(error) = execute!(stdout(), LeaveAlternateScreen, Show) {
        tracing::warn!("could not restore the terminal screen: {error}");
    }
}

pub fn run(context: Context) -> Result<()> {
    let Context {
        paths,
        settings,
        database,
        ..
    } = context;
    let snapshot = Snapshot::load(&database, &settings, today())?;
    let mut app = App::new(snapshot);
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let (sender, receiver) = mpsc::channel::<RefreshMessage>();
    event_loop(
        &mut terminal,
        &mut app,
        &database,
        &paths,
        &settings,
        &sender,
        &receiver,
    )
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    database: &Database,
    paths: &AppPaths,
    settings: &Settings,
    sender: &Sender<RefreshMessage>,
    receiver: &Receiver<RefreshMessage>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;
        if event::poll(POLL_INTERVAL)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.handle_key(key) {
                        Action::Quit => return Ok(()),
                        Action::Refresh => start_refresh(app, paths, settings, sender),
                        Action::None => {}
                    }
                }
            }
        }
        while let Ok(RefreshMessage::Finished(status)) = receiver.try_recv() {
            let snapshot = Snapshot::load(database, settings, today())?;
            app.apply_snapshot(snapshot, status);
        }
    }
}

fn start_refresh(
    app: &mut App,
    paths: &AppPaths,
    settings: &Settings,
    sender: &Sender<RefreshMessage>,
) {
    app.refreshing = true;
    app.status = "Refreshing repositories in the background…".to_owned();
    let paths = paths.clone();
    let settings = settings.clone();
    let sender = sender.clone();
    thread::spawn(move || {
        let status = match refresh_all(&paths, &settings) {
            Ok(summary) => summary,
            Err(error) => format!("Refresh failed: {error}"),
        };
        if sender.send(RefreshMessage::Finished(status)).is_err() {
            tracing::debug!("dashboard closed before the refresh finished");
        }
    });
}

fn refresh_all(paths: &AppPaths, settings: &Settings) -> Result<String> {
    let mut database = Database::open(&paths.database_path(settings))?;
    analyzer::apply_timezone_mode(&mut database, settings)?;
    let records = repositories::list(database.connection())?;
    let mut failures = 0_usize;
    let mut new_commits = 0_usize;
    for record in &records {
        match analyzer::sync_repository(&mut database, record, settings) {
            Ok(outcome) => new_commits += outcome.new_commits,
            Err(error) => {
                tracing::warn!("refresh of {} failed: {error}", record.name);
                failures += 1;
            }
        }
    }
    if records.is_empty() {
        return Err(OmniDevError::NoDefaultRepository);
    }
    Ok(match failures {
        0 => format!(
            "Refreshed {} repo(s): {new_commits} new commit(s)",
            records.len()
        ),
        count => format!("Refreshed with {count} failure(s): {new_commits} new commit(s)"),
    })
}
