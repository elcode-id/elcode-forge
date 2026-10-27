use std::error::Error;
use std::fs;

use forge_core::config::parse_config_yaml;
use forge_core::ir::Severity;
use forge_core::policy::{
    glob_path_match, load_policy, parse_policy_yaml, PolicyDocument, PolicyWaiver,
};

#[test]
fn test_policy_document_default() {
    let policy = PolicyDocument::default();
    assert_eq!(policy.version, 0);
    assert_eq!(policy.name, "");
    assert!(policy.organization.is_none());
    assert!(policy.constraints.min_health_score.is_none());
    assert!(policy.waivers.is_empty());
    assert!(policy.overrides.is_empty());
}

#[test]
fn test_parse_policy_yaml_comprehensive() -> Result<(), Box<dyn Error>> {
    let yaml = r#"
name: "enterprise-baseline"
version: 1
organization: "acme-corp"
constraints:
  min_health_score: 85
  min_dimension_scores:
    security: 90
    maintainability: 80
  disallowed_severities:
    - critical
    - high
  forbidden_licenses:
    - "GPL-3.0-only"
    - "AGPL-3.0-or-later"
  max_circular_dependencies: 0
  max_blast_radius_modules: 5
  max_modified_lines_without_tests: 100
  required_rule_packs:
    - "security-extended"
    - "enterprise-governance"
  enforce_secret_zero_tolerance: true
waivers:
  - rule_id: "arch/no-circular-dependencies"
    path_pattern: "legacy/circular/**"
    expires_at: "2027-01-01T00:00:00Z"
    justification: "Legacy module refactor in flight, ticket SEC-402"
overrides:
  - rule_id: "style/explicit-types"
    severity: "info"
    enabled: true
"#;

    let policy = parse_policy_yaml(yaml)?;
    assert_eq!(policy.name, "enterprise-baseline");
    assert_eq!(policy.version, 1);
    assert_eq!(policy.organization.as_deref(), Some("acme-corp"));

    let c = &policy.constraints;
    assert_eq!(c.min_health_score, Some(85));
    assert_eq!(c.min_dimension_scores.get("security"), Some(&90));
    assert_eq!(c.min_dimension_scores.get("maintainability"), Some(&80));
    assert_eq!(
        c.disallowed_severities,
        vec![Severity::Critical, Severity::High]
    );
    assert_eq!(
        c.forbidden_licenses,
        vec!["GPL-3.0-only", "AGPL-3.0-or-later"]
    );
    assert_eq!(c.max_circular_dependencies, Some(0));
    assert_eq!(c.max_blast_radius_modules, Some(5));
    assert_eq!(c.max_modified_lines_without_tests, Some(100));
    assert_eq!(
        c.required_rule_packs,
        vec!["security-extended", "enterprise-governance"]
    );
    assert!(c.enforce_secret_zero_tolerance);

    assert_eq!(policy.waivers.len(), 1);
    let w = &policy.waivers[0];
    assert_eq!(w.rule_id, "arch/no-circular-dependencies");
    assert_eq!(w.path_pattern.as_deref(), Some("legacy/circular/**"));
    assert_eq!(w.expires_at.as_deref(), Some("2027-01-01T00:00:00Z"));
    assert_eq!(
        w.justification,
        "Legacy module refactor in flight, ticket SEC-402"
    );

    assert_eq!(policy.overrides.len(), 1);
    let o = &policy.overrides[0];
    assert_eq!(o.rule_id, "style/explicit-types");
    assert_eq!(o.severity, Some(Severity::Info));
    assert!(o.enabled);
    Ok(())
}

#[test]
fn test_policy_in_forge_config_yaml() -> Result<(), Box<dyn Error>> {
    let yaml = r#"
version: 1
project_name: "test-monorepo"

policy:
  name: "monorepo-gate"
  version: 2
  organization: "elcode"
  constraints:
    min_health_score: 90
    enforce_secret_zero_tolerance: true
  waivers:
    - rule_id: "security/hardcoded-secret"
      path_pattern: "fixtures/**"
      justification: "Test fixture mock keys"
"#;

    let config = parse_config_yaml(yaml)?;
    let policy = match config.policy {
        Some(p) => p,
        None => return Err("Config missing policy".into()),
    };
    assert_eq!(policy.name, "monorepo-gate");
    assert_eq!(policy.version, 2);
    assert_eq!(policy.organization.as_deref(), Some("elcode"));
    assert_eq!(policy.constraints.min_health_score, Some(90));
    assert!(policy.constraints.enforce_secret_zero_tolerance);

    assert_eq!(policy.waivers.len(), 1);
    assert_eq!(policy.waivers[0].rule_id, "security/hardcoded-secret");
    assert_eq!(
        policy.waivers[0].path_pattern.as_deref(),
        Some("fixtures/**")
    );
    assert_eq!(policy.waivers[0].justification, "Test fixture mock keys");
    Ok(())
}

#[test]
fn test_waiver_expiration() -> Result<(), Box<dyn Error>> {
    let fixed_now =
        chrono::DateTime::parse_from_rfc3339("2026-10-27T12:00:00Z")?.with_timezone(&chrono::Utc);

    // 1. Future expiration
    let future_waiver = PolicyWaiver {
        rule_id: "test-rule".to_string(),
        path_pattern: None,
        expires_at: Some("2026-12-31T23:59:59Z".to_string()),
        justification: "Future waiver".to_string(),
    };
    assert!(!future_waiver.is_expired_at(fixed_now));

    // 2. Past expiration
    let past_waiver = PolicyWaiver {
        rule_id: "test-rule".to_string(),
        path_pattern: None,
        expires_at: Some("2026-01-01T00:00:00Z".to_string()),
        justification: "Expired waiver".to_string(),
    };
    assert!(past_waiver.is_expired_at(fixed_now));

    // 3. Permanent waiver (no expires_at)
    let perm_waiver = PolicyWaiver {
        rule_id: "test-rule".to_string(),
        path_pattern: None,
        expires_at: None,
        justification: "Permanent waiver".to_string(),
    };
    assert!(!perm_waiver.is_expired_at(fixed_now));

    // 4. Invalid date string (treated as expired for safety)
    let invalid_waiver = PolicyWaiver {
        rule_id: "test-rule".to_string(),
        path_pattern: None,
        expires_at: Some("not-a-date".to_string()),
        justification: "Invalid waiver".to_string(),
    };
    assert!(invalid_waiver.is_expired_at(fixed_now));
    Ok(())
}

#[test]
fn test_waiver_path_matching() {
    let waiver = PolicyWaiver {
        rule_id: "arch/no-circular-deps".to_string(),
        path_pattern: Some("legacy/modules/**".to_string()),
        expires_at: None,
        justification: "Legacy refactor".to_string(),
    };

    assert!(waiver.matches("arch/no-circular-deps", Some("legacy/modules/sub/file.ts")));
    assert!(waiver.matches("arch/no-circular-deps", Some("legacy/modules/file.ts")));
    assert!(!waiver.matches("arch/no-circular-deps", Some("src/components/button.tsx")));
    assert!(!waiver.matches("other-rule", Some("legacy/modules/file.ts")));
    assert!(!waiver.matches("arch/no-circular-deps", None));

    let star_waiver = PolicyWaiver {
        rule_id: "*".to_string(),
        path_pattern: Some("fixtures/*".to_string()),
        expires_at: None,
        justification: "Any fixture issue".to_string(),
    };
    assert!(star_waiver.matches("any-rule", Some("fixtures/test.json")));
    assert!(!star_waiver.matches("any-rule", Some("fixtures/sub/nested.json")));
}

#[test]
fn test_glob_path_match() {
    assert!(glob_path_match("*", "anything.rs"));
    assert!(glob_path_match("**", "a/b/c/d.rs"));
    assert!(glob_path_match("src/**", "src/lib.rs"));
    assert!(glob_path_match("src/**", "src/sub/mod.rs"));
    assert!(!glob_path_match("src/**", "tests/lib.rs"));
    assert!(glob_path_match("**/test.rs", "crates/forge/test.rs"));
    assert!(glob_path_match("crates/*", "crates/forge-core"));
    assert!(!glob_path_match("crates/*", "crates/forge-core/src/lib.rs"));
}

#[test]
fn test_load_policy_file() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_policy_load");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;

    let yaml_path = temp_dir.join("policy.yaml");
    fs::write(
        &yaml_path,
        r#"
name: "file-loaded-policy"
version: 1
constraints:
  min_health_score: 95
"#,
    )?;

    let loaded_yaml = load_policy(&yaml_path)?;
    assert_eq!(loaded_yaml.name, "file-loaded-policy");
    assert_eq!(loaded_yaml.constraints.min_health_score, Some(95));

    let json_path = temp_dir.join("policy.json");
    fs::write(
        &json_path,
        r#"{"name": "json-policy", "version": 1, "constraints": {"min_health_score": 80}}"#,
    )?;

    let loaded_json = load_policy(&json_path)?;
    assert_eq!(loaded_json.name, "json-policy");
    assert_eq!(loaded_json.constraints.min_health_score, Some(80));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
