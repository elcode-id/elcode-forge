//! License policy checker evaluating copyleft restrictions, allowlists, and denylists.

use std::path::Path;

use forge_core::config::LicenseConfig;
use forge_core::{Confidence, Dependency, Finding, Severity, SourceLocation};

use super::category::{categorize_license, LicenseCategory};
use super::detector::{detect_dependency_license, detect_project_license};

pub const RULE_DISALLOWED_COPYLEFT: &str = "license/disallowed-copyleft";
pub const RULE_DENIED_LICENSE: &str = "license/denied-license";
pub const RULE_UNAPPROVED_LICENSE: &str = "license/unapproved-license";
pub const RULE_MISSING_PROJECT_LICENSE: &str = "license/missing-project-license";

/// Evaluates license compliance policies across project and package dependencies.
pub fn evaluate_license_compliance(
    repo_root: &Path,
    policy: Option<&LicenseConfig>,
    dependencies: &[Dependency],
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Check Project License
    let project_lic = detect_project_license(repo_root);
    match project_lic {
        Some(info) if !info.is_private && info.spdx_id == "UNLICENSED" => {
            findings.push(Finding {
                id: format!("{}-root", RULE_MISSING_PROJECT_LICENSE),
                category: "license".to_string(),
                severity: Severity::Low,
                confidence: Confidence::Certain,
                title: "Missing project license".to_string(),
                description:
                    "The repository does not declare a standard open-source or commercial license."
                        .to_string(),
                evidence: "No LICENSE file or open-source license declared in project manifest."
                    .to_string(),
                location: Some(SourceLocation {
                    file_path: info.source,
                    start_line: Some(1),
                    end_line: Some(1),
                }),
                recommendation:
                    "Add a LICENSE file (e.g. MIT, Apache-2.0) or declare 'private: true' if proprietary."
                        .to_string(),
                rule_id: RULE_MISSING_PROJECT_LICENSE.to_string(),
            });
        }
        None => {
            findings.push(Finding {
                id: format!("{}-root", RULE_MISSING_PROJECT_LICENSE),
                category: "license".to_string(),
                severity: Severity::Low,
                confidence: Confidence::Certain,
                title: "Missing project license".to_string(),
                description:
                    "The repository does not declare a license in a LICENSE file or package manifest."
                        .to_string(),
                evidence: "No LICENSE or manifest license found.".to_string(),
                location: None,
                recommendation:
                    "Add a LICENSE file (e.g. MIT, Apache-2.0) or declare 'private: true' in manifest."
                        .to_string(),
                rule_id: RULE_MISSING_PROJECT_LICENSE.to_string(),
            });
        }
        _ => {}
    }

    let Some(policy) = policy else {
        return findings;
    };

    // 2. Check Dependencies against Policy
    for dep in dependencies {
        let license_str = detect_dependency_license(repo_root, dep);
        let category = categorize_license(&license_str);

        // A. Denylist Check (Highest precedence)
        let is_denied = policy.denylist.iter().any(|d| {
            d.eq_ignore_ascii_case(&license_str) || d.eq_ignore_ascii_case(&category.to_string())
        });

        if is_denied {
            findings.push(Finding {
                id: format!("{}-{}-{}", RULE_DENIED_LICENSE, dep.ecosystem, dep.name),
                category: "license".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: format!(
                    "Prohibited license '{}' in dependency '{}'",
                    license_str, dep.name
                ),
                description: format!(
                    "Dependency '{}' ({} {}) uses prohibited license '{}' explicitly listed in configuration denylist.",
                    dep.name, dep.ecosystem, dep.version_req, license_str
                ),
                evidence: format!(
                    "Package: {} ({}) | License: {} (Category: {:?})",
                    dep.name, dep.ecosystem, license_str, category
                ),
                location: None,
                recommendation: format!(
                    "Replace dependency '{}' with a permissible alternative or update license policy.",
                    dep.name
                ),
                rule_id: RULE_DENIED_LICENSE.to_string(),
            });
            continue;
        }

        // B. Copyleft Policy Check
        if policy.disallow_copyleft && category == LicenseCategory::StrongCopyleft {
            findings.push(Finding {
                id: format!(
                    "{}-{}-{}",
                    RULE_DISALLOWED_COPYLEFT, dep.ecosystem, dep.name
                ),
                category: "license".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: format!(
                    "Disallowed strong copyleft license '{}' in '{}'",
                    license_str, dep.name
                ),
                description: format!(
                    "Dependency '{}' uses a strong copyleft license ('{}') which is disallowed by workspace policy ('disallow_copyleft: true').",
                    dep.name, license_str
                ),
                evidence: format!(
                    "Package: {} ({}) | License: {} (StrongCopyleft)",
                    dep.name, dep.ecosystem, license_str
                ),
                location: None,
                recommendation: format!(
                    "Replace '{}' with a permissively licensed alternative to protect proprietary IP.",
                    dep.name
                ),
                rule_id: RULE_DISALLOWED_COPYLEFT.to_string(),
            });
            continue;
        }

        // C. Allowlist Check
        if !policy.allowlist.is_empty() {
            let is_allowed = policy.allowlist.iter().any(|a| {
                a.eq_ignore_ascii_case(&license_str)
                    || a.eq_ignore_ascii_case(&category.to_string())
            });

            if !is_allowed {
                findings.push(Finding {
                    id: format!("{}-{}-{}", RULE_UNAPPROVED_LICENSE, dep.ecosystem, dep.name),
                    category: "license".to_string(),
                    severity: Severity::Medium,
                    confidence: Confidence::High,
                    title: format!(
                        "Unapproved license '{}' in dependency '{}'",
                        license_str, dep.name
                    ),
                    description: format!(
                        "Dependency '{}' uses license '{}' (Category: {:?}) which is not in the approved license allowlist.",
                        dep.name, license_str, category
                    ),
                    evidence: format!(
                        "Package: {} ({}) | License: {}",
                        dep.name, dep.ecosystem, license_str
                    ),
                    location: None,
                    recommendation: format!(
                        "Add '{}' to the license allowlist if approved by legal, or replace the package.",
                        license_str
                    ),
                    rule_id: RULE_UNAPPROVED_LICENSE.to_string(),
                });
            }
        }
    }

    // Deterministic sorting
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    findings
}
