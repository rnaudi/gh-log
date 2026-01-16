use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use std::collections::BTreeMap;
use std::fmt;

use crate::{
    config::{Config, SizeConfig},
    github,
};

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
    pub prs_by_repo: Vec<Vec<PRDetail>>,
    pub reviewers: Vec<ReviewerData>,
    pub reviewed_count: usize,
}

impl MonthData {
    fn empty(month: &str) -> Self {
        let parts: Vec<&str> = month.split('-').collect();
        let year: i32 = parts[0].parse().unwrap();
        let month: u32 = parts[1].parse().unwrap();
        let month_start = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();

        Self {
            month_start,
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
            prs_by_repo: Vec::new(),
            reviewers: Vec::new(),
            reviewed_count: 0,
        }
    }

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

pub fn build_month_data(
    month: &str,
    mut prs: Vec<github::PullRequest>,
    reviewed_count: usize,
    cfg: &Config,
) -> MonthData {
    if prs.is_empty() {
        return MonthData::empty(month);
    }

    prs.retain(|pr| !cfg.should_exclude_pr_title(&pr.title));
    prs.retain(|pr| !cfg.should_exclude_repo(&pr.repository.name_with_owner));
    if prs.is_empty() {
        return MonthData::empty(month);
    }

    let reviewers = extract_reviewers(&prs);
    let mut pr_data = match build_pr_data(&prs) {
        Some(data) => data,
        None => return MonthData::empty(month),
    };

    pr_data.retain(|pr| !cfg.should_ignore_repo(&pr.repo_name));
    pr_data.retain(|pr| !cfg.should_ignore_pr_title(&pr.title));
    if pr_data.is_empty() {
        return MonthData::empty(month);
    }

    let first_pr_date = pr_data.first().unwrap().created_at;
    let last_pr_date = pr_data.last().unwrap().created_at;

    let by_week = group_prs_by_week(&pr_data, first_pr_date, last_pr_date);
    let by_repo = group_prs_by_repo(&pr_data);

    let month_start = Utc
        .with_ymd_and_hms(first_pr_date.year(), first_pr_date.month(), 1, 0, 0, 0)
        .unwrap();
    let avg_lead_time = avg_duration(&pr_data.iter().map(|pr| pr.lead_time).collect::<Vec<_>>());
    let time_span_days = (last_pr_date - first_pr_date).num_days().max(1) as f64;
    let frequency = pr_data.len() as f64 / (time_span_days / 7.0).max(1.0);
    let week_data = build_week_data(&by_week);
    let pr_details_by_week = build_pr_details_by_week(&by_week);
    let repos = build_repo_data(&by_repo, cfg);
    let (size_s, size_m, size_l, size_xl) = compute_size_counts(&pr_data, cfg);
    let prs_by_repo = build_prs_by_repo(&repos, &by_repo);

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
        prs_by_repo,
        reviewers,
        reviewed_count,
    }
}

fn group_prs_by_week(
    pr_data: &[PRData],
    first_pr_date: DateTime<Utc>,
    last_pr_date: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>, Vec<PRData>)> {
    let days_from_monday = first_pr_date.weekday().num_days_from_monday() as i64;
    let week1_start = (first_pr_date - Duration::days(days_from_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

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

    for pr in pr_data {
        for (start, end, prs) in &mut weeks {
            if *start <= pr.created_at && pr.created_at <= *end {
                prs.push(pr.clone());
                break;
            }
        }
    }

    weeks
}

fn group_prs_by_repo(pr_data: &[PRData]) -> BTreeMap<String, Vec<PRData>> {
    let mut by_repo: BTreeMap<String, Vec<PRData>> = BTreeMap::new();
    for pr in pr_data {
        by_repo
            .entry(pr.repo_name.clone())
            .or_default()
            .push(pr.clone());
    }
    by_repo
}

fn build_week_data(weeks: &[(DateTime<Utc>, DateTime<Utc>, Vec<PRData>)]) -> Vec<WeekData> {
    weeks
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
        .collect()
}

fn build_pr_details_by_week(
    weeks: &[(DateTime<Utc>, DateTime<Utc>, Vec<PRData>)],
) -> Vec<Vec<PRDetail>> {
    weeks
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
        .collect()
}

fn build_repo_data(by_repo: &BTreeMap<String, Vec<PRData>>, cfg: &Config) -> Vec<RepoData> {
    let mut repos: Vec<RepoData> = by_repo
        .iter()
        .map(|(name, repo_prs)| {
            let lead_times: Vec<Duration> = repo_prs.iter().map(|pr| pr.lead_time).collect();
            let (size_s, size_m, size_l, size_xl) = compute_size_counts(repo_prs, cfg);

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
    repos
}

fn compute_size_counts<T: AsRef<PRData>>(prs: &[T], cfg: &Config) -> (usize, usize, usize, usize) {
    let mut size_s = 0;
    let mut size_m = 0;
    let mut size_l = 0;
    let mut size_xl = 0;

    for pr in prs {
        let pr = pr.as_ref();
        match compute_pr_size(pr.additions, pr.deletions, pr.changed_files, &cfg.size) {
            PRSize::S => size_s += 1,
            PRSize::M => size_m += 1,
            PRSize::L => size_l += 1,
            PRSize::XL => size_xl += 1,
        }
    }

    (size_s, size_m, size_l, size_xl)
}

fn extract_reviewers(prs: &[crate::github::PullRequest]) -> Vec<ReviewerData> {
    let mut reviewer_map: BTreeMap<String, usize> = BTreeMap::new();
    for pr in prs {
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
    reviewers
}

fn build_prs_by_repo(
    repos: &[RepoData],
    by_repo: &BTreeMap<String, Vec<PRData>>,
) -> Vec<Vec<PRDetail>> {
    repos
        .iter()
        .map(|repo| {
            by_repo
                .get(&repo.name)
                .map(|repo_prs| {
                    repo_prs
                        .iter()
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
                .unwrap_or_default()
        })
        .collect()
}

impl AsRef<PRData> for PRData {
    fn as_ref(&self) -> &PRData {
        self
    }
}

fn build_pr_data(prs: &[github::PullRequest]) -> Option<Vec<PRData>> {
    let mut pr_data: Vec<PRData> = Vec::with_capacity(prs.len());
    for pr in prs {
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
    Some(pr_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{Author, PullRequest, Repository, Review, Reviews};

    fn create_test_pr(
        number: u32,
        title: &str,
        repo_name: &str,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        additions: u32,
        deletions: u32,
        changed_files: u32,
        reviewers: Vec<&str>,
    ) -> PullRequest {
        PullRequest {
            number,
            title: title.to_string(),
            body: Some(format!("Description for {}", title)),
            repository: Repository {
                name_with_owner: repo_name.to_string(),
            },
            created_at,
            updated_at,
            additions,
            deletions,
            changed_files,
            reviews: Reviews {
                nodes: reviewers
                    .into_iter()
                    .map(|login| Review {
                        author: Author {
                            login: login.to_string(),
                        },
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn test_build_month_data_empty_input() {
        let config = Config::default().unwrap();
        let prs = vec![];

        let result = build_month_data("2024-01", prs, 0, &config);

        assert_eq!(result.total_prs, 0);
        assert_eq!(result.weeks.len(), 0);
        assert_eq!(result.repos.len(), 0);
    }

    #[test]
    fn test_build_month_data_single_pr() {
        let config = Config::default().unwrap();
        let base_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();

        let prs = vec![create_test_pr(
            1,
            "Add feature",
            "owner/repo-a",
            base_date,
            base_date + Duration::hours(5),
            30,
            10,
            3,
            vec!["reviewer1"],
        )];

        let result = build_month_data("2024-01", prs, 1, &config);

        assert_eq!(result.total_prs, 1);
        assert_eq!(result.size_s, 1);
        assert_eq!(result.reviewed_count, 1);
        assert_eq!(result.reviewers.len(), 1);
        assert_eq!(result.reviewers[0].login, "reviewer1");
        assert_eq!(result.repos.len(), 1);
        assert_eq!(result.repos[0].name, "owner/repo-a");
    }

    #[test]
    fn test_build_month_data_multiple_repos_sorted_by_pr_count() {
        let config = Config::default().unwrap();
        let base_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();

        let prs = vec![
            create_test_pr(
                1,
                "PR 1",
                "owner/repo-a",
                base_date,
                base_date + Duration::hours(2),
                20,
                10,
                2,
                vec![],
            ),
            create_test_pr(
                2,
                "PR 2",
                "owner/repo-b",
                base_date + Duration::hours(1),
                base_date + Duration::hours(3),
                30,
                15,
                3,
                vec![],
            ),
            create_test_pr(
                3,
                "PR 3",
                "owner/repo-a",
                base_date + Duration::hours(2),
                base_date + Duration::hours(4),
                40,
                20,
                4,
                vec![],
            ),
        ];

        let result = build_month_data("2024-01", prs, 0, &config);

        assert_eq!(result.total_prs, 3);
        assert_eq!(result.repos.len(), 2);
        // Repos should be sorted by PR count (repo-a has 2, repo-b has 1)
        assert_eq!(result.repos[0].name, "owner/repo-a");
        assert_eq!(result.repos[0].pr_count, 2);
        assert_eq!(result.repos[1].name, "owner/repo-b");
        assert_eq!(result.repos[1].pr_count, 1);
    }

    #[test]
    fn test_build_month_data_size_distribution() {
        let config = Config::default().unwrap();
        let base_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();

        let prs = vec![
            create_test_pr(
                1,
                "Small PR",
                "owner/repo",
                base_date,
                base_date + Duration::hours(1),
                20,
                10,
                2,
                vec![],
            ),
            create_test_pr(
                2,
                "Medium PR",
                "owner/repo",
                base_date + Duration::hours(1),
                base_date + Duration::hours(3),
                100,
                50,
                5,
                vec![],
            ),
            create_test_pr(
                3,
                "Large PR",
                "owner/repo",
                base_date + Duration::hours(2),
                base_date + Duration::hours(5),
                300,
                100,
                10,
                vec![],
            ),
            create_test_pr(
                4,
                "XL PR",
                "owner/repo",
                base_date + Duration::hours(3),
                base_date + Duration::hours(7),
                600,
                200,
                15,
                vec![],
            ),
        ];

        let result = build_month_data("2024-01", prs, 0, &config);

        assert_eq!(result.total_prs, 4);
        assert_eq!(result.size_s, 1);
        assert_eq!(result.size_m, 1);
        assert_eq!(result.size_l, 1);
        assert_eq!(result.size_xl, 1);
        assert_eq!(result.format_size_distribution(), "1S 1M 1L 1XL");
    }

    #[test]
    fn test_build_month_data_week_grouping() {
        let config = Config::default().unwrap();
        let base_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(); // Monday

        let prs = vec![
            create_test_pr(
                1,
                "Week 1 PR 1",
                "owner/repo",
                base_date,
                base_date + Duration::hours(2),
                20,
                10,
                2,
                vec![],
            ),
            create_test_pr(
                2,
                "Week 1 PR 2",
                "owner/repo",
                base_date + Duration::days(2),
                base_date + Duration::days(2) + Duration::hours(3),
                30,
                15,
                3,
                vec![],
            ),
            create_test_pr(
                3,
                "Week 2 PR",
                "owner/repo",
                base_date + Duration::days(8),
                base_date + Duration::days(8) + Duration::hours(4),
                40,
                20,
                4,
                vec![],
            ),
        ];

        let result = build_month_data("2024-01", prs, 0, &config);

        assert_eq!(result.total_prs, 3);
        assert!(result.weeks.len() >= 2);
        assert_eq!(result.prs_by_week[0].len(), 2);
        assert_eq!(result.prs_by_week[1].len(), 1);
    }

    #[test]
    fn test_build_prs_by_repo() {
        let mut by_repo = BTreeMap::new();

        by_repo.insert(
            "owner/repo-a".to_string(),
            vec![PRData {
                number: 1,
                title: "PR 1".to_string(),
                body: None,
                created_at: Utc::now(),
                lead_time: Duration::hours(1),
                repo_name: "owner/repo-a".to_string(),
                additions: 10,
                deletions: 5,
                changed_files: 2,
            }],
        );

        by_repo.insert(
            "owner/repo-b".to_string(),
            vec![PRData {
                number: 2,
                title: "PR 2".to_string(),
                body: None,
                created_at: Utc::now(),
                lead_time: Duration::hours(2),
                repo_name: "owner/repo-b".to_string(),
                additions: 20,
                deletions: 10,
                changed_files: 3,
            }],
        );

        let repos = vec![
            RepoData {
                name: "owner/repo-a".to_string(),
                pr_count: 1,
                avg_lead_time: Duration::hours(1),
                size_s: 1,
                size_m: 0,
                size_l: 0,
                size_xl: 0,
            },
            RepoData {
                name: "owner/repo-b".to_string(),
                pr_count: 1,
                avg_lead_time: Duration::hours(2),
                size_s: 1,
                size_m: 0,
                size_l: 0,
                size_xl: 0,
            },
        ];

        let prs_by_repo = build_prs_by_repo(&repos, &by_repo);

        assert_eq!(prs_by_repo.len(), 2);
        assert_eq!(prs_by_repo[0].len(), 1);
        assert_eq!(prs_by_repo[0][0].number, 1);
        assert_eq!(prs_by_repo[1].len(), 1);
        assert_eq!(prs_by_repo[1][0].number, 2);
    }
}
