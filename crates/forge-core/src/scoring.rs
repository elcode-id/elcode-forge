use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ir::{DimensionScore, Finding, HealthScore, Repository, Severity};

/// Categorical risk level for a pull request or working-tree change set.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Explainable change risk assessment (distinct from overall project health).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RiskReport {
    pub level: RiskLevel,
    pub score: u8,
    pub reasons: Vec<String>,
}

const DIMENSIONS: &[(&str, &str, u8)] = &[
    ("Architecture", "architecture", 20),
    ("Security", "security", 20),
    ("Maintainability", "maintainability", 15),
    ("Testing", "testing", 15),
    ("Dependencies", "dependencies", 10),
    ("Documentation", "documentation", 10),
    ("Performance", "performance", 5),
    ("CI/CD", "ci_cd", 5),
];

/// Calculates a deterministic, transparent 8-dimension [`HealthScore`] for `repo`.
///
/// Every dimension includes explicit human-readable `reasons` explaining all deductions and signals.
pub fn calculate_health_score(repo: &Repository) -> HealthScore {
    let mut dimensions = Vec::with_capacity(DIMENSIONS.len());
    let mut weighted_sum: u32 = 0;

    let has_test_files = repo.files.iter().any(|f| f.is_test);
    let has_docs = repo.files.iter().any(|f| {
        let lower = f.relative_path.to_ascii_lowercase();
        lower == "readme.md" || lower.starts_with("docs/") || lower.ends_with(".md")
    });
    let has_ci = repo
        .files
        .iter()
        .any(|f| f.relative_path.starts_with(".github/workflows/"));

    for &(label, category_key, weight_pct) in DIMENSIONS {
        let mut score: i32 = 100;
        let mut reasons = Vec::new();

        let matching: Vec<&Finding> = repo
            .findings
            .iter()
            .filter(|f| f.category.eq_ignore_ascii_case(category_key))
            .collect();

        for finding in &matching {
            score -= severity_deduction(finding.severity);
        }

        if !matching.is_empty() {
            reasons.push(format!(
                "{} {} finding(s) detected",
                matching.len(),
                category_key
            ));
        }

        match category_key {
            "testing" => {
                if !has_test_files && !repo.files.is_empty() {
                    score -= 15;
                    reasons.push("No test files (*.test.*, *.spec.*) detected".to_string());
                } else if has_test_files {
                    reasons.push("Test suite files detected in repository".to_string());
                }
            }
            "documentation" => {
                if !has_docs && !repo.files.is_empty() {
                    score -= 20;
                    reasons.push("No README.md or documentation files detected".to_string());
                } else if has_docs {
                    reasons.push("Markdown documentation detected".to_string());
                }
            }
            "ci_cd" => {
                if !has_ci && !repo.files.is_empty() {
                    score -= 10;
                    reasons.push("No .github/workflows CI configuration detected".to_string());
                } else if has_ci {
                    reasons.push("GitHub Actions workflow detected".to_string());
                }
            }
            _ => {}
        }

        if reasons.is_empty() {
            reasons.push(format!("0 {category_key} issues detected"));
        }

        let clamped = score.clamp(0, 100) as u8;
        weighted_sum += u32::from(clamped) * u32::from(weight_pct);

        dimensions.push(DimensionScore {
            dimension: label.to_string(),
            score: clamped,
            weight_pct,
            reasons,
        });
    }

    let overall = ((weighted_sum + 50) / 100).min(100) as u8;

    HealthScore {
        overall,
        dimensions,
    }
}

/// Calculates a deterministic [`RiskReport`] for a Git diff or PR change set.
pub fn calculate_change_risk(
    findings: &[Finding],
    changed_files_count: usize,
    affected_modules_count: usize,
    untested_changes_count: usize,
) -> RiskReport {
    let mut points: u32 = 0;
    let mut reasons = Vec::new();
    let mut has_critical_finding = false;

    for f in findings {
        match f.severity {
            Severity::Critical => {
                has_critical_finding = true;
                points += 40;
            }
            Severity::High => points += 20,
            Severity::Medium => points += 10,
            Severity::Low => points += 4,
            Severity::Info => {}
        }
    }

    if !findings.is_empty() {
        reasons.push(format!(
            "{} finding(s) introduced or present in affected scope",
            findings.len()
        ));
    }

    if changed_files_count > 0 {
        points += (changed_files_count as u32).saturating_mul(2).min(20);
        reasons.push(format!("{changed_files_count} file(s) modified"));
    }

    if affected_modules_count > 0 {
        points += (affected_modules_count as u32).saturating_mul(5).min(25);
        reasons.push(format!(
            "{affected_modules_count} module(s) in blast radius"
        ));
    }

    if untested_changes_count > 0 {
        points += (untested_changes_count as u32).saturating_mul(8).min(30);
        reasons.push(format!(
            "{untested_changes_count} changed source file(s) without matching tests"
        ));
    }

    if reasons.is_empty() {
        reasons.push("No risky changes or findings detected".to_string());
    }

    let score = points.min(100) as u8;
    let level = if has_critical_finding || score >= 75 {
        RiskLevel::Critical
    } else if score >= 50 {
        RiskLevel::High
    } else if score >= 25 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    RiskReport {
        level,
        score,
        reasons,
    }
}

fn severity_deduction(severity: Severity) -> i32 {
    match severity {
        Severity::Critical => 20,
        Severity::High => 10,
        Severity::Medium => 5,
        Severity::Low => 2,
        Severity::Info => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Confidence, FileNode};

    #[test]
    fn calculates_explainable_eight_dimension_health_score() {
        let repo = Repository {
            root_path: "/workspace/app".to_string(),
            name: "app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec!["nextjs".to_string()],
            package_managers: vec!["npm".to_string()],
            files: vec![
                FileNode {
                    relative_path: "README.md".to_string(),
                    language: None,
                    size_bytes: 100,
                    line_count: 10,
                    content_hash: "h1".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: ".github/workflows/ci.yml".to_string(),
                    language: None,
                    size_bytes: 100,
                    line_count: 20,
                    content_hash: "h2".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/user.test.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 15,
                    content_hash: "h3".to_string(),
                    is_test: true,
                },
            ],
            modules: vec![],
            dependencies: vec![],
            findings: vec![Finding {
                id: "ARCH-1".to_string(),
                category: "architecture".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Layer violation".to_string(),
                description: "UI imports DB".to_string(),
                evidence: "src/components -> src/db".to_string(),
                location: None,
                recommendation: "Use service layer".to_string(),
                rule_id: "arch/no-direct-db-in-ui".to_string(),
            }],
            health: None,
        };

        let health = calculate_health_score(&repo);
        assert_eq!(health.dimensions.len(), 8);
        // Architecture lost 10 points (90 * 20% = 18), all others 100 -> overall = 98
        assert_eq!(health.overall, 98);
        assert!(health.dimensions.iter().all(|d| !d.reasons.is_empty()));
    }

    #[test]
    fn calculates_change_risk_levels_and_reasons_deterministically() {
        let low = calculate_change_risk(&[], 1, 1, 0);
        assert_eq!(low.level, RiskLevel::Low);

        let critical_finding = Finding {
            id: "SEC-1".to_string(),
            category: "security".to_string(),
            severity: Severity::Critical,
            confidence: Confidence::Certain,
            title: "Secret leaked".to_string(),
            description: "API key".to_string(),
            evidence: "sk-proj-****REDACTED****".to_string(),
            location: None,
            recommendation: "Rotate".to_string(),
            rule_id: "security/hardcoded-secret".to_string(),
        };

        let crit = calculate_change_risk(&[critical_finding], 2, 2, 1);
        assert_eq!(crit.level, RiskLevel::Critical);
        assert!(!crit.reasons.is_empty());
    }
}
