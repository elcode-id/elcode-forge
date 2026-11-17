//! Phase 11 hardening and regression end-to-end test suite.
//!
//! Verifies:
//! 1. Quote-aware YAML configuration parsing (`#` inside strings) & `FileFindingsCache`.
//! 2. Multiline TS imports, UTF-8 comments, and `tsconfig.json` path aliases (`@app/*`, `$lib/*`).
//! 3. PHP incremental AST architecture parity on `fixtures/laravel-app`.
//! 4. Full vs `--incremental` `eforge scan` security findings preservation.
//! 5. JSON-safe `eforge fix` removing the final dependency in `package.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_analyzer::graph::build_polyglot_architecture_graph;
use forge_analyzer::incremental::{analyze_architecture_incremental, FileAstCache};
use forge_analyzer::scan_repository;
use forge_analyzer::ts_imports::{
    extract_ts_imports_and_exports, resolve_import_specifier_with_paths, TsConfigPaths,
};
use forge_cli::cli::OutputFormat;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_core::cache::{CachedFileFindings, FileFindingsCache};
use forge_core::config::{load_config, parse_config_yaml};
use forge_core::ir::{Confidence, Finding, Severity, SourceLocation};
use forge_core::Repository;
use forge_rules::autofix::generate_fix_plan;
use forge_rules::tx::{apply_fix_plan_transactional, rollback_transaction};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_quote_aware_yaml_and_file_findings_cache() -> Result<(), Box<dyn Error>> {
    // 1. Verify YAML comment stripper preserves hashes inside quoted strings
    let yaml = r#"
version: 1 # top-level comment
project:
  name: "forge#project-v1" # trailing comment on double-quote
ignore:
  - 'src/vendor#dir/**/*.ts' # single-quoted hash
  - "legacy/#archive"
security:
  allowlist: ["env#token", 'auth#secret'] # inline list with hashes
"#;
    let cfg = parse_config_yaml(yaml)?;
    assert_eq!(cfg.project_name.as_deref(), Some("forge#project-v1"));
    assert_eq!(
        cfg.ignore_patterns,
        vec!["src/vendor#dir/**/*.ts", "legacy/#archive"]
    );
    let security_cfg = cfg.security.ok_or("missing security config")?;
    assert_eq!(security_cfg.allowlist, vec!["env#token", "auth#secret"]);

    // 2. Verify FileFindingsCache serialization, loading, and retention
    let temp_repo = std::env::temp_dir().join("eforge_phase11_findings_cache_test");
    let _ = fs::remove_dir_all(&temp_repo);
    fs::create_dir_all(&temp_repo)?;

    let mut cache = FileFindingsCache::load(&temp_repo, "config_hash_abc");
    assert!(cache.entries.is_empty());

    let dummy_finding = Finding {
        id: "SEC-TEST-001".to_string(),
        category: "security".to_string(),
        severity: Severity::High,
        confidence: Confidence::Certain,
        title: "Test secret detected".to_string(),
        description: "Hardcoded key".to_string(),
        evidence: "KEY=123".to_string(),
        location: Some(SourceLocation {
            file_path: "src/auth.ts".to_string(),
            start_line: Some(10),
            end_line: Some(10),
        }),
        recommendation: "Rotate key".to_string(),
        rule_id: "security/no-secrets".to_string(),
    };

    cache.entries.insert(
        "src/auth.ts".to_string(),
        CachedFileFindings {
            relative_path: "src/auth.ts".to_string(),
            content_hash: "hash_auth_123".to_string(),
            findings: vec![dummy_finding],
        },
    );
    cache.entries.insert(
        "src/obsolete.ts".to_string(),
        CachedFileFindings {
            relative_path: "src/obsolete.ts".to_string(),
            content_hash: "hash_obs_999".to_string(),
            findings: vec![],
        },
    );

    cache.save(&temp_repo)?;

    // Reload cache and assert persistence
    let reloaded = FileFindingsCache::load(&temp_repo, "config_hash_abc");
    assert_eq!(reloaded.entries.len(), 2);
    assert_eq!(reloaded.entries["src/auth.ts"].findings.len(), 1);

    // Retain only active files
    let mut retained_cache = reloaded;
    let mut active = BTreeSet::new();
    active.insert("src/auth.ts");
    retained_cache.retain_files(&active);
    assert_eq!(retained_cache.entries.len(), 1);
    assert!(!retained_cache.entries.contains_key("src/obsolete.ts"));

    let _ = fs::remove_dir_all(&temp_repo);
    Ok(())
}

#[test]
fn test_multiline_ts_imports_utf8_and_tsconfig_aliases() -> Result<(), Box<dyn Error>> {
    let source = r#"
// 🛡️ Security gateway check: 認証トークン
import {
  AuthService,
  TokenValidator,
  type SecurityRole,
} from '@/services/auth';

import {
  formatDate,
  type AppConfig,
} from '$lib/utils/format';

/*
  Multi-line UTF-8 comment:
  Emoji: 🚀 🔍 🛡️
*/
import * as db from '../db/client';
"#;

    let mut paths_map = BTreeMap::new();
    paths_map.insert("@/*".to_string(), vec!["src/*".to_string()]);
    let tsconfig = TsConfigPaths {
        base_url: Some(".".to_string()),
        paths: paths_map,
    };

    let (imports, _exports) = extract_ts_imports_and_exports(source);

    // Verify all 3 multiline & UTF-8 imports are cleanly extracted
    assert_eq!(imports.len(), 3);
    assert!(imports.contains(&"@/services/auth".to_string()));
    assert!(imports.contains(&"$lib/utils/format".to_string()));
    assert!(imports.contains(&"../db/client".to_string()));

    let mut known = BTreeSet::new();
    known.insert("src/services/auth.ts".to_string());
    known.insert("src/lib/utils/format.ts".to_string());
    known.insert("src/db/client.ts".to_string());

    // Verify @/* alias resolved to src/services/auth.ts
    let res1 = resolve_import_specifier_with_paths(
        "src/pages/dashboard.ts",
        "@/services/auth",
        &known,
        Some(&tsconfig),
    );
    assert_eq!(res1.as_deref(), Some("src/services/auth.ts"));

    // Verify $lib/* alias resolved to src/lib/utils/format.ts
    let res2 = resolve_import_specifier_with_paths(
        "src/pages/dashboard.ts",
        "$lib/utils/format",
        &known,
        Some(&tsconfig),
    );
    assert_eq!(res2.as_deref(), Some("src/lib/utils/format.ts"));

    // Verify relative import resolved correctly to src/db/client.ts
    let res3 = resolve_import_specifier_with_paths(
        "src/pages/dashboard.ts",
        "../db/client",
        &known,
        Some(&tsconfig),
    );
    assert_eq!(res3.as_deref(), Some("src/db/client.ts"));

    Ok(())
}

#[test]
fn test_php_incremental_architecture_parity_laravel_app() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("laravel-app");
    let config = load_config(&root)?;
    let discovery = scan_repository(&root, &config.ignore_patterns)?;

    // 1. Full polyglot architecture graph
    let full_graph = build_polyglot_architecture_graph(&root, &discovery.files)?;

    // 2. Incremental architecture graph with AST cache
    let mut ast_cache = FileAstCache::default();
    let inc_result = analyze_architecture_incremental(&root, &discovery.files, &mut ast_cache)?;
    let inc_graph = inc_result.graph;

    // Parity assertions
    assert_eq!(
        inc_graph.modules.len(),
        full_graph.modules.len(),
        "Module count mismatch between full and incremental PHP analysis"
    );

    let full_mod_names: Vec<String> = full_graph.modules.iter().map(|m| m.name.clone()).collect();
    let inc_mod_names: Vec<String> = inc_graph.modules.iter().map(|m| m.name.clone()).collect();
    assert_eq!(full_mod_names, inc_mod_names);

    assert_eq!(
        inc_graph.circular_dependencies, full_graph.circular_dependencies,
        "Circular dependency parity mismatch"
    );
    assert_eq!(
        inc_graph.high_centrality_modules, full_graph.high_centrality_modules,
        "High centrality modules parity mismatch"
    );
    assert_eq!(
        inc_graph.external_imports, full_graph.external_imports,
        "External imports parity mismatch"
    );

    Ok(())
}

#[test]
fn test_full_vs_incremental_scan_security_findings_preservation() -> Result<(), Box<dyn Error>> {
    let temp_repo = std::env::temp_dir().join("eforge_phase11_scan_parity_test");
    let _ = fs::remove_dir_all(&temp_repo);
    let src_dir = temp_repo.join("src");
    fs::create_dir_all(&src_dir)?;

    fs::write(
        temp_repo.join("eforge.config.yaml"),
        "version: 1\nproject:\n  name: 'phase11-scan-test'\n",
    )?;

    // Untouched file with secret
    let secret_file = src_dir.join("secrets.env");
    fs::write(
        &secret_file,
        "AWS_SECRET_ACCESS_KEY=\"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\"\n",
    )?;

    // Modifiable source file
    let source_file = src_dir.join("main.ts");
    fs::write(
        &source_file,
        "export function run(): string { return 'v1'; }\n",
    )?;

    // Run 1: Full scan
    let full_opts = ScanOptions {
        repo_root: temp_repo.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo_full_1, _) = execute_scan(&full_opts)?;
    let secret_findings_1: Vec<_> = repo_full_1
        .findings
        .iter()
        .filter(|f| f.rule_id.contains("secret") || f.category == "security")
        .cloned()
        .collect();
    assert_eq!(secret_findings_1.len(), 1);

    // Run 2: Incremental scan with 0 file changes
    let inc_opts = ScanOptions {
        repo_root: temp_repo.clone(),
        incremental: true,
        format: OutputFormat::Json,
    };
    let (repo_inc_0, _) = execute_scan(&inc_opts)?;
    assert_eq!(
        repo_inc_0.findings, repo_full_1.findings,
        "Zero-change incremental findings must match full scan findings"
    );

    // Run 3: Modify main.ts (secrets.env remains untouched)
    fs::write(
        &source_file,
        "export function run(): string { return 'v2-modified'; }\n",
    )?;
    let (repo_inc_1, _) = execute_scan(&inc_opts)?;
    let secret_findings_after: Vec<_> = repo_inc_1
        .findings
        .iter()
        .filter(|f| f.rule_id.contains("secret") || f.category == "security")
        .cloned()
        .collect();
    assert_eq!(
        secret_findings_1, secret_findings_after,
        "Security findings in untouched file must be preserved in incremental scan"
    );

    // Run 4: Fresh full scan on modified repo must match incremental findings
    let (repo_full_2, _) = execute_scan(&full_opts)?;
    assert_eq!(
        repo_inc_1.findings, repo_full_2.findings,
        "Incremental scan and full scan on modified repo must produce identical findings"
    );

    let _ = fs::remove_dir_all(&temp_repo);
    Ok(())
}

#[test]
fn test_json_safe_autofix_removes_final_package_json_dependency() -> Result<(), Box<dyn Error>> {
    let temp_repo = std::env::temp_dir().join("eforge_phase11_autofix_json_test");
    let _ = fs::remove_dir_all(&temp_repo);
    fs::create_dir_all(&temp_repo)?;

    let pkg_path = temp_repo.join("package.json");
    let original_pkg = r#"{
  "name": "phase11-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.2.0",
    "unused-dep": "^1.0.0"
  }
}
"#;
    fs::write(&pkg_path, original_pkg)?;

    let repo = Repository {
        root_path: temp_repo.to_string_lossy().into_owned(),
        name: "phase11-app".to_string(),
        detected_languages: vec!["typescript".to_string()],
        detected_frameworks: vec![],
        package_managers: vec!["npm".to_string()],
        files: vec![],
        modules: vec![],
        dependencies: vec![],
        findings: vec![Finding {
            id: "F-DEP-LAST".to_string(),
            category: "dependencies".to_string(),
            severity: Severity::Low,
            confidence: Confidence::Certain,
            title: "Unused production dependency 'unused-dep'".to_string(),
            description: "Package 'unused-dep' is declared but never imported.".to_string(),
            evidence: "\"unused-dep\": \"^1.0.0\"".to_string(),
            location: Some(SourceLocation {
                file_path: "package.json".to_string(),
                start_line: Some(6),
                end_line: Some(6),
            }),
            recommendation: "Remove unused-dep from package.json".to_string(),
            rule_id: "deps/unused-dependency".to_string(),
        }],
        health: None,
    };

    // 1. Generate fix plan
    let plan = generate_fix_plan(&temp_repo, &repo, None)?;
    assert_eq!(plan.suggestions.len(), 1);
    assert_eq!(plan.total_edits, 1);

    // 2. Apply fix transactionally
    let tx_record = apply_fix_plan_transactional(&temp_repo, &plan.suggestions)?;
    assert_eq!(tx_record.edits_applied, 1);

    // 3. Verify resulting package.json is strictly valid JSON without trailing commas
    let updated_pkg = fs::read_to_string(&pkg_path)?;
    assert!(!updated_pkg.contains("unused-dep"));
    let parsed: serde_json::Value = serde_json::from_str(&updated_pkg)?;
    assert_eq!(parsed["dependencies"]["react"], "^18.2.0");
    assert!(parsed["dependencies"].get("unused-dep").is_none());

    // 4. Rollback transaction and verify byte-for-byte exact restoration
    let rollback_record = rollback_transaction(&temp_repo, Some(&tx_record.tx_id))?;
    assert_eq!(rollback_record.tx_id, tx_record.tx_id);

    let restored_pkg = fs::read_to_string(&pkg_path)?;
    assert_eq!(restored_pkg, original_pkg);

    let _ = fs::remove_dir_all(&temp_repo);
    Ok(())
}
