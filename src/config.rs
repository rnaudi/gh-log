//! gh-log configuration subsystem.
//!
//! Loads the on-disk TOML config, applies repo/title filters, and keeps size thresholds consistent
//! across the CLI.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, panic};

/// Config mirrors the on-disk TOML layout, exposes filters and size thresholds, and keeps the resolved path cached.
/// CLI commands load it once so they can print or rewrite the same file without reparsing directory hints from scratch.
///
/// # Examples
/// ```rust,no_run
/// # use gh_log::config::Config;
/// let cfg = Config::default().expect("load config once");
/// println!("excluded repos: {}", cfg.filter.exclude_repos.len());
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// Filters and pattern rules that control which PRs are hidden or skipped in metrics.
    #[serde(default)]
    pub filter: FilterConfig,
    /// Size thresholds that bucket PRs into S/M/L/XL bands for analytics output.
    #[serde(default)]
    pub size: SizeConfig,
    /// Cached on-disk location of the underlying TOML file for reuse by CLI commands.
    #[serde(skip)]
    config_path: PathBuf,
}

/// Filter lists come in exclude/ignore pairs so analytics can either hide noisy repos
/// entirely or keep them visible while skipping their contribution to aggregates.
/// Mirroring the pairs keeps the mental model clear for users editing the config.
///
/// # Examples
/// ```rust
/// # use gh_log::config::FilterConfig;
/// let filters = FilterConfig {
///     exclude_repos: vec!["example/noise".into()],
///     ignore_patterns: vec!["^docs:".into()],
///     ..Default::default()
/// };
/// assert!(filters.exclude_repos.contains(&"example/noise".to_string()));
/// ```
///
/// Checklist: keep `validate()` and `matches_patterns()` in sync when adding new filter fields.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilterConfig {
    /// Repository names removed entirely from analytics output.
    #[serde(default)]
    pub exclude_repos: Vec<String>,
    /// Regexes that drop PRs outright when their titles match.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Repository names that stay visible in detail views but are ignored in aggregates.
    #[serde(default)]
    pub ignore_repos: Vec<String>,
    /// Regexes that keep PRs visible yet exclude them from key performance metrics.
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Size thresholds (in total line changes) used to categorize pull requests so every output mode
/// labels PRs as S/M/L/XL the same way.
///
/// # Examples
/// ```rust
/// # use gh_log::config::SizeConfig;
/// let sizes = SizeConfig::new(50, 200, 500);
/// assert!(sizes.small < sizes.medium && sizes.medium < sizes.large);
/// ```
pub struct SizeConfig {
    /// Maximum line-change count that still qualifies as a small (S) pull request.
    pub small: u32,
    /// Maximum line-change count considered medium (M); larger values progress toward L/XL.
    pub medium: u32,
    /// Maximum line-change count considered large (L); values above this are treated as XL.
    pub large: u32,
}

impl FilterConfig {
    fn validate(&self) -> anyhow::Result<()> {
        for pattern in &self.exclude_patterns {
            Regex::new(pattern)
                .with_context(|| format!("Invalid exclude_pattern: '{}'", pattern))?;
        }

        for pattern in &self.ignore_patterns {
            Regex::new(pattern)
                .with_context(|| format!("Invalid ignore_pattern: '{}'", pattern))?;
        }

        Ok(())
    }
}

impl SizeConfig {
    /// Build size thresholds and assert they increase strictly.
    ///
    /// # Examples
    /// ```rust
    /// use gh_log::config::SizeConfig;
    /// let sizes = SizeConfig::new(50, 200, 500);
    /// assert_eq!(sizes.large, 500);
    /// ```
    pub fn new(small: u32, medium: u32, large: u32) -> Self {
        assert!(
            small < medium && medium < large,
            "Thresholds must be in ascending order"
        );

        Self {
            small,
            medium,
            large,
        }
    }
}

impl Default for SizeConfig {
    fn default() -> Self {
        Self {
            small: 50,
            medium: 200,
            large: 500,
        }
    }
}

impl Config {
    /// Load configuration from the standard OS directory, creating a template when missing.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// let cfg = Config::default().expect("load config");
    /// println!("{}", cfg.size.medium);
    /// ```
    pub fn default() -> Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "gh-log").context("Failed to determine config directory")?;
        let config_dir = project_dirs.config_dir().to_path_buf();

        Self::new(config_dir)
    }

    /// Load configuration from a specific directory, creating the file if it does not exist yet.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// # use std::path::PathBuf;
    /// let dir = PathBuf::from("/tmp/gh-log-config");
    /// let cfg = Config::new(dir).expect("load config");
    /// ```
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;

        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            example(&config_path)?;
            eprintln!("Created config: {}", config_path.display());
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let mut config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        config
            .filter
            .validate()
            .context("Invalid regex patterns in config")?;

        config.config_path = config_path;
        Ok(config)
    }

    /// Returns `true` when the repository is listed under `filter.exclude_repos`.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// let cfg = Config::default().expect("load config");
    /// let skip_repo = cfg.should_exclude_repo("example/noise");
    /// println!("skip repo: {}", skip_repo);
    /// ```
    pub fn should_exclude_repo(&self, repo_name: &str) -> bool {
        self.filter.exclude_repos.contains(&repo_name.to_string())
    }

    /// Returns `true` when the pull request title matches any `filter.exclude_patterns` entry.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// let cfg = Config::default().expect("load config");
    /// let skip_title = cfg.should_exclude_pr_title("docs: update handbook");
    /// println!("skip title: {}", skip_title);
    /// ```
    pub fn should_exclude_pr_title(&self, title: &str) -> bool {
        self.matches_patterns(title, &self.filter.exclude_patterns)
    }

    /// Returns `true` when the repository is listed under `filter.ignore_repos`.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// let cfg = Config::default().expect("load config");
    /// let ignore_repo = cfg.should_ignore_repo("example/playground");
    /// println!("ignore repo metrics: {}", ignore_repo);
    /// ```
    pub fn should_ignore_repo(&self, repo_name: &str) -> bool {
        self.filter.ignore_repos.contains(&repo_name.to_string())
    }

    /// Returns `true` when the pull request title matches any pattern in `filter.ignore_patterns`.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::config::Config;
    /// let cfg = Config::default().expect("load config");
    /// let ignore_title = cfg.should_ignore_pr_title("chore: update docs");
    /// println!("ignore title metrics: {}", ignore_title);
    /// ```
    pub fn should_ignore_pr_title(&self, title: &str) -> bool {
        self.matches_patterns(title, &self.filter.ignore_patterns)
    }

    fn matches_patterns(&self, text: &str, patterns: &[String]) -> bool {
        // validate() already proved each pattern compiles; recompiling here keeps the helper
        // side-effect free, and the tiny lists make the cost imperceptible.
        patterns.iter().any(|pattern| {
            let re = Regex::new(pattern).unwrap_or_else(|err| {
                panic!("Failed to compile regex pattern `{}`: {}", pattern, err)
            });
            re.is_match(text)
        })
    }
}

/// Write a sample configuration file to the given path, seeding default filters and size thresholds.
///
/// Overwrites any existing file contents so first-time users start with a documented template.
pub fn example(config_path: &PathBuf) -> Result<()> {
    let example_config = Config {
        filter: FilterConfig {
            exclude_repos: vec!["username/spam".to_string()],
            exclude_patterns: vec!["^test:".to_string(), "^tmp:".to_string()],
            ignore_repos: vec!["username/private".to_string(), "username/notes".to_string()],
            ignore_patterns: vec!["^docs:".to_string(), "^meeting:".to_string()],
        },
        size: SizeConfig::new(50, 200, 500),
        config_path: config_path.clone(),
    };

    let toml_string = toml::to_string_pretty(&example_config)
        .with_context(|| "Failed to serialize example config")?;

    let comment = "# gh-log configuration\n\
                  # \n\
                  # [filter]\n\
                  # exclude_* = not shown at all (filtered out completely)\n\
                  # ignore_*  = shown but not counted in metrics\n\
                  # \n\
                  # exclude_repos = [\"username/spam\"]  # Not shown\n\
                  # exclude_patterns = [\"^test:\", \"^tmp:\"]  # Not shown (regex)\n\
                  # ignore_repos = [\"username/private\"]  # Shown but not in metrics\n\
                  # ignore_patterns = [\"^docs:\", \"^meeting:\"]  # Shown but not in metrics (regex)\n\
                  # \n\
                  # [size]\n\
                  # small = 50    # S: <= 50 lines changed\n\
                  # medium = 200  # M: 51-200 lines\n\
                  # large = 500   # L: 201-500 lines, XL: > 500 lines\n\n";

    fs::write(config_path, format!("{}{}", comment, toml_string))
        .with_context(|| format!("Failed to write example config: {:?}", config_path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(filter: FilterConfig, size: SizeConfig, config_path: PathBuf) -> Config {
        Config {
            filter,
            size,
            config_path,
        }
    }

    #[test]
    fn test_config_new_with_toml_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();
        let config_path = config_dir.join("config.toml");

        let toml_content = r#"
[filter]
exclude_repos = ["user/spam"]
exclude_patterns = ["^test:", "^wip:"]
ignore_repos = ["user/notes"]
ignore_patterns = ["^docs:"]

[size]
small = 75
medium = 250
large = 600
"#;
        fs::write(&config_path, toml_content).unwrap();

        let config = Config::new(config_dir).unwrap();

        assert_eq!(config.filter.exclude_repos, vec!["user/spam"]);
        assert_eq!(config.filter.exclude_patterns, vec!["^test:", "^wip:"]);
        assert_eq!(config.filter.ignore_repos, vec!["user/notes"]);
        assert_eq!(config.filter.ignore_patterns, vec!["^docs:"]);
        assert_eq!(config.size.small, 75);
        assert_eq!(config.size.medium, 250);
        assert_eq!(config.size.large, 600);
    }

    #[test]
    #[should_panic(expected = "Failed to compile regex pattern `[invalid`")]
    fn test_invalid_regex_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(
            FilterConfig {
                exclude_patterns: vec!["[invalid".to_string()],
                ..Default::default()
            },
            SizeConfig::default(),
            temp_dir.path().join("config.toml"),
        );

        config.should_exclude_pr_title("test: something");
    }

    #[test]
    #[should_panic(expected = "Thresholds must be in ascending order")]
    fn test_same_thresholds_panics() {
        let _config = SizeConfig::new(100, 100, 100);
    }

    #[test]
    #[should_panic(expected = "Thresholds must be in ascending order")]
    fn test_descending_thresholds_panics() {
        let _config = SizeConfig::new(500, 200, 100);
    }

    #[test]
    fn test_validate_invalid_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(
            FilterConfig {
                exclude_patterns: vec!["[invalid".to_string()],
                ..Default::default()
            },
            SizeConfig::default(),
            temp_dir.path().join("config.toml"),
        );

        let result = config.filter.validate();
        assert!(result.is_err());
        insta::assert_snapshot!(result.unwrap_err().to_string());
    }

    #[test]
    fn test_validate_invalid_ignore_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(
            FilterConfig {
                ignore_patterns: vec!["^valid".to_string(), "***broken".to_string()],
                ..Default::default()
            },
            SizeConfig::default(),
            temp_dir.path().join("config.toml"),
        );

        let result = config.filter.validate();
        assert!(result.is_err());
        insta::assert_snapshot!(result.unwrap_err().to_string());
    }

    #[test]
    fn test_validate_all_valid_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(
            FilterConfig {
                exclude_patterns: vec!["^test:".to_string(), "^tmp:".to_string()],
                ignore_patterns: vec!["^docs:".to_string(), "^meeting:".to_string()],
                ..Default::default()
            },
            SizeConfig::default(),
            temp_dir.path().join("config.toml"),
        );

        let result = config.filter.validate();
        assert!(result.is_ok());
    }
}
