use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::github::PullRequest;

const MAX_SIZE: usize = 10_000;

#[derive(Debug)]
pub struct Cache {
    cache_dir: PathBuf,
    max_prs_in_cache: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedData {
    pub month: String,
    pub timestamp: DateTime<Utc>,
    pub prs: Vec<PullRequest>,
    pub reviewed_count: usize,
}

impl Cache {
    pub fn default() -> anyhow::Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "gh-log").context("Failed to determine cache directory")?;
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        Self::new(cache_dir, MAX_SIZE)
    }

    pub fn new(cache_dir: PathBuf, max_prs_in_cache: usize) -> anyhow::Result<Self> {
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        Ok(Cache {
            cache_dir,
            max_prs_in_cache,
        })
    }

    pub fn get_cache_file_path(&self, month: &str) -> Result<PathBuf> {
        Ok(self.cache_dir.join(format!("{}.json", month)))
    }

    pub fn load_from_cache(&self, month: &str) -> Result<Option<CachedData>> {
        let cache_file = self
            .get_cache_file_path(month)
            .with_context(|| format!("Failed to get cache file path for {}", month))?;
        if !cache_file.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&cache_file)
            .with_context(|| format!("Failed to read cache file for {}", month))?;
        let cached: CachedData = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse cache file for {}", month))?;

        if is_cache_fresh(month, cached.timestamp) {
            return Ok(Some(cached));
        }

        fs::remove_file(&cache_file)
            .with_context(|| format!("Failed to remove file for {}", month))?;

        Ok(None)
    }

    pub fn save_to_cache(&self, data: &CachedData) -> Result<()> {
        if data.prs.len() > self.max_prs_in_cache {
            bail!(
                "Too many PRs to cache: {}. Max {}",
                data.prs.len(),
                self.max_prs_in_cache
            );
        }

        let cache_file = self.get_cache_file_path(&data.month)?;
        let json = serde_json::to_string_pretty(data)
            .with_context(|| format!("Failed to serialize cache data for month {}", data.month))?;
        fs::write(&cache_file, json)
            .with_context(|| format!("Failed to write cache file: {:?}", cache_file))?;

        Ok(())
    }
}

fn is_cache_fresh(month: &str, cache_time: DateTime<Utc>) -> bool {
    let now = Utc::now();
    let age = now - cache_time;

    let current_month = now.format("%Y-%m").to_string();
    let last_month = (now - Duration::days(30)).format("%Y-%m").to_string();

    match month {
        m if m == current_month => age < Duration::hours(6),
        m if m == last_month => age < Duration::hours(24),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_pr() -> PullRequest {
        use crate::github::{Repository, Reviews};
        use chrono::TimeZone;
        let fixed_time = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        PullRequest {
            number: 1,
            title: "Test PR".to_string(),
            body: None,
            repository: Repository {
                name_with_owner: "test/repo".to_string(),
            },
            created_at: fixed_time,
            updated_at: fixed_time,
            additions: 10,
            deletions: 5,
            changed_files: 2,
            reviews: Reviews { nodes: vec![] },
        }
    }

    fn create_test_cached_data(month: &str, pr_count: usize) -> CachedData {
        use chrono::TimeZone;
        let fixed_time = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        CachedData {
            month: month.to_string(),
            timestamp: fixed_time,
            prs: (0..pr_count).map(|_| create_test_pr()).collect(),
            reviewed_count: 0,
        }
    }

    #[test]
    fn test_cache_freshness() {
        let now = Utc::now();
        let current_month = now.format("%Y-%m").to_string();

        let cache_time = now - Duration::hours(1);
        assert!(is_cache_fresh(&current_month, cache_time));

        let cache_time = now - Duration::hours(7);
        assert!(!is_cache_fresh(&current_month, cache_time));

        let old_month = "2020-01";
        let cache_time = now - Duration::days(365);
        assert!(is_cache_fresh(old_month, cache_time));
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::new(temp_dir.path().to_path_buf(), 3).unwrap();

        let data = create_test_cached_data("2025-01", 2);
        cache.save_to_cache(&data).unwrap();

        let loaded = cache.load_from_cache("2025-01").unwrap();
        assert!(loaded.is_some());

        let cache_file = cache.get_cache_file_path("2025-01").unwrap();
        let json = fs::read_to_string(cache_file).unwrap();
        insta::assert_snapshot!(json);
    }

    #[test]
    fn test_save_fails_with_too_many_prs() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::new(temp_dir.path().to_path_buf(), 10).unwrap();

        let data = create_test_cached_data("2025-01", 11);
        let result = cache.save_to_cache(&data);

        assert!(result.is_err());
        insta::assert_snapshot!(result.unwrap_err());
    }

    #[test]
    fn test_stale_cache_is_removed() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::new(temp_dir.path().to_path_buf(), 100).unwrap();

        let now = Utc::now();
        let current_month = now.format("%Y-%m").to_string();
        let stale_timestamp = now - Duration::hours(10);

        let stale_data = CachedData {
            month: current_month.clone(),
            timestamp: stale_timestamp,
            prs: vec![create_test_pr()],
            reviewed_count: 0,
        };

        cache.save_to_cache(&stale_data).unwrap();
        let cache_file = cache.get_cache_file_path(&current_month).unwrap();
        assert!(cache_file.exists());

        let result = cache.load_from_cache(&current_month).unwrap();
        assert!(result.is_none());
        assert!(!cache_file.exists());
    }

    #[test]
    fn test_corrupted_cache_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::new(temp_dir.path().to_path_buf(), 100).unwrap();

        let cache_file = cache.get_cache_file_path("2025-01").unwrap();
        fs::write(&cache_file, "{ invalid json }").unwrap();

        let result = cache.load_from_cache("2025-01");
        assert!(result.is_err());
        insta::assert_snapshot!(result.unwrap_err());
    }
}
