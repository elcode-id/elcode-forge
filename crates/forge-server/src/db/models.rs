//! Database row models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Record representing a registered project repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Record representing an ingested snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct ApiKeyRow {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub is_active: i64,
}
