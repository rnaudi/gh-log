mod cache;
mod config;
mod data;
mod github;
mod view;

use anyhow::bail;
use clap::{Parser, Subcommand};
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

fn get_data_with_cache(
    month: &str,
    use_cache: bool,
) -> anyhow::Result<(Vec<github::PullRequest>, usize)> {
    let cache = cache::Cache::default()?;
    if use_cache && let Some(cached) = cache.load(month)? {
        eprintln!("Loading from cache...");
        return Ok((cached.prs, cached.reviewed_count));
    }

    eprintln!("Fetching data from GitHub...");
    let client = github::CommandClient::new()?;
    let prs = client.fetch_prs(month)?;
    let reviewed_count = client.fetch_reviewed_prs(month)?;

    let cached_data = cache::CachedData {
        month: month.to_string(),
        timestamp: chrono::Utc::now(),
        prs: prs.clone(),
        reviewed_count,
    };

    cache.save(&cached_data)?;
    Ok((prs, reviewed_count))
}

fn run_view_mode(month: &str, force: bool) -> anyhow::Result<()> {
    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    let cfg = config::Config::default()?;
    let month_data = data::build_month_data(month, prs, reviewed_count, &cfg);

    view::run(month_data, cfg)
}

fn run_print_mode(month: &str, force: bool, format: OutputFormat) -> anyhow::Result<()> {
    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    let cfg = config::Config::default()?;
    let data = data::build_month_data(month, prs, reviewed_count, &cfg);

    match format {
        OutputFormat::Raw => view::print_data(&data, month, &cfg.size),
        OutputFormat::Json => view::print_json(&data, &cfg.size)?,
        OutputFormat::Csv => view::print_csv(&data, &cfg.size)?,
    }

    Ok(())
}

fn run_doctor() -> anyhow::Result<()> {
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
                                let datetime: chrono::DateTime<chrono::Utc> = modified.into();
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

fn run_config() -> anyhow::Result<()> {
    match directories::ProjectDirs::from("", "", "gh-log") {
        Some(dirs) => {
            let config_path = dirs.config_dir().join("config.toml");
            if config_path.exists() {
                let config = config::Config::default()?;
                println!("{}", toml::to_string_pretty(&config)?);
                eprintln!("\n# {}", config_path.display());
            } else {
                config::example(&config_path)?;
                println!("Created config: {}", config_path.display());
            }
        }
        None => {
            eprintln!("Error: Could not determine config directory");
        }
    }
    Ok(())
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
        Commands::Doctor => run_doctor(),
        Commands::Config => run_config(),
    }
}
