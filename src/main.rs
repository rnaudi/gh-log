mod cache;
mod config;
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
use serde::Deserialize;
use std::io::stdout;
use std::process::Command;

#[derive(Parser)]
#[command(name = "gh-log")]
#[command(about = "View your GitHub PRs summary", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Raw,
    Json,
    Csv,
}

#[derive(Subcommand)]
enum Commands {
    /// Open interactive TUI to view PRs
    #[command(override_usage = "gh-log view [OPTIONS] --month <YYYY-MM>")]
    View {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11",
            value_parser = parser_month
        )]
        month: String,
        #[arg(long, help = "Force refresh data from GitHub API, bypassing cache")]
        force: bool,
    },
    /// Print PRs to terminal
    #[command(override_usage = "gh-log print [OPTIONS] --month <YYYY-MM>")]
    Print {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11",
            value_parser = parser_month
        )]
        month: String,
        #[arg(long, help = "Force refresh data from GitHub API, bypassing cache")]
        force: bool,
        #[arg(long, help = "Output data in JSON format")]
        json: bool,
        #[arg(long, help = "Output data in CSV format")]
        csv: bool,
    },
    /// Show or create configuration file
    #[command(name = "config")]
    Config,
}

fn parser_month(s: &str) -> anyhow::Result<String> {
    let re = regex::Regex::new(r"^\d{4}-\d{2}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        bail!("Month must be in format YYYY-MM, e.g. 2025-11")
    }
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: GraphQLData,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    search: SearchResults,
}

#[derive(Debug, Deserialize)]
struct SearchResults {
    nodes: Vec<GraphQLPullRequest>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLPullRequest {
    number: u32,
    title: String,
    repository: input::Repository,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
    additions: u32,
    deletions: u32,
    #[serde(rename = "changedFiles")]
    changed_files: u32,
    reviews: input::Reviews,
}

fn fetch_prs(month: &str) -> anyhow::Result<Vec<input::PullRequest>> {
    let mut all_prs = Vec::new();
    let mut has_next_page = true;
    let mut cursor: Option<String> = None;

    while has_next_page {
        let after_clause = cursor
            .as_ref()
            .map(|c| format!(r#", after: "{}""#, c))
            .unwrap_or_default();

        let query = format!(
            r#"{{
  search(query: "is:pr author:@me created:{}", type: ISSUE, first: 100{}) {{
    pageInfo {{
      hasNextPage
      endCursor
    }}
    nodes {{
      ... on PullRequest {{
        number
        title
        repository {{
          nameWithOwner
        }}
        createdAt
        updatedAt
        additions
        deletions
        changedFiles
        reviews(first: 10) {{
          nodes {{
            author {{
              login
            }}
          }}
        }}
      }}
    }}
  }}
}}"#,
            month, after_clause
        );

        let output = Command::new("gh")
            .arg("api")
            .arg("graphql")
            .arg("-f")
            .arg(format!("query={}", query))
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("GraphQL query failed: {}", stderr);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let response: GraphQLResponse = serde_json::from_str(&json_str)?;

        for pr in response.data.search.nodes {
            all_prs.push(input::PullRequest {
                number: pr.number,
                title: pr.title,
                repository: pr.repository,
                created_at: pr.created_at,
                updated_at: pr.updated_at,
                additions: pr.additions,
                deletions: pr.deletions,
                changed_files: pr.changed_files,
                reviews: pr.reviews,
            });
        }

        has_next_page = response.data.search.page_info.has_next_page;
        cursor = response.data.search.page_info.end_cursor;
    }

    Ok(all_prs)
}

fn fetch_reviewed_prs(month: &str) -> anyhow::Result<usize> {
    let mut total_count = 0;
    let mut has_next_page = true;
    let mut cursor: Option<String> = None;

    while has_next_page {
        let after_clause = cursor
            .as_ref()
            .map(|c| format!(r#", after: "{}""#, c))
            .unwrap_or_default();

        let query = format!(
            r#"{{
  search(query: "is:pr reviewed-by:@me created:{}", type: ISSUE, first: 100{}) {{
    pageInfo {{
      hasNextPage
      endCursor
    }}
    issueCount
  }}
}}"#,
            month, after_clause
        );

        let output = Command::new("gh")
            .arg("api")
            .arg("graphql")
            .arg("-f")
            .arg(format!("query={}", query))
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("GraphQL query failed: {}", stderr);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let response: serde_json::Value = serde_json::from_str(&json_str)?;

        if let Some(issue_count) = response["data"]["search"]["issueCount"].as_u64() {
            total_count = issue_count as usize;
        }

        has_next_page = response["data"]["search"]["pageInfo"]["hasNextPage"]
            .as_bool()
            .unwrap_or(false);
        cursor = response["data"]["search"]["pageInfo"]["endCursor"]
            .as_str()
            .map(|s| s.to_string());
    }

    Ok(total_count)
}

fn get_data_with_cache(
    month: &str,
    use_cache: bool,
) -> anyhow::Result<(Vec<input::PullRequest>, usize)> {
    if use_cache && let Some(cached) = cache::load_from_cache(month)? {
        eprintln!("Loading from cache...");
        return Ok((cached.prs, cached.reviewed_count));
    }

    eprintln!("Fetching data from GitHub...");
    let prs = fetch_prs(month)?;
    let reviewed_count = fetch_reviewed_prs(month)?;

    // Save to cache
    let cached_data = cache::CachedData {
        month: month.to_string(),
        timestamp: chrono::Utc::now(),
        prs: prs.clone(),
        reviewed_count,
    };
    let _ = cache::save_to_cache(&cached_data);

    Ok((prs, reviewed_count))
}

fn run_view_mode(month: &str, force: bool) -> anyhow::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    let config = config::Config::load()?;
    let data = data::process_prs(prs, reviewed_count, &config);

    let mut current_view = view::View::Summary;
    let mut scroll_state = view::ScrollState::default();

    loop {
        match current_view {
            view::View::Summary => view::render_summary(
                &mut terminal,
                &data,
                current_view,
                &mut scroll_state,
                &config,
            )?,
            view::View::Detail => view::render_detail(
                &mut terminal,
                &data,
                current_view,
                &mut scroll_state,
                &config,
            )?,
            view::View::Tail => view::render_tail(
                &mut terminal,
                &data,
                current_view,
                &mut scroll_state,
                &config,
            )?,
        }

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
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

fn run_print_mode(month: &str, force: bool, format: OutputFormat) -> anyhow::Result<()> {
    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    let config = config::Config::load()?;
    let data = data::process_prs(prs, reviewed_count, &config);

    match format {
        OutputFormat::Raw => print_data(&data, month, &config),
        OutputFormat::Json => print_json(&data, &config)?,
        OutputFormat::Csv => print_csv(&data, &config)?,
    }

    Ok(())
}

fn print_json(data: &data::MonthData, config: &config::Config) -> anyhow::Result<()> {
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
                        lead_time_hours: pr.lead_time.num_seconds() as f64 / 3600.0,
                        size: pr.size(&config.size).to_string(),
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

fn print_csv(data: &data::MonthData, config: &config::Config) -> anyhow::Result<()> {
    // Print header
    println!("created_at,repo,number,title,lead_time_hours,size,additions,deletions,changed_files");

    // Print each PR
    for week_prs in &data.prs_by_week {
        for pr in week_prs {
            let lead_time_hours = pr.lead_time.num_seconds() as f64 / 3600.0;
            println!(
                "{},{},{},\"{}\",{:.2},{},{},{},{}",
                format_date(pr.created_at),
                pr.repo,
                pr.number,
                pr.title.replace("\"", "\"\""), // Escape quotes in CSV
                lead_time_hours,
                pr.size(&config.size),
                pr.additions,
                pr.deletions,
                pr.changed_files
            );
        }
    }

    Ok(())
}

fn print_data(data: &data::MonthData, month: &str, config: &config::Config) {
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
                pr.size(&config.size)
            );
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
        Commands::View { month, force } => run_view_mode(&month, force),
        Commands::Print {
            month,
            force,
            json,
            csv,
        } => {
            let format = if json {
                OutputFormat::Json
            } else if csv {
                OutputFormat::Csv
            } else {
                OutputFormat::Raw
            };
            run_print_mode(&month, force, format)
        }
        Commands::Config => {
            match directories::ProjectDirs::from("", "", "gh-log") {
                Some(dirs) => {
                    let config_path = dirs.config_dir().join("config.toml");
                    if config_path.exists() {
                        let config = config::Config::load()?;
                        println!("{}", toml::to_string_pretty(&config)?);
                        eprintln!("\n# {}", config_path.display());
                    } else {
                        let path = config::Config::create_example()?;
                        println!("Created config: {}", path.display());
                    }
                }
                None => {
                    eprintln!("Error: Could not determine config directory");
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    lead_time: chrono::Duration::hours(3),
                    additions: 100,
                    deletions: 50,
                    changed_files: 5,
                },
            ]],
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
        let config = config::Config::default();
        let result = print_json(&data, &config);
        assert!(result.is_ok(), "JSON output should succeed");
    }

    #[test]
    fn test_print_csv_output() {
        let data = create_test_month_data();
        let config = config::Config::default();
        let result = print_csv(&data, &config);
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
        let date = Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(format_date(date), "2026-01-15");
    }
}
