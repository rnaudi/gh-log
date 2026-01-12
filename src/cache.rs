use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::input::PullRequest;

/// Cached data for a specific month.
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedData {
    pub month: String,
    pub timestamp: DateTime<Utc>,
    pub prs: Vec<PullRequest>,
    pub reviewed_count: usize,
}

/// Returns the cache file path for a specific month.
fn get_cache_file_path(month: &str) -> Result<PathBuf> {
    let project_dirs =
        ProjectDirs::from("", "", "gh-log").context("Failed to determine cache directory")?;
    let cache_dir = project_dirs.cache_dir().to_path_buf();
    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
    Ok(cache_dir.join(format!("{}.json", month)))
}

/// Determines if cached data is still fresh based on the month.
///
/// Cache TTL rules:
/// - Current month: 6 hours
/// - Last month: 24 hours
/// - Older months: never expires
fn is_cache_fresh(month: &str, cache_time: DateTime<Utc>) -> bool {
    let now = Utc::now();
    let age = now - cache_time;

    let current_month = now.format("%Y-%m").to_string();
    let last_month = (now - Duration::days(30)).format("%Y-%m").to_string();

    match month {
        m if m == current_month => age < Duration::hours(6),
        m if m == last_month => age < Duration::hours(24),
        _ => true, // Old months never expire
    }
}

/// Loads cached data from disk if it exists and is fresh.
pub fn load_from_cache(month: &str) -> Result<Option<CachedData>> {
    let cache_file = get_cache_file_path(month)?;
    if !cache_file.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&cache_file).context("Failed to read cache file")?;

    let cached: CachedData =
        serde_json::from_str(&contents).context("Failed to parse cache file")?;

    if is_cache_fresh(month, cached.timestamp) {
        Ok(Some(cached))
    } else {
        // Cache is stale, remove it
        let _ = fs::remove_file(&cache_file);
        Ok(None)
    }
}

/// Saves data to cache.
pub fn save_to_cache(data: &CachedData) -> Result<()> {
    let cache_file = get_cache_file_path(&data.month)?;

    let json = serde_json::to_string_pretty(data).context("Failed to serialize cache data")?;

    fs::write(&cache_file, json).context("Failed to write cache file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_freshness() {
        let now = Utc::now();
        let current_month = now.format("%Y-%m").to_string();

        // Fresh cache (1 hour old)
        let cache_time = now - Duration::hours(1);
        assert!(is_cache_fresh(&current_month, cache_time));

        // Stale cache (7 hours old)
        let cache_time = now - Duration::hours(7);
        assert!(!is_cache_fresh(&current_month, cache_time));

        // Old month (always fresh)
        let old_month = "2020-01";
        let cache_time = now - Duration::days(365);
        assert!(is_cache_fresh(old_month, cache_time));
    }
}
