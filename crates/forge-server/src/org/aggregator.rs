//! Multi-repository organization aggregator and materialized rollup engine.

use std::collections::{BTreeMap, BTreeSet};

use crate::db::models::{CrossRepoVulnerability, OrganizationRollupRow, OrganizationSummary};
use crate::db::Db;
use crate::error::ServerError;

type VulnerabilityEntry = (String, String, BTreeSet<String>, usize);
type VulnerabilityMap = BTreeMap<(String, String), VulnerabilityEntry>;

/// Computes the materialized rollup for a given organization without persisting it.
pub async fn compute_organization_rollup(
    db: &Db,
    org_id: &str,
) -> Result<OrganizationRollupRow, ServerError> {
    let org = db
        .get_organization(org_id)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("Organization '{org_id}' not found")))?;

    let project_ids = db.get_organization_projects(&org.id).await?;
    let now = chrono::Utc::now().timestamp();

    if project_ids.is_empty() {
        return Ok(OrganizationRollupRow {
            org_id: org.id,
            health_score: 100,
            total_projects: 0,
            total_vulnerabilities: 0,
            matrix_json: "[]".to_string(),
            updated_at: now,
        });
    }

    let mut health_sum: i64 = 0;
    let mut projects_with_snapshots: i64 = 0;

    // Map: (rule_id, title) -> (category, severity, set_of_affected_projects, occurrence_count)
    let mut vuln_map: VulnerabilityMap = BTreeMap::new();

    for proj_id in &project_ids {
        if let Some((snap, findings)) = db.get_latest_snapshot_for_project(proj_id).await? {
            health_sum += snap.health_overall;
            projects_with_snapshots += 1;

            for f in findings {
                let category_lower = f.category.to_lowercase();
                let severity_lower = f.severity.to_lowercase();
                let rule_lower = f.rule_id.to_lowercase();

                let is_security_relevant = category_lower == "security"
                    || category_lower == "vulnerability"
                    || rule_lower.starts_with("security/")
                    || rule_lower.starts_with("vuln/")
                    || severity_lower == "critical"
                    || severity_lower == "high";

                if is_security_relevant {
                    let key = (f.rule_id.clone(), f.title.clone());
                    let entry = vuln_map.entry(key).or_insert_with(|| {
                        (f.category.clone(), f.severity.clone(), BTreeSet::new(), 0)
                    });
                    entry.2.insert(proj_id.clone());
                    entry.3 += 1;
                }
            }
        }
    }

    let health_score = if projects_with_snapshots > 0 {
        (health_sum / projects_with_snapshots).clamp(0, 100)
    } else {
        100
    };

    let mut matrix = Vec::with_capacity(vuln_map.len());
    let mut total_vulnerabilities: i64 = 0;

    for ((rule_id, title), (category, severity, affected_set, count)) in vuln_map {
        total_vulnerabilities += count as i64;
        matrix.push(CrossRepoVulnerability {
            rule_id,
            severity,
            title,
            category,
            affected_projects: affected_set.into_iter().collect(),
            occurrence_count: count,
        });
    }

    // Sort matrix deterministically: multi-repo affected count desc, occurrence desc, rule_id asc
    matrix.sort_by(|a, b| {
        b.affected_projects
            .len()
            .cmp(&a.affected_projects.len())
            .then_with(|| b.occurrence_count.cmp(&a.occurrence_count))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    let matrix_json = serde_json::to_string(&matrix)?;

    Ok(OrganizationRollupRow {
        org_id: org.id,
        health_score,
        total_projects: project_ids.len() as i64,
        total_vulnerabilities,
        matrix_json,
        updated_at: now,
    })
}

/// Recomputes and persists the materialized rollup for an organization.
pub async fn recompute_org_rollup(
    db: &Db,
    org_id: &str,
) -> Result<OrganizationRollupRow, ServerError> {
    let rollup = compute_organization_rollup(db, org_id).await?;
    db.save_organization_rollup(&rollup).await?;
    Ok(rollup)
}

/// Recomputes rollups for all organizations that contain a given project.
pub async fn recompute_all_orgs_for_project(db: &Db, project_id: &str) -> Result<(), ServerError> {
    let org_ids = db.get_organizations_for_project(project_id).await?;
    for org_id in org_ids {
        let _ = recompute_org_rollup(db, &org_id).await?;
    }
    Ok(())
}

/// Retrieves the organization summary in O(1) time reading from the materialized rollup.
/// Falls back to computing the rollup if it has not yet been materialized.
pub async fn get_organization_summary(
    db: &Db,
    id_or_slug: &str,
) -> Result<OrganizationSummary, ServerError> {
    let org = db
        .get_organization(id_or_slug)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("Organization '{id_or_slug}' not found")))?;

    let rollup = match db.get_organization_rollup(&org.id).await? {
        Some(r) => r,
        None => recompute_org_rollup(db, &org.id).await?,
    };

    let cross_repo_vulnerabilities: Vec<CrossRepoVulnerability> =
        serde_json::from_str(&rollup.matrix_json).unwrap_or_default();
    let project_ids = db.get_organization_projects(&org.id).await?;

    Ok(OrganizationSummary {
        id: org.id,
        name: org.name,
        slug: org.slug,
        health_score: rollup.health_score as u8,
        total_projects: rollup.total_projects as usize,
        total_vulnerabilities: rollup.total_vulnerabilities as usize,
        cross_repo_vulnerabilities,
        project_ids,
        updated_at: rollup.updated_at,
    })
}
