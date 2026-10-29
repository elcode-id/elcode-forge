use std::error::Error;

use forge_core::ir::{Confidence, Finding, Severity, SourceLocation};
use forge_security::compliance::{
    evaluate_compliance, render_compliance_markdown, render_compliance_terminal,
    ComplianceClauseStatus, ComplianceStandard,
};

fn make_test_finding(rule_id: &str, category: &str, title: &str) -> Finding {
    Finding {
        id: format!("test-{rule_id}"),
        rule_id: rule_id.to_string(),
        category: category.to_string(),
        severity: Severity::High,
        confidence: Confidence::Certain,
        title: title.to_string(),
        description: "Test description".to_string(),
        evidence: "Test evidence".to_string(),
        location: Some(SourceLocation {
            file_path: "src/auth.ts".to_string(),
            start_line: Some(42),
            end_line: Some(45),
        }),
        recommendation: "Fix recommendation".to_string(),
    }
}

#[test]
fn test_soc2_compliance_clean_repository() {
    let report = evaluate_compliance(ComplianceStandard::Soc2, &[], &[]);
    assert!(report.overall_passed);
    assert_eq!(report.clauses.len(), 4);
    for clause in &report.clauses {
        assert_eq!(clause.status, ComplianceClauseStatus::Pass);
        assert!(clause.evidence_findings.is_empty());
    }
}

#[test]
fn test_soc2_compliance_secret_and_vulnerability_failure() -> Result<(), Box<dyn Error>> {
    let f1 = make_test_finding(
        "security/hardcoded-secret",
        "security",
        "AWS Secret Key Detected",
    );
    let f2 = make_test_finding(
        "security/vulnerable-dependency",
        "vulnerability",
        "Lodash Prototype Pollution",
    );

    let report = evaluate_compliance(ComplianceStandard::Soc2, &[f1, f2], &[]);
    assert!(!report.overall_passed);

    let cc6_1 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "CC6.1")
        .ok_or("CC6.1 must exist")?;
    assert_eq!(cc6_1.status, ComplianceClauseStatus::Fail);
    assert_eq!(cc6_1.evidence_findings.len(), 1);

    let cc7_1 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "CC7.1")
        .ok_or("CC7.1 must exist")?;
    assert_eq!(cc7_1.status, ComplianceClauseStatus::Fail);
    assert_eq!(cc7_1.evidence_findings.len(), 1);

    let cc6_6 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "CC6.6")
        .ok_or("CC6.6 must exist")?;
    assert_eq!(cc6_6.status, ComplianceClauseStatus::Pass);
    Ok(())
}

#[test]
fn test_owasp_compliance_injection_failure() -> Result<(), Box<dyn Error>> {
    let eval_finding = make_test_finding("security/no-eval", "security", "Use of eval() detected");
    let report = evaluate_compliance(ComplianceStandard::Owasp, &[eval_finding], &[]);
    assert!(!report.overall_passed);

    let a03 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "A03:2021")
        .ok_or("A03 must exist")?;
    assert_eq!(a03.status, ComplianceClauseStatus::Fail);
    assert_eq!(a03.evidence_findings.len(), 1);

    let a01 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "A01:2021")
        .ok_or("A01 must exist")?;
    assert_eq!(a01.status, ComplianceClauseStatus::Pass);
    Ok(())
}

#[test]
fn test_iso27001_compliance_controls() -> Result<(), Box<dyn Error>> {
    let vuln_finding = make_test_finding(
        "security/vulnerable-dependency",
        "vulnerability",
        "CVE-2023-12345 in libxml",
    );

    let report = evaluate_compliance(ComplianceStandard::Iso27001, &[vuln_finding], &[]);
    assert!(!report.overall_passed);
    assert_eq!(report.clauses.len(), 2);

    let c8_8 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "8.8")
        .ok_or("8.8 must exist")?;
    assert_eq!(c8_8.status, ComplianceClauseStatus::Fail);
    assert_eq!(c8_8.evidence_findings.len(), 1);
    Ok(())
}

#[test]
fn test_compliance_report_serialization_and_rendering() -> Result<(), Box<dyn Error>> {
    let f = make_test_finding(
        "security/hardcoded-secret",
        "security",
        "API Key Exposed in Config",
    );
    let report = evaluate_compliance(ComplianceStandard::Soc2, &[f], &[]);

    // JSON round-trip
    let json_str = serde_json::to_string_pretty(&report)?;
    assert!(json_str.contains("\"standard\": \"soc2\""));
    assert!(json_str.contains("\"overall_passed\": false"));

    // Markdown rendering
    let md = render_compliance_markdown(&report);
    assert!(md.contains("# Compliance Audit Report: SOC 2 Type II"));
    assert!(md.contains("**FAILED**"));
    assert!(md.contains("| `CC6.1` |"));
    assert!(md.contains("## Non-Compliant Clause Findings"));
    assert!(md.contains("API Key Exposed in Config"));

    // Terminal rendering
    let term = render_compliance_terminal(&report);
    assert!(term.contains("ELCODE Forge Compliance Audit — SOC 2 Type II"));
    assert!(term.contains("Status:            FAILED"));
    assert!(term.contains("[FAIL] CC6.1"));

    Ok(())
}
