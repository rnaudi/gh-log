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

use crate::config::Config;
use crate::data::{MonthData, PRDetail, PRSize};

const HORIZONTAL_MARGIN: u16 = 2;
const SCROLLBAR_SPACE: u16 = 1;
const SECTION_SPACING: usize = 1;

#[derive(Clone, Copy)]
pub enum View {
    Summary,
    Detail(DetailMode),
    Tail,
}

#[derive(Clone, Copy)]
pub enum DetailMode {
    ByWeek,
    ByRepo,
}

impl DetailMode {
    pub fn cycle(self) -> Self {
        match self {
            DetailMode::ByWeek => DetailMode::ByRepo,
            DetailMode::ByRepo => DetailMode::ByWeek,
        }
    }
}

pub struct ScrollState {
    position: usize,
    content_height: usize,
    viewport_height: usize,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            position: 0,
            content_height: 0,
            viewport_height: 0,
        }
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }

    pub fn scroll_up(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
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

pub fn run(month_data: MonthData, cfg: Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut current_view = View::Summary;
    let mut scroll_state = ScrollState::new();

    loop {
        match current_view {
            View::Summary => render_summary(&mut terminal, &month_data, &mut scroll_state)?,
            View::Detail(mode) => {
                render_detail(&mut terminal, &month_data, &mut scroll_state, &cfg, mode)?
            }
            View::Tail => render_tail(&mut terminal, &month_data, &mut scroll_state, &cfg)?,
        }

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('s') => {
                    current_view = View::Summary;
                    scroll_state.reset();
                }
                KeyCode::Char('d') => {
                    current_view = match current_view {
                        View::Detail(mode) => View::Detail(mode.cycle()),
                        _ => View::Detail(DetailMode::ByWeek),
                    };
                    scroll_state.reset();
                }
                KeyCode::Char('t') => {
                    current_view = View::Tail;
                    scroll_state.reset();
                }
                KeyCode::Up | KeyCode::Char('k') => scroll_state.scroll_up(),
                KeyCode::Down | KeyCode::Char('j') => scroll_state.scroll_down(),
                _ => {}
            }
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
