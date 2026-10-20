//! End-to-end integration tests for custom declarative rules, rule packs,
//! and schema verification against `fixtures/custom-rules-app`.

use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::report::{build_report_bundle, ReportOptions};
use forge_cli::commands::rules::{run_rules_check, run_rules_list};
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_core::{ForgeError, Severity};

fn fixture_custom_rules_app() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("custom-rules-app")
}

#[test]
fn test_custom_rules_list_resolves_all_rules_and_packs() -> Result<(), ForgeError> {
    let fixture = fixture_custom_rules_app();

    // 1. Unfiltered rules list
    let report = run_rules_list(&fixture, None, false)?;

    assert_eq!(report.extended_packs, vec!["recommended".to_string()]);
    assert_eq!(report.total_rules, 9);
    assert_eq!(report.enabled_rules, 9);

    // Verify all 5 custom rules are registered
    let rule_ids: Vec<&str> = report.rules.iter().map(|r| r.id.as_str()).collect();
    assert!(rule_ids.contains(&"arch/no-db-in-ui"));
    assert!(rule_ids.contains(&"quality/require-service-tests"));
    assert!(rule_ids.contains(&"quality/controller-naming-convention"));
    assert!(rule_ids.contains(&"security/no-eval-dynamic-code"));
    assert!(rule_ids.contains(&"deps/disallow-unpinned-and-deprecated"));

    // Verify deterministic sorting: Critical first, then High, etc.
    assert_eq!(report.rules[0].severity, Severity::Critical);

    // 2. Filter by category "architecture"
    let arch_report = run_rules_list(&fixture, Some("architecture"), false)?;
    assert!(arch_report
        .rules
        .iter()
        .all(|r| r.category == "architecture"));
    assert!(arch_report.rules.iter().any(|r| r.id == "arch/no-db-in-ui"));

    // 3. Filter by category "security"
    let sec_report = run_rules_list(&fixture, Some("security"), false)?;
    assert!(sec_report.rules.iter().all(|r| r.category == "security"));
    assert!(sec_report
        .rules
        .iter()
        .any(|r| r.id == "security/no-eval-dynamic-code"));

    Ok(())
}

#[test]
fn test_custom_rules_check_validates_syntax() -> Result<(), ForgeError> {
    let fixture = fixture_custom_rules_app();
    let rules_yaml = fixture.join(".eforge/rules/custom-arch.yaml");

    let check = run_rules_check(&fixture, Some(&rules_yaml))?;

    assert_eq!(check.total_files_checked, 1);
    assert_eq!(check.valid_files, 1);
    assert_eq!(check.invalid_files, 0);
    assert_eq!(check.results.len(), 1);
    assert_eq!(check.results[0].rules_count, 5);
    assert!(check.results[0].errors.is_empty());

    Ok(())
}

#[test]
fn test_custom_rules_scan_detects_all_declarative_violations() -> Result<(), ForgeError> {
    let fixture = fixture_custom_rules_app();
    let scan_opts = ScanOptions {
        repo_root: fixture,
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo, _) = execute_scan(&scan_opts)?;

    // 1. Forbidden Imports matcher: UI importing DB
    let arch_import_finding = repo.findings.iter().find(|f| {
        f.rule_id == "arch/no-db-in-ui"
            || f.id.contains("DECL-IMPORT-ARCH_NO_DB_IN_UI-SRC_UI_SRC_DB")
    });
    assert!(
        arch_import_finding.is_some(),
        "arch/no-db-in-ui violation must be detected"
    );
    if let Some(arch_finding) = arch_import_finding {
        assert_eq!(arch_finding.severity, Severity::Critical);
    }

    // 2. Required Companion matcher: BillingService missing test
    let companion_finding = repo
        .findings
        .iter()
        .find(|f| f.id.contains("BILLINGSERVICE"));
    assert!(
        companion_finding.is_some(),
        "Missing companion test for BillingService must be detected"
    );

    // 3. File Naming matcher: order_handler.ts not *Controller.ts
    let naming_finding = repo
        .findings
        .iter()
        .find(|f| f.rule_id == "quality/controller-naming-convention");
    assert!(
        naming_finding.is_some(),
        "Naming convention violation on order_handler.ts must be detected"
    );
    if let Some(naming) = naming_finding {
        let file_path = naming.location.as_ref().map(|loc| loc.file_path.as_str());
        assert_eq!(file_path, Some("src/controllers/order_handler.ts"));
    }

    // 4. Regex Pattern matcher: eval() in calculator.ts
    let eval_finding = repo
        .findings
        .iter()
        .find(|f| f.rule_id == "security/no-eval-dynamic-code");
    assert!(
        eval_finding.is_some(),
        "Regex pattern eval() violation in calculator.ts must be detected"
    );
    if let Some(finding) = eval_finding {
        assert_eq!(finding.severity, Severity::Critical);
    }

    // 5. Dependency Restriction matcher: left-pad and wildcard unpinned express
    let dep_findings: Vec<&forge_core::Finding> = repo
        .findings
        .iter()
        .filter(|f| f.rule_id == "deps/disallow-unpinned-and-deprecated")
        .collect();
    assert!(
        dep_findings.len() >= 2,
        "Dependency restrictions must detect deprecated package and unpinned version"
    );
    assert!(dep_findings.iter().any(|f| f.id.contains("LEFT_PAD")));
    assert!(dep_findings.iter().any(|f| f.id.contains("EXPRESS")));

    // 6. False Positive check: UserService.ts has test, UserController.ts matches pattern
    assert!(
        !repo
            .findings
            .iter()
            .any(|f| f.id.contains("DECL-COMPANION") && f.id.contains("USERSERVICE")),
        "UserService has tests/UserService.test.ts, must not produce companion violation"
    );
    assert!(
        !repo
            .findings
            .iter()
            .any(|f| f.id.contains("DECL-NAMING") && f.id.contains("USERCONTROLLER")),
        "UserController matches *Controller.ts pattern, must not produce naming violation"
    );

    Ok(())
}

#[test]
fn test_custom_rules_report_bundle_serialization_and_determinism() -> Result<(), ForgeError> {
    let fixture = fixture_custom_rules_app();
    let opts = ReportOptions {
        hotspots_top_n: 10,
        include_git: false,
    };

    let bundle1 = build_report_bundle(&fixture, &opts)?;
    let bundle2 = build_report_bundle(&fixture, &opts)?;

    let json1 = serde_json::to_string_pretty(&bundle1).map_err(|e| ForgeError::Serialization {
        message: e.to_string(),
    })?;
    let json2 = serde_json::to_string_pretty(&bundle2).map_err(|e| ForgeError::Serialization {
        message: e.to_string(),
    })?;

    assert_eq!(
        json1, json2,
        "ReportBundle for custom-rules-app must be byte-for-byte deterministic"
    );
    assert_eq!(bundle1.rules.len(), 9);
    assert!(bundle1
        .rules
        .iter()
        .any(|r| r.id == "deps/disallow-unpinned-and-deprecated"));

    Ok(())
}
