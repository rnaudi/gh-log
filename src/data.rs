use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use std::collections::BTreeMap;
use std::fmt;

/// Pull request size categorization.
///
/// Computed based on lines changed and files modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PRSize {
    /// Small: <= 50 lines changed
    S,
    /// Medium: 51-200 lines changed
    M,
    /// Large: 201-500 lines changed
    L,
    /// Extra Large: > 500 lines changed
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

/// Computes PR size based on additions, deletions, and number of files changed.
///
/// # Size Categories
///
/// - `S` (Small): <= 50 lines changed
/// - `M` (Medium): 51-200 lines changed
/// - `L` (Large): 201-500 lines changed
/// - `XL` (Extra Large): > 500 lines changed
///
/// # File Count Overrides
///
/// - >= 25 files: always `XL`
/// - >= 15 files: at least `L` (bumps to `XL` if > 500 lines)
///
/// # Examples
///
/// ```
/// use gh_log::data::{compute_pr_size, PRSize};
///
/// assert_eq!(compute_pr_size(25, 25, 3), PRSize::S);
/// assert_eq!(compute_pr_size(100, 50, 5), PRSize::M);
/// assert_eq!(compute_pr_size(300, 100, 8), PRSize::L);
/// assert_eq!(compute_pr_size(1000, 500, 10), PRSize::XL);
/// ```
pub fn compute_pr_size(additions: u32, deletions: u32, changed_files: u32) -> PRSize {
    let total_lines = additions + deletions;
    if changed_files >= 25 {
        return PRSize::XL;
    }

    if changed_files >= 15 {
        if total_lines > 500 {
            return PRSize::XL;
        }
        return PRSize::L;
    }

    match total_lines {
        0..=50 => PRSize::S,
        51..=200 => PRSize::M,
        201..=500 => PRSize::L,
        _ => PRSize::XL,
    }
}

/// Aggregated PR metrics for a single week.
#[derive(Debug)]
pub struct WeekData {
    pub week_num: usize,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub pr_count: usize,
    pub avg_lead_time: Duration,
}

/// Aggregated PR metrics for a single repository.
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
    /// Returns a formatted string of the size distribution (e.g., "18S 0M 0L 0XL").
    pub fn format_size_distribution(&self) -> String {
        format!(
            "{}S {}M {}L {}XL",
            self.size_s, self.size_m, self.size_l, self.size_xl
        )
    }
}

/// Detailed information about a single pull request.
#[derive(Debug, Clone)]
pub struct PRDetail {
    pub created_at: DateTime<Utc>,
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub lead_time: Duration,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

impl PRDetail {
    /// Returns the computed size category for this PR.
    pub fn size(&self) -> PRSize {
        compute_pr_size(self.additions, self.deletions, self.changed_files)
    }
}

/// Aggregated PR metrics and details for a calendar month.
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
        }
    }
}

impl MonthData {
    /// Returns a formatted string of the size distribution (e.g., "26S 3M 4L 1XL").
    pub fn format_size_distribution(&self) -> String {
        format!(
            "{}S {}M {}L {}XL",
            self.size_s, self.size_m, self.size_l, self.size_xl
        )
    }
}

/// Computes the average of a list of durations.
pub fn avg_duration(durations: &[Duration]) -> Duration {
    if durations.is_empty() {
        return Duration::zero();
    }
    let mut total_seconds: i64 = 0;
    for d in durations {
        total_seconds += d.num_seconds();
    }
    Duration::seconds(total_seconds / durations.len() as i64)
}

#[derive(Clone)]
struct PRData {
    number: u32,
    title: String,
    created_at: DateTime<Utc>,
    lead_time: Duration,
    repository: Repository,
    additions: u32,
    deletions: u32,
    changed_files: u32,
}

#[derive(Clone)]
struct Repository {
    name_with_owner: String,
}

/// Processes a list of pull requests and computes aggregated metrics.
///
/// Groups PRs by week and repository, calculates lead times, and computes frequency.
pub fn process_prs(prs: Vec<crate::input::PullRequest>) -> MonthData {
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
            created_at: pr.created_at,
            lead_time,
            repository: Repository {
                name_with_owner: pr.repository.name_with_owner.clone(),
            },
            additions: pr.additions,
            deletions: pr.deletions,
            changed_files: pr.changed_files,
        });
    }

    pr_data.sort_by_key(|pr| pr.created_at);

    let first_pr_date = pr_data.first().unwrap().created_at;
    let last_pr_date = pr_data.last().unwrap().created_at;

    let month_start = {
        let dt = first_pr_date;
        Utc.with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
            .unwrap()
    };

    // Find the Monday of the week containing the first PR
    let first_pr_weekday = first_pr_date.weekday();
    let days_from_monday = match first_pr_weekday {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };
    let week1_start = first_pr_date - Duration::days(days_from_monday as i64);
    let week1_start = week1_start
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
            .entry(pr.repository.name_with_owner.clone())
            .or_default()
            .push(pr.clone());
    }

    let avg_lead_time = avg_duration(
        &pr_data
            .iter()
            .map(|pr| pr.lead_time)
            .collect::<Vec<Duration>>(),
    );

    // Calculate frequency based on actual time span
    let time_span_days = (last_pr_date - first_pr_date).num_days().max(1) as f64;
    let time_span_weeks = time_span_days / 7.0;
    let frequency = if time_span_weeks > 0.0 {
        pr_data.len() as f64 / time_span_weeks
    } else {
        pr_data.len() as f64
    };

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
                    repo: pr.repository.name_with_owner.clone(),
                    number: pr.number,
                    title: pr.title.clone(),
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
                match compute_pr_size(pr.additions, pr.deletions, pr.changed_files) {
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

    for pr in &pr_data {
        match compute_pr_size(pr.additions, pr.deletions, pr.changed_files) {
            PRSize::S => size_s += 1,
            PRSize::M => size_m += 1,
            PRSize::L => size_l += 1,
            PRSize::XL => size_xl += 1,
        }
    }

    MonthData {
        month_start,
        total_prs: pr_data.len(),
        avg_lead_time,
        frequency,
        size_s,
        size_m,
        size_l,
        size_xl,
        weeks: week_data,
        repos,
        prs_by_week: pr_details_by_week,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pr_size() {
        // Small: <= 50 lines
        assert_eq!(compute_pr_size(10, 5, 1), PRSize::S);
        assert_eq!(compute_pr_size(25, 25, 3), PRSize::S);
        assert_eq!(compute_pr_size(50, 0, 5), PRSize::S);
        assert_eq!(compute_pr_size(0, 50, 2), PRSize::S);

        // Medium: 51-200 lines
        assert_eq!(compute_pr_size(51, 0, 1), PRSize::M);
        assert_eq!(compute_pr_size(100, 50, 5), PRSize::M);
        assert_eq!(compute_pr_size(150, 50, 8), PRSize::M);
        assert_eq!(compute_pr_size(200, 0, 10), PRSize::M);

        // Large: 201-500 lines
        assert_eq!(compute_pr_size(201, 0, 1), PRSize::L);
        assert_eq!(compute_pr_size(300, 100, 8), PRSize::L);
        assert_eq!(compute_pr_size(250, 250, 12), PRSize::L);
        assert_eq!(compute_pr_size(500, 0, 14), PRSize::L);

        // XL: > 500 lines
        assert_eq!(compute_pr_size(501, 0, 1), PRSize::XL);
        assert_eq!(compute_pr_size(1000, 500, 10), PRSize::XL);
        assert_eq!(compute_pr_size(5000, 2000, 20), PRSize::XL);

        // File count overrides: >= 15 files bumps to at least L
        assert_eq!(compute_pr_size(10, 5, 15), PRSize::L);
        assert_eq!(compute_pr_size(50, 50, 20), PRSize::L);

        // File count overrides: >= 15 files with > 500 lines is XL
        assert_eq!(compute_pr_size(300, 300, 15), PRSize::XL);

        // File count overrides: >= 25 files is always XL
        assert_eq!(compute_pr_size(10, 5, 25), PRSize::XL);
        assert_eq!(compute_pr_size(1, 1, 30), PRSize::XL);
        assert_eq!(compute_pr_size(100, 50, 50), PRSize::XL);

        // TODO: Add property-based test to verify:
        // - Size is monotonic with respect to total lines
        // - File count overrides work correctly across all ranges
        // - All inputs produce valid PRSize variants
    }

    #[test]
    fn test_pr_detail_size_method() {
        let pr = PRDetail {
            created_at: Utc::now(),
            repo: "test/repo".to_string(),
            number: 1,
            title: "Test PR".to_string(),
            lead_time: Duration::hours(1),
            additions: 100,
            deletions: 50,
            changed_files: 5,
        };
        assert_eq!(pr.size(), PRSize::M);
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
        };
        assert_eq!(month.format_size_distribution(), "26S 3M 4L 1XL");
    }
}
