use chrono::{DateTime, Datelike, Duration, Utc};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Row, Scrollbar, ScrollbarState, Table},
};
use std::io::Result;

use crate::data::{MonthData, PRDetail, RepoData, WeekData};

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

// Simple domain structures - no ratatui dependencies
struct TableData {
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

// Build domain data structures
fn build_weeks_data(weeks: &[WeekData]) -> TableData {
    let headers = vec![
        "Week".to_string(),
        "PRs".to_string(),
        "Avg Lead Time".to_string(),
    ];
    let rows = weeks
        .iter()
        .map(|w| {
            vec![
                format!(
                    "{} ({})",
                    w.week_num,
                    format_date_range(w.week_start, w.week_end)
                ),
                w.pr_count.to_string(),
                format_duration(w.avg_lead_time),
            ]
        })
        .collect();

    TableData {
        title: "Weeks".to_string(),
        headers,
        rows,
    }
}

fn build_repos_data(repos: &[RepoData]) -> TableData {
    let headers = vec![
        "Repository".to_string(),
        "PRs".to_string(),
        "Avg Lead Time".to_string(),
    ];
    let rows = repos
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                r.pr_count.to_string(),
                format_duration(r.avg_lead_time),
            ]
        })
        .collect();

    TableData {
        title: "Repositories".to_string(),
        headers,
        rows,
    }
}

fn build_pr_details_data(title: String, prs: &[PRDetail]) -> TableData {
    let headers = vec![
        "Date".to_string(),
        "Repository".to_string(),
        "PR".to_string(),
        "Lead Time".to_string(),
    ];
    let rows = prs
        .iter()
        .map(|pr| {
            vec![
                format_date(pr.created_at),
                pr.repo.clone(),
                format!("{} {}", pr.number, pr.title),
                format_duration(pr.lead_time),
            ]
        })
        .collect();

    TableData {
        title,
        headers,
        rows,
    }
}

// Convert domain data to ratatui widget
fn table_data_to_widget(data: &TableData, scroll_offset: usize) -> Table<'static> {
    let visible_rows = if scroll_offset < data.rows.len() {
        &data.rows[scroll_offset..]
    } else {
        &[]
    };

    let rows: Vec<Row> = visible_rows
        .iter()
        .map(|row| Row::new(row.clone()))
        .collect();

    let header = Row::new(data.headers.clone()).bold();
    let constraints = vec![Constraint::Min(0); data.headers.len()];

    Table::new(rows, constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(data.title.clone()),
        )
        .column_spacing(1)
}

pub fn render_summary(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    data: &MonthData,
    view: View,
    scroll_state: &mut ScrollState,
) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.size();

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

        // View header
        let help_text = format!(
            "View: {} | [s] Summary | [d] Detail | [t] Tail | [q] Quit | [↑↓/jk] Scroll",
            view.name()
        );
        frame.render_widget(Paragraph::new(help_text).bold(), chunks[0]);

        // Month stats header
        let header_chunks =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(chunks[1]);

        frame.render_widget(
            Paragraph::new(format_month(data.month_start)).bold(),
            header_chunks[0],
        );

        let stats = Layout::horizontal([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(header_chunks[1]);

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

        // Tables
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
        let table_chunks = layout.split(chunks[3]);

        let viewport_height = table_chunks[0].height as usize;
        let max_table_height = data.weeks.len().max(data.repos.len()) + 2;
        scroll_state.set_viewport_height(viewport_height);
        scroll_state.set_content_height(max_table_height);

        let scroll_offset = scroll_state.offset();

        let weeks_data = build_weeks_data(&data.weeks);
        let weeks_table = table_data_to_widget(&weeks_data, scroll_offset);
        frame.render_widget(weeks_table, table_chunks[0]);

        let repos_data = build_repos_data(&data.repos);
        let repos_table = table_data_to_widget(&repos_data, scroll_offset);
        frame.render_widget(repos_table, table_chunks[2]);
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

        // View header
        let help_text = format!(
            "View: {} | [s] Summary | [d] Detail | [t] Tail | [q] Quit | [↑↓/jk] Scroll",
            view.name()
        );
        frame.render_widget(Paragraph::new(help_text).bold(), chunks[0]);

        frame.render_widget(
            Paragraph::new(format_month(data.month_start)).bold(),
            chunks[1],
        );

        let detail_height = chunks[2].height as usize;
        scroll_state.set_viewport_height(detail_height);

        let total_weeks = data.weeks.len();
        // Each week needs: title (1) + header (1) + rows (N) + borders (2) + spacing (1) = N + 5
        let avg_rows_per_week = if total_weeks > 0 {
            data.total_prs / total_weeks
        } else {
            0
        };
        let estimated_week_height = avg_rows_per_week + 5;
        let total_content_height = total_weeks * estimated_week_height;
        scroll_state.set_content_height(total_content_height);

        if detail_height > 0 {
            let scroll_offset = scroll_state.offset();
            let start_week = scroll_offset / estimated_week_height;
            let visible_weeks = (detail_height / estimated_week_height).max(1) + 1;
            let end_week = (start_week + visible_weeks).min(total_weeks);

            if start_week < total_weeks {
                let weeks_to_show = &data.weeks[start_week..end_week];
                let prs_to_show: Vec<&Vec<PRDetail>> =
                    data.prs_by_week[start_week..end_week].iter().collect();

                // Calculate constraints dynamically based on actual content
                let mut constraints = Vec::new();
                for (i, _week) in weeks_to_show.iter().enumerate() {
                    let prs = prs_to_show[i];
                    let table_height = prs.len() + 3; // rows + header + borders
                    constraints.push(Constraint::Length(table_height as u16));
                    if i < weeks_to_show.len() - 1 {
                        constraints.push(Constraint::Length(1)); // spacing
                    }
                }

                let detail_chunks = Layout::vertical(constraints).split(chunks[2]);

                for (local_idx, (week, prs)) in
                    weeks_to_show.iter().zip(prs_to_show.iter()).enumerate()
                {
                    let chunk_idx = local_idx * 2;

                    let week_title = format!(
                        "Week {} ({}) - PRs: {} | Avg Lead Time: {}",
                        week.week_num,
                        format_date_range(week.week_start, week.week_end),
                        week.pr_count,
                        format_duration(week.avg_lead_time)
                    );

                    let table_data = build_pr_details_data(week_title, prs);
                    let table = table_data_to_widget(&table_data, 0);
                    frame.render_widget(table, detail_chunks[chunk_idx]);
                }
            }

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

        let mut all_prs: Vec<PRDetail> = data.prs_by_week.iter().flatten().cloned().collect();
        all_prs.sort_by(|a, b| b.lead_time.cmp(&a.lead_time));

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

        // View header
        let help_text = format!(
            "View: {} | [s] Summary | [d] Detail | [t] Tail | [q] Quit | [↑↓/jk] Scroll",
            view.name()
        );
        frame.render_widget(Paragraph::new(help_text).bold(), chunks[0]);

        frame.render_widget(
            Paragraph::new(format_month(data.month_start)).bold(),
            chunks[1],
        );

        let viewport_height = chunks[2].height as usize;
        let content_height = all_prs.len() + 2;
        scroll_state.set_viewport_height(viewport_height);
        scroll_state.set_content_height(content_height);

        let scroll_offset = scroll_state.offset();
        let visible_prs = if scroll_offset < all_prs.len() {
            &all_prs[scroll_offset..]
        } else {
            &[]
        };

        let prs_title = format!(
            "All PRs sorted by Lead Time (longest first) - Total: {}",
            data.total_prs
        );

        let table_data = build_pr_details_data(prs_title, visible_prs);
        let table = table_data_to_widget(&table_data, 0);
        frame.render_widget(table, chunks[2]);

        let mut scrollbar_state = ScrollbarState::new(content_height).position(scroll_offset);
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
