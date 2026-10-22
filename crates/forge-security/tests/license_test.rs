use std::error::Error;
use std::fs;

use forge_core::config::LicenseConfig;
use forge_core::Dependency;
use forge_security::license::{
    categorize_license, detect_dependency_license, detect_project_license,
    evaluate_license_compliance, LicenseCategory, RULE_DENIED_LICENSE, RULE_DISALLOWED_COPYLEFT,
    RULE_UNAPPROVED_LICENSE,
};

#[test]
fn test_license_categorization() {
    assert_eq!(categorize_license("MIT"), LicenseCategory::Permissive);
    assert_eq!(
        categorize_license("Apache-2.0"),
        LicenseCategory::Permissive
    );
    assert_eq!(
        categorize_license("BSD-2-Clause"),
        LicenseCategory::Permissive
    );
    assert_eq!(
        categorize_license("BSD-3-Clause"),
        LicenseCategory::Permissive
    );
    assert_eq!(categorize_license("ISC"), LicenseCategory::Permissive);
    assert_eq!(categorize_license("Unlicense"), LicenseCategory::Permissive);
    assert_eq!(categorize_license("CC0-1.0"), LicenseCategory::Permissive);

    assert_eq!(
        categorize_license("LGPL-2.1"),
        LicenseCategory::WeakCopyleft
    );
    assert_eq!(
        categorize_license("LGPL-3.0"),
        LicenseCategory::WeakCopyleft
    );
    assert_eq!(categorize_license("MPL-2.0"), LicenseCategory::WeakCopyleft);
    assert_eq!(categorize_license("EPL-2.0"), LicenseCategory::WeakCopyleft);

    assert_eq!(
        categorize_license("GPL-2.0"),
        LicenseCategory::StrongCopyleft
    );
    assert_eq!(
        categorize_license("GPL-3.0"),
        LicenseCategory::StrongCopyleft
    );
    assert_eq!(
        categorize_license("AGPL-3.0"),
        LicenseCategory::StrongCopyleft
    );
    assert_eq!(
        categorize_license("SSPL-1.0"),
        LicenseCategory::StrongCopyleft
    );

    assert_eq!(
        categorize_license("UNLICENSED"),
        LicenseCategory::Proprietary
    );
    assert_eq!(
        categorize_license("Proprietary"),
        LicenseCategory::Proprietary
    );
    assert_eq!(
        categorize_license("Unknown-Custom"),
        LicenseCategory::Unknown
    );
}

#[test]
fn test_detect_project_license_from_files_and_manifests() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_license_detect");
    let _ = fs::create_dir_all(&temp_dir);

    // 1. Initially no license
    let none_found = detect_project_license(&temp_dir);
    assert!(none_found.is_none());

    // 2. Add LICENSE file with MIT text
    let mit_file = temp_dir.join("LICENSE");
    fs::write(
        &mit_file,
        "MIT License\n\nPermission is hereby granted, free of charge, to any person obtaining a copy...",
    )?;
    let found_mit = detect_project_license(&temp_dir);
    let Some(info) = found_mit else {
        return Err("Expected to detect MIT license from LICENSE file".into());
    };
    assert_eq!(info.spdx_id, "MIT");
    assert_eq!(info.source, "LICENSE");
    assert!(!info.is_private);

    // 3. Remove LICENSE file, add package.json with Apache-2.0
    fs::remove_file(&mit_file)?;
    let pkg_json = temp_dir.join("package.json");
    fs::write(
        &pkg_json,
        r#"{"name": "test-pkg", "license": "Apache-2.0"}"#,
    )?;
    let found_pkg = detect_project_license(&temp_dir);
    let Some(info_pkg) = found_pkg else {
        return Err("Expected to detect Apache-2.0 from package.json".into());
    };
    assert_eq!(info_pkg.spdx_id, "Apache-2.0");
    assert_eq!(info_pkg.source, "package.json");
    assert!(!info_pkg.is_private);

    // 4. Test private package
    fs::write(&pkg_json, r#"{"name": "test-pkg", "private": true}"#)?;
    let found_private = detect_project_license(&temp_dir);
    let Some(info_priv) = found_private else {
        return Err("Expected to detect private package".into());
    };
    assert_eq!(info_priv.spdx_id, "UNLICENSED");
    assert!(info_priv.is_private);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_license_policy_evaluation() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_license_policy");
    let _ = fs::create_dir_all(&temp_dir);

    // Create a LICENSE file so missing project license is not flagged
    fs::write(
        temp_dir.join("LICENSE"),
        "MIT License\n\nPermission is hereby granted...",
    )?;

    // Create dummy node_modules to test dependency license detection
    let lodash_pkg = temp_dir.join("node_modules").join("lodash");
    fs::create_dir_all(&lodash_pkg)?;
    fs::write(
        lodash_pkg.join("package.json"),
        r#"{"name": "lodash", "version": "4.17.21", "license": "MIT"}"#,
    )?;

    let gpl_pkg = temp_dir.join("node_modules").join("strict-gpl-pkg");
    fs::create_dir_all(&gpl_pkg)?;
    fs::write(
        gpl_pkg.join("package.json"),
        r#"{"name": "strict-gpl-pkg", "version": "1.0.0", "license": "GPL-3.0"}"#,
    )?;

    let agpl_pkg = temp_dir.join("node_modules").join("network-agpl-pkg");
    fs::create_dir_all(&agpl_pkg)?;
    fs::write(
        agpl_pkg.join("package.json"),
        r#"{"name": "network-agpl-pkg", "version": "2.0.0", "license": "AGPL-3.0"}"#,
    )?;

    let bsd_pkg = temp_dir.join("node_modules").join("some-bsd-pkg");
    fs::create_dir_all(&bsd_pkg)?;
    fs::write(
        bsd_pkg.join("package.json"),
        r#"{"name": "some-bsd-pkg", "version": "1.0.0", "license": "BSD-3-Clause"}"#,
    )?;

    let deps = vec![
        Dependency {
            name: "lodash".to_string(),
            version_req: "^4.17.21".to_string(),
            resolved_version: Some("4.17.21".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
        Dependency {
            name: "strict-gpl-pkg".to_string(),
            version_req: "1.0.0".to_string(),
            resolved_version: Some("1.0.0".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
        Dependency {
            name: "network-agpl-pkg".to_string(),
            version_req: "2.0.0".to_string(),
            resolved_version: Some("2.0.0".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
        Dependency {
            name: "some-bsd-pkg".to_string(),
            version_req: "1.0.0".to_string(),
            resolved_version: Some("1.0.0".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
    ];

    // Verify detector picks up licenses from node_modules
    assert_eq!(detect_dependency_license(&temp_dir, &deps[0]), "MIT");
    assert_eq!(detect_dependency_license(&temp_dir, &deps[1]), "GPL-3.0");
    assert_eq!(detect_dependency_license(&temp_dir, &deps[2]), "AGPL-3.0");
    assert_eq!(
        detect_dependency_license(&temp_dir, &deps[3]),
        "BSD-3-Clause"
    );

    // 1. Policy: disallow_copyleft = true
    let copyleft_policy = LicenseConfig {
        disallow_copyleft: true,
        allowlist: Vec::new(),
        denylist: Vec::new(),
    };
    let copyleft_findings = evaluate_license_compliance(&temp_dir, Some(&copyleft_policy), &deps);
    let copyleft_rule_ids: Vec<&str> = copyleft_findings
        .iter()
        .map(|f| f.rule_id.as_str())
        .collect();
    assert!(copyleft_rule_ids.contains(&RULE_DISALLOWED_COPYLEFT));
    assert_eq!(
        copyleft_findings
            .iter()
            .filter(|f| f.rule_id == RULE_DISALLOWED_COPYLEFT)
            .count(),
        2 // strict-gpl-pkg and network-agpl-pkg
    );

    // 2. Policy: denylist = ["AGPL-3.0"]
    let denylist_policy = LicenseConfig {
        disallow_copyleft: false,
        allowlist: Vec::new(),
        denylist: vec!["AGPL-3.0".to_string()],
    };
    let denylist_findings = evaluate_license_compliance(&temp_dir, Some(&denylist_policy), &deps);
    assert_eq!(
        denylist_findings
            .iter()
            .filter(|f| f.rule_id == RULE_DENIED_LICENSE)
            .count(),
        1
    );
    assert!(denylist_findings
        .iter()
        .any(|f| f.rule_id == RULE_DENIED_LICENSE && f.evidence.contains("network-agpl-pkg")));

    // 3. Policy: allowlist = ["MIT", "Apache-2.0"]
    let allowlist_policy = LicenseConfig {
        disallow_copyleft: false,
        allowlist: vec!["MIT".to_string(), "Apache-2.0".to_string()],
        denylist: Vec::new(),
    };
    let allowlist_findings = evaluate_license_compliance(&temp_dir, Some(&allowlist_policy), &deps);
    let unapproved: Vec<&str> = allowlist_findings
        .iter()
        .filter(|f| f.rule_id == RULE_UNAPPROVED_LICENSE)
        .map(|f| f.evidence.as_str())
        .collect();

    // strict-gpl-pkg, network-agpl-pkg, and some-bsd-pkg should be unapproved; lodash (MIT) is approved
    assert_eq!(unapproved.len(), 3);
    assert!(unapproved.iter().any(|e| e.contains("some-bsd-pkg")));
    assert!(!unapproved.iter().any(|e| e.contains("lodash")));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
