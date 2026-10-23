//! Database row models.

use serde::{Deserialize, Serialize};

/// Record representing a registered project repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Record representing an ingested snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub id: String,
    pub repo_id: String,
    pub branch: String,
    pub commit_hash: String,
    pub content_hash: String,
    pub tool_version: String,
    pub health_overall: i64,
    pub health_json: String,
    pub findings_count: i64,
    pub modules_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: i64,
}

/// Record representing an individual finding within a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingRow {
    pub id: String,
    pub snapshot_id: String,
    pub finding_id: String,
    pub rule_id: String,
    pub category: String,
    pub severity: String,
    pub confidence: String,
    pub title: String,
    pub finding_json: String,
}

/// Record representing an authorized API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyRow {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub is_active: i64,
}

/// Record representing a configured outbound webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookRow {
    pub id: String,
    pub repo_id: Option<String>,
    pub url: String,
    pub secret: Option<String>,
    pub events_json: String,
    pub min_health_threshold: Option<i64>,
    pub is_active: i64,
    pub created_at: i64,
}

/// Lightweight summary of a registered project with latest snapshot information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub repo_id: String,
    pub name: String,
    pub latest_branch: Option<String>,
    pub latest_commit: Option<String>,
    pub latest_health: Option<u8>,
    pub findings_count: Option<usize>,
    pub updated_at: i64,
}

/// Summary of a branch's current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchSummary {
    pub branch: String,
    pub latest_commit: String,
    pub latest_health: u8,
    pub findings_count: usize,
    pub updated_at: i64,
}

/// Historical score point along a branch timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub commit_hash: String,
    pub branch: String,
    pub timestamp_secs: i64,
    pub overall_health: u8,
    pub findings_count: usize,
    pub health_json: String,
}
