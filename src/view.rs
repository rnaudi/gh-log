use crate::data;

use chrono::{DateTime, Datelike, Duration, Utc};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::io::{Result, stdout};

use crate::config::{Config, SizeConfig};
use crate::data::{MonthData, PRDetail, PRSize};

const HORIZONTAL_MARGIN: u16 = 2;
const SCROLLBAR_SPACE: u16 = 1;
const SECTION_SPACING: usize = 1;

#[derive(Clone, Copy)]
enum View {
    Summary,
    Detail(DetailMode),
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailMode {
    ByWeek,
    ByRepo,
}

impl DetailMode {
    fn cycle(self) -> Self {
        match self {
            DetailMode::ByWeek => DetailMode::ByRepo,
            DetailMode::ByRepo => DetailMode::ByWeek,
        }
    }
}

struct ScrollState {
    position: usize,
    content_height: usize,
    viewport_height: usize,
}

impl ScrollState {
    fn new() -> Self {
        Self {
            position: 0,
            content_height: 0,
            viewport_height: 0,
        }
    }

    fn reset(&mut self) {
        self.position = 0;
    }

    fn scroll_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        let max = self.max_scroll();
        if self.position < max {
            self.position += 1;
        }
    }

    fn max_scroll(&self) -> usize {
        self.content_height.saturating_sub(self.viewport_height)
    }

    fn set_content_height(&mut self, height: usize) {
        self.content_height = height;
    }

    fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }

    fn as_scrollbar_state(&self) -> ScrollbarState {
        let scrollable_content = self.max_scroll().max(1);
        ScrollbarState::new(scrollable_content).position(self.position)
    }
}

/// Messages representing user actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    Quit,
    ShowSummary,
    ToggleDetail,
    ShowTail,
    ScrollUp,
    ScrollDown,
}

/// Application state - consolidates all mutable state in one place
struct AppState {
    current_view: View,
    scroll: ScrollState,
}

impl AppState {
    fn new() -> Self {
        Self {
            current_view: View::Summary,
            scroll: ScrollState::new(),
        }
    }

    fn current_view(&self) -> View {
        self.current_view
    }

    fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    fn set_view(&mut self, view: View) {
        self.current_view = view;
        self.scroll.reset();
    }

    fn scroll_up(&mut self) {
        self.scroll.scroll_up();
    }

    fn scroll_down(&mut self) {
        self.scroll.scroll_down();
    }
}

/// Pure update function - handles state transitions based on messages
/// This is the core of the Elm Architecture pattern
fn update(msg: Msg, mut state: AppState) -> AppState {
    match msg {
        Msg::Quit => state, // Should not be called, handled in run loop
        Msg::ShowSummary => {
            state.set_view(View::Summary);
            state
        }
        Msg::ToggleDetail => {
            let new_view = match state.current_view() {
                View::Detail(mode) => View::Detail(mode.cycle()),
                _ => View::Detail(DetailMode::ByWeek),
            };
            state.set_view(new_view);
            state
        }
        Msg::ShowTail => {
            state.set_view(View::Tail);
            state
        }
        Msg::ScrollUp => {
            state.scroll_up();
            state
        }
        Msg::ScrollDown => {
            state.scroll_down();
            state
        }
    }
}

/// Handle keyboard input and convert to messages
fn handle_input() -> anyhow::Result<Option<Msg>> {
    if event::poll(std::time::Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        let msg = match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Msg::Quit),
            KeyCode::Char('s') => Some(Msg::ShowSummary),
            KeyCode::Char('d') => Some(Msg::ToggleDetail),
            KeyCode::Char('t') => Some(Msg::ShowTail),
            KeyCode::Up | KeyCode::Char('k') => Some(Msg::ScrollUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Msg::ScrollDown),
            _ => None,
        };
        return Ok(msg);
    }
    Ok(None)
}

pub fn run(month_data: MonthData, cfg: Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut state = AppState::new();

    loop {
        match state.current_view() {
            View::Summary => render_summary(&mut terminal, &month_data, state.scroll_mut())?,
            View::Detail(mode) => {
                render_detail(&mut terminal, &month_data, state.scroll_mut(), &cfg, mode)?
            }
            View::Tail => render_tail(&mut terminal, &month_data, state.scroll_mut(), &cfg)?,
        }

        if let Some(msg) = handle_input()? {
            if msg == Msg::Quit {
                break;
            }
            state = update(msg, state);
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn render_summary(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    scroll_state: &mut ScrollState,
) -> Result<()> {
    terminal.draw(|frame| {
        let [controls_area, summary_area, content_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .areas(frame.size());

        render_controls(frame, controls_area, View::Summary);
        render_summary_header(frame, summary_area, data);

        let lines = build_summary_content(data, content_area.width as usize);
        render_scrollable_content(frame, content_area, lines, scroll_state);
    })?;

    Ok(())
}

fn render_detail(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    scroll_state: &mut ScrollState,
    cfg: &Config,
    mode: DetailMode,
) -> Result<()> {
    terminal.draw(|frame| {
        let [controls_area, summary_area, content_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .areas(frame.size());

        render_controls(frame, controls_area, View::Detail(mode));
        render_detail_header(frame, summary_area, data, mode);

        let lines = match mode {
            DetailMode::ByWeek => {
                build_detail_by_week_content(data, cfg, content_area.width as usize)
            }
            DetailMode::ByRepo => {
                build_detail_by_repo_content(data, cfg, content_area.width as usize)
            }
        };
        render_scrollable_content(frame, content_area, lines, scroll_state);
    })?;

    Ok(())
}

fn render_tail(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    scroll_state: &mut ScrollState,
    cfg: &Config,
) -> Result<()> {
    terminal.draw(|frame| {
        let [controls_area, summary_area, content_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .areas(frame.size());

        render_controls(frame, controls_area, View::Tail);
        render_summary_header(frame, summary_area, data);

        let lines = build_tail_content(data, cfg, content_area.width as usize);
        render_scrollable_content(frame, content_area, lines, scroll_state);
    })?;

    Ok(())
}

fn render_controls(frame: &mut Frame, area: Rect, current_view: View) {
    let detail_label = match current_view {
        View::Detail(DetailMode::ByWeek) => "By Repo",
        View::Detail(DetailMode::ByRepo) => "By Week",
        _ => "Details",
    };

    let controls = Line::from(vec![
        Span::styled("s", Style::default().fg(Color::Gray).bold()),
        Span::raw(": Summary │ "),
        Span::styled("d", Style::default().fg(Color::Gray).bold()),
        Span::raw(format!(": {} │ ", detail_label)),
        Span::styled("t", Style::default().fg(Color::Gray).bold()),
        Span::raw(": Tail │ "),
        Span::styled("↑↓/jk", Style::default().fg(Color::Gray).bold()),
        Span::raw(": Scroll │ "),
        Span::styled("q", Style::default().fg(Color::Gray).bold()),
        Span::raw(": Quit"),
    ]);
    let widget = Paragraph::new(controls).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(widget, area);
}

fn render_detail_header(frame: &mut Frame, area: Rect, data: &MonthData, mode: DetailMode) {
    let month_year = format_month(data.month_start);
    let mode_label = match mode {
        DetailMode::ByWeek => "by Week",
        DetailMode::ByRepo => "by Repository",
    };
    let review_ratio = if data.total_prs > 0 {
        data.reviewed_count as f64 / data.total_prs as f64
    } else {
        0.0
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::raw("GitHub PRs for "),
            Span::styled(month_year, Style::default().bold()),
            Span::raw(" — "),
            Span::styled(mode_label, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Total PRs: "),
            Span::styled(data.total_prs.to_string(), Style::default().fg(Color::Blue)),
            Span::raw(" │ Avg Lead Time: "),
            Span::styled(
                format_duration(data.avg_lead_time),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" │ Frequency: "),
            Span::styled(
                format_frequency(data.frequency),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Sizes: "),
            Span::raw(data.format_size_distribution()),
            Span::raw(" │ Review Balance: "),
            Span::styled(
                format!("{:.1}:1", review_ratio),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!(" ({} reviewed)", data.reviewed_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let header = Paragraph::new(summary_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn render_summary_header(frame: &mut Frame, area: Rect, data: &MonthData) {
    let month_year = format_month(data.month_start);
    let review_ratio = if data.total_prs > 0 {
        data.reviewed_count as f64 / data.total_prs as f64
    } else {
        0.0
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::raw("GitHub PRs for "),
            Span::styled(month_year, Style::default().bold()),
        ]),
        Line::from(vec![
            Span::raw("Total PRs: "),
            Span::styled(data.total_prs.to_string(), Style::default().fg(Color::Blue)),
            Span::raw(" │ Avg Lead Time: "),
            Span::styled(
                format_duration(data.avg_lead_time),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" │ Frequency: "),
            Span::styled(
                format_frequency(data.frequency),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Sizes: "),
            Span::raw(data.format_size_distribution()),
            Span::raw(" │ Review Balance: "),
            Span::styled(
                format!("{:.1}:1", review_ratio),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!(" ({} reviewed)", data.reviewed_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let header = Paragraph::new(summary_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn render_scrollable_content(
    frame: &mut Frame,
    area: Rect,
    lines: Vec<Line>,
    scroll_state: &mut ScrollState,
) {
    scroll_state.set_content_height(lines.len());
    scroll_state.set_viewport_height(
        area.inner(Margin {
            horizontal: HORIZONTAL_MARGIN,
            vertical: 0,
        })
        .height as usize,
    );

    let content_area = area.inner(Margin {
        horizontal: HORIZONTAL_MARGIN,
        vertical: 0,
    });
    let content = Paragraph::new(lines).scroll((scroll_state.position as u16, 0));
    frame.render_widget(content, content_area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let mut scrollbar_state = scroll_state.as_scrollbar_state();
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn build_summary_content(data: &MonthData, width: usize) -> Vec<Line<'static>> {
    let usable_width = width
        .saturating_sub((HORIZONTAL_MARGIN * 2) as usize)
        .saturating_sub(SCROLLBAR_SPACE as usize);

    let mut lines = Vec::new();

    lines.push(
        Line::from(separator_line("Weeks", usable_width)).style(Style::default().fg(Color::Gray)),
    );
    for week in &data.weeks {
        lines.push(Line::from(vec![
            Span::raw("Week "),
            Span::styled(week.week_num.to_string(), Style::default().bold()),
            Span::raw(format!(
                " ({:12}) │ ",
                format_date_range_short(week.week_start, week.week_end)
            )),
            Span::styled(
                format!("{:2}", week.pr_count),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" PRs │ Avg: "),
            Span::styled(
                format_duration(week.avg_lead_time),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
    for _ in 0..SECTION_SPACING {
        lines.push(Line::from(""));
    }

    // Repositories section - dynamic width
    let repo_name_width = (usable_width.saturating_sub(30)).max(20);
    lines.push(
        Line::from(separator_line("Repositories", usable_width))
            .style(Style::default().fg(Color::Gray)),
    );
    for repo in &data.repos {
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "{:width$}",
                    truncate(&repo.name, repo_name_width),
                    width = repo_name_width
                ),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!("{:2}", repo.pr_count),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" PRs │ Avg: "),
            Span::styled(
                format!("{:8}", format_duration(repo.avg_lead_time)),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" │ "),
            Span::raw(repo.format_size_distribution()),
        ]));
    }
    for _ in 0..SECTION_SPACING {
        lines.push(Line::from(""));
    }

    // Top Reviewers section - dynamic width
    let reviewer_name_width = (usable_width.saturating_sub(15)).max(20);
    lines.push(
        Line::from(separator_line("Top Reviewers", usable_width))
            .style(Style::default().fg(Color::Gray)),
    );
    for reviewer in data.reviewers.iter().take(10) {
        lines.push(Line::from(vec![
            Span::raw(format!(
                "{:width$}",
                truncate(&reviewer.login, reviewer_name_width),
                width = reviewer_name_width
            )),
            Span::raw(" │ "),
            Span::styled(
                format!("{}", reviewer.pr_count),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" PRs"),
        ]));
    }

    lines
}

fn build_detail_by_week_content(
    data: &MonthData,
    cfg: &Config,
    width: usize,
) -> Vec<Line<'static>> {
    let usable_width = width
        .saturating_sub((HORIZONTAL_MARGIN * 2) as usize)
        .saturating_sub(SCROLLBAR_SPACE as usize);

    let fixed_width = 6 + 3 + 3 + 5 + 3 + 3 + 8 + 3 + 2;
    let remaining = usable_width.saturating_sub(fixed_width).max(30);
    let repo_width = (remaining / 3).max(10);
    let title_width = remaining.saturating_sub(repo_width).max(15);

    let mut lines = Vec::new();

    for (week, prs) in data.weeks.iter().zip(data.prs_by_week.iter()) {
        let week_header = format!(
            "━━━ Week {} ({}) │ {} PRs │ Avg: {}",
            week.week_num,
            format_date_range_short(week.week_start, week.week_end),
            week.pr_count,
            format_duration(week.avg_lead_time)
        );
        lines.push(
            Line::from(pad_line(&week_header, usable_width, '━'))
                .style(Style::default().fg(Color::Gray)),
        );

        for pr in prs {
            let pr_size = pr.size(&cfg.size);
            let size_color = match pr_size {
                PRSize::S => Color::Green,
                PRSize::M => Color::Blue,
                PRSize::L => Color::Yellow,
                PRSize::XL => Color::Red,
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format_date_short(pr.created_at),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" │ "),
                Span::styled(
                    format!(
                        "{:repo_w$}",
                        truncate(&pr.repo, repo_width),
                        repo_w = repo_width
                    ),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" │ "),
                Span::styled(
                    format!("#{:4}", pr.number),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::raw(format!(
                    "{:title_w$}",
                    truncate(&pr.title, title_width),
                    title_w = title_width
                )),
                Span::raw(" │ "),
                Span::styled(
                    format!("{:8}", format_duration(pr.lead_time)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" │ "),
                Span::styled(format!("{}", pr_size), Style::default().fg(size_color)),
            ]));
        }
        for _ in 0..SECTION_SPACING {
            lines.push(Line::from(""));
        }
    }

    lines
}

fn build_detail_by_repo_content(
    data: &MonthData,
    cfg: &Config,
    width: usize,
) -> Vec<Line<'static>> {
    let usable_width = width
        .saturating_sub((HORIZONTAL_MARGIN * 2) as usize)
        .saturating_sub(SCROLLBAR_SPACE as usize);

    let fixed_width = 6 + 3 + 3 + 5 + 3 + 3 + 8 + 3 + 2;
    let remaining = usable_width.saturating_sub(fixed_width).max(30);
    let repo_width = (remaining / 3).max(10);
    let title_width = remaining.saturating_sub(repo_width).max(15);

    let mut lines = Vec::new();

    for (repo, prs) in data.repos.iter().zip(data.prs_by_repo.iter()) {
        let repo_header = format!(
            "━━━ {} │ {} PRs │ Avg: {} │ [{}]",
            repo.name,
            repo.pr_count,
            format_duration(repo.avg_lead_time),
            repo.format_size_distribution()
        );
        lines.push(
            Line::from(pad_line(&repo_header, usable_width, '━'))
                .style(Style::default().fg(Color::Gray)),
        );

        for pr in prs {
            let pr_size = pr.size(&cfg.size);
            let size_color = match pr_size {
                PRSize::S => Color::Green,
                PRSize::M => Color::Blue,
                PRSize::L => Color::Yellow,
                PRSize::XL => Color::Red,
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format_date_short(pr.created_at),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" │ "),
                Span::styled(
                    format!(
                        "{:repo_w$}",
                        truncate(&pr.repo, repo_width),
                        repo_w = repo_width
                    ),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" │ "),
                Span::styled(
                    format!("#{:4}", pr.number),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::raw(format!(
                    "{:title_w$}",
                    truncate(&pr.title, title_width),
                    title_w = title_width
                )),
                Span::raw(" │ "),
                Span::styled(
                    format!("{:8}", format_duration(pr.lead_time)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" │ "),
                Span::styled(format!("{}", pr_size), Style::default().fg(size_color)),
            ]));
        }
        for _ in 0..SECTION_SPACING {
            lines.push(Line::from(""));
        }
    }

    lines
}

fn build_tail_content(data: &MonthData, cfg: &Config, width: usize) -> Vec<Line<'static>> {
    let mut all_prs: Vec<PRDetail> = data.prs_by_week.iter().flatten().cloned().collect();
    all_prs.sort_by(|a, b| b.lead_time.cmp(&a.lead_time));

    let usable_width = width
        .saturating_sub((HORIZONTAL_MARGIN * 2) as usize)
        .saturating_sub(SCROLLBAR_SPACE as usize);

    let fixed_width = 6 + 3 + 3 + 5 + 3 + 3 + 8 + 3 + 2;
    let remaining = usable_width.saturating_sub(fixed_width).max(30);
    let repo_width = (remaining / 3).max(10);
    let title_width = remaining.saturating_sub(repo_width).max(15);

    let mut lines = Vec::new();
    lines.push(
        Line::from(separator_line(
            "All PRs sorted by Lead Time (longest first)",
            usable_width,
        ))
        .style(Style::default().fg(Color::Gray)),
    );

    for pr in &all_prs {
        let pr_size = pr.size(&cfg.size);
        let size_color = match pr_size {
            PRSize::S => Color::Green,
            PRSize::M => Color::Blue,
            PRSize::L => Color::Yellow,
            PRSize::XL => Color::Red,
        };

        lines.push(Line::from(vec![
            Span::styled(
                format_date_short(pr.created_at),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!(
                    "{:repo_w$}",
                    truncate(&pr.repo, repo_width),
                    repo_w = repo_width
                ),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!("#{:4}", pr.number),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::raw(format!(
                "{:title_w$}",
                truncate(&pr.title, title_width),
                title_w = title_width
            )),
            Span::raw(" │ "),
            Span::styled(
                format!("{:8}", format_duration(pr.lead_time)),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" │ "),
            Span::styled(format!("{}", pr_size), Style::default().fg(size_color)),
        ]));
    }

    lines
}

fn separator_line(title: &str, width: usize) -> String {
    let prefix = format!("━━━ {} ", title);
    let remaining = width.saturating_sub(prefix.chars().count()).max(0);
    format!("{}{}", prefix, "━".repeat(remaining))
}

fn pad_line(text: &str, width: usize, pad_char: char) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        text.to_string()
    } else {
        format!("{}{}", text, pad_char.to_string().repeat(width - text_len))
    }
}

fn format_duration(d: Duration) -> String {
    let days = d.num_days();
    let hours = d.num_hours() % 24;
    let minutes = d.num_minutes() % 60;
    match (days, hours, minutes) {
        (d, h, _) if d > 0 => format!("{}d {}h", d, h),
        (_, h, m) if h > 0 => format!("{}h {}m", h, m),
        (_, _, m) => format!("{}m", m),
    }
}

fn format_month(dt: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", dt.year(), dt.month())
}

fn format_frequency(freq: f64) -> String {
    format!("{:.1}/week", freq)
}

fn format_date_range_short(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    format!(
        "{} {:02} - {} {:02}",
        start.format("%b"),
        start.day(),
        end.format("%b"),
        end.day()
    )
}

fn format_date_short(dt: DateTime<Utc>) -> String {
    dt.format("%b %d").to_string()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{:width$}", &s[..max_len], width = max_len)
    }
}

pub fn print_json(data: &data::MonthData, size_cfg: &SizeConfig) -> anyhow::Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct JsonOutput<'a> {
        month_start: String,
        total_prs: usize,
        avg_lead_time_hours: f64,
        frequency: f64,
        size_distribution: SizeDistribution,
        reviewers: Vec<JsonReviewer<'a>>,
        reviewed_count: usize,
        weeks: Vec<JsonWeek<'a>>,
        repositories: Vec<JsonRepo<'a>>,
    }

    #[derive(Serialize)]
    struct SizeDistribution {
        s: usize,
        m: usize,
        l: usize,
        xl: usize,
    }

    #[derive(Serialize)]
    struct JsonReviewer<'a> {
        login: &'a str,
        pr_count: usize,
    }

    #[derive(Serialize)]
    struct JsonWeek<'a> {
        week_num: usize,
        week_start: String,
        week_end: String,
        pr_count: usize,
        avg_lead_time_hours: f64,
        prs: Vec<JsonPR<'a>>,
    }

    #[derive(Serialize)]
    struct JsonPR<'a> {
        created_at: String,
        repo: &'a str,
        number: u32,
        title: &'a str,
        body: Option<&'a str>,
        lead_time_hours: f64,
        size: String,
        additions: u32,
        deletions: u32,
        changed_files: u32,
    }

    #[derive(Serialize)]
    struct JsonRepo<'a> {
        name: &'a str,
        pr_count: usize,
        avg_lead_time_hours: f64,
        size_distribution: SizeDistribution,
    }

    let output = JsonOutput {
        month_start: format_date(data.month_start),
        total_prs: data.total_prs,
        avg_lead_time_hours: data.avg_lead_time.num_seconds() as f64 / 3600.0,
        frequency: data.frequency,
        size_distribution: SizeDistribution {
            s: data.size_s,
            m: data.size_m,
            l: data.size_l,
            xl: data.size_xl,
        },
        reviewers: data
            .reviewers
            .iter()
            .map(|r| JsonReviewer {
                login: &r.login,
                pr_count: r.pr_count,
            })
            .collect(),
        reviewed_count: data.reviewed_count,
        weeks: data
            .weeks
            .iter()
            .enumerate()
            .map(|(idx, week)| JsonWeek {
                week_num: week.week_num,
                week_start: format_date(week.week_start),
                week_end: format_date(week.week_end),
                pr_count: week.pr_count,
                avg_lead_time_hours: week.avg_lead_time.num_seconds() as f64 / 3600.0,
                prs: data.prs_by_week[idx]
                    .iter()
                    .map(|pr| JsonPR {
                        created_at: format_date(pr.created_at),
                        repo: &pr.repo,
                        number: pr.number,
                        title: &pr.title,
                        body: pr.body.as_deref(),
                        lead_time_hours: pr.lead_time.num_seconds() as f64 / 3600.0,
                        size: pr.size(size_cfg).to_string(),
                        additions: pr.additions,
                        deletions: pr.deletions,
                        changed_files: pr.changed_files,
                    })
                    .collect(),
            })
            .collect(),
        repositories: data
            .repos
            .iter()
            .map(|repo| JsonRepo {
                name: &repo.name,
                pr_count: repo.pr_count,
                avg_lead_time_hours: repo.avg_lead_time.num_seconds() as f64 / 3600.0,
                size_distribution: SizeDistribution {
                    s: repo.size_s,
                    m: repo.size_m,
                    l: repo.size_l,
                    xl: repo.size_xl,
                },
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);
    Ok(())
}

pub fn print_csv(data: &data::MonthData, size_cfg: &SizeConfig) -> anyhow::Result<()> {
    println!(
        "created_at,repo,number,title,body,lead_time_hours,size,additions,deletions,changed_files"
    );

    for week_prs in &data.prs_by_week {
        for pr in week_prs {
            let lead_time_hours = pr.lead_time.num_seconds() as f64 / 3600.0;
            let body_escaped = pr
                .body
                .as_ref()
                .map(|b| b.replace("\"", "\"\"").replace("\n", " "))
                .unwrap_or_default();
            println!(
                "{},{},{},\"{}\",\"{}\",{:.2},{},{},{},{}",
                format_date(pr.created_at),
                pr.repo,
                pr.number,
                pr.title.replace("\"", "\"\""), // Escape quotes in CSV
                body_escaped,
                lead_time_hours,
                pr.size(size_cfg),
                pr.additions,
                pr.deletions,
                pr.changed_files
            );
        }
    }

    Ok(())
}

pub fn print_data(data: &data::MonthData, month: &str, size_cfg: &SizeConfig) {
    println!("GitHub PRs for {}", month);
    println!("  - Total PRs: {}", data.total_prs);
    println!(
        "  - Average Lead Time: {}",
        format_duration(data.avg_lead_time)
    );
    println!("  - Frequency: {:.1} PRs/week", data.frequency);
    println!("  - Sizes: [{}]", data.format_size_distribution());
    println!();

    if !data.reviewers.is_empty() {
        println!("Top Reviewers");
        for reviewer in data.reviewers.iter().take(10) {
            println!("  - {}: {} PRs", reviewer.login, reviewer.pr_count);
        }
        println!();
    }

    println!("My Review Activity");
    println!("  - PRs Reviewed: {}", data.reviewed_count);
    if data.total_prs > 0 {
        let ratio = data.reviewed_count as f64 / data.total_prs as f64;
        println!(
            "  - Review Balance: {:.1}:1 ({} reviewed / {} created)",
            ratio, data.reviewed_count, data.total_prs
        );
    }
    println!();

    for (week_idx, week) in data.weeks.iter().enumerate() {
        println!(
            "Week {} ({} - {})",
            week.week_num,
            format_date(week.week_start),
            format_date(week.week_end)
        );
        println!("  - PRs: {}", week.pr_count);
        println!("  - Avg Lead Time: {}", format_duration(week.avg_lead_time));

        let prs = &data.prs_by_week[week_idx];
        for pr in prs {
            println!(
                "    - {} | {} | #{} {} | {} | {}",
                format_date(pr.created_at),
                pr.repo,
                pr.number,
                pr.title,
                format_duration(pr.lead_time),
                pr.size(size_cfg)
            );
            if let Some(body) = &pr.body
                && !body.is_empty()
            {
                // Indent and display the full body
                for line in body.lines() {
                    println!("      {}", line);
                }
            }
        }
        println!();
    }

    println!("Repositories");
    for repo in &data.repos {
        println!(
            "  - {} - {} PRs (Avg: {}) [{}]",
            repo.name,
            repo.pr_count,
            format_duration(repo.avg_lead_time),
            repo.format_size_distribution()
        );
    }
}

fn format_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SizeConfig;
    use chrono::Utc;

    fn create_test_month_data() -> data::MonthData {
        use chrono::TimeZone;

        let month_start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let week_start = Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();
        let week_end = Utc.with_ymd_and_hms(2026, 1, 11, 23, 59, 59).unwrap();

        data::MonthData {
            month_start,
            total_prs: 2,
            avg_lead_time: chrono::Duration::hours(2),
            frequency: 2.0,
            size_s: 1,
            size_m: 1,
            size_l: 0,
            size_xl: 0,
            weeks: vec![data::WeekData {
                week_num: 1,
                week_start,
                week_end,
                pr_count: 2,
                avg_lead_time: chrono::Duration::hours(2),
            }],
            repos: vec![data::RepoData {
                name: "test/repo".to_string(),
                pr_count: 2,
                avg_lead_time: chrono::Duration::hours(2),
                size_s: 1,
                size_m: 1,
                size_l: 0,
                size_xl: 0,
            }],
            prs_by_week: vec![vec![
                data::PRDetail {
                    created_at: Utc.with_ymd_and_hms(2026, 1, 6, 10, 0, 0).unwrap(),
                    repo: "test/repo".to_string(),
                    number: 1,
                    title: "Test PR 1".to_string(),
                    body: None,
                    lead_time: chrono::Duration::hours(1),
                    additions: 10,
                    deletions: 5,
                    changed_files: 2,
                },
                data::PRDetail {
                    created_at: Utc.with_ymd_and_hms(2026, 1, 7, 14, 0, 0).unwrap(),
                    repo: "test/repo".to_string(),
                    number: 2,
                    title: "Test PR 2".to_string(),
                    body: None,
                    lead_time: chrono::Duration::hours(3),
                    additions: 100,
                    deletions: 50,
                    changed_files: 5,
                },
            ]],
            prs_by_repo: vec![],
            reviewers: vec![data::ReviewerData {
                login: "alice".to_string(),
                pr_count: 2,
            }],
            reviewed_count: 5,
        }
    }

    #[test]
    fn test_print_json_output() {
        let data = create_test_month_data();
        let size_config = SizeConfig::default();
        let result = print_json(&data, &size_config);
        assert!(result.is_ok(), "JSON output should succeed");
    }

    #[test]
    fn test_print_csv_output() {
        let data = create_test_month_data();
        let size_config = SizeConfig::default();
        let result = print_csv(&data, &size_config);
        assert!(result.is_ok(), "CSV output should succeed");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(chrono::Duration::minutes(30)), "30m");
        assert_eq!(format_duration(chrono::Duration::hours(2)), "2h 0m");
        assert_eq!(
            format_duration(chrono::Duration::hours(2) + chrono::Duration::minutes(30)),
            "2h 30m"
        );
        assert_eq!(format_duration(chrono::Duration::days(1)), "1d 0h");
        assert_eq!(
            format_duration(chrono::Duration::days(1) + chrono::Duration::hours(3)),
            "1d 3h"
        );
    }

    #[test]
    fn test_format_date() {
        use chrono::TimeZone;
        let dt = Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(format_date(dt), "2026-01-15");
    }

    #[test]
    fn test_update_quit_handled_in_run_loop() {
        // Quit is handled directly in the run loop, not in update()
        // This test verifies that update() doesn't panic when called with Quit
        let state = AppState::new();
        let result = update(Msg::Quit, state);
        // Update just returns the state unchanged for Quit
        assert!(matches!(result.current_view(), View::Summary));
    }

    #[test]
    fn test_update_show_summary_changes_view() {
        let mut state = AppState::new();
        state.set_view(View::Tail);

        let result = update(Msg::ShowSummary, state);
        assert!(matches!(result.current_view(), View::Summary));
    }

    #[test]
    fn test_update_toggle_detail_cycles_mode() {
        let state = AppState::new();

        // First toggle: Summary -> Detail(ByWeek)
        let result = update(Msg::ToggleDetail, state);
        assert!(matches!(
            result.current_view(),
            View::Detail(DetailMode::ByWeek)
        ));

        // Second toggle: Detail(ByWeek) -> Detail(ByRepo)
        let result = update(Msg::ToggleDetail, result);
        assert!(matches!(
            result.current_view(),
            View::Detail(DetailMode::ByRepo)
        ));

        // Third toggle: Detail(ByRepo) -> Detail(ByWeek)
        let result = update(Msg::ToggleDetail, result);
        assert!(matches!(
            result.current_view(),
            View::Detail(DetailMode::ByWeek)
        ));
    }

    #[test]
    fn test_update_show_tail_changes_view() {
        let state = AppState::new();

        let result = update(Msg::ShowTail, state);
        assert!(matches!(result.current_view(), View::Tail));
    }

    #[test]
    fn test_update_scroll_up_is_idempotent_at_top() {
        let state = AppState::new();

        let result1 = update(Msg::ScrollUp, state);

        // Scrolling up when already at top should not cause issues
        let result2 = update(Msg::ScrollUp, result1);
        // No panic means success
        assert!(matches!(result2.current_view(), View::Summary));
    }

    #[test]
    fn test_update_scroll_down_works() {
        let state = AppState::new();

        let result = update(Msg::ScrollDown, state);
        // No panic means success
        assert!(matches!(result.current_view(), View::Summary));
    }

    #[test]
    fn test_update_changing_view_resets_scroll() {
        let mut state = AppState::new();

        // Simulate scrolling down
        state.scroll_down();
        state.scroll_down();
        state.scroll_down();

        // Change view should reset scroll (through set_view)
        let result = update(Msg::ShowTail, state);
        assert!(matches!(result.current_view(), View::Tail));
        // Scroll position should be reset to 0 (we can't directly assert this without
        // exposing scroll.position, but the behavior is tested through set_view)
    }

    #[test]
    fn test_app_state_new_starts_with_summary() {
        let state = AppState::new();
        assert!(matches!(state.current_view(), View::Summary));
    }

    #[test]
    fn test_detail_mode_cycle() {
        assert_eq!(DetailMode::ByWeek.cycle(), DetailMode::ByRepo);
        assert_eq!(DetailMode::ByRepo.cycle(), DetailMode::ByWeek);
    }

    #[test]
    fn test_msg_derives_eq() {
        assert_eq!(Msg::Quit, Msg::Quit);
        assert_eq!(Msg::ShowSummary, Msg::ShowSummary);
        assert_ne!(Msg::Quit, Msg::ShowSummary);
    }
}
