use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Repository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub repository: Repository,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

