use std::collections::BTreeSet;
use std::path::Path;

use forge_ai::build_pruned_centrality_context;
use forge_analyzer::{
    analyze_polyglot_dependencies, build_polyglot_architecture_graph, detector::is_test_file,
    extract_file_import_edges, scanner::scan_repository, simulate_refactor, RefactorOperation,
};
use forge_core::{
    config::load_config,
    scoring::{calculate_change_risk, calculate_health_score},
    Finding, ForgeError, Repository, Severity,
};
use forge_git::{
    collect_working_tree_diff, compute_change_impact, compute_impacted_tests, parse_unified_diff,
};
use forge_rules::RuleEngine;
use forge_security::SecurityScanner;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::boundary::wrap_untrusted_payload;

/// Descriptor for an MCP tool exposed in the `eforge_*` namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Detailed rule descriptor exposed via `eforge_get_rules`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRuleDescriptor {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
    pub source: String,
    pub enabled: bool,
}

/// Returns the deterministic catalog of `eforge_*` MCP tools (11 tools in v1.1).
#[must_use]
pub fn list_mcp_tools() -> Vec<McpToolDescriptor> {
    vec![
        McpToolDescriptor {
            name: "eforge_project_info".to_string(),
            description: "Returns deterministic repository profile, detected languages/frameworks, inventory counts, and 8-dimension health score.".to_string(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "eforge_get_architecture".to_string(),
            description: "Returns the repository module architecture graph, inter-module imports, and detected circular dependency cycles.".to_string(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "eforge_get_dependencies".to_string(),
            description: "Returns external package dependencies and dependency hygiene findings (unused, unpinned, duplicate).".to_string(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "eforge_get_findings".to_string(),
            description: "Returns deterministic architectural, quality, dependency, and security findings with optional severity/rule filtering.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "severity": {"type": "string", "description": "Optional filter: critical, high, medium, low, info"},
                    "rule_prefix": {"type": "string", "description": "Optional rule ID prefix filter (e.g. 'arch/', 'security/')"}
                },
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_get_change_impact".to_string(),
            description: "Computes direct and transitive module blast radius and change risk score for a unified diff or working tree.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "diff_text": {"type": "string", "description": "Optional unified git diff text; defaults to working tree diff"}
                },
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_explain_module".to_string(),
            description: "Explains the structural role, inbound/outbound module links, files, and findings for a module or path prefix.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "Module ID or relative path prefix (e.g. 'src/services')"}
                },
                "required": ["target"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_get_rules".to_string(),
            description: "Returns active architectural, quality, security, and dependency rule definitions with pack inheritance and status.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Optional category filter: architecture, quality, security, dependencies"
                    },
                    "enabled_only": {
                        "type": "boolean",
                        "description": "Optional flag: if true, returns only currently enabled rules"
                    }
                },
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_verify_patch".to_string(),
            description: "Closed-loop pre-flight verification for AI agents: validates a proposed unified diff in memory against security rules, blast radius, and policy gates before writing to disk.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "diff_text": {"type": "string", "description": "Proposed unified diff text to verify before applying to disk"}
                },
                "required": ["diff_text"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_simulate_refactor".to_string(),
            description: "Simulates moving files or modules in memory and reports resolved/introduced Tarjan SCC cycles and required import specifier rewrites.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "Source file or module path"},
                    "to": {"type": "string", "description": "Destination file or module path"}
                },
                "required": ["from", "to"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_get_impacted_tests".to_string(),
            description: "Maps a unified diff or working tree changes through reverse-import BFS to Tier 1/2/3 impacted test files and synthesizes test runner commands.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "diff_text": {"type": "string", "description": "Optional unified diff text; defaults to working tree diff"}
                },
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "eforge_get_pruned_context".to_string(),
            description: "Returns a token-budgeted context pack around focal files/modules ranked by hop distance and graph centrality.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "focal_targets": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Focal module IDs or file paths"
                    },
                    "token_budget": {"type": "integer", "description": "Maximum token budget (default: 4000)"},
                    "max_hops": {"type": "integer", "description": "Maximum import graph hops (default: 2)"}
                },
                "additionalProperties": false
            }),
        },
    ]
}

struct AnalyzedContext {
    repo: Repository,
    cycles: Vec<Vec<String>>,
    test_files: BTreeSet<String>,
}

fn build_analyzed_repo(repo_root: &Path) -> Result<AnalyzedContext, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch = build_polyglot_architecture_graph(repo_root, &discovery.files)?;
    let dep_report =
        analyze_polyglot_dependencies(repo_root, &discovery.files, &arch.external_imports)?;

    let mut repo = Repository {
        name: config.project_name.clone().unwrap_or_else(|| {
            repo_root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("repository")
                .to_string()
        }),
        root_path: repo_root.to_string_lossy().replace('\\', "/"),
        detected_languages: discovery.languages,
        detected_frameworks: discovery.frameworks,
        package_managers: discovery.package_managers,
        modules: arch.modules,
        files: discovery.files,
        dependencies: dep_report.dependencies,
        findings: Vec::new(),
        health: None,
    };

    let engine = RuleEngine::from_config(repo_root, &config)?;
    let mut findings = engine.evaluate(&repo)?;
    findings.extend(dep_report.findings);
    findings.extend(SecurityScanner::scan_repository_files_with_config(
        repo_root,
        &repo.files,
        config.security.as_ref(),
    )?);
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    repo.findings = findings;
    repo.health = Some(calculate_health_score(&repo));

    let test_files = repo
        .files
        .iter()
        .filter(|f| is_test_file(&f.relative_path))
        .map(|f| f.relative_path.clone())
        .collect();

    Ok(AnalyzedContext {
        repo,
        cycles: arch.circular_dependencies,
        test_files,
    })
}

fn to_envelope_json<T: Serialize>(payload: T, raw_texts: &[&str]) -> Result<Value, ForgeError> {
    serde_json::to_value(wrap_untrusted_payload(payload, raw_texts)).map_err(|e| {
        ForgeError::Serialization {
            message: e.to_string(),
        }
    })
}

/// Dispatches an `eforge_*` MCP tool invocation and wraps the result in an [`UntrustedEnvelope`](crate::UntrustedEnvelope).
pub fn call_mcp_tool(
    repo_root: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, ForgeError> {
    match tool_name {
        "eforge_get_rules" => {
            let config = load_config(repo_root)?;
            let engine = RuleEngine::from_config(repo_root, &config)?;

            let extended_packs = if config.rule_packs.is_empty() {
                vec!["recommended".to_string()]
            } else {
                config.rule_packs.clone()
            };

            let mut rules = Vec::new();

            for r in engine.rules() {
                let (name, category, desc, remed) = match r.id.as_str() {
                    "arch/no-direct-db-in-ui" => (
                        "No Direct DB in UI",
                        "architecture",
                        "UI components must not import database modules directly.",
                        "Route data access through services in src/services/.",
                    ),
                    "arch/no-circular-dependencies" => (
                        "No Circular Dependencies",
                        "architecture",
                        "Module import graph must be an acyclic directed graph (DAG).",
                        "Refactor cyclic imports into shared contracts or interface abstractions.",
                    ),
                    "quality/require-service-tests" => (
                        "Require Service Tests",
                        "quality",
                        "Application services should have companion test files.",
                        "Add companion test file for uncovered services.",
                    ),
                    "quality/max-file-lines" => (
                        "Max File Lines",
                        "quality",
                        "Source files should not exceed configured line limits.",
                        "Split file into smaller single-responsibility modules.",
                    ),
                    _ => (
                        "Built-in Rule",
                        "quality",
                        "Engine rule",
                        "Follow best practices.",
                    ),
                };

                rules.push(McpRuleDescriptor {
                    id: r.id.clone(),
                    name: name.to_string(),
                    category: category.to_string(),
                    severity: r.severity,
                    description: desc.to_string(),
                    remediation: remed.to_string(),
                    source: "builtin".to_string(),
                    enabled: r.enabled,
                });
            }

            for d in engine.declarative_rules() {
                rules.push(McpRuleDescriptor {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    category: d.category.as_str().to_string(),
                    severity: d.severity,
                    description: d.description.clone(),
                    remediation: d.remediation.clone(),
                    source: d.source.to_string(),
                    enabled: d.enabled,
                });
            }

            let category_arg = arguments.get("category").and_then(Value::as_str);
            if let Some(cat) = category_arg {
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

            let enabled_only = arguments
                .get("enabled_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if enabled_only {
                rules.retain(|r| r.enabled);
            }

            rules.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));

            let total_rules = rules.len();
            let enabled_rules = rules.iter().filter(|r| r.enabled).count();

            let raw_texts: Vec<&str> = category_arg.into_iter().collect();

            to_envelope_json(
                json!({
                    "total_rules": total_rules,
                    "enabled_rules": enabled_rules,
                    "extended_packs": extended_packs,
                    "rules": rules,
                }),
                &raw_texts,
            )
        }
        "eforge_project_info" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let repo = ctx.repo;
            to_envelope_json(
                json!({
                    "name": repo.name,
                    "languages": repo.detected_languages,
                    "frameworks": repo.detected_frameworks,
                    "package_managers": repo.package_managers,
                    "file_count": repo.files.len(),
                    "module_count": repo.modules.len(),
                    "dependency_count": repo.dependencies.len(),
                    "finding_count": repo.findings.len(),
                    "health": repo.health,
                }),
                &[],
            )
        }
        "eforge_get_architecture" => {
            let ctx = build_analyzed_repo(repo_root)?;
            to_envelope_json(
                json!({
                    "modules": ctx.repo.modules,
                    "circular_dependencies": ctx.cycles,
                }),
                &[],
            )
        }
        "eforge_get_dependencies" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let dep_findings: Vec<&Finding> = ctx
                .repo
                .findings
                .iter()
                .filter(|f| f.rule_id.starts_with("deps/"))
                .collect();
            to_envelope_json(
                json!({
                    "dependencies": ctx.repo.dependencies,
                    "findings": dep_findings,
                }),
                &[],
            )
        }
        "eforge_get_findings" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let sev_filter = arguments
                .get("severity")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase);
            let prefix_filter = arguments.get("rule_prefix").and_then(Value::as_str);

            let filtered: Vec<&Finding> = ctx
                .repo
                .findings
                .iter()
                .filter(|f| {
                    let sev_ok = sev_filter.as_deref().map_or(true, |s| {
                        format!("{:?}", f.severity).to_ascii_lowercase() == s
                    });
                    let pref_ok = prefix_filter.map_or(true, |p| f.rule_id.starts_with(p));
                    sev_ok && pref_ok
                })
                .collect();

            to_envelope_json(
                json!({
                    "total_count": filtered.len(),
                    "findings": filtered,
                }),
                &[],
            )
        }
        "eforge_get_change_impact" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let repo = ctx.repo;
            let diff_text = match arguments.get("diff_text").and_then(Value::as_str) {
                Some(text) => text.to_string(),
                None => collect_working_tree_diff(repo_root).unwrap_or_default(),
            };
            let changes = parse_unified_diff(&diff_text)?;
            let impact = compute_change_impact(&changes, &repo.modules, &ctx.test_files);
            let changed_paths: BTreeSet<&str> = changes.iter().map(|c| c.path.as_str()).collect();
            let diff_findings: Vec<Finding> = repo
                .findings
                .iter()
                .filter(|f| {
                    f.location
                        .as_ref()
                        .is_some_and(|loc| changed_paths.contains(loc.file_path.as_str()))
                })
                .cloned()
                .collect();
            let total_affected =
                impact.directly_affected_modules.len() + impact.transitively_affected_modules.len();
            let risk = calculate_change_risk(
                &diff_findings,
                changes.len(),
                total_affected,
                impact.changed_files_without_tests.len(),
            );
            to_envelope_json(
                json!({
                    "impact": impact,
                    "risk": risk,
                }),
                &[&diff_text],
            )
        }
        "eforge_explain_module" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let repo = ctx.repo;
            let target = arguments
                .get("target")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim_matches('/');
            if target.is_empty() {
                return Err(ForgeError::Config {
                    message: "Missing required 'target' argument for eforge_explain_module"
                        .to_string(),
                });
            }
            let matched_modules: Vec<_> = repo
                .modules
                .iter()
                .filter(|m| m.id == target || m.root_path.starts_with(target))
                .collect();
            let outbound: BTreeSet<String> = matched_modules
                .iter()
                .flat_map(|m| m.imports.iter().cloned())
                .collect();
            let inbound: BTreeSet<String> = repo
                .modules
                .iter()
                .filter(|m| {
                    m.imports
                        .iter()
                        .any(|imp| imp == target || imp.starts_with(target))
                })
                .map(|m| m.id.clone())
                .collect();
            let files: Vec<&str> = repo
                .files
                .iter()
                .filter(|f| f.relative_path == target || f.relative_path.starts_with(target))
                .map(|f| f.relative_path.as_str())
                .collect();
            let findings: Vec<&Finding> = repo
                .findings
                .iter()
                .filter(|f| {
                    f.location.as_ref().is_some_and(|loc| {
                        loc.file_path == target || loc.file_path.starts_with(target)
                    })
                })
                .collect();
            to_envelope_json(
                json!({
                    "target": target,
                    "matched_modules": matched_modules.iter().map(|m| &m.id).collect::<Vec<_>>(),
                    "files": files,
                    "inbound_dependents": inbound,
                    "outbound_dependencies": outbound,
                    "findings": findings,
                }),
                &[target],
            )
        }
        "eforge_verify_patch" => {
            let diff_text = arguments
                .get("diff_text")
                .and_then(Value::as_str)
                .ok_or_else(|| ForgeError::Config {
                    message: "Missing required 'diff_text' argument for eforge_verify_patch"
                        .to_string(),
                })?;

            let ctx = build_analyzed_repo(repo_root)?;
            let changes = parse_unified_diff(diff_text)?;
            let file_edges = extract_file_import_edges(repo_root, &ctx.repo.files)?;
            let impact = compute_change_impact(&changes, &ctx.repo.modules, &ctx.test_files);
            let impacted_tests = compute_impacted_tests(
                &changes,
                &ctx.repo.modules,
                &file_edges,
                &ctx.test_files,
                &ctx.repo.detected_languages,
                &ctx.repo.package_managers,
            );

            // Scan added lines inside the diff for secrets and unsafe patterns in memory
            let added_lines_only: String = diff_text
                .lines()
                .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
                .map(|l| &l[1..])
                .collect::<Vec<_>>()
                .join("\n");
            let patch_security_findings =
                SecurityScanner::scan_file_content("proposed-patch.ts", &added_lines_only);

            let approved = patch_security_findings.is_empty();
            let baseline_health = ctx.repo.health.as_ref().map_or(100, |h| h.overall);
            let penalty = (patch_security_findings.len() * 15).min(100) as u8;
            let projected_health = baseline_health.saturating_sub(penalty);

            to_envelope_json(
                json!({
                    "approved": approved,
                    "baseline_health_score": baseline_health,
                    "projected_health_score": projected_health,
                    "health_delta": projected_health as i16 - baseline_health as i16,
                    "patch_security_findings": patch_security_findings,
                    "impact": impact,
                    "impacted_tests": impacted_tests,
                }),
                &[diff_text],
            )
        }
        "eforge_simulate_refactor" => {
            let from = arguments
                .get("from")
                .and_then(Value::as_str)
                .ok_or_else(|| ForgeError::Config {
                    message: "Missing required 'from' argument for eforge_simulate_refactor"
                        .to_string(),
                })?;
            let to =
                arguments
                    .get("to")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ForgeError::Config {
                        message: "Missing required 'to' argument for eforge_simulate_refactor"
                            .to_string(),
                    })?;

            let config = load_config(repo_root)?;
            let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
            let is_file_move = discovery.files.iter().any(|f| f.relative_path == from);
            let op = if is_file_move {
                RefactorOperation::MoveFile {
                    from: from.to_string(),
                    to: to.to_string(),
                }
            } else {
                RefactorOperation::MoveModule {
                    from_prefix: from.to_string(),
                    to_prefix: to.to_string(),
                }
            };

            let sim_report = simulate_refactor(repo_root, &discovery.files, &[op])?;
            to_envelope_json(sim_report, &[from, to])
        }
        "eforge_get_impacted_tests" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let diff_text = match arguments.get("diff_text").and_then(Value::as_str) {
                Some(text) => text.to_string(),
                None => collect_working_tree_diff(repo_root).unwrap_or_default(),
            };
            let changes = parse_unified_diff(&diff_text)?;
            let file_edges = extract_file_import_edges(repo_root, &ctx.repo.files)?;
            let report = compute_impacted_tests(
                &changes,
                &ctx.repo.modules,
                &file_edges,
                &ctx.test_files,
                &ctx.repo.detected_languages,
                &ctx.repo.package_managers,
            );
            to_envelope_json(report, &[&diff_text])
        }
        "eforge_get_pruned_context" => {
            let ctx = build_analyzed_repo(repo_root)?;
            let focal_targets: Vec<String> = arguments
                .get("focal_targets")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            let token_budget = arguments
                .get("token_budget")
                .and_then(Value::as_u64)
                .unwrap_or(4000) as u32;
            let max_hops = arguments
                .get("max_hops")
                .and_then(Value::as_u64)
                .unwrap_or(2) as usize;

            let pruned =
                build_pruned_centrality_context(&ctx.repo, &focal_targets, max_hops, token_budget);
            to_envelope_json(pruned, &[])
        }
        other => Err(ForgeError::Config {
            message: format!("Unknown MCP tool '{other}'"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_next_app() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn list_mcp_tools_exposes_eleven_eforge_tools() {
        let tools = list_mcp_tools();
        assert_eq!(tools.len(), 11);
        assert!(tools.iter().all(|t| t.name.starts_with("eforge_")));
        assert!(tools.iter().any(|t| t.name == "eforge_get_rules"));
        assert!(tools.iter().any(|t| t.name == "eforge_verify_patch"));
        assert!(tools.iter().any(|t| t.name == "eforge_simulate_refactor"));
        assert!(tools.iter().any(|t| t.name == "eforge_get_impacted_tests"));
        assert!(tools.iter().any(|t| t.name == "eforge_get_pruned_context"));
    }

    #[test]
    fn call_mcp_tools_returns_untrusted_envelope_on_fixture() {
        let root = fixture_next_app();
        let info = call_mcp_tool(&root, "eforge_project_info", &json!({}));
        assert!(info.is_ok());
        if let Ok(val) = info {
            assert_eq!(val["trust_level"], "untrusted_repository_data");
            assert!(val["payload"]["file_count"].as_u64().unwrap_or(0) >= 4);
        }

        let explain = call_mcp_tool(
            &root,
            "eforge_explain_module",
            &json!({"target": "src/services"}),
        );
        assert!(explain.is_ok());
        if let Ok(val) = explain {
            assert_eq!(val["payload"]["target"], "src/services");
        }
    }

    #[test]
    fn call_mcp_preflight_verify_and_simulate_work_in_memory() {
        let root = fixture_next_app();
        let diff = "--- a/src/db/client.ts\n+++ b/src/db/client.ts\n@@ -1,2 +1,3 @@\n+export const clean = true;\n";
        let verify = call_mcp_tool(&root, "eforge_verify_patch", &json!({"diff_text": diff}));
        assert!(verify.is_ok());
        if let Ok(val) = verify {
            assert_eq!(val["payload"]["approved"], true);
        }

        let sim = call_mcp_tool(
            &root,
            "eforge_simulate_refactor",
            &json!({"from": "src/db/client.ts", "to": "src/services/client.ts"}),
        );
        assert!(sim.is_ok());
        if let Ok(val) = sim {
            assert_eq!(val["payload"]["is_safe_to_apply"], true);
        }

        let pruned = call_mcp_tool(
            &root,
            "eforge_get_pruned_context",
            &json!({"focal_targets": ["src/services"], "token_budget": 1000, "max_hops": 1}),
        );
        assert!(pruned.is_ok());
        if let Ok(val) = pruned {
            assert!(!val["payload"]["included_modules"]
                .as_array()
                .map_or(true, Vec::is_empty));
        }
    }

    #[test]
    fn call_mcp_get_rules_returns_envelope_and_filters_rules() {
        let root = fixture_next_app();
        let rules_res = call_mcp_tool(&root, "eforge_get_rules", &json!({}));
        assert!(rules_res.is_ok());
        if let Ok(val) = rules_res {
            assert_eq!(val["trust_level"], "untrusted_repository_data");
            let total = val["payload"]["total_rules"].as_u64().unwrap_or(0);
            assert!(total >= 4);
            let rules = val["payload"]["rules"].as_array();
            assert!(rules.is_some());
        }

        let filtered_res = call_mcp_tool(
            &root,
            "eforge_get_rules",
            &json!({"category": "architecture", "enabled_only": true}),
        );
        assert!(filtered_res.is_ok());
        if let Ok(val) = filtered_res {
            let rules = val["payload"]["rules"].as_array();
            assert!(rules.is_some());
            if let Some(arr) = rules {
                assert!(arr.iter().all(|r| r["category"] == "architecture"));
            }
        }
    }
}
