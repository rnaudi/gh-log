use anyhow::{Context, Result};
use directories::ProjectDirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, panic};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default)]
    pub size: SizeConfig,
    #[serde(skip)]
    config_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilterConfig {
    #[serde(default)]
    pub exclude_repos: Vec<String>,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub ignore_repos: Vec<String>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SizeConfig {
    pub small: u32,
    pub medium: u32,
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
    pub fn default() -> Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "gh-log").context("Failed to determine config directory")?;
        let config_dir = project_dirs.config_dir().to_path_buf();

        Self::new(config_dir)
    }

    pub fn new(config_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;

        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            create_example(&config_path)?;
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
            let re = Regex::new(pattern).unwrap_or_else(|err| {
                panic!("Failed to compile regex pattern `{}`: {}", pattern, err)
            });
            re.is_match(text)
        })
    }
}

pub fn create_example(config_path: &PathBuf) -> Result<()> {
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
                exclude_repos: vec![],
                exclude_patterns: vec!["[invalid".to_string()],
                ignore_repos: vec![],
                ignore_patterns: vec![],
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
