mod data;
mod input;
mod view;

use anyhow::bail;
use clap::Parser;
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
#[command(about = "View your GitHub PRs summary in a TUI", long_about = None)]
struct Cli {
    #[arg(
        long,
        value_name = "YYYY-MM",
        help = "Month in format YYYY-MM, e.g. 2025-11",
        value_parser = parser_month
    )]
    month: String,
}

fn parser_month(s: &str) -> anyhow::Result<String> {
    let re = regex::Regex::new(r"^\d{4}-\d{2}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        bail!("Month must be in format YYYY-MM, e.g. 2025-11")
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let output = Command::new("gh")
        .arg("search")
        .arg("prs")
        .arg("--author=@me")
        .arg(format!("--created={}", cli.month))
        .arg("--limit")
        .arg("100")
        .arg("--json")
        .arg("createdAt,number,repository,title,updatedAt,url")
        .output()?;

    let json_str = String::from_utf8_lossy(&output.stdout);
    let prs: Vec<input::PullRequest> = serde_json::from_str(&json_str)?;
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

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
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
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
