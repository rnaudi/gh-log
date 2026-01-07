use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use std::collections::BTreeMap;

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
}

#[derive(Debug, Clone)]
pub struct PRDetail {
    pub created_at: DateTime<Utc>,
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub lead_time: Duration,
}

#[derive(Debug)]
pub struct MonthData {
    pub month_start: DateTime<Utc>,
    pub total_prs: usize,
    pub avg_lead_time: Duration,
    pub frequency: f64,
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
            weeks: Vec::new(),
            repos: Vec::new(),
            prs_by_week: Vec::new(),
        }
    }
}

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

// Internal processing struct
#[derive(Clone)]
struct PRData {
    number: u32,
    title: String,
    created_at: DateTime<Utc>,
    lead_time: Duration,
    repository: Repository,
}

// Internal repository struct
#[derive(Clone)]
struct Repository {
    name_with_owner: String,
}

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
                })
                .collect()
        })
        .collect();

    let mut repos: Vec<RepoData> = by_repo
        .iter()
        .map(|(name, repo_prs)| {
            let lead_times: Vec<Duration> = repo_prs.iter().map(|pr| pr.lead_time).collect();
            RepoData {
                name: name.clone(),
                pr_count: repo_prs.len(),
                avg_lead_time: avg_duration(&lead_times),
            }
        })
        .collect();
    repos.sort_by(|a, b| b.pr_count.cmp(&a.pr_count));

    MonthData {
        month_start,
        total_prs: pr_data.len(),
        avg_lead_time,
        frequency,
        weeks: week_data,
        repos,
        prs_by_week: pr_details_by_week,
    }
}
