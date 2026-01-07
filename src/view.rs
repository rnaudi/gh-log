use chrono::{DateTime, Datelike, Duration, Utc};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table as RatatuiTable},
};
use std::io::Result;

use crate::data::{MonthData, PRDetail};

#[derive(Clone, Copy)]
pub enum View {
    Summary,
    Detail,
    Tail,
}

impl View {
    pub fn name(&self) -> &'static str {
        match self {
            View::Summary => "Summary",
            View::Detail => "Detail",
            View::Tail => "Tail",
        }
    }
}

pub struct ScrollState {
    offset: usize,
    content_height: usize,
    viewport_height: usize,
}

impl ScrollState {
    pub fn default() -> Self {
        Self {
            offset: 0,
            content_height: 0,
            viewport_height: 0,
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn set_content_height(&mut self, height: usize) {
        self.content_height = height;
        self.adjust_offset();
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        self.adjust_offset();
    }

    pub fn scroll_up(&mut self) {
        if self.offset > 0 {
            self.offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.offset < self.max_offset() {
            self.offset += 1;
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    fn max_offset(&self) -> usize {
        self.content_height.saturating_sub(self.viewport_height)
    }

    fn adjust_offset(&mut self) {
        self.offset = self.offset.min(self.max_offset());
    }
}

struct Table;

trait FrameExt {
    fn add_table<T>(
        &mut self,
        area: Rect,
        title: &str,
        headers: &[&str],
        items: &[T],
        row_fn: impl Fn(&T) -> Vec<String>,
        scroll_offset: usize,
    );
}

impl FrameExt for Frame<'_> {
    fn add_table<T>(
        &mut self,
        area: Rect,
        title: &str,
        headers: &[&str],
        items: &[T],
        row_fn: impl Fn(&T) -> Vec<String>,
        scroll_offset: usize,
    ) {
        let constraints = vec![Constraint::Min(0); headers.len()];
        let visible_items = if scroll_offset < items.len() {
            &items[scroll_offset..]
        } else {
            &[]
        };
        let table = Table::from_data(title, headers, &constraints, visible_items, row_fn);
        self.render_widget(table, area);
    }
}

impl Table {
    fn new(
        title: String,
        headers: Vec<String>,
        rows: Vec<Row<'static>>,
        constraints: Vec<Constraint>,
    ) -> RatatuiTable<'static> {
        let header_row = Row::new(headers).bold();
        RatatuiTable::new(rows, constraints)
            .header(header_row)
            .block(Block::default().borders(Borders::ALL).title(title))
            .column_spacing(1)
    }

    fn from_data<T>(
        title: &str,
        headers: &[&str],
        constraints: &[Constraint],
        items: &[T],
        row_fn: impl Fn(&T) -> Vec<String>,
    ) -> RatatuiTable<'static> {
        let rows: Vec<Row<'static>> = items.iter().map(|item| Row::new(row_fn(item))).collect();
        let constraints = if constraints.is_empty() {
            vec![Constraint::Min(0); headers.len()]
        } else {
            constraints.to_vec()
        };
        Self::new(
            title.to_string(),
            headers.iter().copied().map(String::from).collect(),
            rows,
            constraints,
        )
    }
}

pub fn render_summary(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    view: View,
    scroll_state: &mut ScrollState,
) -> Result<()> {
    terminal.draw(|frame| {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(frame.size());

        render_view_header(frame, chunks[0], view);
        render_header(frame, chunks[1], data);
        render_tables(frame, chunks[3], data, scroll_state);
    })?;

    Ok(())
}

pub fn render_detail(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    view: View,
    scroll_state: &mut ScrollState,
) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.size();
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

        render_view_header(frame, chunks[0], view);
        frame.render_widget(
            Paragraph::new(format_month(data.month_start)).bold(),
            chunks[1],
        );

        let detail_height = chunks[2].height as usize;
        scroll_state.set_viewport_height(detail_height);

        // Calculate total content height (all weeks + spacing)
        let total_weeks = data.weeks.len();
        let estimated_week_height = 4;
        let spacing = total_weeks.saturating_sub(1);
        let total_content_height = (total_weeks * estimated_week_height) + spacing;
        scroll_state.set_content_height(total_content_height);

        if detail_height > 0 {
            // Calculate which weeks to show based on scroll offset
            let scroll_offset = scroll_state.offset();
            let start_week = scroll_offset / estimated_week_height;
            let visible_weeks = (detail_height / estimated_week_height).max(1);
            let end_week = (start_week + visible_weeks).min(total_weeks);

            if start_week < total_weeks {
                let weeks_to_show = &data.weeks[start_week..end_week];
                let prs_to_show: Vec<&Vec<PRDetail>> =
                    data.prs_by_week[start_week..end_week].iter().collect();

                let detail_chunks = Layout::vertical(
                    weeks_to_show
                        .iter()
                        .enumerate()
                        .flat_map(|(i, _)| {
                            vec![
                                Constraint::Min(4),
                                if i < weeks_to_show.len() - 1 {
                                    Constraint::Length(1)
                                } else {
                                    Constraint::Length(0)
                                },
                            ]
                        })
                        .collect::<Vec<_>>(),
                )
                .split(chunks[2]);

                for (local_idx, week) in weeks_to_show.iter().enumerate() {
                    let prs = prs_to_show[local_idx];
                    let chunk_idx = local_idx * 2;

                    let week_title = format!(
                        "Week {} ({}) - PRs: {} | Avg Lead Time: {}",
                        week.week_num,
                        format_date_range(week.week_start, week.week_end),
                        week.pr_count,
                        format_duration(week.avg_lead_time)
                    );
                    frame.add_table(
                        detail_chunks[chunk_idx],
                        &week_title,
                        &["Date", "Repository", "PR", "Lead Time"],
                        prs,
                        |pr| {
                            vec![
                                format_date(pr.created_at),
                                pr.repo.clone(),
                                format!("{} {}", pr.number, pr.title),
                                format_duration(pr.lead_time),
                            ]
                        },
                        0,
                    );
                }
            }

            // Render scrollbar
            let mut scrollbar_state =
                ScrollbarState::new(total_content_height).position(scroll_offset);
            let scrollbar = Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, chunks[2], &mut scrollbar_state);
        }
    })?;

    Ok(())
}

pub fn render_tail(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    view: View,
    scroll_state: &mut ScrollState,
) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.size();

        let mut all_prs: Vec<&PRDetail> = data.prs_by_week.iter().flatten().collect();
        all_prs.sort_by(|a, b| b.lead_time.cmp(&a.lead_time));

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

        render_view_header(frame, chunks[0], view);
        frame.render_widget(
            Paragraph::new(format_month(data.month_start)).bold(),
            chunks[1],
        );

        let viewport_height = chunks[2].height as usize;
        let content_height = all_prs.len() + 2; // +2 for header and borders
        scroll_state.set_viewport_height(viewport_height);
        scroll_state.set_content_height(content_height);

        let prs_title = format!(
            "All PRs sorted by Lead Time (longest first) - Total: {}",
            data.total_prs
        );
        frame.add_table(
            chunks[2],
            &prs_title,
            &["Date", "Repository", "PR", "Lead Time"],
            &all_prs,
            |pr| {
                vec![
                    format_date(pr.created_at),
                    pr.repo.clone(),
                    format!("{} {}", pr.number, pr.title),
                    format_duration(pr.lead_time),
                ]
            },
            scroll_state.offset(),
        );

        // Render scrollbar
        let mut scrollbar_state =
            ScrollbarState::new(content_height).position(scroll_state.offset());
        let scrollbar =
            Scrollbar::default().orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(scrollbar, chunks[2], &mut scrollbar_state);
    })?;

    Ok(())
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

fn format_date_range(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    format!(
        "{}/{} - {}/{}",
        start.month(),
        start.day(),
        end.month(),
        end.day()
    )
}

fn format_date(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

fn render_view_header(frame: &mut Frame, area: Rect, view: View) {
    let help_text = format!(
        "View: {} | [s] Summary | [d] Detail | [t] Tail | [q] Quit | [↑↓/jk] Scroll",
        view.name()
    );
    frame.render_widget(Paragraph::new(help_text).bold(), area);
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, data: &MonthData) {
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    frame.render_widget(
        Paragraph::new(format_month(data.month_start)).bold(),
        chunks[0],
    );

    let stats = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
    ])
    .split(chunks[1]);

    frame.render_widget(Paragraph::new(format!("PRs: {}", data.total_prs)), stats[0]);
    frame.render_widget(
        Paragraph::new(format!(
            "Avg Lead Time: {}",
            format_duration(data.avg_lead_time)
        )),
        stats[1],
    );
    frame.render_widget(
        Paragraph::new(format!("Frequency: {}", format_frequency(data.frequency))),
        stats[2],
    );
}

fn render_tables(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    data: &MonthData,
    scroll_state: &mut ScrollState,
) {
    let layout = if area.width > 120 {
        Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
    } else {
        Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
    };
    let chunks = layout.split(area);

    let viewport_height = chunks[0].height as usize;
    let max_table_height = data.weeks.len().max(data.repos.len()) + 2;
    scroll_state.set_viewport_height(viewport_height);
    scroll_state.set_content_height(max_table_height);

    frame.add_table(
        chunks[0],
        "Weeks",
        &["Week", "PRs", "Avg Lead Time"],
        &data.weeks,
        |w| {
            vec![
                format!(
                    "{} ({})",
                    w.week_num,
                    format_date_range(w.week_start, w.week_end)
                ),
                w.pr_count.to_string(),
                format_duration(w.avg_lead_time),
            ]
        },
        scroll_state.offset(),
    );

    frame.add_table(
        chunks[2],
        "Repositories",
        &["Repository", "PRs", "Avg Lead Time"],
        &data.repos,
        |r| {
            vec![
                r.name.clone(),
                r.pr_count.to_string(),
                format_duration(r.avg_lead_time),
            ]
        },
        scroll_state.offset(),
    );
}
