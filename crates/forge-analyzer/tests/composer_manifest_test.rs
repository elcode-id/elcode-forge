use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_analyzer::composer_manifest::{
    analyze_composer_dependencies, is_composer_unpinned, parse_composer_json_content,
    parse_composer_manifest, read_composer_lock_versions,
};

fn create_temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[test]
fn test_is_composer_unpinned() {
    assert!(is_composer_unpinned("*"));
    assert!(is_composer_unpinned(""));
    assert!(is_composer_unpinned(">=1.0"));
    assert!(is_composer_unpinned(">2.0"));
    assert!(is_composer_unpinned("dev-master"));
    assert!(is_composer_unpinned("dev-main"));

    assert!(!is_composer_unpinned("^1.0"));
    assert!(!is_composer_unpinned("~1.2.0"));
    assert!(!is_composer_unpinned("1.0.*"));
    assert!(!is_composer_unpinned(">=1.0 <2.0"));
    assert!(!is_composer_unpinned(">=1.0, <2.0"));
    assert!(!is_composer_unpinned("1.2.3"));
}

#[test]
fn test_parse_composer_json_basic() -> Result<(), Box<dyn Error>> {
    let composer_json = r#"{
        "name": "elcode/test-app",
        "require": {
            "php": "^8.2",
            "laravel/framework": "^10.0",
            "guzzlehttp/guzzle": "*"
        },
        "require-dev": {
            "phpunit/phpunit": "^10.0",
            "laravel/framework": "^10.0"
        },
        "autoload": {
            "psr-4": {
                "App\\": "app/",
                "Database\\Seeders\\": "database/seeders"
            }
        }
    }"#;

    let mut lock_versions = BTreeMap::new();
    lock_versions.insert("laravel/framework".to_string(), "v10.34.2".to_string());
    lock_versions.insert("guzzlehttp/guzzle".to_string(), "7.8.1".to_string());

    let manifest = parse_composer_json_content(composer_json, true, &lock_versions)?;

    assert_eq!(manifest.name.as_deref(), Some("elcode/test-app"));
    assert_eq!(manifest.dependencies.len(), 5); // 3 prod + 2 dev

    // Check resolved versions
    let laravel_prod = manifest
        .dependencies
        .iter()
        .find(|d| d.name == "laravel/framework" && !d.is_dev)
        .ok_or("Missing prod laravel/framework")?;
    assert_eq!(laravel_prod.resolved_version.as_deref(), Some("v10.34.2"));

    // Check unpinned
    assert_eq!(manifest.unpinned_packages, vec!["guzzlehttp/guzzle"]);

    // Check duplicate
    assert_eq!(manifest.duplicate_packages, vec!["laravel/framework"]);

    // Check PSR-4 mappings
    assert_eq!(
        manifest.psr4_mappings.get("App\\").map(|s| s.as_str()),
        Some("app")
    );
    assert_eq!(
        manifest
            .psr4_mappings
            .get("Database\\Seeders\\")
            .map(|s| s.as_str()),
        Some("database/seeders")
    );

    Ok(())
}

#[test]
fn test_read_composer_lock_versions() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_dir("composer_lock_test")?;
    let lock_content = r#"{
        "_readme": ["This file locks the dependencies"],
        "content-hash": "abcdef123456",
        "packages": [
            {
                "name": "guzzlehttp/guzzle",
                "version": "7.8.1"
            },
            {
                "name": "laravel/framework",
                "version": "v10.34.2"
            }
        ],
        "packages-dev": [
            {
                "name": "phpunit/phpunit",
                "version": "10.5.2"
            }
        ]
    }"#;

    fs::write(temp_dir.join("composer.lock"), lock_content)?;

    let map = read_composer_lock_versions(&temp_dir);
    assert_eq!(
        map.get("guzzlehttp/guzzle").map(|s| s.as_str()),
        Some("7.8.1")
    );
    assert_eq!(
        map.get("laravel/framework").map(|s| s.as_str()),
        Some("v10.34.2")
    );
    assert_eq!(
        map.get("phpunit/phpunit").map(|s| s.as_str()),
        Some("10.5.2")
    );

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_analyze_composer_dependencies_findings() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_dir("composer_analysis_test")?;

    let composer_json = r#"{
        "name": "my/laravel-app",
        "require": {
            "php": "^8.2",
            "ext-json": "*",
            "laravel/framework": "^10.0",
            "unused/vendor-pkg": "*",
            "used/service-pkg": "^2.1"
        },
        "require-dev": {
            "phpunit/phpunit": "^10.0",
            "used/service-pkg": "^2.1"
        }
    }"#;
    fs::write(temp_dir.join("composer.json"), composer_json)?;
    // Deliberately no composer.lock to test missing lockfile finding

    let mut external_imports = BTreeSet::new();
    external_imports.insert("used/service-pkg".to_string());
    external_imports.insert("laravel/framework".to_string());

    let report = analyze_composer_dependencies(&temp_dir, &external_imports)?;

    // 1. Missing lockfile
    let rule_ids: Vec<&str> = report.findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert!(rule_ids.contains(&"deps/missing-lockfile"));

    // 2. Unpinned version for "unused/vendor-pkg" (and not ext-json)
    assert!(rule_ids.contains(&"deps/unpinned-version"));
    assert!(report
        .wildcard_or_unpinned
        .contains(&"unused/vendor-pkg".to_string()));
    assert!(!report
        .wildcard_or_unpinned
        .contains(&"ext-json".to_string()));

    // 3. Duplicate package for "used/service-pkg"
    assert!(rule_ids.contains(&"deps/duplicate-dependency"));
    assert!(report
        .duplicate_packages
        .contains(&"used/service-pkg".to_string()));

    // 4. Unused production dependency "unused/vendor-pkg"
    assert!(rule_ids.contains(&"deps/unused-dependency"));
    assert!(report
        .unused_dependencies
        .contains(&"unused/vendor-pkg".to_string()));
    assert!(!report.unused_dependencies.contains(&"php".to_string()));
    assert!(!report.unused_dependencies.contains(&"ext-json".to_string()));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_parse_composer_manifest_file() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_dir("composer_manifest_file_test")?;
    let composer_json = r#"{
        "name": "elcode/file-app",
        "require": {
            "laravel/framework": "^10.0"
        }
    }"#;
    fs::write(temp_dir.join("composer.json"), composer_json)?;

    let manifest = parse_composer_manifest(&temp_dir)?;
    assert_eq!(manifest.name.as_deref(), Some("elcode/file-app"));
    assert_eq!(manifest.dependencies.len(), 1);
    assert!(!manifest.has_composer_lock);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
