use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use std::collections::BTreeMap;
use std::fmt;

use crate::config::{Config, SizeConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PRSize {
    S,
    M,
    L,
    XL,
}

impl fmt::Display for PRSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PRSize::S => write!(f, "S"),
            PRSize::M => write!(f, "M"),
            PRSize::L => write!(f, "L"),
            PRSize::XL => write!(f, "XL"),
        }
    }
}

pub fn compute_pr_size(
    additions: u32,
    deletions: u32,
    changed_files: u32,
    size_config: &SizeConfig,
) -> PRSize {
    let total_lines = additions + deletions;
    if changed_files >= 25 {
        return PRSize::XL;
    }

    if changed_files >= 15 {
        if total_lines > size_config.large {
            return PRSize::XL;
        }
        return PRSize::L;
    }

    if total_lines <= size_config.small {
        PRSize::S
    } else if total_lines <= size_config.medium {
        PRSize::M
    } else if total_lines <= size_config.large {
        PRSize::L
    } else {
        PRSize::XL
    }
}

#[derive(Debug)]
pub struct WeekData {
    pub week_num: usize,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub pr_count: usize,
    pub avg_lead_time: Duration,
}

#[derive(Debug)]
pub struct RepoData {
    pub name: String,
    pub pr_count: usize,
    pub avg_lead_time: Duration,
    pub size_s: usize,
    pub size_m: usize,
    pub size_l: usize,
    pub size_xl: usize,
}

impl RepoData {
    pub fn format_size_distribution(&self) -> String {
        format!(
            "{}S {}M {}L {}XL",
            self.size_s, self.size_m, self.size_l, self.size_xl
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReviewerData {
    pub login: String,
    pub pr_count: usize,
}

#[derive(Debug, Clone)]
pub struct PRDetail {
    pub created_at: DateTime<Utc>,
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub lead_time: Duration,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

impl PRDetail {
    pub fn size(&self, size_config: &SizeConfig) -> PRSize {
        compute_pr_size(
            self.additions,
            self.deletions,
            self.changed_files,
            size_config,
        )
    }
}

#[derive(Debug)]
pub struct MonthData {
    pub month_start: DateTime<Utc>,
    pub total_prs: usize,
    pub avg_lead_time: Duration,
    pub frequency: f64,
    pub size_s: usize,
    pub size_m: usize,
    pub size_l: usize,
    pub size_xl: usize,
    pub weeks: Vec<WeekData>,
    pub repos: Vec<RepoData>,
    pub prs_by_week: Vec<Vec<PRDetail>>,
    pub reviewers: Vec<ReviewerData>,
    pub reviewed_count: usize,
}

impl Default for MonthData {
    fn default() -> Self {
        Self {
            month_start: Utc::now(),
            total_prs: 0,
            avg_lead_time: Duration::zero(),
            frequency: 0.0,
            size_s: 0,
            size_m: 0,
            size_l: 0,
            size_xl: 0,
            weeks: Vec::new(),
            repos: Vec::new(),
            prs_by_week: Vec::new(),
            reviewers: Vec::new(),
            reviewed_count: 0,
        }
    }
}

impl MonthData {
    pub fn format_size_distribution(&self) -> String {
        format!(
            "{}S {}M {}L {}XL",
            self.size_s, self.size_m, self.size_l, self.size_xl
        )
    }
}

fn avg_duration(durations: &[Duration]) -> Duration {
    if durations.is_empty() {
        return Duration::zero();
    }
    let total_seconds: i64 = durations.iter().map(|d| d.num_seconds()).sum();
    Duration::seconds(total_seconds / durations.len() as i64)
}

#[derive(Clone)]
struct PRData {
    number: u32,
    title: String,
    body: Option<String>,
    created_at: DateTime<Utc>,
    lead_time: Duration,
    repo_name: String,
    additions: u32,
    deletions: u32,
    changed_files: u32,
}

pub fn process_prs(
    prs: Vec<crate::input::PullRequest>,
    reviewed_count: usize,
    config: &Config,
) -> MonthData {
    if prs.is_empty() {
        return MonthData::default();
    }

    let mut pr_data: Vec<PRData> = Vec::with_capacity(prs.len());
    for pr in &prs {
        let lead_time = pr.updated_at - pr.created_at;
        assert!(
            lead_time >= Duration::zero(),
            "Lead time must be non-negative"
        );
        assert!(
            pr.updated_at >= pr.created_at,
            "Updated date must be >= created date"
        );
        pr_data.push(PRData {
            number: pr.number,
            title: pr.title.clone(),
            body: pr.body.clone(),
            created_at: pr.created_at,
            lead_time,
            repo_name: pr.repository.name_with_owner.clone(),
            additions: pr.additions,
            deletions: pr.deletions,
            changed_files: pr.changed_files,
        });
    }

    pr_data.sort_by_key(|pr| pr.created_at);

    pr_data.retain(|pr| {
        !config.should_exclude_repo(&pr.repo_name) && !config.should_exclude_pr_title(&pr.title)
    });

    let pr_data_for_metrics: Vec<&PRData> = pr_data
        .iter()
        .filter(|pr| {
            !config.should_ignore_repo(&pr.repo_name) && !config.should_ignore_pr_title(&pr.title)
        })
        .collect();

    // If all PRs are excluded, still show them but with zero metrics
    if pr_data_for_metrics.is_empty() {
        // Use all PRs for display purposes
        let first_pr_date = pr_data.first().unwrap().created_at;
        let month_start = {
            let dt = first_pr_date;
            Utc.with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
                .unwrap()
        };

        return MonthData {
            month_start,
            total_prs: 0, // No PRs counted in metrics
            avg_lead_time: Duration::zero(),
            frequency: 0.0,
            size_s: 0,
            size_m: 0,
            size_l: 0,
            size_xl: 0,
            weeks: Vec::new(),
            repos: Vec::new(),
            prs_by_week: Vec::new(),
            reviewers: Vec::new(),
            reviewed_count: 0,
        };
    }

    let first_pr_date = pr_data_for_metrics.first().unwrap().created_at;
    let last_pr_date = pr_data_for_metrics.last().unwrap().created_at;

    let month_start = Utc
        .with_ymd_and_hms(first_pr_date.year(), first_pr_date.month(), 1, 0, 0, 0)
        .unwrap();

    let days_from_monday = first_pr_date.weekday().num_days_from_monday() as i64;
    let week1_start = (first_pr_date - Duration::days(days_from_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    // Calculate how many weeks we need to cover all PRs
    let days_span = (last_pr_date - week1_start).num_days();
    let weeks_needed = ((days_span / 7) + 1).max(1) as usize;

    let mut weeks: Vec<(DateTime<Utc>, DateTime<Utc>, Vec<PRData>)> = Vec::new();
    for i in 0..weeks_needed {
        let start = week1_start + Duration::days((i * 7) as i64);
        let end = (start + Duration::days(6))
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc();
        weeks.push((start, end, Vec::new()));
    }

    for pr in &pr_data {
        for (start, end, prs) in &mut weeks {
            if *start <= pr.created_at && pr.created_at <= *end {
                prs.push(pr.clone());
                break;
            }
        }
    }

    let mut by_repo: BTreeMap<String, Vec<PRData>> = BTreeMap::new();
    for pr in &pr_data {
        by_repo
            .entry(pr.repo_name.clone())
            .or_default()
            .push(pr.clone());
    }

    let avg_lead_time = avg_duration(
        &pr_data_for_metrics
            .iter()
            .map(|pr| pr.lead_time)
            .collect::<Vec<_>>(),
    );

    let time_span_days = (last_pr_date - first_pr_date).num_days().max(1) as f64;
    let frequency = pr_data_for_metrics.len() as f64 / (time_span_days / 7.0).max(1.0);

    let week_data: Vec<WeekData> = weeks
        .iter()
        .enumerate()
        .map(|(i, (start, end, prs))| {
            let lead_times: Vec<Duration> = prs.iter().map(|pr| pr.lead_time).collect();
            WeekData {
                week_num: i + 1,
                week_start: *start,
                week_end: *end,
                pr_count: prs.len(),
                avg_lead_time: avg_duration(&lead_times),
            }
        })
        .collect();

    let pr_details_by_week: Vec<Vec<PRDetail>> = weeks
        .iter()
        .map(|(_, _, prs)| {
            prs.iter()
                .map(|pr| PRDetail {
                    created_at: pr.created_at,
                    repo: pr.repo_name.clone(),
                    number: pr.number,
                    title: pr.title.clone(),
                    body: pr.body.clone(),
                    lead_time: pr.lead_time,
                    additions: pr.additions,
                    deletions: pr.deletions,
                    changed_files: pr.changed_files,
                })
                .collect()
        })
        .collect();

    let mut repos: Vec<RepoData> = by_repo
        .iter()
        .map(|(name, repo_prs)| {
            let lead_times: Vec<Duration> = repo_prs.iter().map(|pr| pr.lead_time).collect();
            let mut size_s = 0;
            let mut size_m = 0;
            let mut size_l = 0;
            let mut size_xl = 0;

            for pr in repo_prs {
                match compute_pr_size(pr.additions, pr.deletions, pr.changed_files, &config.size) {
                    PRSize::S => size_s += 1,
                    PRSize::M => size_m += 1,
                    PRSize::L => size_l += 1,
                    PRSize::XL => size_xl += 1,
                }
            }

            RepoData {
                name: name.clone(),
                pr_count: repo_prs.len(),
                avg_lead_time: avg_duration(&lead_times),
                size_s,
                size_m,
                size_l,
                size_xl,
            }
        })
        .collect();
    repos.sort_by(|a, b| b.pr_count.cmp(&a.pr_count));

    let mut size_s = 0;
    let mut size_m = 0;
    let mut size_l = 0;
    let mut size_xl = 0;

    // Only count sizes for non-excluded PRs
    for pr in &pr_data_for_metrics {
        match compute_pr_size(pr.additions, pr.deletions, pr.changed_files, &config.size) {
            PRSize::S => size_s += 1,
            PRSize::M => size_m += 1,
            PRSize::L => size_l += 1,
            PRSize::XL => size_xl += 1,
        }
    }

    let mut reviewer_map: BTreeMap<String, usize> = BTreeMap::new();
    for pr in &prs {
        for review in &pr.reviews.nodes {
            *reviewer_map.entry(review.author.login.clone()).or_insert(0) += 1;
        }
    }

    let mut reviewers: Vec<ReviewerData> = reviewer_map
        .iter()
        .map(|(login, count)| ReviewerData {
            login: login.clone(),
            pr_count: *count,
        })
        .collect();
    reviewers.sort_by(|a, b| b.pr_count.cmp(&a.pr_count));

    MonthData {
        month_start,
        total_prs: pr_data_for_metrics.len(), // Only count non-excluded PRs
        avg_lead_time,
        frequency,
        size_s,
        size_m,
        size_l,
        size_xl,
        weeks: week_data,
        repos,
        prs_by_week: pr_details_by_week,
        reviewers,
        reviewed_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pr_size() {
        use crate::config::SizeConfig;
        let config = SizeConfig::default();

        // Small: <= 50 lines
        assert_eq!(compute_pr_size(10, 5, 1, &config), PRSize::S);
        assert_eq!(compute_pr_size(25, 25, 3, &config), PRSize::S);
        assert_eq!(compute_pr_size(50, 0, 5, &config), PRSize::S);
        assert_eq!(compute_pr_size(0, 50, 2, &config), PRSize::S);

        // Medium: 51-200 lines
        assert_eq!(compute_pr_size(51, 0, 1, &config), PRSize::M);
        assert_eq!(compute_pr_size(100, 50, 5, &config), PRSize::M);
        assert_eq!(compute_pr_size(150, 50, 8, &config), PRSize::M);
        assert_eq!(compute_pr_size(200, 0, 10, &config), PRSize::M);

        // Large: 201-500 lines
        assert_eq!(compute_pr_size(201, 0, 1, &config), PRSize::L);
        assert_eq!(compute_pr_size(300, 100, 8, &config), PRSize::L);
        assert_eq!(compute_pr_size(250, 250, 12, &config), PRSize::L);
        assert_eq!(compute_pr_size(500, 0, 14, &config), PRSize::L);

        // XL: > 500 lines
        assert_eq!(compute_pr_size(501, 0, 1, &config), PRSize::XL);
        assert_eq!(compute_pr_size(1000, 500, 10, &config), PRSize::XL);
        assert_eq!(compute_pr_size(5000, 2000, 20, &config), PRSize::XL);

        // File count overrides: >= 15 files bumps to at least L
        assert_eq!(compute_pr_size(10, 5, 15, &config), PRSize::L);
        assert_eq!(compute_pr_size(50, 50, 20, &config), PRSize::L);

        // File count overrides: >= 15 files with > 500 lines is XL
        assert_eq!(compute_pr_size(300, 300, 15, &config), PRSize::XL);

        // File count overrides: >= 25 files is always XL
        assert_eq!(compute_pr_size(10, 5, 25, &config), PRSize::XL);
        assert_eq!(compute_pr_size(1, 1, 30, &config), PRSize::XL);
        assert_eq!(compute_pr_size(100, 50, 50, &config), PRSize::XL);

        // Test with custom thresholds
        let custom_config = SizeConfig::new(100, 500, 1000);
        assert_eq!(compute_pr_size(100, 0, 1, &custom_config), PRSize::S);
        assert_eq!(compute_pr_size(101, 0, 1, &custom_config), PRSize::M);
        assert_eq!(compute_pr_size(500, 0, 1, &custom_config), PRSize::M);
        assert_eq!(compute_pr_size(501, 0, 1, &custom_config), PRSize::L);
        assert_eq!(compute_pr_size(1000, 0, 1, &custom_config), PRSize::L);
        assert_eq!(compute_pr_size(1001, 0, 1, &custom_config), PRSize::XL);
    }

    #[test]
    fn test_pr_detail_size_method() {
        use crate::config::SizeConfig;
        let config = SizeConfig::default();
        let pr = PRDetail {
            created_at: Utc::now(),
            repo: "test/repo".to_string(),
            number: 1,
            title: "Test PR".to_string(),
            body: None,
            lead_time: Duration::hours(1),
            additions: 100,
            deletions: 50,
            changed_files: 5,
        };
        assert_eq!(pr.size(&config), PRSize::M);
    }

    #[test]
    fn test_repo_data_format_size_distribution() {
        let repo = RepoData {
            name: "test/repo".to_string(),
            pr_count: 10,
            avg_lead_time: Duration::hours(2),
            size_s: 3,
            size_m: 2,
            size_l: 4,
            size_xl: 1,
        };
        assert_eq!(repo.format_size_distribution(), "3S 2M 4L 1XL");
    }

    #[test]
    fn test_repo_data_format_size_distribution_zeros() {
        let repo = RepoData {
            name: "test/repo".to_string(),
            pr_count: 5,
            avg_lead_time: Duration::minutes(30),
            size_s: 5,
            size_m: 0,
            size_l: 0,
            size_xl: 0,
        };
        assert_eq!(repo.format_size_distribution(), "5S 0M 0L 0XL");
    }

    #[test]
    fn test_month_data_format_size_distribution() {
        let month = MonthData {
            month_start: Utc::now(),
            total_prs: 34,
            avg_lead_time: Duration::minutes(35),
            frequency: 119.0,
            size_s: 26,
            size_m: 3,
            size_l: 4,
            size_xl: 1,
            weeks: Vec::new(),
            repos: Vec::new(),
            prs_by_week: Vec::new(),
            reviewers: Vec::new(),
            reviewed_count: 0,
        };
        assert_eq!(month.format_size_distribution(), "26S 3M 4L 1XL");
    }
}
