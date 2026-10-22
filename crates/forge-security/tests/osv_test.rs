use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::{Dependency, Severity};
use forge_security::{
    normalize_ecosystem, normalize_package_name, SecurityScanner, Version, VulnerabilityMatcher,
    RULE_VULNERABLE_DEPENDENCY,
};

#[test]
fn test_version_parsing_and_ordering() {
    let v_standard = Version::parse("1.2.3");
    let v_with_prefix = Version::parse("v1.2.3");
    let v_with_build = Version::parse("1.2.3+build42");
    let v_prerelease = Version::parse("1.2.3-rc.1");

    assert_eq!(v_standard, v_with_prefix);
    assert_eq!(v_standard, v_with_build);
    assert!(v_prerelease < v_standard);

    let v_short = Version::parse("2.20");
    let v_long = Version::parse("2.20.0");
    assert_eq!(v_short, v_long);

    let v_higher = Version::parse("2.20.1");
    assert!(v_short < v_higher);
}

#[test]
fn test_ecosystem_and_package_normalization() {
    assert_eq!(normalize_ecosystem("npm"), "npm");
    assert_eq!(normalize_ecosystem("Node"), "npm");
    assert_eq!(normalize_ecosystem("PyPI"), "pypi");
    assert_eq!(normalize_ecosystem("pip"), "pypi");
    assert_eq!(normalize_ecosystem("Go"), "go");
    assert_eq!(normalize_ecosystem("crates.io"), "cargo");
    assert_eq!(normalize_ecosystem("Packagist"), "composer");

    // PyPI underscore normalization
    assert_eq!(
        normalize_package_name("pypi", "my_vulnerable_pkg"),
        "my-vulnerable-pkg"
    );
    // Go scheme stripping
    assert_eq!(
        normalize_package_name("go", "https://github.com/gin-gonic/gin"),
        "github.com/gin-gonic/gin"
    );
}

#[test]
fn test_polyglot_embedded_vulnerability_matching() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_osv_polyglot");
    let _ = fs::create_dir_all(&temp_dir);

    let matcher = VulnerabilityMatcher::load(&temp_dir);

    let deps = vec![
        // 1. npm: lodash (vulnerable < 4.17.19)
        Dependency {
            name: "lodash".to_string(),
            version_req: "^4.17.15".to_string(),
            resolved_version: Some("4.17.15".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
        // 2. npm: safe lodash
        Dependency {
            name: "lodash-safe".to_string(),
            version_req: "^4.17.21".to_string(),
            resolved_version: Some("4.17.21".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        },
        // 3. PyPI: requests (vulnerable < 2.20.0)
        Dependency {
            name: "requests".to_string(),
            version_req: "==2.19.1".to_string(),
            resolved_version: Some("2.19.1".to_string()),
            ecosystem: "pypi".to_string(),
            is_dev: false,
            is_direct: true,
        },
        // 4. Go: gin (vulnerable < 1.7.0)
        Dependency {
            name: "github.com/gin-gonic/gin".to_string(),
            version_req: "v1.6.0".to_string(),
            resolved_version: Some("v1.6.0".to_string()),
            ecosystem: "go".to_string(),
            is_dev: false,
            is_direct: true,
        },
        // 5. Cargo: log (vulnerable < 0.4.14)
        Dependency {
            name: "log".to_string(),
            version_req: "0.4.10".to_string(),
            resolved_version: Some("0.4.10".to_string()),
            ecosystem: "cargo".to_string(),
            is_dev: false,
            is_direct: true,
        },
        // 6. Composer: guzzlehttp/guzzle (vulnerable < 7.4.5)
        Dependency {
            name: "guzzlehttp/guzzle".to_string(),
            version_req: "^7.4.0".to_string(),
            resolved_version: Some("7.4.0".to_string()),
            ecosystem: "composer".to_string(),
            is_dev: false,
            is_direct: true,
        },
    ];

    let findings = matcher.match_dependencies(&deps);

    // 5 vulnerable packages matched
    assert_eq!(findings.len(), 5);

    for f in &findings {
        assert_eq!(f.rule_id, RULE_VULNERABLE_DEPENDENCY);
        assert_eq!(f.category, "security");
        assert!(f.severity >= Severity::Medium);
    }

    // Verify specific CVEs in evidence
    let evidence_blob: String = findings.iter().map(|f| f.evidence.as_str()).collect();
    assert!(evidence_blob.contains("CVE-2020-8203"));
    assert!(evidence_blob.contains("CVE-2018-18074"));
    assert!(evidence_blob.contains("CVE-2020-28483"));
    assert!(evidence_blob.contains("CVE-2020-35873"));
    assert!(evidence_blob.contains("CVE-2022-29248"));

    // Verify deterministic sorting: severity descending
    for i in 0..findings.len() - 1 {
        assert!(findings[i].severity >= findings[i + 1].severity);
    }

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_custom_cached_advisory_ingestion() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_custom_osv_cache");
    let cache_dir = temp_dir.join(".eforge").join("cache").join("osv");
    let _ = fs::create_dir_all(&cache_dir);

    // Write custom advisory for "fastapi" in pypi.json
    let custom_advisory = r#"[
      {
        "id": "GHSA-custom-fastapi-test",
        "aliases": ["CVE-2099-99999"],
        "summary": "Synthetic test vulnerability in FastAPI",
        "details": "Details of synthetic vulnerability.",
        "severity": [
          { "type": "CVSS_V3", "score": "9.9" }
        ],
        "database_specific": {
          "severity": "CRITICAL"
        },
        "affected": [
          {
            "package": {
              "name": "fastapi",
              "ecosystem": "PyPI"
            },
            "ranges": [
              {
                "type": "ECOSYSTEM",
                "events": [
                  { "introduced": "0" },
                  { "fixed": "0.100.0" }
                ]
              }
            ]
          }
        ]
      }
    ]"#;

    fs::write(cache_dir.join("pypi.json"), custom_advisory)?;

    let matcher = VulnerabilityMatcher::load(&temp_dir);

    let dep = Dependency {
        name: "fastapi".to_string(),
        version_req: "==0.68.0".to_string(),
        resolved_version: Some("0.68.0".to_string()),
        ecosystem: "pypi".to_string(),
        is_dev: false,
        is_direct: true,
    };

    let findings = matcher.match_dependency(&dep);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Critical);
    assert!(findings[0].title.contains("CVE-2099-99999"));
    assert!(findings[0].recommendation.contains("0.100.0"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_security_scanner_scan_dependencies_helper() {
    let dummy_root = PathBuf::from(".");
    let deps = vec![Dependency {
        name: "minimist".to_string(),
        version_req: "1.2.5".to_string(),
        resolved_version: Some("1.2.5".to_string()),
        ecosystem: "npm".to_string(),
        is_dev: false,
        is_direct: true,
    }];

    let findings = SecurityScanner::scan_dependencies(&dummy_root, &deps);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Critical);
    assert!(findings[0].evidence.contains("CVE-2021-44906"));
    assert!(findings[0].recommendation.contains("1.2.6"));
}
