//! gh-log cache layer.
//!
//! Caches monthly PR snapshots in the OS cache directory so repeat runs avoid extra GitHub calls.
//! The current month refreshes after six hours, the previous month after twenty-four, and older
//! snapshots stick around while respecting `MAX_CACHE_SIZE`.
//!
//! ```rust,no_run
//! # use gh_log::cache::Cache;
//! let cache = Cache::default().expect("cache directory");
//! if let Some(snapshot) = cache.load("2025-01").expect("cache read") {
//!     println!("Cached {} PRs", snapshot.prs.len());
//! }
//! ```

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::github::PullRequest;

// Cache each month's PR snapshot as a standalone JSON file in the OS cache dir.
// Size and TTL caps keep recent data handy without letting old entries pile up.
const MAX_CACHE_SIZE: usize = 10_000;
const CURRENT_MONTH_CACHE_TTL_HOURS: i64 = 6;
const PREVIOUS_MONTH_CACHE_TTL_HOURS: i64 = 24;
const LAST_MONTH_LOOKBACK_DAYS: i64 = 30;

#[derive(Debug)]
/// File-backed cache for monthly PR snapshots stored in the user's cache directory.
/// Each month is serialized into a JSON file while respecting an upper bound on cached PRs.
///
/// # Examples
/// ```rust,no_run
/// # use gh_log::cache::Cache;
/// let cache = Cache::default().expect("cache directory to exist");
/// assert!(cache.load("2099-01").expect("cache read").is_none());
/// ```
pub struct Cache {
    /// Directory on disk where monthly cache files live.
    cache_dir: PathBuf,
    /// Maximum number of pull requests allowed in a cached snapshot.
    max_prs_in_cache: usize,
}

#[derive(Debug, Serialize, Deserialize)]
/// Snapshot of PR analytics cached for a specific month, including review aggregates.
pub struct CachedData {
    /// Month tag (YYYY-MM) that identifies the cache entry.
    pub month: String,
    /// Timestamp when the data was persisted, used to determine freshness.
    pub timestamp: DateTime<Utc>,
    /// Full list of pull requests captured for the month.
    pub prs: Vec<PullRequest>,
    /// Total number of PRs you reviewed during the month.
    pub reviewed_count: usize,
}

impl Cache {
    /// Build a cache rooted in the operating system's cache directory using project defaults.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::cache::Cache;
    /// let cache = Cache::default().expect("cache directory to exist");
    /// ```
    pub fn default() -> anyhow::Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "gh-log").context("Failed to determine cache directory")?;
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        Self::new(cache_dir, MAX_CACHE_SIZE)
    }

    /// Construct a cache at a custom location while capping the number of cached PRs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::cache::Cache;
    /// # use std::path::PathBuf;
    /// let cache_dir = PathBuf::from("/tmp/gh-log-cache");
    /// let cache = Cache::new(cache_dir, 10_000).expect("custom cache directory");
    /// ```
    pub fn new(cache_dir: PathBuf, max_prs_in_cache: usize) -> anyhow::Result<Self> {
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        Ok(Cache {
            cache_dir,
            max_prs_in_cache,
        })
    }

    /// Load cached data for a month when the on-disk snapshot exists and is still considered fresh.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::cache::Cache;
    /// let cache = Cache::default().expect("cache directory");
    /// if let Some(snapshot) = cache.load("2025-01").expect("cache read") {
    ///     println!("Found {} cached PRs", snapshot.prs.len());
    /// }
    /// ```
    pub fn load(&self, month: &str) -> Result<Option<CachedData>> {
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

        // Drop the stale cache so the next request forces a fresh write with the new schema/data.
        fs::remove_file(&cache_file)
            .with_context(|| format!("Failed to remove file for {}", month))?;

        Ok(None)
    }

    /// Persist a month's snapshot to disk after ensuring it fits within cache bounds.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use gh_log::cache::{Cache, CachedData};
    /// # use chrono::Utc;
    /// let cache = Cache::default().expect("cache directory");
    /// let data = CachedData {
    ///     month: "2025-01".into(),
    ///     timestamp: Utc::now(),
    ///     prs: Vec::new(),
    ///     reviewed_count: 0,
    /// };
    /// cache.save(&data).expect("persist snapshot");
    /// ```
    pub fn save(&self, data: &CachedData) -> Result<()> {
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

    fn get_cache_file_path(&self, month: &str) -> Result<PathBuf> {
        Ok(self.cache_dir.join(format!("{}.json", month)))
    }
}

fn is_cache_fresh(month: &str, cache_time: DateTime<Utc>) -> bool {
    let now = Utc::now();
    let age = now - cache_time;

    let current_month = now.format("%Y-%m").to_string();
    let last_month = (now - Duration::days(LAST_MONTH_LOOKBACK_DAYS))
        .format("%Y-%m")
        .to_string();

    match month {
        m if m == current_month => age < Duration::hours(CURRENT_MONTH_CACHE_TTL_HOURS),
        m if m == last_month => age < Duration::hours(PREVIOUS_MONTH_CACHE_TTL_HOURS),
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
        cache.save(&data).unwrap();

        let loaded = cache.load("2025-01").unwrap();
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
        let result = cache.save(&data);

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

        cache.save(&stale_data).unwrap();
        let cache_file = cache.get_cache_file_path(&current_month).unwrap();
        assert!(cache_file.exists());

        let result = cache.load(&current_month).unwrap();
        assert!(result.is_none());
        assert!(!cache_file.exists());
    }

    #[test]
    fn test_corrupted_cache_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::new(temp_dir.path().to_path_buf(), 100).unwrap();

        let cache_file = cache.get_cache_file_path("2025-01").unwrap();
        fs::write(&cache_file, "{ invalid json }").unwrap();

        let result = cache.load("2025-01");
        assert!(result.is_err());
        insta::assert_snapshot!(result.unwrap_err());
    }
}
