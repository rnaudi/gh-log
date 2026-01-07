mod data;
mod input;
mod view;

use anyhow::bail;
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;
use std::process::Command;

#[derive(Parser)]
#[command(name = "gh-log")]
#[command(about = "View your GitHub PRs summary", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open interactive TUI to view PRs
    View {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11",
            value_parser = parser_month
        )]
        month: String,
    },
    /// Print PRs to terminal
    Print {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11",
            value_parser = parser_month
        )]
        month: String,
    },
}

fn parser_month(s: &str) -> anyhow::Result<String> {
    let re = regex::Regex::new(r"^\d{4}-\d{2}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        bail!("Month must be in format YYYY-MM, e.g. 2025-11")
    }
}

fn fetch_prs(month: &str) -> anyhow::Result<Vec<input::PullRequest>> {
    let output = Command::new("gh")
        .arg("search")
        .arg("prs")
        .arg("--author=@me")
        .arg(format!("--created={}", month))
        .arg("--limit")
        .arg("200")
        .arg("--json")
        .arg("createdAt,number,repository,title,updatedAt,url")
        .output()?;

    let json_str = String::from_utf8_lossy(&output.stdout);
    let prs: Vec<input::PullRequest> = serde_json::from_str(&json_str)?;
    Ok(prs)
}

fn run_view_mode(month: &str) -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let prs = fetch_prs(month)?;
    let data = data::process_prs(prs);

    let mut current_view = view::View::Summary;
    let mut scroll_state = view::ScrollState::default();

    loop {
        match current_view {
            view::View::Summary => {
                view::render_summary(&mut terminal, &data, current_view, &mut scroll_state)?
            }
            view::View::Detail => {
                view::render_detail(&mut terminal, &data, current_view, &mut scroll_state)?
            }
            view::View::Tail => {
                view::render_tail(&mut terminal, &data, current_view, &mut scroll_state)?
            }
        }

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('s') => {
                            current_view = view::View::Summary;
                            scroll_state.reset();
                        }
                        KeyCode::Char('d') => {
                            current_view = view::View::Detail;
                            scroll_state.reset();
                        }
                        KeyCode::Char('t') => {
                            current_view = view::View::Tail;
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

fn run_print_mode(month: &str) -> anyhow::Result<()> {
    let prs = fetch_prs(month)?;
    let data = data::process_prs(prs);

    println!("GitHub PRs for {}", month);
    println!("  - Total PRs: {}", data.total_prs);
    println!(
        "  - Average Lead Time: {}",
        format_duration(data.avg_lead_time)
    );
    println!("  - Frequency: {:.1} PRs/week", data.frequency);
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
                "    - {} | {} | #{} {} | {}",
                format_date(pr.created_at),
                pr.repo,
                pr.number,
                pr.title,
                format_duration(pr.lead_time)
            );
        }
        println!();
    }

    println!("Repositories");
    for repo in &data.repos {
        println!(
            "  - {} - {} PRs (Avg: {})",
            repo.name,
            repo.pr_count,
            format_duration(repo.avg_lead_time)
        );
    }

    Ok(())
}

fn format_duration(d: chrono::Duration) -> String {
    let days = d.num_days();
    let hours = d.num_hours() % 24;
    let minutes = d.num_minutes() % 60;
    match (days, hours, minutes) {
        (d, h, _) if d > 0 => format!("{}d {}h", d, h),
        (_, h, m) if h > 0 => format!("{}h {}m", h, m),
        (_, _, m) => format!("{}m", m),
    }
}

fn format_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::View { month } => run_view_mode(&month),
        Commands::Print { month } => run_print_mode(&month),
    }
}
