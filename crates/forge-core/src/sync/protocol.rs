use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{Finding, HealthScore, Module};
use crate::scoring::calculate_change_risk;

/// Synchronization status between local workspace and remote Forge Server.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Remote baseline and local state share the exact same content hash.
    UpToDate,
    /// Remote baseline successfully advanced to the new commit state.
    FastForward,
    /// Baseline was newly initialized or updated on the remote.
    BaselineUpdated,
    /// Remote baseline diverged from local base state.
    Conflict,
    /// Repository or baseline was not found on the remote.
    NotFound,
}

/// Content-addressed metadata manifest for a repository analysis snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SyncManifest {
    /// Canonical repository identifier (e.g., "elcode-id/elcode-forge").
    pub repo_id: String,
    /// Human-readable repository or project name.
    pub repo_name: String,
    /// Target Git branch (e.g., "main", "develop").
    pub branch: String,
    /// Git commit hash (SHA-1 or SHA-256).
    pub commit_hash: String,
    /// Snapshot creation timestamp (seconds since Unix epoch).
    pub timestamp_secs: u64,
    /// BLAKE3 content-addressed hash of repository files and findings.
    pub content_hash: String,
    /// Version of the ELCODE Forge engine that produced this snapshot.
    pub tool_version: String,
}

/// Push payload sent to remote Forge Server during `eforge push`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SyncPushPayload {
    /// Snapshot manifest metadata.
    pub manifest: SyncManifest,
    /// Total count of findings detected.
    pub findings_count: usize,
    /// Overall and dimension health scores.
    pub health_score: HealthScore,
    /// All detected findings.
    pub findings: Vec<Finding>,
    /// Logical architecture modules.
    pub modules: Vec<Module>,
    /// Additional metadata tags and scan context.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Pull response received from remote Forge Server during `eforge pull`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SyncPullResponse {
    /// Canonical repository identifier.
    pub repo_id: String,
    /// Branch name.
    pub branch: String,
    /// Baseline commit hash on the remote, if any.
    pub baseline_commit_hash: Option<String>,
    /// Baseline overall health score, if any.
    pub baseline_health_score: Option<HealthScore>,
    /// Baseline findings list on the remote.
    #[serde(default)]
    pub baseline_findings: Vec<Finding>,
    /// Baseline BLAKE3 content-addressed hash, if any.
    pub baseline_content_hash: Option<String>,
    /// Synchronization status.
    pub sync_status: SyncStatus,
}

/// Differential summary between current analysis and remote baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeltaSummary {
    /// Number of new findings introduced compared to baseline.
    pub added_findings: usize,
    /// Number of baseline findings that were fixed or removed.
    pub resolved_findings: usize,
    /// Number of findings existing in both current and baseline.
    pub unchanged_findings: usize,
    /// Change in health score (`current.overall - baseline.overall`).
    pub health_score_delta: i16,
    /// Whether the calculated RiskLevel changed between baseline and current.
    pub risk_level_changed: bool,
    /// Whether the changes introduce zero new findings and non-degraded health score.
    pub is_clean: bool,
}

impl DeltaSummary {
    /// Computes deterministic delta summary comparing current findings against baseline findings.
    pub fn compute(
        current_findings: &[Finding],
        baseline_findings: &[Finding],
        current_health: &HealthScore,
        baseline_health: Option<&HealthScore>,
    ) -> Self {
        let baseline_ids: BTreeSet<&str> =
            baseline_findings.iter().map(|f| f.id.as_str()).collect();
        let current_ids: BTreeSet<&str> = current_findings.iter().map(|f| f.id.as_str()).collect();

        let mut added_findings = 0usize;
        let mut unchanged_findings = 0usize;

        for id in &current_ids {
            if baseline_ids.contains(id) {
                unchanged_findings += 1;
            } else {
                added_findings += 1;
            }
        }

        let mut resolved_findings = 0usize;
        for id in &baseline_ids {
            if !current_ids.contains(id) {
                resolved_findings += 1;
            }
        }

        let baseline_overall = baseline_health
            .map(|h| h.overall as i16)
            .unwrap_or(current_health.overall as i16);
        let health_score_delta = current_health.overall as i16 - baseline_overall;

        let current_risk = calculate_change_risk(current_findings, 0, 0, 0);
        let baseline_risk = calculate_change_risk(baseline_findings, 0, 0, 0);
        let risk_level_changed = current_risk.level != baseline_risk.level;

        let is_clean = added_findings == 0 && health_score_delta >= 0;

        Self {
            added_findings,
            resolved_findings,
            unchanged_findings,
            health_score_delta,
            risk_level_changed,
            is_clean,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Confidence, Severity, SourceLocation};

    fn make_sample_finding(id: &str, sev: Severity) -> Finding {
        Finding {
            id: id.to_string(),
            category: "quality".to_string(),
            severity: sev,
            confidence: Confidence::Certain,
            title: format!("Test finding {id}"),
            description: "Test description".to_string(),
            evidence: "Test evidence".to_string(),
            location: Some(SourceLocation {
                file_path: "src/main.rs".to_string(),
                start_line: Some(10),
                end_line: Some(20),
            }),
            recommendation: "Fix it".to_string(),
            rule_id: "test-rule".to_string(),
        }
    }

    #[test]
    fn test_sync_manifest_serde() {
        let manifest = SyncManifest {
            repo_id: "elcode-id/elcode-forge".to_string(),
            repo_name: "elcode-forge".to_string(),
            branch: "develop".to_string(),
            commit_hash: "a1b2c3d4e5f6".to_string(),
            timestamp_secs: 1770000000,
            content_hash: "0123456789abcdef".to_string(),
            tool_version: "0.9.0".to_string(),
        };

        let json = serde_json::to_string(&manifest);
        assert!(json.is_ok());
        if let Ok(json_str) = json {
            let deserialized: Result<SyncManifest, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());
            if let Ok(d) = deserialized {
                assert_eq!(manifest, d);
            }
        }
    }

    #[test]
    fn test_sync_push_payload_serde() {
        let payload = SyncPushPayload {
            manifest: SyncManifest {
                repo_id: "org/repo".to_string(),
                repo_name: "repo".to_string(),
                branch: "main".to_string(),
                commit_hash: "c0ffee".to_string(),
                timestamp_secs: 1770000000,
                content_hash: "feedface".to_string(),
                tool_version: "0.9.0".to_string(),
            },
            findings_count: 1,
            health_score: HealthScore {
                overall: 85,
                dimensions: vec![],
            },
            findings: vec![make_sample_finding("F1", Severity::Medium)],
            modules: vec![],
            metadata: BTreeMap::from([("ci".to_string(), "true".to_string())]),
        };

        let json = serde_json::to_string(&payload);
        assert!(json.is_ok());
        if let Ok(json_str) = json {
            let deserialized: Result<SyncPushPayload, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());
            if let Ok(d) = deserialized {
                assert_eq!(payload, d);
            }
        }
    }

    #[test]
    fn test_delta_summary_computation() {
        let f1 = make_sample_finding("F1", Severity::Low);
        let f2 = make_sample_finding("F2", Severity::High);
        let f3 = make_sample_finding("F3", Severity::Critical);

        let baseline_findings = vec![f1.clone(), f2.clone()];
        let current_findings = vec![f2.clone(), f3.clone()];

        let baseline_health = HealthScore {
            overall: 80,
            dimensions: vec![],
        };
        let current_health = HealthScore {
            overall: 75,
            dimensions: vec![],
        };

        let delta = DeltaSummary::compute(
            &current_findings,
            &baseline_findings,
            &current_health,
            Some(&baseline_health),
        );

        // F3 is added (not in baseline)
        assert_eq!(delta.added_findings, 1);
        // F1 is resolved (in baseline, not in current)
        assert_eq!(delta.resolved_findings, 1);
        // F2 is unchanged (in both)
        assert_eq!(delta.unchanged_findings, 1);
        // Health score delta: 75 - 80 = -5
        assert_eq!(delta.health_score_delta, -5);
        assert!(!delta.is_clean);
    }

    #[test]
    fn test_delta_summary_clean() {
        let f1 = make_sample_finding("F1", Severity::Low);
        let baseline_findings = vec![f1.clone()];
        let current_findings = vec![f1.clone()];

        let health = HealthScore {
            overall: 95,
            dimensions: vec![],
        };
        let delta = DeltaSummary::compute(
            &current_findings,
            &baseline_findings,
            &health,
            Some(&health),
        );

        assert_eq!(delta.added_findings, 0);
        assert_eq!(delta.resolved_findings, 0);
        assert_eq!(delta.unchanged_findings, 1);
        assert_eq!(delta.health_score_delta, 0);
        assert!(delta.is_clean);
    }
}
