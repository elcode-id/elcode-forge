//! Comprehensive End-to-End Multi-Ecosystem Security Integration Test Suite for Phase 8.
//!
//! Validates:
//! 1. OSV Dependency Vulnerability Audit (`eforge security --audit`) with SARIF & JSON exports.
//! 2. Software Bill of Materials (SBOM) generation for CycloneDX v1.5 JSON & SPDX v2.3 JSON.
//! 3. Open-source license compatibility checking and copyleft policy safeguards.
//! 4. Shannon entropy secret scanner with verified credential patterns, path allowlists & mandatory redaction.
//! 5. Execution performance (< 3s).

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::sbom::run_sbom;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::commands::security::run_security;
use forge_cli::exit_code::ExitCode;
use forge_security::sbom::SbomFormat;

#[test]
fn test_security_e2e_vulnerability_and_sarif_pipeline() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let temp_dir = std::env::temp_dir().join("forge_test_security_e2e_vulnerabilities");
    let _ = fs::create_dir_all(&temp_dir);

    // Multi-ecosystem manifests with known vulnerable dependencies
    // 1. npm: lodash 4.17.15 (CVE-2020-8203)
    let package_json = r#"{
      "name": "e2e-security-demo",
      "version": "1.0.0",
      "dependencies": {
        "lodash": "4.17.15"
      }
    }"#;
    fs::write(temp_dir.join("package.json"), package_json)?;

    // 2. Python: requests 2.19.1 (CVE-2018-18074)
    let reqs = "requests==2.19.1\n";
    fs::write(temp_dir.join("requirements.txt"), reqs)?;

    // Run security audit with SARIF output
    let (sarif_output, exit_code) = run_security(&temp_dir, OutputFormat::Sarif, true, true, None)?;

    assert_eq!(exit_code, ExitCode::Success);
    assert!(sarif_output.contains("https://json.schemastore.org/sarif-2.1.0.json"));
    assert!(sarif_output.contains("security/vulnerable-dependency"));
    assert!(sarif_output.contains("CVE-2020-8203"));
    assert!(sarif_output.contains("CVE-2018-18074"));
    println!("SARIF LEN: {}", sarif_output.len());
    println!("SARIF:\n{}", sarif_output);

    // Deterministic output check: successive runs must yield identical text
    let (sarif_second, _) = run_security(&temp_dir, OutputFormat::Sarif, true, true, None)?;
    assert_eq!(sarif_output, sarif_second);

    // Gate enforcement on --fail-on high
    let (_, fail_exit) = run_security(&temp_dir, OutputFormat::Terminal, true, true, Some("high"))?;
    assert_eq!(fail_exit, ExitCode::SecurityPolicyFailure);

    assert!(
        start.elapsed().as_secs() < 3,
        "Vulnerability audit exceeded 3s budget"
    );
    Ok(())
}

#[test]
fn test_security_e2e_sbom_cyclonedx_and_spdx() -> Result<(), Box<dyn Error>> {
    let polyglot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("vulnerable-dependencies");

    // 1. CycloneDX v1.5 JSON
    let cyclonedx_json = run_sbom(&polyglot_dir, SbomFormat::CycloneDx)?;
    assert!(cyclonedx_json.contains("\"bomFormat\": \"CycloneDX\""));
    assert!(cyclonedx_json.contains("\"specVersion\": \"1.5\""));
    assert!(cyclonedx_json.contains("pkg:npm/"));
    assert!(cyclonedx_json.contains("pkg:pypi/"));
    assert!(cyclonedx_json.contains("urn:uuid:"));

    // 2. SPDX v2.3 JSON
    let spdx_json = run_sbom(&polyglot_dir, SbomFormat::Spdx)?;
    assert!(spdx_json.contains("\"spdxVersion\": \"SPDX-2.3\""));
    assert!(spdx_json.contains("\"dataLicense\": \"CC0-1.0\""));
    assert!(spdx_json.contains("SPDXRef-Package-"));
    assert!(spdx_json.contains("PACKAGE-MANAGER"));

    // Determinism test: successive generations yield byte-for-byte identical output
    let cdx_repeat = run_sbom(&polyglot_dir, SbomFormat::CycloneDx)?;
    assert_eq!(cyclonedx_json, cdx_repeat);

    let spdx_repeat = run_sbom(&polyglot_dir, SbomFormat::Spdx)?;
    assert_eq!(spdx_json, spdx_repeat);

    Ok(())
}

#[test]
fn test_security_e2e_entropy_secret_and_allowlist() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_security_e2e_secrets");
    let src_dir = temp_dir.join("src");
    let mock_dir = temp_dir.join("mocks");
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&mock_dir)?;

    // Real production file with unredacted high-entropy secret
    let prod_code = concat!(
        "export const AWS_KEY = \"AKIAIOSFODNN7EXAMPLE\";\n",
        "export const ENCRYPTION_KEY = \"bGFyZ2Utc2VjcmV0LWtleS13aXRoLWxvbmctaGlnaC1lbnRyb3B5LXZhbHVlCg==\";\n"
    );
    fs::write(src_dir.join("auth.ts"), prod_code)?;

    // Mock file placed in allowlisted directory
    let mock_code = concat!(
        "export const MOCK_TOKEN = \"ghp_000000000000000000000000000000000000\";\n",
        "export const MOCK_KEY = \"c2VjcmV0LWtleS1mb3ItbW9jay10ZXN0aW5nLXB1cnBvc2VzLW9ubHkK\";\n"
    );
    fs::write(mock_dir.join("fixtures.ts"), mock_code)?;

    // Configuration declaring allowlist for mocks
    let config_yaml = r#"
version: 1
project_name: "secret-e2e-test"
security:
  allowlist:
    - "mocks/**"
"#;
    fs::write(temp_dir.join("eforge.config.yaml"), config_yaml)?;

    // Scan security
    let (rendered, exit_code) =
        run_security(&temp_dir, OutputFormat::Terminal, false, true, Some("high"))?;

    // Policy should fail due to prod secrets in src/auth.ts
    assert_eq!(exit_code, ExitCode::SecurityPolicyFailure);

    // Mocks in mocks/** must be ignored by allowlist
    assert!(!rendered.contains("mocks/fixtures.ts"));

    // Production secrets in src/auth.ts must be detected and strictly redacted
    assert!(rendered.contains("src/auth.ts"));
    assert!(rendered.contains("****REDACTED****"));
    assert!(!rendered.contains("bGFyZ2Utc2VjcmV0"));
    assert!(!rendered.contains("IOSFODNN7EXAMPLE"));

    Ok(())
}

#[test]
fn test_security_e2e_license_policy_compliance() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_security_e2e_license");
    fs::create_dir_all(&temp_dir)?;

    let package_json = r#"{
      "name": "license-policy-test",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.19.2"
      }
    }"#;
    fs::write(temp_dir.join("package.json"), package_json)?;

    let config_yaml = r#"
version: 1
license:
  disallow_copyleft: true
  allowlist:
    - "MIT"
    - "Apache-2.0"
"#;
    fs::write(temp_dir.join("eforge.config.yaml"), config_yaml)?;

    let scan_opts = ScanOptions {
        repo_root: temp_dir,
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo, exit_code) = execute_scan(&scan_opts)?;
    assert_eq!(exit_code, ExitCode::Success);
    assert!(repo.health.is_some());

    Ok(())
}
