use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Sparkline, Wrap};

use crate::analytics::heatmap::{self, DEFAULT_HEATMAP_DAYS, WEEKDAY_LABELS};
use crate::core::activity::{DateRange, format_day};
use crate::core::statistics::DeveloperStatistics;
use crate::dashboard::app::{App, Panel};
use crate::output::formatting::{bar, grouped, percent, truncate};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const SUCCESS: Color = Color::Green;
const WIDE_LAYOUT_WIDTH: u16 = 110;

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);
    render_header(frame, sections[0], app);
    render_footer(frame, sections[2], app);
    if app.expanded {
        render_panel(frame, sections[1], app, app.focus, true);
    } else if sections[1].width >= WIDE_LAYOUT_WIDTH {
        render_wide(frame, sections[1], app);
    } else {
        render_narrow(frame, sections[1], app);
    }
    if app.show_help {
        render_help(frame, area);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let status_style = if app.refreshing {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(SUCCESS)
    };
    let line = Line::from(vec![
        Span::styled(
            " OmniDev ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" v{}  ", env!("CARGO_PKG_VERSION"))),
        Span::styled(
            "Track your code. Understand your growth.",
            Style::default().fg(Color::Blue),
        ),
        Span::raw("   "),
        Span::styled(app.status.clone(), status_style),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT)),
        ),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let hints = if app.expanded {
        "Enter/Esc collapse · Tab switch · ↑↓ scroll · R refresh · H help · Q quit"
    } else {
        "←→ move/shift heatmap · Tab switch · ↑↓ select · Enter expand · R refresh · H help · Q quit"
    };
    frame.render_widget(
        Paragraph::new(hints).style(Style::default().fg(MUTED)),
        area,
    );
}

fn render_wide(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(11),
        Constraint::Length(9),
        Constraint::Min(8),
    ])
    .split(area);
    let top =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(rows[0]);
    render_panel(frame, top[0], app, Panel::Repository, false);
    render_panel(frame, top[1], app, Panel::Statistics, false);
    render_panel(frame, rows[1], app, Panel::Heatmap, false);
    let middle =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[2]);
    render_panel(frame, middle[0], app, Panel::Timeline, false);
    render_panel(frame, middle[1], app, Panel::Languages, false);
    let bottom =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(rows[3]);
    render_panel(frame, bottom[0], app, Panel::Recent, false);
    let side =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).split(bottom[1]);
    render_panel(frame, side[0], app, Panel::Overview, false);
    render_panel(frame, side[1], app, Panel::Projects, false);
}

fn render_narrow(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let order = [
        Panel::Repository,
        Panel::Statistics,
        Panel::Heatmap,
        Panel::Timeline,
        Panel::Languages,
        Panel::Recent,
        Panel::Overview,
        Panel::Projects,
    ];
    let heights = [
        Constraint::Length(7),
        Constraint::Length(12),
        Constraint::Length(11),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Min(8),
        Constraint::Min(6),
        Constraint::Min(6),
    ];
    let rows = Layout::vertical(heights).split(area);
    for (panel, rect) in order.into_iter().zip(rows.iter()) {
        render_panel(frame, *rect, app, panel, false);
    }
}

fn render_panel(frame: &mut Frame<'_>, area: Rect, app: &App, panel: Panel, expanded: bool) {
    let focused = app.focus == panel;
    let border_color = if focused { ACCENT } else { MUTED };
    let title = if focused {
        format!(" ▸ {} ", panel.title())
    } else {
        format!(" {} ", panel.title())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    match panel {
        Panel::Repository => render_repository(frame, inner, app),
        Panel::Statistics => render_statistics(frame, inner, &app.snapshot.statistics),
        Panel::Heatmap => render_heatmap(frame, inner, app),
        Panel::Timeline => render_timeline(frame, inner, &app.snapshot.statistics),
        Panel::Languages => render_languages(frame, inner, app, expanded),
        Panel::Recent => render_recent(frame, inner, app),
        Panel::Overview => render_overview(frame, inner, app),
        Panel::Projects => render_projects(frame, inner, app),
    }
}

fn label_value(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<16}"), Style::default().fg(MUTED)),
        Span::styled(value, Style::default().add_modifier(Modifier::BOLD)),
    ])
}

fn render_repository(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let snapshot = &app.snapshot;
    let lines = match &snapshot.current {
        Some(record) => vec![
            label_value("Name", record.name.clone()),
            label_value(
                "Path",
                truncate(
                    &record.path.to_string_lossy(),
                    area.width.saturating_sub(18) as usize,
                ),
            ),
            label_value("Commits", grouped(snapshot.current_commits as i64)),
            label_value(
                "Head",
                record
                    .head_commit
                    .as_deref()
                    .map(|hash| hash.chars().take(12).collect::<String>())
                    .unwrap_or_else(|| "-".to_owned()),
            ),
            label_value(
                "Last synced",
                record
                    .last_synced_at
                    .clone()
                    .unwrap_or_else(|| "never".to_owned()),
            ),
        ],
        None => vec![
            Line::from(Span::styled(
                "No default repository.",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("Add one with `omnidev projects add <path>`."),
        ],
    };
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_statistics(frame: &mut Frame<'_>, area: Rect, statistics: &DeveloperStatistics) {
    let weekday = statistics
        .most_active_weekday
        .as_ref()
        .map_or("-".to_owned(), |activity| activity.weekday.clone());
    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<16}", "Commits"), Style::default().fg(MUTED)),
            Span::styled(
                grouped(statistics.total_commits as i64),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "   repos {}   contributors {}",
                statistics.total_repositories, statistics.total_contributors
            )),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<16}", "Lines"), Style::default().fg(MUTED)),
            Span::styled(
                format!("+{}", grouped(statistics.insertions as i64)),
                Style::default().fg(SUCCESS),
            ),
            Span::raw(" / "),
            Span::styled(
                format!("-{}", grouped(statistics.deletions as i64)),
                Style::default().fg(Color::Red),
            ),
            Span::raw(format!("   net {}", grouped(statistics.net_lines))),
        ]),
        label_value(
            "Avg / day",
            format!("{:.2}", statistics.average_commits_per_day),
        ),
        label_value(
            "Avg / week",
            format!("{:.2}", statistics.average_commits_per_week),
        ),
        label_value(
            "Frequency",
            format!(
                "{} of days active",
                percent(statistics.contribution_frequency * 100.0)
            ),
        ),
        label_value(
            "Streaks",
            format!(
                "longest {} · current {} days",
                statistics.longest_streak_days, statistics.current_streak_days
            ),
        ),
        label_value("Busiest weekday", weekday),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_heatmap(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let usable_columns = (area.width.saturating_sub(6) / 2).max(1) as i64;
    let weeks = usable_columns.min(DEFAULT_HEATMAP_DAYS / 7 + 1);
    let end = app.snapshot.today - chrono::Days::new((app.heatmap_offset_weeks * 7) as u64);
    let start = end - chrono::Days::new((weeks * 7 - 1) as u64);
    let Ok(range) = DateRange::new(start, end) else {
        return;
    };
    let grid = heatmap::build(range, &app.snapshot.daily);
    let mut lines = Vec::new();
    let mut label_line = vec![' '; 4 + grid.columns * 2];
    for (column, month) in grid.month_labels() {
        for (offset, character) in month.chars().enumerate() {
            if let Some(slot) = label_line.get_mut(4 + column * 2 + offset) {
                *slot = character;
            }
        }
    }
    lines.push(Line::from(Span::styled(
        label_line.into_iter().collect::<String>(),
        Style::default().fg(MUTED),
    )));
    for (row, weekday) in WEEKDAY_LABELS.iter().enumerate() {
        let mut spans = vec![Span::styled(
            format!("{weekday:<4}"),
            Style::default().fg(MUTED),
        )];
        for column in 0..grid.columns {
            match grid.cells[row][column] {
                Some(cell) => spans.push(Span::styled(
                    format!("{} ", heatmap::glyph(cell.level, false)),
                    Style::default().fg(level_color(cell.level)),
                )),
                None => spans.push(Span::raw("  ")),
            }
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} → {}", format_day(start), format_day(end)),
            Style::default().fg(MUTED),
        ),
        Span::raw(format!(
            "   {} commits on {} days",
            grouped(grid.total_commits() as i64),
            grid.active_days()
        )),
    ]));
    frame.render_widget(Paragraph::new(lines), area);
}

fn level_color(level: u8) -> Color {
    match level {
        0 => Color::DarkGray,
        1 => Color::Blue,
        2 => Color::LightBlue,
        3 => Color::Cyan,
        _ => Color::LightGreen,
    }
}

fn render_timeline(frame: &mut Frame<'_>, area: Rect, statistics: &DeveloperStatistics) {
    let skip = statistics.monthly_activity.len().saturating_sub(24);
    let months: Vec<_> = statistics.monthly_activity.iter().skip(skip).collect();
    if months.is_empty() {
        frame.render_widget(Paragraph::new("No activity recorded yet."), area);
        return;
    }
    let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let data: Vec<u64> = months.iter().map(|month| month.commits).collect();
    frame.render_widget(
        Sparkline::default()
            .data(&data)
            .style(Style::default().fg(ACCENT)),
        parts[0],
    );
    let first = months.first().map_or("", |month| month.month.as_str());
    let last = months.last().map_or("", |month| month.month.as_str());
    frame.render_widget(
        Paragraph::new(format!(
            "{first}  →  {last}  ({} months, monthly commits)",
            months.len()
        ))
        .style(Style::default().fg(MUTED)),
        parts[1],
    );
}

fn render_languages(frame: &mut Frame<'_>, area: Rect, app: &App, expanded: bool) {
    let languages = &app.snapshot.languages;
    if languages.is_empty() {
        frame.render_widget(
            Paragraph::new("No recognized source files yet. Run `omnidev refresh`."),
            area,
        );
        return;
    }
    let total_lines: u64 = languages.iter().map(|language| language.lines).sum();
    let limit = if expanded {
        usize::MAX
    } else {
        area.height.saturating_sub(1) as usize
    };
    let width = area.width.saturating_sub(34) as usize;
    let mut lines: Vec<Line<'_>> = languages
        .iter()
        .take(limit.max(1))
        .map(|language| {
            let share = if total_lines == 0 {
                0.0
            } else {
                language.lines as f64 / total_lines as f64
            };
            Line::from(vec![
                Span::styled(
                    format!("{:<12}", truncate(&language.language, 12)),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(bar(share, width.max(4), false), Style::default().fg(ACCENT)),
                Span::raw(format!(" {:>6}", percent(share * 100.0))),
            ])
        })
        .collect();
    lines.push(Line::from(Span::styled(
        "estimated by lines at HEAD",
        Style::default().fg(MUTED),
    )));
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_recent(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let recent = &app.snapshot.recent;
    if recent.is_empty() {
        frame.render_widget(
            Paragraph::new("No commits yet. Run `omnidev analyze` in a repository."),
            area,
        );
        return;
    }
    let visible = area.height as usize;
    let start = app
        .recent_selection
        .saturating_sub(visible.saturating_sub(1));
    let lines: Vec<Line<'_>> = recent
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, commit)| {
            let selected = index == app.recent_selection && app.focus == Panel::Recent;
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let width = area.width.saturating_sub(40) as usize;
            Line::from(vec![
                Span::styled(
                    format!("{} ", commit.hash.chars().take(7).collect::<String>()),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(format!("{} ", commit.day), Style::default().fg(MUTED)),
                Span::styled(
                    format!("{:<14}", truncate(&commit.author_name, 14)),
                    Style::default().fg(ACCENT),
                ),
                Span::styled(truncate(&commit.summary, width), style),
                Span::styled(
                    format!(" +{}/-{}", commit.insertions, commit.deletions),
                    Style::default().fg(MUTED),
                ),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_overview(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = &app.snapshot.repositories;
    if rows.is_empty() {
        frame.render_widget(Paragraph::new("No repositories tracked."), area);
        return;
    }
    let visible = area.height as usize;
    let start = app
        .repository_selection
        .saturating_sub(visible.saturating_sub(1));
    let lines: Vec<Line<'_>> = rows
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, row)| {
            let marker = if row.is_default { "◆" } else { " " };
            let selected = index == app.repository_selection && app.focus == Panel::Overview;
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(format!("{marker} "), Style::default().fg(SUCCESS)),
                Span::styled(format!("{:<18}", truncate(&row.name, 18)), style),
                Span::raw(format!("{:>8} commits  ", grouped(row.commits as i64))),
                Span::styled(
                    row.last_synced
                        .clone()
                        .unwrap_or_else(|| "never synced".to_owned()),
                    Style::default().fg(MUTED),
                ),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_projects(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let rows = &app.snapshot.repositories;
    let total: u64 = rows.iter().map(|row| row.commits).sum();
    let width = area.width.saturating_sub(34) as usize;
    let lines: Vec<Line<'_>> = rows
        .iter()
        .take(area.height as usize)
        .map(|row| {
            let share = if total == 0 {
                0.0
            } else {
                row.commits as f64 / total as f64
            };
            Line::from(vec![
                Span::styled(
                    format!("{:<16}", truncate(&row.name, 16)),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    bar(share, width.max(4), false),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(format!(" {:>6}", percent(share * 100.0))),
            ])
        })
        .collect();
    if lines.is_empty() {
        frame.render_widget(Paragraph::new("Nothing to rank yet."), area);
    } else {
        frame.render_widget(Paragraph::new(lines), area);
    }
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(area, 64, 16);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(Span::styled(
            "Keyboard shortcuts",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Tab / Shift+Tab    switch panel"),
        Line::from("  ↑ ↓                move selection in lists"),
        Line::from("  ← →                shift heatmap by a week (when focused)"),
        Line::from("  Enter              expand or collapse the focused panel"),
        Line::from("  R                  refresh tracked repositories"),
        Line::from("  H or ?             show or hide this help"),
        Line::from("  Q / Esc            quit (Esc collapses an expanded panel first)"),
        Line::from(""),
        Line::from(Span::styled(
            "Refresh runs in the background; the dashboard stays responsive.",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ACCENT))
                    .title(" Help "),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let vertical = Layout::vertical([
        Constraint::Length(area.height.saturating_sub(height) / 2),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Length(area.width.saturating_sub(width) / 2),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(vertical[1])[1]
}
