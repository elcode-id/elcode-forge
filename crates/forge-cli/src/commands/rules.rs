//! `eforge rules` command — rule listing, syntax verification, and template initialization.

use std::fs;
use std::path::{Path, PathBuf};

use forge_core::path_guard::resolve_safe_path;
use forge_core::{config::load_config, ForgeError, Severity};
use forge_rules::{parse_rule_file, validate_rule, RuleEngine, RuleSource};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;

/// Starter template YAML containing comprehensive examples for all 6 matchers.
pub const STARTER_RULES_TEMPLATE: &str = r#"# ELCODE Forge Declarative Custom Rules Template
# Documentation: https://github.com/elcode-id/elcode-forge
version: 1

rules:
  # 1. Forbidden Imports: Enforce architectural layer boundaries
  - id: "arch/no-db-in-ui"
    name: "No Direct DB in UI Components"
    category: "architecture"
    severity: "high"
    confidence: "certain"
    description: "UI components must not import database modules or direct drivers."
    remediation: "Route data access through services in src/services/."
    match:
      type: "forbidden_imports"
      files: ["src/ui/**", "src/components/**", "src/pages/**"]
      forbidden: ["src/db/**", "@prisma/client", "pg", "mysql2", "typeorm"]

  # 2. Required Companion: Require dedicated test companion for services
  - id: "quality/service-test-companion"
    name: "Service Test Companion"
    category: "quality"
    severity: "medium"
    confidence: "certain"
    description: "Every service in src/services/ must have a corresponding test companion."
    remediation: "Create companion test file in tests/{name}.test.ts."
    match:
      type: "required_companion"
      files: ["src/services/**"]
      companion: "tests/{name}.test.ts"

  # 3. File Naming: Enforce controller naming conventions
  - id: "quality/controller-naming"
    name: "Controller Suffix Convention"
    category: "quality"
    severity: "low"
    confidence: "certain"
    description: "All controllers must end with Controller.ts."
    remediation: "Rename file to adhere to *Controller.ts pattern."
    match:
      type: "file_naming"
      files: ["src/controllers/**"]
      convention: "*Controller.ts"

  # 4. File Size: Enforce maximum line limits
  - id: "quality/max-module-lines"
    name: "Module Line Count Threshold"
    category: "quality"
    severity: "low"
    confidence: "high"
    description: "Source files should not exceed 300 lines."
    remediation: "Refactor and decompose file into smaller modular units."
    match:
      type: "file_size"
      files: ["src/**"]
      max_lines: 300

  # 5. Regex Pattern: Prevent dangerous or prohibited dynamic code evaluation
  - id: "security/no-eval-call"
    name: "Disallow Dynamic Code Execution"
    category: "security"
    severity: "critical"
    confidence: "certain"
    description: "Disallow eval() dynamic code evaluation to prevent code injection."
    remediation: "Use safe structured JSON or explicit dispatch maps."
    match:
      type: "regex_pattern"
      files: ["**/*.ts", "**/*.js"]
      pattern: 'eval\('

  # 6. Dependency Restriction: Disallow vulnerable or deprecated packages
  - id: "deps/prohibit-deprecated-packages"
    name: "Prohibit Deprecated Packages"
    category: "dependencies"
    severity: "high"
    confidence: "certain"
    description: "Prohibit deprecated or unsafe packages in repository dependencies."
    remediation: "Replace deprecated or unapproved package with modern alternatives."
    match:
      type: "dependency_restriction"
      disallowed_packages: ["moment", "left-pad", "event-stream"]
"#;

/// Normalized rule descriptor for CLI display and schema serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleItemDescriptor {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: Severity,
    pub source: String,
    pub enabled: bool,
    pub description: String,
}

/// Structured report emitted by `eforge rules list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesListReport {
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub extended_packs: Vec<String>,
    pub rules: Vec<RuleItemDescriptor>,
}

/// Verification result for a single rule file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleCheckResult {
    pub path: String,
    pub valid: bool,
    pub rules_count: usize,
    pub errors: Vec<String>,
}

/// Structured report emitted by `eforge rules check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesCheckReport {
    pub total_files_checked: usize,
    pub valid_files: usize,
    pub invalid_files: usize,
    pub results: Vec<RuleCheckResult>,
}

/// Structured report emitted by `eforge rules init`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesInitReport {
    pub created_path: String,
    pub starter_rules_count: usize,
}

/// Executes `eforge rules list` against `repo_root`.
pub fn run_rules_list(
    repo_root: &Path,
    category_filter: Option<&str>,
    enabled_only: bool,
) -> Result<RulesListReport, ForgeError> {
    let config = load_config(repo_root)?;
    let engine = RuleEngine::from_config(repo_root, &config)?;

    let extended_packs = if config.rule_packs.is_empty() {
        vec!["recommended".to_string()]
    } else {
        config.rule_packs.clone()
    };

    let mut rules = Vec::new();

    // 1. Built-in rules
    for r in engine.rules() {
        let (name, category, desc) = match r.id.as_str() {
            "arch/no-direct-db-in-ui" => (
                "No Direct DB in UI",
                "architecture",
                "UI components must not import database modules directly.",
            ),
            "arch/no-circular-dependencies" => (
                "No Circular Dependencies",
                "architecture",
                "Module import graph must be an acyclic directed graph (DAG).",
            ),
            "quality/require-service-tests" => (
                "Require Service Tests",
                "quality",
                "Application services should have companion test files.",
            ),
            "quality/max-file-lines" => (
                "Max File Lines",
                "quality",
                "Source files should not exceed configured line limits.",
            ),
            _ => ("Built-in Rule", "quality", "Engine rule"),
        };

        rules.push(RuleItemDescriptor {
            id: r.id.clone(),
            name: name.to_string(),
            category: category.to_string(),
            severity: r.severity,
            source: "builtin".to_string(),
            enabled: r.enabled,
            description: desc.to_string(),
        });
    }

    // 2. Declarative rules
    for d in engine.declarative_rules() {
        rules.push(RuleItemDescriptor {
            id: d.id.clone(),
            name: d.name.clone(),
            category: d.category.as_str().to_string(),
            severity: d.severity,
            source: d.source.to_string(),
            enabled: d.enabled,
            description: d.description.clone(),
        });
    }

    // 3. Filters
    if let Some(cat) = category_filter {
        let cat_lower = cat.to_ascii_lowercase();
        rules.retain(|r| {
            r.category.to_ascii_lowercase().contains(&cat_lower)
                || match cat_lower.as_str() {
                    "arch" => r.category == "architecture",
                    "sec" => r.category == "security",
                    "deps" => r.category == "dependencies",
                    _ => false,
                }
        });
    }

    if enabled_only {
        rules.retain(|r| r.enabled);
    }

    // Deterministic sort: Severity descending, then ID ascending
    rules.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));

    let total_rules = rules.len();
    let enabled_rules = rules.iter().filter(|r| r.enabled).count();

    Ok(RulesListReport {
        total_rules,
        enabled_rules,
        extended_packs,
        rules,
    })
}

/// Renders the `RulesListReport` in `format`.
pub fn render_rules_list(
    report: &RulesListReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Rules Catalog\n\n");
            md.push_str(&format!(
                "- **Packs Extended:** {}\n- **Total Rules:** {} (Active: {}, Disabled: {})\n\n",
                report.extended_packs.join(", "),
                report.total_rules,
                report.enabled_rules,
                report.total_rules.saturating_sub(report.enabled_rules)
            ));

            if !report.rules.is_empty() {
                md.push_str("| Status | Rule ID | Category | Severity | Source | Description |\n");
                md.push_str("| :--- | :--- | :--- | :--- | :--- | :--- |\n");
                for r in &report.rules {
                    md.push_str(&format!(
                        "| `{}` | `{}` | {} | `{:?}` | `{}` | {} |\n",
                        if r.enabled { "ENABLED" } else { "DISABLED" },
                        r.id,
                        r.category,
                        r.severity,
                        r.source,
                        r.description
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge — Registered Architectural & Quality Rules\n");
            out.push_str("======================================================\n");
            out.push_str(&format!(
                "Packs Extended: {}\nTotal Rules:    {} | Active: {} | Disabled: {}\n\n",
                report.extended_packs.join(", "),
                report.total_rules,
                report.enabled_rules,
                report.total_rules.saturating_sub(report.enabled_rules)
            ));

            out.push_str(&format!(
                "{:<10} {:<32} {:<14} {:<10} {}\n",
                "STATUS", "RULE ID", "CATEGORY", "SEVERITY", "SOURCE"
            ));
            out.push_str(&format!("{}\n", "-".repeat(84)));

            for r in &report.rules {
                out.push_str(&format!(
                    "{:<10} {:<32} {:<14} {:<10?} {}\n",
                    if r.enabled { "[ENABLED]" } else { "[DISABLED]" },
                    r.id,
                    r.category,
                    r.severity,
                    r.source
                ));
            }
            Ok(out)
        }
    }
}

/// Executes `eforge rules check` against rule files in `repo_root` or specified path.
pub fn run_rules_check(
    repo_root: &Path,
    target_path: Option<&Path>,
) -> Result<RulesCheckReport, ForgeError> {
    let mut files_to_check: Vec<PathBuf> = Vec::new();

    if let Some(target) = target_path {
        let full = if target.is_absolute() {
            target.to_path_buf()
        } else {
            repo_root.join(target)
        };

        if full.is_file() {
            files_to_check.push(full);
        } else if full.is_dir() {
            collect_rule_files(&full, &mut files_to_check);
        } else {
            return Err(ForgeError::Config {
                message: format!("target path does not exist: {}", target.display()),
            });
        }
    } else {
        let config = load_config(repo_root)?;
        let eforge_rules = repo_root.join(".eforge").join("rules");
        if eforge_rules.is_dir() {
            collect_rule_files(&eforge_rules, &mut files_to_check);
        }

        for p in &config.custom_rule_paths {
            if let Ok(safe_p) = resolve_safe_path(repo_root, Path::new(p)) {
                if safe_p.is_file() {
                    files_to_check.push(safe_p);
                } else if safe_p.is_dir() {
                    collect_rule_files(&safe_p, &mut files_to_check);
                }
            }
        }
    }

    files_to_check.sort();
    files_to_check.dedup();

    let mut results = Vec::new();

    for file_path in &files_to_check {
        let rel_path = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                results.push(RuleCheckResult {
                    path: rel_path,
                    valid: false,
                    rules_count: 0,
                    errors: vec![format!("Failed to read file: {e}")],
                });
                continue;
            }
        };

        let parsed_rules = match parse_rule_file(&content, RuleSource::Custom(rel_path.clone())) {
            Ok(r) => r,
            Err(e) => {
                results.push(RuleCheckResult {
                    path: rel_path,
                    valid: false,
                    rules_count: 0,
                    errors: vec![format!("Syntax/parse error: {e}")],
                });
                continue;
            }
        };

        let mut errors = Vec::new();
        for r in &parsed_rules {
            if let Err(e) = validate_rule(r) {
                errors.push(format!("Rule '{}': {e}", r.id));
            }
        }

        let valid = errors.is_empty();
        results.push(RuleCheckResult {
            path: rel_path,
            valid,
            rules_count: parsed_rules.len(),
            errors,
        });
    }

    let total_files_checked = results.len();
    let valid_files = results.iter().filter(|r| r.valid).count();
    let invalid_files = results.iter().filter(|r| !r.valid).count();

    Ok(RulesCheckReport {
        total_files_checked,
        valid_files,
        invalid_files,
        results,
    })
}

fn collect_rule_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let lower = path.to_string_lossy().to_ascii_lowercase();
            if lower.ends_with(".yaml") || lower.ends_with(".yml") || lower.ends_with(".json") {
                out.push(path);
            }
        } else if path.is_dir() {
            collect_rule_files(&path, out);
        }
    }
}

/// Renders the `RulesCheckReport` in `format`.
pub fn render_rules_check(
    report: &RulesCheckReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Rule Syntax Check\n\n");
            md.push_str(&format!(
                "- **Files Checked:** {}\n- **Valid:** {}\n- **Invalid:** {}\n\n",
                report.total_files_checked, report.valid_files, report.invalid_files
            ));

            if !report.results.is_empty() {
                md.push_str("| Status | File | Rules Count | Details |\n");
                md.push_str("| :--- | :--- | :--- | :--- |\n");
                for r in &report.results {
                    md.push_str(&format!(
                        "| {} | `{}` | {} | {} |\n",
                        if r.valid { "`PASS`" } else { "`FAIL`" },
                        r.path,
                        r.rules_count,
                        if r.valid {
                            "Valid syntax".to_string()
                        } else {
                            r.errors.join("; ")
                        }
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge — Rule Syntax Verification\n");
            out.push_str("======================================\n");
            out.push_str(&format!(
                "Checked {} rule file(s): {} valid, {} invalid\n\n",
                report.total_files_checked, report.valid_files, report.invalid_files
            ));

            for r in &report.results {
                if r.valid {
                    out.push_str(&format!(
                        "  ✓ {} ({} rule(s) valid)\n",
                        r.path, r.rules_count
                    ));
                } else {
                    out.push_str(&format!("  ✗ {}:\n", r.path));
                    for err in &r.errors {
                        out.push_str(&format!("    - {err}\n"));
                    }
                }
            }
            Ok(out)
        }
    }
}

/// Executes `eforge rules init` to generate a starter rule template.
pub fn run_rules_init(
    repo_root: &Path,
    output: Option<&Path>,
    force: bool,
) -> Result<RulesInitReport, ForgeError> {
    let dest_path = if let Some(p) = output {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            repo_root.join(p)
        }
    } else {
        repo_root
            .join(".eforge")
            .join("rules")
            .join("custom-rules.yaml")
    };

    if dest_path.exists() && !force {
        return Err(ForgeError::Config {
            message: format!(
                "Rule file already exists at '{}'. Use --force to overwrite.",
                dest_path.display()
            ),
        });
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&dest_path, STARTER_RULES_TEMPLATE).map_err(|source| ForgeError::Io {
        path: dest_path.clone(),
        source,
    })?;

    let parsed = parse_rule_file(
        STARTER_RULES_TEMPLATE,
        RuleSource::Custom("starter".to_string()),
    )
    .unwrap_or_default();

    let rel_str = dest_path
        .strip_prefix(repo_root)
        .unwrap_or(&dest_path)
        .to_string_lossy()
        .replace('\\', "/");

    Ok(RulesInitReport {
        created_path: rel_str,
        starter_rules_count: parsed.len(),
    })
}

/// Renders the `RulesInitReport` in `format`.
pub fn render_rules_init(
    report: &RulesInitReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Starter Rules Template Generated\n\n");
            md.push_str(&format!(
                "- **Created:** `{}`\n- **Starter Rules Included:** {}\n",
                report.created_path, report.starter_rules_count
            ));
            Ok(md)
        }
        OutputFormat::Terminal => Ok(format!(
            "✓ Generated starter rules template at {} ({} rules configured)\n",
            report.created_path, report.starter_rules_count
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_template_parses_and_validates_all_six_rules() {
        let res = parse_rule_file(
            STARTER_RULES_TEMPLATE,
            RuleSource::Custom("starter.yaml".to_string()),
        );
        assert!(res.is_ok());

        if let Ok(rules) = res {
            assert_eq!(rules.len(), 6);
            for r in &rules {
                let v = validate_rule(r);
                assert!(
                    v.is_ok(),
                    "Validation failed for rule {}: {:?}",
                    r.id,
                    v.err()
                );
            }
        }
    }

    #[test]
    fn rules_init_creates_file_and_protects_existing_without_force() {
        let temp_dir =
            std::env::temp_dir().join(format!("forge_test_rules_init_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let res = run_rules_init(&temp_dir, None, false);
        assert!(res.is_ok());

        if let Ok(rep) = res {
            assert_eq!(rep.created_path, ".eforge/rules/custom-rules.yaml");
            assert_eq!(rep.starter_rules_count, 6);
        }

        // Second run without force must return error
        let res2 = run_rules_init(&temp_dir, None, false);
        assert!(res2.is_err());

        // With force must succeed
        let res3 = run_rules_init(&temp_dir, None, true);
        assert!(res3.is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn rules_check_validates_starter_rules() {
        let temp_dir =
            std::env::temp_dir().join(format!("forge_test_rules_check_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let init_res = run_rules_init(&temp_dir, None, false);
        assert!(init_res.is_ok());

        let check_res = run_rules_check(&temp_dir, None);
        assert!(check_res.is_ok());

        if let Ok(rep) = check_res {
            assert_eq!(rep.total_files_checked, 1);
            assert_eq!(rep.valid_files, 1);
            assert_eq!(rep.invalid_files, 0);
            assert_eq!(rep.results[0].rules_count, 6);
            assert!(rep.results[0].valid);
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
