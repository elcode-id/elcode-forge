//! SOC 2 Type II Trust Services Criteria mapping and clause evaluation.

use forge_core::{Dependency, Finding};

use super::{ComplianceClauseResult, ComplianceClauseStatus};

/// Evaluates findings against SOC 2 Type II Trust Services Criteria.
pub fn evaluate_soc2_clauses(
    findings: &[Finding],
    _dependencies: &[Dependency],
) -> Vec<ComplianceClauseResult> {
    vec![
        evaluate_cc6_1(findings),
        evaluate_cc6_6(findings),
        evaluate_cc7_1(findings),
        evaluate_cc8_1(findings),
    ]
}

/// CC6.1 — Logical Access Controls & Credential Protection:
/// Restricts access to sensitive keys, tokens, and secrets within codebases.
fn evaluate_cc6_1(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.category == "security"
            && (f.rule_id == "security/hardcoded-secret"
                || f.rule_id == "security/private-key"
                || f.rule_id == "security/high-entropy-secret"
                || f.rule_id.contains("secret")
                || f.rule_id.contains("private-key"))
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
        "Zero plaintext credentials, hardcoded secrets, or private keys detected in repository source files.".to_string()
    } else {
        format!(
            "Detected {} exposed credential(s) or private key(s) violating access control safeguards.",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "CC6.1".to_string(),
        title: "Logical Access Controls & Secret Protection".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// CC6.6 — Boundary Protection & Interface Controls:
/// Protects system interfaces, enforces boundary policies, and prevents unauthorized remote execution.
fn evaluate_cc6_6(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "security/permissive-cors"
            || f.rule_id.contains("shell-exec")
            || f.rule_id.contains("command-injection")
            || f.rule_id.contains("subprocess")
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
        "No permissive CORS headers, unescaped shell executions, or perimeter boundary risks detected.".to_string()
    } else {
        format!(
            "Detected {} boundary protection risk(s) (permissive CORS or unsanitized shell executions).",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "CC6.6".to_string(),
        title: "Boundary Protection & Interface Hardening".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// CC7.1 — Vulnerability Detection & Remediation Management:
/// Asserts vulnerability scanning and detection in external dependencies and third-party modules.
fn evaluate_cc7_1(findings: &[Finding]) -> ComplianceClauseResult {
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
        "All declared external dependencies are free of known OSV security advisories and CVEs."
            .to_string()
    } else {
        format!(
            "Identified {} open third-party package vulnerability(ies) requiring remediation.",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "CC7.1".to_string(),
        title: "Vulnerability Management & Dependency Auditing".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// CC8.1 — Change & Quality Management:
/// Enforces architectural durability, avoidance of circular import structures, and service test verification.
fn evaluate_cc8_1(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "arch/no-circular-dependencies"
            || f.rule_id == "quality/require-service-tests"
            || (f.category == "architecture" && f.rule_id.contains("circular"))
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
        "Architectural modules satisfy quality change controls with zero circular dependencies and companion test suites.".to_string()
    } else {
        format!(
            "Detected {} change management issue(s) (circular module dependencies or untested service files).",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "CC8.1".to_string(),
        title: "Change Management & Architectural Quality".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}
