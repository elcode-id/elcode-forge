//! Automated enterprise compliance auditing and framework mapping (SOC 2, OWASP Top 10, ISO 27001).

use std::fmt;

use forge_core::{Dependency, Finding};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod iso27001;
pub mod owasp;
pub mod soc2;

/// Supported enterprise compliance and regulatory frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ComplianceStandard {
    /// AICPA SOC 2 Type II Trust Services Criteria
    Soc2,
    /// OWASP Top 10 (2021) Web Application Security Risks
    Owasp,
    /// ISO/IEC 27001:2022 Information Security Management Controls
    Iso27001,
}

impl fmt::Display for ComplianceStandard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Soc2 => write!(f, "SOC 2 Type II"),
            Self::Owasp => write!(f, "OWASP Top 10 (2021)"),
            Self::Iso27001 => write!(f, "ISO/IEC 27001:2022"),
        }
    }
}

/// Audit verification status for an individual compliance clause or control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ComplianceClauseStatus {
    /// Control requirements fully satisfied; zero non-compliant findings detected.
    Pass,
    /// Control requirement violated by one or more active security/architectural findings.
    Fail,
    /// Control requires external human or policy review.
    NeedsReview,
}

/// Structured outcome and evidence for an evaluated compliance clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ComplianceClauseResult {
    pub clause_id: String,
    pub title: String,
    pub status: ComplianceClauseStatus,
    #[serde(default)]
    pub evidence_findings: Vec<Finding>,
    pub notes: String,
}

/// Consolidated compliance audit report produced by evaluating repository findings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ComplianceReport {
    pub standard: ComplianceStandard,
    pub overall_passed: bool,
    pub clauses: Vec<ComplianceClauseResult>,
    pub generated_at: String,
}

/// Evaluates repository findings and dependency metadata against the specified compliance standard.
pub fn evaluate_compliance(
    standard: ComplianceStandard,
    findings: &[Finding],
    dependencies: &[Dependency],
) -> ComplianceReport {
    let clauses = match standard {
        ComplianceStandard::Soc2 => soc2::evaluate_soc2_clauses(findings, dependencies),
        ComplianceStandard::Owasp => owasp::evaluate_owasp_clauses(findings, dependencies),
        ComplianceStandard::Iso27001 => iso27001::evaluate_iso27001_clauses(findings, dependencies),
    };

    let overall_passed = clauses
        .iter()
        .all(|c| matches!(c.status, ComplianceClauseStatus::Pass));
    let generated_at = chrono::Utc::now().to_rfc3339();

    ComplianceReport {
        standard,
        overall_passed,
        clauses,
        generated_at,
    }
}

/// Renders a [`ComplianceReport`] into human-readable GitHub-flavored Markdown.
pub fn render_compliance_markdown(report: &ComplianceReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# Compliance Audit Report: {}\n\n",
        report.standard
    ));
    let verdict_str = if report.overall_passed {
        "**PASSED** :white_check_mark:"
    } else {
        "**FAILED** :x:"
    };

    out.push_str(&format!("- **Overall Audit Status:** {}\n", verdict_str));
    out.push_str(&format!("- **Audit Framework:** {}\n", report.standard));
    out.push_str(&format!("- **Generated At:** {}\n\n", report.generated_at));

    out.push_str("## Control Clause Evaluation\n\n");
    out.push_str("| Clause | Control Title | Status | Issues | Notes |\n");
    out.push_str("| :--- | :--- | :--- | :---: | :--- |\n");

    for c in &report.clauses {
        let status_badge = match c.status {
            ComplianceClauseStatus::Pass => ":white_check_mark: Pass",
            ComplianceClauseStatus::Fail => ":x: Fail",
            ComplianceClauseStatus::NeedsReview => ":warning: Review",
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            c.clause_id,
            c.title,
            status_badge,
            c.evidence_findings.len(),
            c.notes
        ));
    }
    out.push('\n');

    let failed_clauses: Vec<_> = report
        .clauses
        .iter()
        .filter(|c| matches!(c.status, ComplianceClauseStatus::Fail))
        .collect();

    if !failed_clauses.is_empty() {
        out.push_str("## Non-Compliant Clause Findings\n\n");
        for c in failed_clauses {
            out.push_str(&format!("### Clause {} — {}\n\n", c.clause_id, c.title));
            for f in &c.evidence_findings {
                let loc = f
                    .location
                    .as_ref()
                    .map(|l| format!("{}:{}", l.file_path, l.start_line.unwrap_or(1)))
                    .unwrap_or_else(|| "-".to_string());
                out.push_str(&format!(
                    "- **[{:?}]** {} (`{}`) — {}\n",
                    f.severity, f.title, loc, f.rule_id
                ));
            }
            out.push('\n');
        }
    }

    out
}

/// Renders a [`ComplianceReport`] into plain terminal text.
pub fn render_compliance_terminal(report: &ComplianceReport) -> String {
    let mut out = String::new();

    out.push_str("============================================================\n");
    out.push_str(&format!(
        " ELCODE Forge Compliance Audit — {}\n",
        report.standard
    ));
    out.push_str("============================================================\n");

    let status_str = if report.overall_passed {
        "PASSED"
    } else {
        "FAILED"
    };
    out.push_str(&format!("Status:            {}\n", status_str));
    out.push_str(&format!("Generated At:      {}\n\n", report.generated_at));

    out.push_str("Control Evaluation:\n");
    for c in &report.clauses {
        let mark = match c.status {
            ComplianceClauseStatus::Pass => "[PASS]",
            ComplianceClauseStatus::Fail => "[FAIL]",
            ComplianceClauseStatus::NeedsReview => "[REVW]",
        };
        out.push_str(&format!(
            "  {} {:<8} {} (issues: {})\n",
            mark,
            c.clause_id,
            c.title,
            c.evidence_findings.len()
        ));
        out.push_str(&format!("         {}\n", c.notes));
    }
    out.push('\n');

    out
}
