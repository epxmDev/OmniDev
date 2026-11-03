use std::collections::BTreeMap;

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::Settings;
use crate::core::statistics::{DeveloperStatistics, StatisticsOptions, compute};
use crate::database::Database;
use crate::database::commits::{self, CommitFilter, CommitListing};
use crate::database::languages::{self, LanguageTally};
use crate::database::repositories::{self, RepositoryRecord};
use crate::errors::Result;

pub const RECENT_LIMIT: u64 = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub is_default: bool,
    pub commits: u64,
    pub last_synced: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub today: NaiveDate,
    pub current: Option<RepositoryRecord>,
    pub current_commits: u64,
    pub statistics: DeveloperStatistics,
    pub daily: BTreeMap<NaiveDate, u64>,
    pub languages: Vec<LanguageTally>,
    pub recent: Vec<CommitListing>,
    pub repositories: Vec<RepositoryRow>,
}

impl Snapshot {
    pub fn load(database: &Database, settings: &Settings, today: NaiveDate) -> Result<Self> {
        let connection = database.connection();
        let include_merges = settings.analytics.include_merges;
        let current = repositories::default_repository(connection)?;
        let current_commits = match &current {
            Some(record) => commits::count_commits(
                connection,
                &CommitFilter {
                    repository_id: Some(record.id),
                    include_merges,
                    ..CommitFilter::default()
                },
            )?,
            None => 0,
        };
        let records = commits::load_activity_records(
            connection,
            &CommitFilter {
                include_merges: true,
                ..CommitFilter::default()
            },
        )?;
        let statistics = compute(
            &records,
            StatisticsOptions {
                include_merges,
                period: None,
                today,
            },
        );
        let daily = commits::daily_totals(connection, None, include_merges)?
            .into_iter()
            .collect();
        let recent = commits::list_commits(
            connection,
            &CommitFilter {
                include_merges,
                ..CommitFilter::default()
            },
            RECENT_LIMIT,
            0,
        )?;
        let counts = commits::repository_commit_counts(connection, include_merges)?;
        let mut repositories_rows = Vec::new();
        for record in repositories::list(connection)? {
            let commit_count = counts
                .iter()
                .find(|(id, _, _)| *id == record.id)
                .map_or(0, |(_, _, count)| *count);
            repositories_rows.push(RepositoryRow {
                id: record.id,
                name: record.name,
                path: record.path.to_string_lossy().into_owned(),
                is_default: record.is_default,
                commits: commit_count,
                last_synced: record.last_synced_at,
            });
        }
        repositories_rows.sort_by(|left, right| {
            right
                .commits
                .cmp(&left.commits)
                .then(left.name.cmp(&right.name))
        });
        Ok(Self {
            today,
            current,
            current_commits,
            statistics,
            daily,
            languages: languages::aggregate_languages(connection, None)?,
            recent,
            repositories: repositories_rows,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Repository,
    Statistics,
    Heatmap,
    Timeline,
    Languages,
    Recent,
    Overview,
    Projects,
}

impl Panel {
    pub const ORDER: [Panel; 8] = [
        Panel::Repository,
        Panel::Statistics,
        Panel::Heatmap,
        Panel::Timeline,
        Panel::Languages,
        Panel::Recent,
        Panel::Overview,
        Panel::Projects,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Panel::Repository => "Current repository",
            Panel::Statistics => "Global statistics",
            Panel::Heatmap => "Contribution heatmap",
            Panel::Timeline => "Activity timeline",
            Panel::Languages => "Language distribution",
            Panel::Recent => "Recent commits",
            Panel::Overview => "Repository overview",
            Panel::Projects => "Most active projects",
        }
    }

    fn next(self) -> Panel {
        let index = Self::ORDER
            .iter()
            .position(|panel| *panel == self)
            .unwrap_or(0);
        Self::ORDER[(index + 1) % Self::ORDER.len()]
    }

    fn previous(self) -> Panel {
        let index = Self::ORDER
            .iter()
            .position(|panel| *panel == self)
            .unwrap_or(0);
        Self::ORDER[(index + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Refresh,
    Quit,
}

pub struct App {
    pub snapshot: Snapshot,
    pub focus: Panel,
    pub expanded: bool,
    pub show_help: bool,
    pub heatmap_offset_weeks: i64,
    pub recent_selection: usize,
    pub repository_selection: usize,
    pub status: String,
    pub refreshing: bool,
}

impl App {
    pub fn new(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            focus: Panel::Heatmap,
            expanded: false,
            show_help: false,
            heatmap_offset_weeks: 0,
            recent_selection: 0,
            repository_selection: 0,
            status: "Ready".to_owned(),
            refreshing: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        if self.show_help {
            let closes = matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('h' | 'H' | 'q' | 'Q' | '?')
            );
            if closes {
                self.show_help = false;
            }
            return Action::None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            KeyCode::Esc => {
                if self.expanded {
                    self.expanded = false;
                    Action::None
                } else {
                    Action::Quit
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
                self.show_help = true;
                Action::None
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if self.refreshing {
                    Action::None
                } else {
                    Action::Refresh
                }
            }
            KeyCode::Tab => {
                self.focus = self.focus.next();
                Action::None
            }
            KeyCode::BackTab => {
                self.focus = self.focus.previous();
                Action::None
            }
            KeyCode::Enter => {
                self.expanded = !self.expanded;
                Action::None
            }
            KeyCode::Up => {
                self.move_selection(-1);
                Action::None
            }
            KeyCode::Down => {
                self.move_selection(1);
                Action::None
            }
            KeyCode::Left => {
                if self.focus == Panel::Heatmap {
                    self.heatmap_offset_weeks += 1;
                } else {
                    self.focus = self.focus.previous();
                }
                Action::None
            }
            KeyCode::Right => {
                if self.focus == Panel::Heatmap {
                    self.heatmap_offset_weeks = (self.heatmap_offset_weeks - 1).max(0);
                } else {
                    self.focus = self.focus.next();
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Panel::Recent => {
                self.recent_selection =
                    step(self.recent_selection, delta, self.snapshot.recent.len());
            }
            Panel::Overview => {
                self.repository_selection = step(
                    self.repository_selection,
                    delta,
                    self.snapshot.repositories.len(),
                );
            }
            _ => {}
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: Snapshot, status: String) {
        self.snapshot = snapshot;
        self.refreshing = false;
        self.status = status;
        self.recent_selection = self
            .recent_selection
            .min(self.snapshot.recent.len().saturating_sub(1));
        self.repository_selection = self
            .repository_selection
            .min(self.snapshot.repositories.len().saturating_sub(1));
    }
}

fn step(current: usize, delta: isize, length: usize) -> usize {
    if length == 0 {
        return 0;
    }
    let next = current as isize + delta;
    next.clamp(0, length as isize - 1) as usize
}
