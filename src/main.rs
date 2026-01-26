//! gh-log: GitHub PR analytics for your terminal.
//!
//! # Overview
//! `gh-log` collects your pull requests through the GitHub CLI and turns them into the metrics you
//! need for performance reviews, planning, or status updates. This crate is the CLI entry point that
//! wires command-line parsing, configuration loading, caching, and output rendering together.
//!
//! # Why
//! Performance review season. Stop manually digging through GitHub repos and waiting on CSV exports.
//! `gh-log` surfaces the insights you usually compile by hand:
//! - Lead time, cadence, and PR size distribution (S/M/L/XL)
//! - Weekly breakdowns, repo-level trends, and top reviewers
//! - JSON/CSV exports that drop straight into notes or LLM prompts
//!
//! # Primary commands
//! - `view`: Launch an interactive dashboard with weekly summaries, repo stats, and sortable PR lists.
//! - `print`: Export data as text, JSON, or CSV so you can feed it to an LLM or drop it into a doc.
//! - `doctor`: Verify your GitHub CLI setup and reveal cache/config locations.
//! - `config`: Open or scaffold the configuration file used to tune filters and size thresholds.
//! - `completions`: Generate tab-completion scripts for popular shells.
//!
//! # Quick start
//! ```text
//! gh-log view
//! gh-log print --json | claude "Summarize into 3 key accomplishments"
//! gh-log doctor
//! ```
//!
//! Requires the GitHub CLI (`gh`) to be installed and authenticated. Results are cached to keep
//! repeated queries fast; pass `--force` to refresh. For installation instructions and screenshots,
//! see the project README.
//!
mod cache;
mod config;
mod data;
mod github;
mod view;

use anyhow::bail;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::io;
use std::process::Command;

fn view_help() -> &'static str {
    "Navigate PRs with an interactive terminal UI.

Discussion:
    Launch an interactive TUI to browse your PRs. The interface has three
    views that you can toggle between:

    - Summary (s): Weekly and repo statistics
    - Detail (d): Detailed list, cycle between grouped by week or by repo
    - Tail (t): All PRs sorted by lead time (longest first)

    Use arrow keys or j/k to scroll, q or Esc to quit.

    Data is cached after the first fetch. Use --force to bypass cache and
    fetch fresh data from GitHub.

Examples:
    # View current month
    gh-log view

    # View a specific month
    gh-log view --month 2025-12

    # Force fresh data (bypass cache)
    gh-log view --force"
}

fn print_help() -> &'static str {
    "Output PR data to terminal or pipe to other tools.

Discussion:
    Print PR data in various formats for different use cases:

    - Default: Human-readable text with PR descriptions
    - --json: Structured data for LLMs, scripts, or further processing
    - --csv: Spreadsheet-compatible format

    This is particularly useful for performance reviews - pipe the output
    to your clipboard, feed it to an LLM, or export to a spreadsheet.

    Data is cached after the first fetch. Use --force to bypass cache.

Examples:
    # Copy to clipboard for performance review
    gh-log print | pbcopy                    # macOS
    gh-log print | xclip -selection c        # Linux

    # Let AI write your review
    gh-log print --json | claude 'Summarize into 3 key accomplishments'

    # Export to spreadsheet
    gh-log print --csv > prs-2025-01.csv

    # Specific month with fresh data
    gh-log print --month 2024-12 --force --json"
}

fn config_help() -> &'static str {
    "Create or edit the configuration file.

Discussion:
    Opens the configuration file in your default editor (via $EDITOR or $VISUAL).
    If the file doesn't exist, a template will be created.

    Configuration allows you to:
    - Exclude repos/PRs completely (won't be shown)
    - Ignore repos/PRs (shown but not counted in metrics)
    - Customize PR size thresholds (S/M/L/XL)

    Patterns use regex syntax and are applied to PR titles.

    If a repo appears in both exclude and ignore lists, it gets excluded.

Config location:
    macOS:   ~/Library/Application Support/gh-log/config.toml
    Linux:   ~/.config/gh-log/config.toml
    Windows: %APPDATA%\\gh-log\\config.toml

Example configuration:
    [filter]
    exclude_repos = [\"username/spam-repo\"]
    exclude_patterns = [\"^test:\", \"^tmp:\"]
    ignore_repos = [\"username/personal-notes\"]
    ignore_patterns = [\"^docs:\", \"^meeting:\"]

    [size]
    small = 50
    medium = 200
    large = 500

Common regex patterns:
    ^prefix:     Match titles starting with \"prefix:\"
    (?i)keyword  Case-insensitive match
    (foo|bar)    Match either foo or bar"
}

fn completions_help() -> &'static str {
    "Generate tab-completion scripts for your shell.

The script is output on `stdout`, allowing you to redirect the output to
the file of your choosing. Where you place the file will depend on which
shell, and which operating system you are using. Your particular
configuration may also determine where these scripts need to be placed.

Here are some common setups for the supported shells under Unix and
similar operating systems (such as GNU/Linux).

BASH:

Completion files are commonly stored in `/etc/bash_completion.d/` for
system-wide commands, but can be stored in
`~/.local/share/bash-completion/completions` for user-specific commands.

Run the command:

    $ mkdir -p ~/.local/share/bash-completion/completions
    $ gh-log completions bash > ~/.local/share/bash-completion/completions/gh-log

This installs the completion script. You may have to log out and log
back in to your shell session for the changes to take effect.

BASH (macOS/Homebrew):

Homebrew stores bash completion files within the Homebrew directory.
With the `bash-completion` brew formula installed, run the command:

    $ mkdir -p $(brew --prefix)/etc/bash_completion.d
    $ gh-log completions bash > $(brew --prefix)/etc/bash_completion.d/gh-log

ZSH:

ZSH completions are commonly stored in any directory listed in your
`$fpath` variable. To use these completions, you must either add the
generated script to one of those directories, or add your own to this list.

Adding a custom directory is often the safest bet if you are unsure of
which directory to use. First create the directory; for this example
we'll create a hidden directory inside our `$HOME` directory:

    $ mkdir -p ~/.zsh/completions

Then add the following lines to your `.zshrc` just before `compinit`:

    fpath=(~/.zsh/completions $fpath)

Now you can install the completions script using the following command:

    $ gh-log completions zsh > ~/.zsh/completions/_gh-log

You must then restart your shell or run:

    $ exec zsh

for the new completions to take effect.

FISH:

Fish completion files are commonly stored in
`$HOME/.config/fish/completions`. Run the command:

    $ mkdir -p ~/.config/fish/completions
    $ gh-log completions fish > ~/.config/fish/completions/gh-log.fish

This installs the completion script. You may have to log out and log
back in to your shell session for the changes to take effect.

POWERSHELL:

The PowerShell completion scripts require PowerShell v5.0+ (which comes
with Windows 10, but can be downloaded separately for Windows 7 or 8.1).

First, check if a profile has already been set:

    PS C:\\> Test-Path $profile

If the above command returns `False` run the following:

    PS C:\\> New-Item -path $profile -type file -force

Now open the file provided by `$profile` (if you used the `New-Item`
command it will be
`${env:USERPROFILE}\\Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1`)

Next, we either save the completions file into our profile, or into a
separate file and source it inside our profile. To save the completions
into our profile simply use:

    PS C:\\> gh-log completions powershell >> $profile

CUSTOM LOCATIONS:

Alternatively, you could save these files to the place of your choosing,
such as a custom directory inside your $HOME. Doing so will require you
to add the proper directives, such as `source`ing inside your login
script. Consult your shell's documentation for how to add such directives."
}

fn doctor_help() -> &'static str {
    "Verify system setup and show diagnostic information.

Discussion:
    Runs a series of checks to verify that gh-log is properly configured
    and can communicate with GitHub.

    Checks performed:
    - GitHub CLI (gh) installation and version
    - GitHub authentication status

    Also displays the locations of:
    - Cache directory (where PR data is stored)
    - Configuration file (if it exists)

    Use this command to troubleshoot issues or find where your data is stored.

Common issues:
    'gh not found'
    → Install GitHub CLI: https://cli.github.com/

    'not authenticated'
    → Run: gh auth login

    Stale data showing
    → Use --force flag with view/print commands to refresh"
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
    /// Generate shell completion scripts for your shell
    #[command(long_about = completions_help())]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
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

fn get_data_with_cache(
    month: &str,
    use_cache: bool,
) -> anyhow::Result<(Vec<github::PullRequest>, usize)> {
    let cache = cache::Cache::default()?;
    // Reuse cached data when allowed to avoid redundant API calls.
    if use_cache && let Some(cached) = cache.load(month)? {
        eprintln!("Loading from cache...");
        return Ok((cached.prs, cached.reviewed_count));
    }

    // Fetch live data when the cache misses or a refresh is forced.
    eprintln!("Fetching data from GitHub...");
    let client = github::CommandClient::new()?;
    let prs = client.fetch_prs(month)?;
    let reviewed_count = client.fetch_reviewed_prs(month)?;

    // Persist the fresh snapshot so the next call can reuse it.
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
    // We reload config on every run so edits from `gh-log config` take effect immediately.
    let cfg = config::Config::default()?;
    let month_data = data::build_month_data(month, prs, reviewed_count, &cfg);

    view::run(month_data, cfg)
}

fn run_print_mode(month: &str, force: bool, format: OutputFormat) -> anyhow::Result<()> {
    let use_cache = !force;
    let (prs, reviewed_count) = get_data_with_cache(month, use_cache)?;
    // We reload config on every run so edits from `gh-log config` take effect immediately.
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
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "gh-log", &mut io::stdout());
            Ok(())
        }
    }
}
