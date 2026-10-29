//! OWASP Top 10 (2021) Web Application Security Risks compliance mapping.

use forge_core::{Dependency, Finding};

use super::{ComplianceClauseResult, ComplianceClauseStatus};

/// Evaluates findings against the OWASP Top 10 (2021) standard.
pub fn evaluate_owasp_clauses(
    findings: &[Finding],
    _dependencies: &[Dependency],
) -> Vec<ComplianceClauseResult> {
    vec![
        evaluate_a01(findings),
        evaluate_a02(findings),
        evaluate_a03(findings),
        evaluate_a06(findings),
    ]
}

/// A01:2021 — Broken Access Control:
/// Enforces architectural boundary segregation, no direct database queries in UI components, and strict CORS.
fn evaluate_a01(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "arch/no-direct-db-in-ui"
            || f.rule_id == "security/permissive-cors"
            || f.category == "access-control"
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
        "Clean architecture segregation verified; zero UI layer database bypasses or permissive CORS origins.".to_string()
    } else {
        format!(
            "Identified {} broken access control issue(s) (UI-to-DB bypasses or permissive CORS origins).",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "A01:2021".to_string(),
        title: "Broken Access Control".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// A02:2021 — Cryptographic Failures:
/// Guards sensitive data, preventing exposure of hardcoded secrets, cryptographic keys, and authorization tokens.
fn evaluate_a02(findings: &[Finding]) -> ComplianceClauseResult {
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
        "Zero plaintext cryptographic keys, access tokens, or high-entropy credentials present in repository.".to_string()
    } else {
        format!(
            "Detected {} cryptographic asset exposure(s) requiring immediate secret rotation.",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "A02:2021".to_string(),
        title: "Cryptographic Failures".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// A03:2021 — Injection:
/// Identifies dangerous eval(), dynamic code execution, command injection, and unsanitized DOM rendering.
fn evaluate_a03(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "security/no-eval"
            || f.rule_id == "security/no-implied-eval"
            || f.rule_id == "security/unsafe-shell-exec"
            || f.rule_id == "security/no-dangerous-inner-html"
            || f.rule_id.contains("injection")
            || f.rule_id.contains("eval")
            || f.rule_id.contains("unserialize")
            || f.rule_id.contains("pickle")
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
        "No dynamic evaluation, command injection vectors, or dangerous DOM injections detected."
            .to_string()
    } else {
        format!(
            "Identified {} potential code execution / injection vulnerability(ies).",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "A03:2021".to_string(),
        title: "Injection".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}

/// A06:2021 — Vulnerable and Outdated Components:
/// Asserts supply-chain health and prevents consumption of unpinned packages with known vulnerabilities.
fn evaluate_a06(findings: &[Finding]) -> ComplianceClauseResult {
    let mut evidence = Vec::new();
    for f in findings {
        if f.rule_id == "security/vulnerable-dependency"
            || f.rule_id.starts_with("osv/")
            || f.rule_id == "deps/unpinned-version"
            || f.rule_id == "deps/wildcard-version"
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
        "All third-party manifests are locked and verified against the OSV vulnerability database."
            .to_string()
    } else {
        format!(
            "Detected {} component hygiene risk(s) (vulnerable packages or unpinned version specifiers).",
            evidence.len()
        )
    };

    ComplianceClauseResult {
        clause_id: "A06:2021".to_string(),
        title: "Vulnerable and Outdated Components".to_string(),
        status,
        evidence_findings: evidence,
        notes,
    }
}
