//! Pure Policy-as-Code gate evaluation engine operating on normalized IR and impact summaries.

use std::collections::BTreeMap;

use forge_core::ir::{Dependency, Finding, HealthScore, Severity};
use forge_core::policy::{PolicyDocument, PolicyImpactSummary, PolicyReport, PolicyVerdict};

/// Pure gate evaluator that asserts all organization policy constraints against repository state.
///
/// Evaluates:
/// - Overall health score threshold
/// - Per-dimension health score thresholds
/// - Disallowed finding severities (taking into account active waivers and overrides)
/// - Secret zero-tolerance enforcement
/// - Forbidden open-source license detection
/// - Circular dependency limits
/// - Blast-radius module change thresholds
/// - Untested modified code line thresholds
/// - Required rule pack configurations
pub fn evaluate_policy(
    policy: &PolicyDocument,
    findings: &[Finding],
    health: &HealthScore,
    impact: Option<&PolicyImpactSummary>,
    _dependencies: &[Dependency],
    configured_rule_packs: &[String],
) -> PolicyReport {
    let mut passed_checks = Vec::new();
    let mut warnings = Vec::new();
    let mut violations = Vec::new();

    // 1. Partition waivers into active vs expired
    let now = chrono::Utc::now();
    let mut active_waivers = Vec::new();

    for waiver in &policy.waivers {
        if waiver.is_expired_at(now) {
            warnings.push(format!(
                "Waiver for rule '{}' expired on {} and is no longer valid",
                waiver.rule_id,
                waiver.expires_at.as_deref().unwrap_or("unknown date")
            ));
        } else {
            active_waivers.push(waiver.clone());
        }
    }

    // 2. Build override lookups
    let mut override_map: BTreeMap<&str, (bool, Option<Severity>)> = BTreeMap::new();
    for over in &policy.overrides {
        override_map.insert(over.rule_id.as_str(), (over.enabled, over.severity));
    }

    // 3. Filter findings: apply overrides and active waivers
    let mut effective_findings: Vec<(&Finding, Severity)> = Vec::new();
    let mut waived_count = 0;

    for finding in findings {
        // Check rule override: if disabled, skip finding entirely
        if let Some(&(enabled, severity_override)) = override_map.get(finding.rule_id.as_str()) {
            if !enabled {
                continue;
            }
            let effective_sev = severity_override.unwrap_or(finding.severity);

            // Check if waived
            let file_path = finding.location.as_ref().map(|l| l.file_path.as_str());
            if active_waivers
                .iter()
                .any(|w| w.matches(&finding.rule_id, file_path))
            {
                waived_count += 1;
                continue;
            }
            effective_findings.push((finding, effective_sev));
        } else {
            let file_path = finding.location.as_ref().map(|l| l.file_path.as_str());
            if active_waivers
                .iter()
                .any(|w| w.matches(&finding.rule_id, file_path))
            {
                waived_count += 1;
                continue;
            }
            effective_findings.push((finding, finding.severity));
        }
    }

    if waived_count > 0 {
        warnings.push(format!(
            "Policy evaluated with {} finding(s) waived by active waivers",
            waived_count
        ));
    }

    // 4. Assert overall health score constraint
    if let Some(min_health) = policy.constraints.min_health_score {
        if health.overall < min_health {
            violations.push(format!(
                "Overall health score {} is below required minimum threshold ({})",
                health.overall, min_health
            ));
        } else {
            passed_checks.push(format!(
                "Overall health score {} meets required minimum threshold ({})",
                health.overall, min_health
            ));
        }
    }

    // 5. Assert per-dimension health score constraints
    for (dim_name, min_score) in &policy.constraints.min_dimension_scores {
        let matched = health
            .dimensions
            .iter()
            .find(|d| d.dimension.eq_ignore_ascii_case(dim_name));
        match matched {
            Some(dim) => {
                if dim.score < *min_score {
                    violations.push(format!(
                        "Dimension '{}' score {} is below required minimum threshold ({})",
                        dim.dimension, dim.score, min_score
                    ));
                } else {
                    passed_checks.push(format!(
                        "Dimension '{}' score {} meets required minimum threshold ({})",
                        dim.dimension, dim.score, min_score
                    ));
                }
            }
            None => {
                violations.push(format!(
                    "Dimension '{}' was not found in calculated health score",
                    dim_name
                ));
            }
        }
    }

    // 6. Assert disallowed severities constraint
    if !policy.constraints.disallowed_severities.is_empty() {
        let mut disallowed_found = 0;
        for (finding, sev) in &effective_findings {
            if policy.constraints.disallowed_severities.contains(sev) {
                disallowed_found += 1;
                let loc_str = finding
                    .location
                    .as_ref()
                    .map(|l| format!(" at {}:{}", l.file_path, l.start_line.unwrap_or(1)))
                    .unwrap_or_default();
                violations.push(format!(
                    "Disallowed {:?} finding detected: [{}] {}{}",
                    sev, finding.rule_id, finding.title, loc_str
                ));
            }
        }
        if disallowed_found == 0 {
            passed_checks.push(format!(
                "No active findings with disallowed severities ({:?})",
                policy.constraints.disallowed_severities
            ));
        }
    }

    // 7. Assert secret zero-tolerance constraint
    if policy.constraints.enforce_secret_zero_tolerance {
        let mut secret_count = 0;
        for (finding, _) in &effective_findings {
            let is_secret = finding.category == "security"
                && (finding.rule_id == "security/hardcoded-secret"
                    || finding.rule_id.contains("secret")
                    || finding.rule_id.contains("private-key"));
            if is_secret {
                secret_count += 1;
                violations.push(format!(
                    "Zero tolerance violation: secret detected in [{}] {}",
                    finding.rule_id, finding.title
                ));
            }
        }
        if secret_count == 0 {
            passed_checks.push("Zero tolerance check: zero hardcoded secrets detected".to_string());
        }
    }

    // 8. Assert forbidden licenses constraint
    if !policy.constraints.forbidden_licenses.is_empty() {
        let mut forbidden_found = 0;
        for (finding, _) in &effective_findings {
            let is_lic_finding = finding.category == "license"
                || finding.rule_id.starts_with("license/")
                || finding.rule_id.contains("license");

            if is_lic_finding {
                for forbidden in &policy.constraints.forbidden_licenses {
                    let forbidden_lower = forbidden.to_ascii_lowercase();
                    let matches_evidence = finding
                        .evidence
                        .to_ascii_lowercase()
                        .contains(&forbidden_lower);
                    let matches_title = finding
                        .title
                        .to_ascii_lowercase()
                        .contains(&forbidden_lower);
                    let matches_desc = finding
                        .description
                        .to_ascii_lowercase()
                        .contains(&forbidden_lower);

                    if matches_evidence || matches_title || matches_desc {
                        forbidden_found += 1;
                        violations.push(format!(
                            "Forbidden license '{}' detected: [{}] {}",
                            forbidden, finding.rule_id, finding.title
                        ));
                    }
                }
            }
        }
        if forbidden_found == 0 {
            passed_checks.push(format!(
                "No forbidden licenses detected ({})",
                policy.constraints.forbidden_licenses.join(", ")
            ));
        }
    }

    // 9. Assert circular dependencies constraint
    if let Some(max_cycles) = policy.constraints.max_circular_dependencies {
        let cycle_count = effective_findings
            .iter()
            .filter(|(f, _)| {
                f.rule_id == "arch/no-circular-dependencies"
                    || (f.category == "architecture" && f.rule_id.contains("circular"))
            })
            .count();

        if cycle_count > max_cycles {
            violations.push(format!(
                "Circular dependencies count ({}) exceeds allowed maximum ({})",
                cycle_count, max_cycles
            ));
        } else {
            passed_checks.push(format!(
                "Circular dependencies count ({}) is within allowed limit ({})",
                cycle_count, max_cycles
            ));
        }
    }

    // 10. Assert blast-radius module constraint
    if let Some(max_modules) = policy.constraints.max_blast_radius_modules {
        if let Some(imp) = impact {
            if imp.affected_module_count > max_modules {
                violations.push(format!(
                    "Blast-radius affected module count ({}) exceeds allowed maximum ({})",
                    imp.affected_module_count, max_modules
                ));
            } else {
                passed_checks.push(format!(
                    "Blast-radius affected module count ({}) is within allowed limit ({})",
                    imp.affected_module_count, max_modules
                ));
            }
        }
    }

    // 11. Assert modified lines without tests constraint
    if let Some(max_untested_lines) = policy.constraints.max_modified_lines_without_tests {
        if let Some(imp) = impact {
            let untested_lines = if imp.untested_modified_lines > 0 {
                imp.untested_modified_lines
            } else if imp.untested_changed_files > 0 {
                imp.total_modified_lines
            } else {
                0
            };

            if untested_lines > max_untested_lines {
                violations.push(format!(
                    "Modified lines without tests ({}) exceeds allowed maximum ({})",
                    untested_lines, max_untested_lines
                ));
            } else {
                passed_checks.push(format!(
                    "Modified lines without tests ({}) is within allowed limit ({})",
                    untested_lines, max_untested_lines
                ));
            }
        }
    }

    // 12. Assert required rule packs constraint
    for pack in &policy.constraints.required_rule_packs {
        let is_configured = configured_rule_packs
            .iter()
            .any(|p| p.eq_ignore_ascii_case(pack));
        if is_configured {
            passed_checks.push(format!("Required rule pack '{}' is configured", pack));
        } else {
            violations.push(format!("Required rule pack '{}' is not configured", pack));
        }
    }

    // 13. Determine final policy verdict
    let verdict = if !violations.is_empty() {
        PolicyVerdict::Violation
    } else if !warnings.is_empty() {
        PolicyVerdict::Warning
    } else {
        PolicyVerdict::Passed
    };

    PolicyReport {
        verdict,
        policy_name: policy.name.clone(),
        passed_checks,
        warnings,
        violations,
        active_waivers,
    }
}
