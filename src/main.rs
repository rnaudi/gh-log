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

// Helper functions for CLI help text
fn view_help() -> &'static str {
    "Navigate PRs with an interactive terminal UI.

VIEWS:
  s - Summary (weekly & repo stats)
  d - Detail (cycle: by week ↔ by repo)
  t - Tail (all PRs sorted by lead time)

NAVIGATION:
  ↑↓ or j/k - Scroll up/down
  q or Esc  - Quit"
}

fn print_help() -> &'static str {
    "Output PR data to terminal or pipe to other tools.

FORMATS:
  (default) - Human-readable with PR descriptions
  --json    - JSON format (great for LLMs/scripts)
  --csv     - CSV format (import to spreadsheet)

EXAMPLES:
  gh-log print | pbcopy
  gh-log print --json | claude 'summarize'
  gh-log print --csv > prs-2025-01.csv"
}

fn config_help() -> &'static str {
    "Create/edit configuration file to customize filtering and PR size thresholds.

LOCATION:
  macOS:   ~/Library/Application Support/gh-log/config.toml
  Linux:   ~/.config/gh-log/config.toml
  Windows: %APPDATA%\\gh-log\\config.toml

CONFIGURATION OPTIONS:

[filter]
  exclude_repos    - Hide repos completely (not shown anywhere)
  exclude_patterns - Hide PRs matching regex (e.g., \"^test:\", \"^wip:\")
  ignore_repos     - Show but don't count in metrics
  ignore_patterns  - Show but don't count in metrics (e.g., \"^docs:\", \"^meeting:\")

[size]
  small  - Max lines for S size (default: 50)
  medium - Max lines for M size (default: 200)
  large  - Max lines for L size (default: 500)
  (XL = anything above large threshold)

PATTERN SYNTAX:
  Uses regex syntax. Common patterns:
    ^prefix:        - Matches PR titles starting with \"prefix:\"
    (?i)keyword     - Case-insensitive match
    (foo|bar)       - Match either foo or bar

EXAMPLE CONFIG:
  [filter]
  exclude_repos = [\"username/spam-repo\"]
  exclude_patterns = [\"^test:\", \"^tmp:\", \"^wip:\"]
  ignore_repos = [\"username/personal-notes\"]
  ignore_patterns = [\"^docs:\", \"^meeting:\", \"^review:\"]

  [size]
  small = 50
  medium = 200
  large = 500

NOTES:
  - If a repo is both excluded and ignored, it gets excluded
  - Patterns are applied to PR titles
  - Size = additions + deletions + file count heuristic"
}

fn doctor_help() -> &'static str {
    "Verify system setup and show diagnostic information.

CHECKS:
  - GitHub CLI (gh) installation and version
  - Authentication status

DISPLAYS:
  - Cache directory location and contents
  - Configuration file location and status

PATHS:
  Cache:
    macOS:   ~/Library/Caches/gh-log/
    Linux:   ~/.cache/gh-log/
    Windows: %LOCALAPPDATA%\\gh-log\\cache\\

  Config:
    macOS:   ~/Library/Application Support/gh-log/config.toml
    Linux:   ~/.config/gh-log/config.toml
    Windows: %APPDATA%\\gh-log\\config.toml"
}

#[derive(Parser)]
#[command(name = "gh-log")]
#[command(about = "GitHub PR analytics for your terminal")]
#[command(
    long_about = "Pull your GitHub PR data in seconds. View interactively or export to JSON/CSV.\n\nRequires: GitHub CLI (gh) installed and authenticated\nCaching: Speeds up repeated queries. Current month cached 6h, last month 24h, older months permanent.\n         Use --force flag to refresh cached data.\n\nExamples:\n  gh-log view                    # Interactive TUI for current month\n  gh-log print --json | claude   # Feed to LLM for performance review\n  gh-log doctor                  # Check setup"
)]
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
    /// Interactive TUI - press 's' summary, 'd' detail (cycles by week/repo), 't' tail, 'q' quit
    #[command(long_about = view_help())]
    #[command(override_usage = "gh-log view [OPTIONS]")]
    View {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11 (defaults to current month)",
            value_parser = parser_month
        )]
        month: Option<String>,
        #[arg(long, help = "Force refresh data from GitHub API, bypassing cache")]
        force: bool,
    },
    /// Print PRs as text/json/csv - pipe to LLMs, clipboard, or files
    #[command(long_about = print_help())]
    #[command(override_usage = "gh-log print [OPTIONS]")]
    Print {
        #[arg(
            long,
            value_name = "YYYY-MM",
            help = "Month in format YYYY-MM, e.g. 2025-11 (defaults to current month)",
            value_parser = parser_month
        )]
        month: Option<String>,
        #[arg(long, help = "Force refresh data from GitHub API, bypassing cache")]
        force: bool,
        #[arg(long, help = "Output data in JSON format")]
        json: bool,
        #[arg(long, help = "Output data in CSV format")]
        csv: bool,
    },
    /// Create/edit config - exclude/ignore repos, customize PR size thresholds
    #[command(long_about = config_help())]
    #[command(name = "config")]
    Config,
    /// Verify GitHub CLI (gh) is installed and show cache/config paths
    #[command(long_about = doctor_help())]
    #[command(name = "doctor")]
    Doctor,
}

fn parser_month(s: &str) -> anyhow::Result<String> {
    let re = regex::Regex::new(r"^\d{4}-\d{2}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        bail!("Month must be in format YYYY-MM, e.g. 2025-11")
    }
}

fn check_gh_installed() -> anyhow::Result<()> {
    match Command::new("gh").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => bail!(
            "GitHub CLI (gh) is installed but not working correctly.\nRun 'gh auth login' to authenticate."
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("GitHub CLI (gh) is not installed.\nInstall it from: https://cli.github.com/")
        }
        Err(e) => bail!("Failed to check for GitHub CLI: {}", e),
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
    body: Option<String>,
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
    check_gh_installed()?;

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
        body
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
                body: pr.body,
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
    check_gh_installed()?;

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
    let cache = cache::Cache::default()?;
    if use_cache && let Some(cached) = cache.load_from_cache(month)? {
        eprintln!("Loading from cache...");
        return Ok((cached.prs, cached.reviewed_count));
    }

    eprintln!("Fetching data from GitHub...");
    let prs = fetch_prs(month)?;
    let reviewed_count = fetch_reviewed_prs(month)?;

    let cached_data = cache::CachedData {
        month: month.to_string(),
        timestamp: chrono::Utc::now(),
        prs: prs.clone(),
        reviewed_count,
    };

    cache.save_to_cache(&cached_data)?;
    Ok((prs, reviewed_count))
}

fn run_view_mode(month: &str, force: bool) -> anyhow::Result<()> {
    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    let config = config::Config::default()?;
    let data = data::process_prs(prs, reviewed_count, &config);

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut current_view = view::View::Summary;
    let mut scroll_state = view::ScrollState::new();

    loop {
        match current_view {
            view::View::Summary => {
                view::render_summary(&mut terminal, &data, &mut scroll_state, &config)?
            }
            view::View::Detail(mode) => {
                view::render_detail(&mut terminal, &data, &mut scroll_state, &config, mode)?
            }
            view::View::Tail => {
                view::render_tail(&mut terminal, &data, &mut scroll_state, &config)?
            }
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
                    current_view = match current_view {
                        view::View::Detail(mode) => view::View::Detail(mode.cycle()),
                        _ => view::View::Detail(view::DetailMode::ByWeek),
                    };
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
    let config = config::Config::default()?;
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
    println!(
        "created_at,repo,number,title,body,lead_time_hours,size,additions,deletions,changed_files"
    );

    // Print each PR
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
        Commands::View { month, force } => {
            let month = month.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());
            run_view_mode(&month, force)
        }
        Commands::Print {
            month,
            force,
            json,
            csv,
        } => {
            let month = month.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());
            let format = if json {
                OutputFormat::Json
            } else if csv {
                OutputFormat::Csv
            } else {
                OutputFormat::Raw
            };
            run_print_mode(&month, force, format)
        }
        Commands::Doctor => {
            println!("gh-log diagnostics\n");
            match Command::new("gh").arg("--version").output() {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    println!("✓ GitHub CLI: {}", version);
                }
                Ok(_) => {
                    println!("✗ GitHub CLI: installed but not authenticated");
                    println!("  Run: gh auth login");
                }
                Err(_) => {
                    println!("✗ GitHub CLI: not installed");
                    println!("  Install from: https://cli.github.com/");
                }
            }

            match directories::ProjectDirs::from("", "", "gh-log") {
                Some(dirs) => {
                    let cache_dir = dirs.cache_dir();
                    let config_dir = dirs.config_dir();
                    let config_path = config_dir.join("config.toml");
                    println!("\nCache directory: {}", cache_dir.display());

                    if cache_dir.exists() {
                        if let Ok(entries) = std::fs::read_dir(cache_dir) {
                            let mut cache_files: Vec<_> = entries
                                .filter_map(|e| e.ok())
                                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                                .collect();

                            if cache_files.is_empty() {
                                println!("  (no cache files)");
                            } else {
                                cache_files.sort_by_key(|e| e.path());
                                for entry in cache_files {
                                    if let Ok(metadata) = entry.metadata()
                                        && let Ok(modified) = metadata.modified()
                                    {
                                        let datetime: chrono::DateTime<chrono::Utc> =
                                            modified.into();
                                        println!(
                                            "  {} ({})",
                                            entry.file_name().to_string_lossy(),
                                            datetime.format("%Y-%m-%d %H:%M:%S UTC")
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        println!("  (directory does not exist yet)");
                    }

                    println!("\nConfiguration file: {}", config_path.display());
                    if config_path.exists() {
                        println!("  (exists)");
                    } else {
                        println!("  (not created yet, using defaults)");
                    }
                }
                None => {
                    println!("\n✗ Could not determine cache/config directories");
                }
            }

            Ok(())
        }
        Commands::Config => {
            match directories::ProjectDirs::from("", "", "gh-log") {
                Some(dirs) => {
                    let config_path = dirs.config_dir().join("config.toml");
                    if config_path.exists() {
                        let config = config::Config::default()?;
                        println!("{}", toml::to_string_pretty(&config)?);
                        eprintln!("\n# {}", config_path.display());
                    } else {
                        config::create_example(&config_path)?;
                        println!("Created config: {}", config_path.display());
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
        let config = config::Config::default().unwrap();
        let result = print_json(&data, &config);
        assert!(result.is_ok(), "JSON output should succeed");
    }

    #[test]
    fn test_print_csv_output() {
        let data = create_test_month_data();
        let config = config::Config::default().unwrap();
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
