use std::collections::BTreeSet;
use std::path::Path;

use forge_analyzer::{
    dep_health::analyze_npm_dependencies, detector::is_test_file,
    graph::build_ts_architecture_graph, scanner::scan_repository,
};
use forge_core::{
    config::load_config,
    scoring::{calculate_change_risk, calculate_health_score},
    Finding, ForgeError, Repository,
};
use forge_git::{collect_working_tree_diff, compute_change_impact, parse_unified_diff};
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

/// Returns the deterministic catalog of `eforge_*` MCP tools.
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
    let arch = build_ts_architecture_graph(repo_root, &discovery.files)?;
    let dep_report = analyze_npm_dependencies(repo_root, &arch.external_imports)?;

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

    let engine = RuleEngine::with_config_overrides(&config.rules);
    let mut findings = engine.evaluate(&repo)?;
    findings.extend(dep_report.findings);
    findings.extend(SecurityScanner::scan_repository_files(
        repo_root,
        &repo.files,
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

/// Dispatches an `eforge_*` MCP tool invocation and wraps the result in an [`UntrustedEnvelope`].
pub fn call_mcp_tool(
    repo_root: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, ForgeError> {
    let ctx = build_analyzed_repo(repo_root)?;
    let repo = ctx.repo;
    let cycles = ctx.cycles;
    let test_files = ctx.test_files;

    match tool_name {
        "eforge_project_info" => to_envelope_json(
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
        ),
        "eforge_get_architecture" => to_envelope_json(
            json!({
                "modules": repo.modules,
                "circular_dependencies": cycles,
            }),
            &[],
        ),
        "eforge_get_dependencies" => {
            let dep_findings: Vec<&Finding> = repo
                .findings
                .iter()
                .filter(|f| f.rule_id.starts_with("deps/"))
                .collect();
            to_envelope_json(
                json!({
                    "dependencies": repo.dependencies,
                    "findings": dep_findings,
                }),
                &[],
            )
        }
        "eforge_get_findings" => {
            let sev_filter = arguments
                .get("severity")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase);
            let prefix_filter = arguments.get("rule_prefix").and_then(Value::as_str);

            let filtered: Vec<&Finding> = repo
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
            let diff_text = match arguments.get("diff_text").and_then(Value::as_str) {
                Some(text) => text.to_string(),
                None => collect_working_tree_diff(repo_root).unwrap_or_default(),
            };
            let changes = parse_unified_diff(&diff_text)?;
            let impact = compute_change_impact(&changes, &repo.modules, &test_files);
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
    fn list_mcp_tools_exposes_six_eforge_tools() {
        let tools = list_mcp_tools();
        assert_eq!(tools.len(), 6);
        assert!(tools.iter().all(|t| t.name.starts_with("eforge_")));
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
}
