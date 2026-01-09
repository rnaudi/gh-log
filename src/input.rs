use chrono::{DateTime, Utc};
use serde::Deserialize;

/// GitHub repository information.
#[derive(Debug, Deserialize, Clone)]
pub struct Repository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

/// Pull request data fetched from GitHub GraphQL API.
///
/// Includes timing information, size metrics, and repository details.
#[derive(Debug, Deserialize, Clone)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub repository: Repository,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub additions: u32,
    pub deletions: u32,
    #[serde(rename = "changedFiles")]
    pub changed_files: u32,
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
                        repository,
                        created_at,
                        updated_at,
                        additions,
                        deletions,
                        changed_files,
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
