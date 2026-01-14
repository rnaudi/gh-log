use anyhow::bail;
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Review {
    pub author: Author,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reviews {
    pub nodes: Vec<Review>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub repository: Repository,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub additions: u32,
    pub deletions: u32,
    #[serde(rename = "changedFiles")]
    pub changed_files: u32,
    pub reviews: Reviews,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: GraphQLData,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    search: SearchResults,
}

#[derive(Debug, Deserialize)]
struct SearchResults {
    nodes: Vec<GraphQLPullRequest>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLPullRequest {
    number: u32,
    title: String,
    body: Option<String>,
    repository: Repository,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
    additions: u32,
    deletions: u32,
    #[serde(rename = "changedFiles")]
    changed_files: u32,
    reviews: Reviews,
}

pub struct CommandClient {}

impl CommandClient {
    pub fn new() -> anyhow::Result<Self> {
        check_gh_installed()?;
        Ok(CommandClient {})
    }

    pub fn fetch_prs(&self, month: &str) -> anyhow::Result<Vec<PullRequest>> {
        let mut all_prs = Vec::new();
        let mut has_next_page = true;
        let mut cursor: Option<String> = None;

        while has_next_page {
            let after_clause = cursor
                .as_ref()
                .map(|c| format!(r#", after: "{}""#, c))
                .unwrap_or_default();

            let query = format!(
                r#"{{
  search(query: "is:pr author:@me created:{}", type: ISSUE, first: 100{}) {{
    pageInfo {{
      hasNextPage
      endCursor
    }}
    nodes {{
      ... on PullRequest {{
        number
        title
        body
        repository {{
          nameWithOwner
        }}
        createdAt
        updatedAt
        additions
        deletions
        changedFiles
        reviews(first: 10) {{
          nodes {{
            author {{
              login
            }}
          }}
        }}
      }}
    }}
  }}
}}"#,
                month, after_clause
            );

            let output = Command::new("gh")
                .arg("api")
                .arg("graphql")
                .arg("-f")
                .arg(format!("query={}", query))
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("GraphQL query failed: {}", stderr);
            }

            let json_str = String::from_utf8_lossy(&output.stdout);
            let response: GraphQLResponse = serde_json::from_str(&json_str)?;

            for pr in response.data.search.nodes {
                all_prs.push(PullRequest {
                    number: pr.number,
                    title: pr.title,
                    body: pr.body,
                    repository: pr.repository,
                    created_at: pr.created_at,
                    updated_at: pr.updated_at,
                    additions: pr.additions,
                    deletions: pr.deletions,
                    changed_files: pr.changed_files,
                    reviews: pr.reviews,
                });
            }

            has_next_page = response.data.search.page_info.has_next_page;
            cursor = response.data.search.page_info.end_cursor;
        }

        Ok(all_prs)
    }

    pub fn fetch_reviewed_prs(&self, month: &str) -> anyhow::Result<usize> {
        let mut total_count = 0;
        let mut has_next_page = true;
        let mut cursor: Option<String> = None;

        while has_next_page {
            let after_clause = cursor
                .as_ref()
                .map(|c| format!(r#", after: "{}""#, c))
                .unwrap_or_default();

            let query = format!(
                r#"{{
  search(query: "is:pr reviewed-by:@me created:{}", type: ISSUE, first: 100{}) {{
    pageInfo {{
      hasNextPage
      endCursor
    }}
    issueCount
  }}
}}"#,
                month, after_clause
            );

            let output = Command::new("gh")
                .arg("api")
                .arg("graphql")
                .arg("-f")
                .arg(format!("query={}", query))
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("GraphQL query failed: {}", stderr);
            }

            let json_str = String::from_utf8_lossy(&output.stdout);
            let response: serde_json::Value = serde_json::from_str(&json_str)?;

            if let Some(issue_count) = response["data"]["search"]["issueCount"].as_u64() {
                total_count = issue_count as usize;
            }

            has_next_page = response["data"]["search"]["pageInfo"]["hasNextPage"]
                .as_bool()
                .unwrap_or(false);
            cursor = response["data"]["search"]["pageInfo"]["endCursor"]
                .as_str()
                .map(|s| s.to_string());
        }

        Ok(total_count)
    }
}

fn check_gh_installed() -> anyhow::Result<()> {
    match Command::new("gh").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => bail!(
            "GitHub CLI (gh) is installed but not working correctly.\nRun 'gh auth login' to authenticate."
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("GitHub CLI (gh) is not installed.\nInstall it from: https://cli.github.com/")
        }
        Err(e) => bail!("Failed to check for GitHub CLI: {}", e),
    }
}

#[cfg(test)]
pub mod prop_strategies {
    use super::*;
    use chrono::{TimeZone, Utc};
    use proptest::prelude::*;

    pub fn datetime_strategy() -> impl Strategy<Value = DateTime<Utc>> {
        (
            2020i32..=2030,
            1u32..=12,
            1u32..=28,
            0u32..24,
            0u32..60,
            0u32..60,
        )
            .prop_map(|(year, month, day, hour, minute, second)| {
                Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
                    .unwrap()
            })
    }

    pub fn repository_strategy() -> impl Strategy<Value = Repository> {
        "[a-z]{3,10}/[a-z]{3,10}".prop_map(|name| Repository {
            name_with_owner: name,
        })
    }

    pub fn title_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Fix critical bug".to_string()),
            Just("Add new feature".to_string()),
            Just("Refactor authentication".to_string()),
            Just("Update dependencies".to_string()),
            Just("Improve performance".to_string()),
            "[A-Z][a-z]{3,10} [a-z]{3,10}".prop_map(|s| s),
        ]
    }

    pub fn pull_request_strategy() -> impl Strategy<Value = PullRequest> {
        (
            1u32..10000,
            title_strategy(),
            repository_strategy(),
            datetime_strategy(),
            0i64..=(7 * 24 * 3600),
            0u32..5000,
            0u32..5000,
            1u32..100,
        )
            .prop_map(
                |(
                    number,
                    title,
                    repository,
                    created_at,
                    lead_time_secs,
                    additions,
                    deletions,
                    changed_files,
                )| {
                    let updated_at = created_at + chrono::Duration::seconds(lead_time_secs);
                    PullRequest {
                        number,
                        title,
                        body: None,
                        repository,
                        created_at,
                        updated_at,
                        additions,
                        deletions,
                        changed_files,
                        reviews: Reviews { nodes: Vec::new() },
                    }
                },
            )
    }

    pub fn pull_requests_strategy(
        min_size: usize,
        max_size: usize,
    ) -> impl Strategy<Value = Vec<PullRequest>> {
        prop::collection::vec(pull_request_strategy(), min_size..=max_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_pull_request_dates_are_valid(pr in prop_strategies::pull_request_strategy()) {
            prop_assert!(pr.created_at <= pr.updated_at);
        }

        #[test]
        fn test_pull_request_number_is_positive(pr in prop_strategies::pull_request_strategy()) {
            prop_assert!(pr.number > 0);
        }

        #[test]
        fn test_repository_name_format(pr in prop_strategies::pull_request_strategy()) {
            prop_assert!(pr.repository.name_with_owner.contains('/'));
        }

        #[test]
        fn test_multiple_prs_generation(prs in prop_strategies::pull_requests_strategy(1, 50)) {
            prop_assert!(!prs.is_empty());
            prop_assert!(prs.len() <= 50);
            for pr in prs {
                prop_assert!(pr.created_at <= pr.updated_at);
            }
        }
    }
}
