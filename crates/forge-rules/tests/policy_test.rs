use std::collections::BTreeMap;

use forge_core::ir::{Confidence, DimensionScore, Finding, HealthScore, Severity, SourceLocation};
use forge_core::policy::{
    PolicyConstraints, PolicyDocument, PolicyImpactSummary, PolicyRuleOverride, PolicyVerdict,
    PolicyWaiver,
};
use forge_rules::policy_engine::evaluate_policy;

fn make_sample_health_score(overall: u8, security: u8, maintainability: u8) -> HealthScore {
    HealthScore {
        overall,
        dimensions: vec![
            DimensionScore {
                dimension: "security".to_string(),
                score: security,
                weight_pct: 20,
                reasons: Vec::new(),
            },
            DimensionScore {
                dimension: "maintainability".to_string(),
                score: maintainability,
                weight_pct: 20,
                reasons: Vec::new(),
            },
            DimensionScore {
                dimension: "architecture".to_string(),
                score: 90,
                weight_pct: 20,
                reasons: Vec::new(),
            },
        ],
    }
}

fn make_finding(
    id: &str,
    rule_id: &str,
    category: &str,
    severity: Severity,
    path: &str,
) -> Finding {
    Finding {
        id: id.to_string(),
        rule_id: rule_id.to_string(),
        category: category.to_string(),
        severity,
        confidence: Confidence::Certain,
        title: format!("Finding title for {rule_id}"),
        description: format!("Description for {rule_id}"),
        evidence: format!("Evidence for {rule_id}"),
        location: Some(SourceLocation {
            file_path: path.to_string(),
            start_line: Some(10),
            end_line: Some(15),
        }),
        recommendation: "Fix it".to_string(),
    }
}

#[test]
fn test_evaluate_policy_all_passed() {
    let mut min_dim = BTreeMap::new();
    min_dim.insert("security".to_string(), 85);
    min_dim.insert("maintainability".to_string(), 80);

    let policy = PolicyDocument {
        name: "clean-gate".to_string(),
        version: 1,
        organization: Some("elcode".to_string()),
        constraints: PolicyConstraints {
            min_health_score: Some(85),
            min_dimension_scores: min_dim,
            disallowed_severities: vec![Severity::Critical, Severity::High],
            forbidden_licenses: vec!["GPL-3.0-only".to_string()],
            max_circular_dependencies: Some(0),
            max_blast_radius_modules: Some(5),
            max_modified_lines_without_tests: Some(100),
            required_rule_packs: vec!["recommended".to_string()],
            enforce_secret_zero_tolerance: true,
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let health = make_sample_health_score(90, 95, 88);
    let findings: Vec<Finding> = Vec::new();
    let impact = PolicyImpactSummary {
        affected_module_count: 2,
        untested_changed_files: 0,
        total_modified_lines: 50,
        untested_modified_lines: 0,
    };
    let configured_packs = vec!["recommended".to_string()];

    let report = evaluate_policy(
        &policy,
        &findings,
        &health,
        Some(&impact),
        &[],
        &configured_packs,
    );

    assert_eq!(report.verdict, PolicyVerdict::Passed);
    assert!(report.is_passed());
    assert!(!report.is_violation());
    assert!(report.violations.is_empty());
    assert!(report.passed_checks.len() >= 7);
}

#[test]
fn test_evaluate_policy_health_score_failure() {
    let policy = PolicyDocument {
        name: "strict-health".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            min_health_score: Some(90),
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let health = make_sample_health_score(82, 85, 80);
    let report = evaluate_policy(&policy, &[], &health, None, &[], &[]);

    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report.is_violation());
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Overall health score 82 is below required minimum")));
}

#[test]
fn test_evaluate_policy_dimension_score_failure() {
    let mut min_dim = BTreeMap::new();
    min_dim.insert("security".to_string(), 90);

    let policy = PolicyDocument {
        name: "security-gate".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            min_dimension_scores: min_dim,
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let health = make_sample_health_score(92, 75, 95);
    let report = evaluate_policy(&policy, &[], &health, None, &[], &[]);

    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Dimension 'security' score 75 is below required minimum")));
}

#[test]
fn test_evaluate_policy_disallowed_severity_and_waiver() {
    let policy_without_waiver = PolicyDocument {
        name: "disallowed-sev".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            disallowed_severities: vec![Severity::High, Severity::Critical],
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let finding = make_finding(
        "F1",
        "arch/no-circular-deps",
        "architecture",
        Severity::High,
        "legacy/mod/a.ts",
    );
    let health = make_sample_health_score(90, 90, 90);

    // 1. Without waiver -> Violation
    let report1 = evaluate_policy(
        &policy_without_waiver,
        std::slice::from_ref(&finding),
        &health,
        None,
        &[],
        &[],
    );
    assert_eq!(report1.verdict, PolicyVerdict::Violation);
    assert!(report1
        .violations
        .iter()
        .any(|v| v.contains("Disallowed High finding detected")));

    // 2. With active waiver -> Warning (finding waived)
    let policy_with_waiver = PolicyDocument {
        name: "disallowed-sev".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            disallowed_severities: vec![Severity::High, Severity::Critical],
            ..Default::default()
        },
        waivers: vec![PolicyWaiver {
            rule_id: "arch/no-circular-deps".to_string(),
            path_pattern: Some("legacy/**".to_string()),
            expires_at: Some("2030-01-01T00:00:00Z".to_string()),
            justification: "Temporary architectural exemption".to_string(),
        }],
        overrides: Vec::new(),
    };

    let report2 = evaluate_policy(
        &policy_with_waiver,
        std::slice::from_ref(&finding),
        &health,
        None,
        &[],
        &[],
    );
    assert_eq!(report2.verdict, PolicyVerdict::Warning);
    assert!(report2.is_warning());
    assert!(report2.violations.is_empty());
    assert!(report2
        .warnings
        .iter()
        .any(|w| w.contains("waived by active waivers")));

    // 3. With expired waiver -> Violation
    let policy_with_expired_waiver = PolicyDocument {
        name: "disallowed-sev".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            disallowed_severities: vec![Severity::High, Severity::Critical],
            ..Default::default()
        },
        waivers: vec![PolicyWaiver {
            rule_id: "arch/no-circular-deps".to_string(),
            path_pattern: Some("legacy/**".to_string()),
            expires_at: Some("2020-01-01T00:00:00Z".to_string()),
            justification: "Expired exemption".to_string(),
        }],
        overrides: Vec::new(),
    };

    let report3 = evaluate_policy(
        &policy_with_expired_waiver,
        &[finding],
        &health,
        None,
        &[],
        &[],
    );
    assert_eq!(report3.verdict, PolicyVerdict::Violation);
    assert!(report3
        .warnings
        .iter()
        .any(|w| w.contains("expired on 2020-01-01T00:00:00Z")));
}

#[test]
fn test_evaluate_policy_rule_override() {
    let policy = PolicyDocument {
        name: "override-test".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            disallowed_severities: vec![Severity::Critical],
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: vec![
            // Downgrade critical finding to info
            PolicyRuleOverride {
                rule_id: "downgraded-rule".to_string(),
                severity: Some(Severity::Info),
                enabled: true,
            },
            // Disable suppressed rule entirely
            PolicyRuleOverride {
                rule_id: "disabled-rule".to_string(),
                severity: None,
                enabled: false,
            },
        ],
    };

    let f1 = make_finding(
        "F1",
        "downgraded-rule",
        "security",
        Severity::Critical,
        "src/index.ts",
    );
    let f2 = make_finding(
        "F2",
        "disabled-rule",
        "security",
        Severity::Critical,
        "src/auth.ts",
    );
    let health = make_sample_health_score(90, 90, 90);

    let report = evaluate_policy(&policy, &[f1, f2], &health, None, &[], &[]);
    assert_eq!(report.verdict, PolicyVerdict::Passed);
    assert!(report.violations.is_empty());
}

#[test]
fn test_evaluate_policy_secret_zero_tolerance() {
    let policy = PolicyDocument {
        name: "secret-check".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            enforce_secret_zero_tolerance: true,
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let secret_finding = make_finding(
        "F1",
        "security/hardcoded-secret",
        "security",
        Severity::High,
        "src/config.ts",
    );
    let health = make_sample_health_score(90, 90, 90);

    let report = evaluate_policy(&policy, &[secret_finding], &health, None, &[], &[]);
    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Zero tolerance violation: secret detected")));
}

#[test]
fn test_evaluate_policy_forbidden_licenses() {
    let policy = PolicyDocument {
        name: "license-gate".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            forbidden_licenses: vec!["GPL-3.0".to_string()],
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let mut license_finding = make_finding(
        "F1",
        "license/disallowed-copyleft",
        "license",
        Severity::High,
        "package.json",
    );
    license_finding.evidence = "Dependency foo uses GPL-3.0 license".to_string();
    let health = make_sample_health_score(90, 90, 90);

    let report = evaluate_policy(&policy, &[license_finding], &health, None, &[], &[]);
    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Forbidden license 'GPL-3.0' detected")));
}

#[test]
fn test_evaluate_policy_blast_radius_and_untested_lines() {
    let policy = PolicyDocument {
        name: "change-safety".to_string(),
        version: 1,
        organization: None,
        constraints: PolicyConstraints {
            max_blast_radius_modules: Some(3),
            max_modified_lines_without_tests: Some(50),
            ..Default::default()
        },
        waivers: Vec::new(),
        overrides: Vec::new(),
    };

    let health = make_sample_health_score(90, 90, 90);
    let bad_impact = PolicyImpactSummary {
        affected_module_count: 5,
        untested_changed_files: 2,
        total_modified_lines: 100,
        untested_modified_lines: 75,
    };

    let report = evaluate_policy(&policy, &[], &health, Some(&bad_impact), &[], &[]);
    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Blast-radius affected module count (5) exceeds allowed maximum (3)")));
    assert!(report
        .violations
        .iter()
        .any(|v| v.contains("Modified lines without tests (75) exceeds allowed maximum (50)")));
}
