//! ISO/IEC 27001:2022 Information Security Management Controls mapping.

use forge_core::{Dependency, Finding};

use super::{ComplianceClauseResult, ComplianceClauseStatus};

/// Evaluates findings against ISO/IEC 27001:2022 controls.
pub fn evaluate_iso27001_clauses(
    findings: &[Finding],
    _dependencies: &[Dependency],
) -> Vec<ComplianceClauseResult> {
    vec![
        evaluate_control_8_8(findings),
        evaluate_control_8_28(findings),
    ]
}

/// Control 8.8 — Management of Technical Vulnerabilities:
/// Information about technical vulnerabilities of information systems being used shall be obtained,
/// the organization's exposure evaluated, and appropriate measures taken.
fn evaluate_control_8_8(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "security/vulnerable-dependency"
            || f.rule_id.starts_with("osv/")
            || f.category == "vulnerability"
        {
            evidence.push(f.clone());
        }
    }

    let status = if evidence.is_empty() {
        ComplianceClauseStatus::Pass
    } else {
        ComplianceClauseStatus::Fail
    };

    let notes = if evidence.is_empty() {
        "Continuous automated vulnerability scanning active; zero known CVEs or OSV security advisories.".to_string()
    } else {
        format!(
            "Identified {} open technical vulnerability(ies) in software dependencies.",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "8.8".to_string(),
        title: "Management of Technical Vulnerabilities".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// Control 8.28 — Secure Coding:
/// Secure coding principles shall be applied to software development.
fn evaluate_control_8_28(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.category == "security" {
            evidence.push(f.clone());
        }
    }

    let status = if evidence.is_empty() {
        ComplianceClauseStatus::Pass
    } else {
        ComplianceClauseStatus::Fail
    };

    let notes = if evidence.is_empty() {
        "Secure coding standards verified: zero hardcoded secrets, unsafe dynamic executions, or injection patterns.".to_string()
    } else {
        format!(
            "Detected {} security issue(s) violating organizational secure coding standards.",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "8.28".to_string(),
        title: "Secure Coding".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}
