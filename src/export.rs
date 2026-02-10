//! Non-interactive output: JSON, CSV, and plain-text export of monthly PR analytics.

use crate::config::SizeConfig;
use crate::data;
use crate::view::format_duration;

fn format_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

/// Render the monthly analytics as JSON for downstream tooling or AI prompts.
///
/// # Errors
/// Returns an error if serialization fails or writing to stdout encounters an I/O failure.
pub fn print_json(data: &data::MonthData, size_cfg: &SizeConfig) -> anyhow::Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct JsonOutput<'a> {
        month_start: String,
        total_prs: usize,
        avg_lead_time_hours: f64,
        frequency: f64,
        size_distribution: SizeDistribution,
        reviewers: Vec<JsonReviewer<'a>>,
        reviewed_count: usize,
        weeks: Vec<JsonWeek<'a>>,
        repositories: Vec<JsonRepo<'a>>,
    }

    #[derive(Serialize)]
    struct SizeDistribution {
        s: usize,
        m: usize,
        l: usize,
        xl: usize,
    }

    #[derive(Serialize)]
    struct JsonReviewer<'a> {
        login: &'a str,
        pr_count: usize,
    }

    #[derive(Serialize)]
    struct JsonWeek<'a> {
        week_num: usize,
        week_start: String,
        week_end: String,
        pr_count: usize,
        avg_lead_time_hours: f64,
        prs: Vec<JsonPR<'a>>,
    }

    #[derive(Serialize)]
    struct JsonPR<'a> {
        created_at: String,
        repo: &'a str,
        number: u32,
        title: &'a str,
        body: Option<&'a str>,
        lead_time_hours: f64,
        size: String,
        additions: u32,
        deletions: u32,
        changed_files: u32,
    }

    #[derive(Serialize)]
    struct JsonRepo<'a> {
        name: &'a str,
        pr_count: usize,
        avg_lead_time_hours: f64,
        size_distribution: SizeDistribution,
    }

    let output = JsonOutput {
        month_start: format_date(data.month_start),
        total_prs: data.total_prs,
        avg_lead_time_hours: data.avg_lead_time.num_seconds() as f64 / 3600.0,
        frequency: data.frequency,
        size_distribution: SizeDistribution {
            s: data.size_s,
            m: data.size_m,
            l: data.size_l,
            xl: data.size_xl,
        },
        reviewers: data
            .reviewers
            .iter()
            .map(|r| JsonReviewer {
                login: &r.login,
                pr_count: r.pr_count,
            })
            .collect(),
        reviewed_count: data.reviewed_count,
        weeks: data
            .weeks
            .iter()
            .enumerate()
            .map(|(idx, week)| JsonWeek {
                week_num: week.week_num,
                week_start: format_date(week.week_start),
                week_end: format_date(week.week_end),
                pr_count: week.pr_count,
                avg_lead_time_hours: week.avg_lead_time.num_seconds() as f64 / 3600.0,
                prs: data.prs_by_week[idx]
                    .iter()
                    .map(|pr| JsonPR {
                        created_at: format_date(pr.created_at),
                        repo: &pr.repo,
                        number: pr.number,
                        title: &pr.title,
                        body: pr.body.as_deref(),
                        lead_time_hours: pr.lead_time.num_seconds() as f64 / 3600.0,
                        size: pr.size(size_cfg).to_string(),
                        additions: pr.additions,
                        deletions: pr.deletions,
                        changed_files: pr.changed_files,
                    })
                    .collect(),
            })
            .collect(),
        repositories: data
            .repos
            .iter()
            .map(|repo| JsonRepo {
                name: &repo.name,
                pr_count: repo.pr_count,
                avg_lead_time_hours: repo.avg_lead_time.num_seconds() as f64 / 3600.0,
                size_distribution: SizeDistribution {
                    s: repo.size_s,
                    m: repo.size_m,
                    l: repo.size_l,
                    xl: repo.size_xl,
                },
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);
    Ok(())
}

/// Render the monthly analytics as CSV suitable for spreadsheets or further processing.
///
/// # Errors
/// Returns an error if writing to stdout encounters an I/O failure.
pub fn print_csv(data: &data::MonthData, size_cfg: &SizeConfig) -> anyhow::Result<()> {
    println!(
        "created_at,repo,number,title,body,lead_time_hours,size,additions,deletions,changed_files"
    );

    for week_prs in &data.prs_by_week {
        for pr in week_prs {
            let lead_time_hours = pr.lead_time.num_seconds() as f64 / 3600.0;
            let body_escaped = pr
                .body
                .as_ref()
                .map(|b| b.replace("\"", "\"\"").replace("\n", " "))
                .unwrap_or_default();
            println!(
                "\"{}\",\"{}\",{},\"{}\",\"{}\",{:.2},\"{}\",{},{},{}",
                format_date(pr.created_at),
                pr.repo.replace("\"", "\"\""),
                pr.number,
                pr.title.replace("\"", "\"\""),
                body_escaped,
                lead_time_hours,
                pr.size(size_cfg),
                pr.additions,
                pr.deletions,
                pr.changed_files
            );
        }
    }

    Ok(())
}

/// Render a human-readable summary of the monthly analytics directly to stdout.
pub fn print_data(data: &data::MonthData, month: &str, size_cfg: &SizeConfig) {
    println!("GitHub PRs for {}", month);
    println!("  - Total PRs: {}", data.total_prs);
    println!(
        "  - Average Lead Time: {}",
        format_duration(data.avg_lead_time)
    );
    println!("  - Frequency: {:.1} PRs/week", data.frequency);
    println!("  - Sizes: [{}]", data.format_size_distribution());
    println!();

    if !data.reviewers.is_empty() {
        println!("Top Reviewers");
        for reviewer in data.reviewers.iter().take(10) {
            println!("  - {}: {} PRs", reviewer.login, reviewer.pr_count);
        }
        println!();
    }

    println!("My Review Activity");
    println!("  - PRs Reviewed: {}", data.reviewed_count);
    if data.total_prs > 0 {
        let ratio = data.reviewed_count as f64 / data.total_prs as f64;
        println!(
            "  - Review Balance: {:.1}:1 ({} reviewed / {} created)",
            ratio, data.reviewed_count, data.total_prs
        );
    }
    println!();

    for (week_idx, week) in data.weeks.iter().enumerate() {
        println!(
            "Week {} ({} - {})",
            week.week_num,
            format_date(week.week_start),
            format_date(week.week_end)
        );
        println!("  - PRs: {}", week.pr_count);
        println!("  - Avg Lead Time: {}", format_duration(week.avg_lead_time));

        let prs = &data.prs_by_week[week_idx];
        for pr in prs {
            println!(
                "    - {} | {} | #{} {} | {} | {}",
                format_date(pr.created_at),
                pr.repo,
                pr.number,
                pr.title,
                format_duration(pr.lead_time),
                pr.size(size_cfg)
            );
            if let Some(body) = &pr.body
                && !body.is_empty()
            {
                for line in body.lines() {
                    println!("      {}", line);
                }
            }
        }
        println!();
    }

    println!("Repositories");
    for repo in &data.repos {
        println!(
            "  - {} - {} PRs (Avg: {}) [{}]",
            repo.name,
            repo.pr_count,
            format_duration(repo.avg_lead_time),
            repo.format_size_distribution()
        );
    }
}
