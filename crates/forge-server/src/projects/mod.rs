//! Project workspace management and overview summarizer.

use serde::{Deserialize, Serialize};

use crate::db::models::{BranchSummary, HistoryPoint};
use crate::db::Db;
use crate::error::ServerError;
use forge_core::ir::HealthScore;

/// Detailed overview of a registered project workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectOverview {
    pub repo_id: String,
    pub name: String,
    pub branches_count: usize,
    pub branches: Vec<BranchSummary>,
    pub latest_health: Option<HealthScore>,
    pub open_vulnerabilities_count: usize,
    pub critical_vulnerabilities_count: usize,
    pub recent_history: Vec<HistoryPoint>,
    pub updated_at: i64,
}

/// Computes a comprehensive project overview for a repository.
pub async fn get_project_overview(
    db: &Db,
    repo_id: &str,
) -> Result<Option<ProjectOverview>, ServerError> {
    let project = match db.get_project(repo_id).await? {
        Some(p) => p,
        None => return Ok(None),
    };

    let branches = db.get_project_branches(repo_id).await?;
    let branches_count = branches.len();

    // Determine default/primary branch (prefer "main", then "master", then first branch)
    let primary_branch = branches
        .iter()
        .find(|b| b.branch == "main" || b.branch == "master")
        .map(|b| b.branch.as_str())
        .or_else(|| branches.first().map(|b| b.branch.as_str()))
        .unwrap_or("main");

    let latest_snapshot_opt = db.get_latest_snapshot(repo_id, primary_branch).await?;

    let (latest_health, open_vulns, crit_vulns) = match latest_snapshot_opt {
        Some((snapshot_row, finding_rows)) => {
            let health: Option<HealthScore> = serde_json::from_str(&snapshot_row.health_json).ok();
            let mut open = 0usize;
            let mut crit = 0usize;

            for f in &finding_rows {
                let sev = f.severity.to_lowercase();
                let cat = f.category.to_lowercase();
                if sev == "critical" {
                    crit += 1;
                }
                if cat == "security" || cat == "vulnerability" || sev == "critical" || sev == "high"
                {
                    open += 1;
                }
            }

            (health, open, crit)
        }
        None => (None, 0, 0),
    };

    let recent_history = db.get_branch_history(repo_id, primary_branch, 10).await?;

    Ok(Some(ProjectOverview {
        repo_id: project.id,
        name: project.name,
        branches_count,
        branches,
        latest_health,
        open_vulnerabilities_count: open_vulns,
        critical_vulnerabilities_count: crit_vulns,
        recent_history,
        updated_at: project.updated_at,
    }))
}
