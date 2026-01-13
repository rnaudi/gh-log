use anyhow::{Context, Result};
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default)]
    pub size: SizeConfig,
}

/// Filter configuration
///
/// - `exclude_*`: Filtered out completely (not shown)
/// - `ignore_*`: Shown but not counted in metrics
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilterConfig {
    /// Repositories excluded (not shown)
    #[serde(default)]
    pub exclude_repos: Vec<String>,
    /// PR title patterns excluded (not shown, regex)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Repositories ignored (shown but not in metrics)
    #[serde(default)]
    pub ignore_repos: Vec<String>,
    /// PR title patterns ignored (shown but not in metrics, regex)
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

/// PR size thresholds (lines changed)
///
/// File count overrides: >=25 files = XL, >=15 files = at least L
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SizeConfig {
    /// S: <= small lines
    pub small: u32,
    /// M: small+1 to medium lines
    pub medium: u32,
    /// L: medium+1 to large lines, XL: > large
    pub large: u32,
}

impl SizeConfig {
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
    pub fn load() -> Result<Self> {
        let config_path = get_config_file_path()?;
        if !config_path.exists() {
            return Ok(Config::default());
        }

        let contents = fs::read_to_string(&config_path).context("Failed to read config file")?;
        toml::from_str(&contents).context("Failed to parse config file")
    }

    pub fn should_exclude_repo(&self, repo_name: &str) -> bool {
        self.filter.exclude_repos.contains(&repo_name.to_string())
    }

    pub fn should_exclude_pr_title(&self, title: &str) -> bool {
        self.matches_patterns(title, &self.filter.exclude_patterns)
    }

    pub fn should_ignore_repo(&self, repo_name: &str) -> bool {
        self.filter.ignore_repos.contains(&repo_name.to_string())
    }

    pub fn should_ignore_pr_title(&self, title: &str) -> bool {
        self.matches_patterns(title, &self.filter.ignore_patterns)
    }

    fn matches_patterns(&self, text: &str, patterns: &[String]) -> bool {
        patterns.iter().any(|pattern| {
            Regex::new(pattern)
                .map(|re| re.is_match(text))
                .unwrap_or(false)
        })
    }

    /// Creates example config file
    pub fn create_example() -> Result<PathBuf> {
        let config_path = get_config_file_path()?;

        let example_config = Config {
            filter: FilterConfig {
                exclude_repos: vec!["username/spam".to_string()],
                exclude_patterns: vec!["^test:".to_string(), "^tmp:".to_string()],
                ignore_repos: vec!["username/private".to_string(), "username/notes".to_string()],
                ignore_patterns: vec!["^docs:".to_string(), "^meeting:".to_string()],
            },
            size: SizeConfig::new(50, 200, 500),
        };

        let toml_string = toml::to_string_pretty(&example_config)
            .context("Failed to serialize example config")?;

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

        fs::write(&config_path, format!("{}{}", comment, toml_string))
            .context("Failed to write example config")?;

        Ok(config_path)
    }
}

/// Returns config file path
fn get_config_file_path() -> Result<PathBuf> {
    let project_dirs =
        ProjectDirs::from("", "", "gh-log").context("Failed to determine config directory")?;
    let config_dir = project_dirs.config_dir().to_path_buf();
    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
    Ok(config_dir.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.filter.exclude_repos.is_empty());
        assert!(config.filter.exclude_patterns.is_empty());
        assert!(config.filter.ignore_repos.is_empty());
        assert!(config.filter.ignore_patterns.is_empty());
        assert_eq!(config.size.small, 50);
        assert_eq!(config.size.medium, 200);
        assert_eq!(config.size.large, 500);
    }

    #[test]
    fn test_should_exclude_repo() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec!["user/spam".to_string()],
                exclude_patterns: vec![],
                ignore_repos: vec!["user/private".to_string()],
                ignore_patterns: vec![],
            },
            size: SizeConfig::default(),
        };

        assert!(config.should_exclude_repo("user/spam"));
        assert!(!config.should_exclude_repo("user/private"));
        assert!(!config.should_exclude_repo("user/public"));
    }

    #[test]
    fn test_should_ignore_repo() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec![],
                exclude_patterns: vec![],
                ignore_repos: vec!["user/private".to_string(), "user/notes".to_string()],
                ignore_patterns: vec![],
            },
            size: SizeConfig::default(),
        };

        assert!(config.should_ignore_repo("user/private"));
        assert!(config.should_ignore_repo("user/notes"));
        assert!(!config.should_ignore_repo("user/public"));
        assert!(!config.should_ignore_repo("other/repo"));
    }

    #[test]
    fn test_should_exclude_pr_title() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec![],
                exclude_patterns: vec!["^test:".to_string(), "^tmp:".to_string()],
                ignore_repos: vec![],
                ignore_patterns: vec![],
            },
            size: SizeConfig::default(),
        };

        assert!(config.should_exclude_pr_title("test: temporary change"));
        assert!(config.should_exclude_pr_title("tmp: debug logging"));
        assert!(!config.should_exclude_pr_title("feat: add new feature"));
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec![],
                exclude_patterns: vec!["[invalid".to_string()],
                ignore_repos: vec![],
                ignore_patterns: vec![],
            },
            size: SizeConfig::default(),
        };

        // Invalid regex should not match anything
        assert!(!config.should_exclude_pr_title("test: something"));
    }

    #[test]
    fn test_should_ignore_pr_title() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec![],
                exclude_patterns: vec![],
                ignore_repos: vec![],
                ignore_patterns: vec![
                    "^docs:".to_string(),
                    "^meeting:".to_string(),
                    "^review:".to_string(),
                ],
            },
            size: SizeConfig::default(),
        };

        assert!(config.should_ignore_pr_title("docs: update README"));
        assert!(config.should_ignore_pr_title("meeting: weekly sync notes"));
        assert!(config.should_ignore_pr_title("review: architecture discussion"));
        assert!(!config.should_ignore_pr_title("feat: add new feature"));
    }

    #[test]
    fn test_size_defaults() {
        let config = SizeConfig::default();
        assert_eq!(config.small, 50);
        assert_eq!(config.medium, 200);
        assert_eq!(config.large, 500);
        assert!(config.small < config.medium);
        assert!(config.medium < config.large);
    }

    #[test]
    fn test_exclude_takes_priority_over_ignore() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec!["user/duplicate".to_string()],
                exclude_patterns: vec!["^duplicate:".to_string()],
                ignore_repos: vec!["user/duplicate".to_string()],
                ignore_patterns: vec!["^duplicate:".to_string()],
            },
            size: SizeConfig::default(),
        };

        // When a repo/pattern is in both exclude and ignore, exclude wins
        assert!(config.should_exclude_repo("user/duplicate"));
        assert!(config.should_exclude_pr_title("duplicate: test"));

        // Both functions can return true independently
        assert!(config.should_ignore_repo("user/duplicate"));
        assert!(config.should_ignore_pr_title("duplicate: test"));
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
            [filter]
            exclude_repos = ["user/spam"]
            exclude_patterns = ["^test:"]
            ignore_repos = ["user/private", "user/notes"]
            ignore_patterns = ["^docs:", "^meeting:"]

            [size]
            small = 100
            medium = 500
            large = 1000
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.filter.exclude_repos.len(), 1);
        assert!(config.should_exclude_repo("user/spam"));
        assert_eq!(config.filter.ignore_repos.len(), 2);
        assert!(config.should_ignore_repo("user/private"));
        assert_eq!(config.filter.exclude_patterns.len(), 1);
        assert_eq!(config.filter.ignore_patterns.len(), 2);
        assert_eq!(config.size.small, 100);
        assert_eq!(config.size.medium, 500);
        assert_eq!(config.size.large, 1000);
    }

    #[test]
    fn test_parse_empty_config() {
        let toml_str = "";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.filter.exclude_repos.is_empty());
        assert!(config.filter.exclude_patterns.is_empty());
        assert!(config.filter.ignore_repos.is_empty());
        assert!(config.filter.ignore_patterns.is_empty());
        assert_eq!(config.size.small, 50);
    }

    #[test]
    fn test_custom_size_thresholds_validation() {
        let valid_config = Config {
            filter: FilterConfig::default(),
            size: SizeConfig::new(100, 500, 1000),
        };
        assert!(valid_config.size.small < valid_config.size.medium);
        assert!(valid_config.size.medium < valid_config.size.large);
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
    fn test_serialize_config() {
        let config = Config {
            filter: FilterConfig {
                exclude_repos: vec!["user/spam".to_string()],
                exclude_patterns: vec!["^test:".to_string()],
                ignore_repos: vec!["user/private".to_string()],
                ignore_patterns: vec!["^docs:".to_string()],
            },
            size: SizeConfig::default(),
        };

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("exclude_repos"));
        assert!(toml_str.contains("user/spam"));
        assert!(toml_str.contains("ignore_repos"));
        assert!(toml_str.contains("user/private"));
        assert!(toml_str.contains("exclude_patterns"));
        assert!(toml_str.contains("^test:"));
        assert!(toml_str.contains("ignore_patterns"));
        assert!(toml_str.contains("^docs:"));
    }
}
