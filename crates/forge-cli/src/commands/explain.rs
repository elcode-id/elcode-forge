//! `eforge explain [target]` command — deterministic structural explanation of a project, module, or file,
//! optionally augmented with structured AI interpretation and dry-run context inspection.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use forge_core::{Finding, ForgeError, Module};
use serde::{Deserialize, Serialize};

use crate::ai::orchestrate::{
    execute_explain_ai, AiExplainOptions, AiExplainOutput, ExplainAiResult,
};
use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};

/// Deterministic structural explanation of a repository, module, or file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainReport {
    pub target: String,
    pub summary: String,
    pub files: Vec<String>,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub exports: Vec<String>,
    pub related_findings: Vec<Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiExplainOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<forge_ai::ContextPack>,
}

fn matches_target(candidate: &str, target: &str) -> bool {
    if target.is_empty() || target == "." {
        return true;
    }
    candidate == target
        || candidate
            .strip_prefix(target)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Computes a deterministic [`ExplainReport`] for `repo_root` and optional `target_module_or_file`.
pub fn execute_explain(
    repo_root: &Path,
    target_module_or_file: Option<&str>,
) -> Result<ExplainReport, ForgeError> {
    execute_explain_with_ai(
        repo_root,
        target_module_or_file,
        &AiExplainOptions::default(),
    )
}

/// Computes an [`ExplainReport`] with optional AI augmentation or `--dry-run` inspection.
pub fn execute_explain_with_ai(
    repo_root: &Path,
    target_module_or_file: Option<&str>,
    ai_opts: &AiExplainOptions,
) -> Result<ExplainReport, ForgeError> {
    let opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&opts)?;

    let normalized_target = target_module_or_file
        .unwrap_or(".")
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    let effective_target = if normalized_target.is_empty() {
        ".".to_string()
    } else {
        normalized_target
    };

    let files: Vec<String> = repo
        .files
        .iter()
        .filter(|f| matches_target(&f.relative_path, &effective_target))
        .map(|f| f.relative_path.clone())
        .collect();

    let matched_modules: Vec<&Module> = repo
        .modules
        .iter()
        .filter(|m| {
            matches_target(&m.id, &effective_target)
                || matches_target(&m.root_path, &effective_target)
                || m.files.iter().any(|f| matches_target(f, &effective_target))
        })
        .collect();

    let mut imports_set = BTreeSet::new();
    let mut exports_set = BTreeSet::new();
    for m in &matched_modules {
        for imp in &m.imports {
            imports_set.insert(imp.clone());
        }
        for exp in &m.exports {
            exports_set.insert(exp.clone());
        }
    }

    let matched_ids: BTreeSet<&str> = matched_modules.iter().map(|m| m.id.as_str()).collect();
    let imported_by: Vec<String> = repo
        .modules
        .iter()
        .filter(|m| {
            !matched_ids.contains(m.id.as_str())
                && m.imports
                    .iter()
                    .any(|imp| matched_ids.contains(imp.as_str()))
        })
        .map(|m| m.id.clone())
        .collect();

    let related_findings: Vec<Finding> = repo
        .findings
        .iter()
        .filter(|f| {
            if effective_target == "." {
                return true;
            }
            f.location
                .as_ref()
                .is_some_and(|loc| matches_target(&loc.file_path, &effective_target))
        })
        .cloned()
        .collect();

    let imports: Vec<String> = imports_set.into_iter().collect();
    let exports: Vec<String> = exports_set.into_iter().collect();

    let summary = if effective_target == "." {
        format!(
            "Repository '{}' contains {} file(s) across {} module(s), with {} outgoing module edge(s) and {} finding(s).",
            repo.name,
            files.len(),
            repo.modules.len(),
            imports.len(),
            related_findings.len()
        )
    } else {
        format!(
            "Target '{}' spans {} file(s) across {} module(s), imports {} module(s), is imported by {} module(s), exports {} symbol(s), and has {} finding(s).",
            effective_target,
            files.len(),
            matched_modules.len(),
            imports.len(),
            imported_by.len(),
            exports.len(),
            related_findings.len()
        )
    };

    let mut report = ExplainReport {
        target: effective_target.clone(),
        summary,
        files,
        imports,
        imported_by,
        exports,
        related_findings,
        ai: None,
        dry_run: None,
    };

    // If dry_run or AI execution is requested
    if ai_opts.dry_run || ai_opts.ai {
        let ai_res = execute_explain_ai(repo_root, &repo, &effective_target, ai_opts)?;
        match ai_res {
            ExplainAiResult::DryRun(pack) => report.dry_run = Some(pack),
            ExplainAiResult::Output(out) => report.ai = Some(out),
        }
    }

    Ok(report)
}

/// Renders an [`ExplainReport`] in Terminal, JSON, or Markdown format.
pub fn render_explain_report(
    report: &ExplainReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut out = String::new();
            let _ = writeln!(out, "# ELCODE Forge — Explain `{}`\n", report.target);
            let _ = writeln!(out, "{}\n", report.summary);

            let _ = writeln!(out, "## Files ({})", report.files.len());
            for f in &report.files {
                let _ = writeln!(out, "- `{f}`");
            }
            let _ = writeln!(
                out,
                "\n## Outgoing Module Imports ({})",
                report.imports.len()
            );
            for imp in &report.imports {
                let _ = writeln!(out, "- `{imp}`");
            }
            let _ = writeln!(
                out,
                "\n## Incoming Dependents ({})",
                report.imported_by.len()
            );
            for dep in &report.imported_by {
                let _ = writeln!(out, "- `{dep}`");
            }
            let _ = writeln!(out, "\n## Exported Symbols ({})", report.exports.len());
            for exp in &report.exports {
                let _ = writeln!(out, "- `{exp}`");
            }
            let _ = writeln!(
                out,
                "\n## Related Findings ({})",
                report.related_findings.len()
            );
            for finding in &report.related_findings {
                let _ = writeln!(
                    out,
                    "- **[{:?}]** `{}` — {}",
                    finding.severity, finding.rule_id, finding.title
                );
            }

            if let Some(dry) = &report.dry_run {
                let _ = writeln!(out, "\n## AI Context Dry Run");
                let _ = writeln!(out, "- **Estimated Tokens:** {}", dry.estimated_tokens);
                let _ = writeln!(
                    out,
                    "- **Included Sections ({}):** {}",
                    dry.included.len(),
                    dry.included.join(", ")
                );
                let _ = writeln!(
                    out,
                    "- **Dropped Sections ({}):** {}",
                    dry.dropped.len(),
                    if dry.dropped.is_empty() {
                        "none".to_string()
                    } else {
                        dry.dropped.join(", ")
                    }
                );
                let _ = writeln!(
                    out,
                    "- **Prompt Injection Signals ({}):** {}",
                    dry.injection_signals.len(),
                    if dry.injection_signals.is_empty() {
                        "none".to_string()
                    } else {
                        dry.injection_signals.join(", ")
                    }
                );
            }

            if let Some(ai) = &report.ai {
                let _ = writeln!(out, "\n## AI Architectural Interpretation");
                let _ = writeln!(
                    out,
                    "> Generated by `{}/{}` (Tokens: {}, Cached: {})\n",
                    ai.provider,
                    ai.model,
                    ai.token_usage.total(),
                    ai.cached
                );
                let _ = writeln!(out, "### Summary\n{}\n", ai.answer.summary);

                let _ = writeln!(out, "### Responsibilities");
                for r in &ai.answer.responsibilities {
                    let _ = writeln!(out, "- {r}");
                }

                let _ = writeln!(out, "\n### Key Dependencies");
                for d in &ai.answer.key_dependencies {
                    let _ = writeln!(out, "- {d}");
                }

                let _ = writeln!(out, "\n### Architectural Risks & Security Warnings");
                for risk in &ai.answer.risks {
                    let tag = if risk.verified {
                        "verified"
                    } else {
                        "unverified"
                    };
                    let _ = writeln!(out, "- **[{tag}]** `{}` — {}", risk.source, risk.text);
                }

                let _ = writeln!(out, "\n### Sources");
                for s in &ai.answer.sources {
                    let _ = writeln!(out, "- `{s}`");
                }
            }

            Ok(out)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            let _ = writeln!(out, "ELCODE Forge — Explain: {}", report.target);
            let _ = writeln!(out, "Summary: {}", report.summary);
            let _ = writeln!(
                out,
                "Files ({}): {}",
                report.files.len(),
                report.files.join(", ")
            );
            let _ = writeln!(
                out,
                "Imports ({}): {}",
                report.imports.len(),
                if report.imports.is_empty() {
                    "none".to_string()
                } else {
                    report.imports.join(", ")
                }
            );
            let _ = writeln!(
                out,
                "Imported By ({}): {}",
                report.imported_by.len(),
                if report.imported_by.is_empty() {
                    "none".to_string()
                } else {
                    report.imported_by.join(", ")
                }
            );
            let _ = writeln!(
                out,
                "Exports ({}): {}",
                report.exports.len(),
                if report.exports.is_empty() {
                    "none".to_string()
                } else {
                    report.exports.join(", ")
                }
            );
            let _ = writeln!(out, "Related Findings: {}", report.related_findings.len());

            if let Some(dry) = &report.dry_run {
                let _ = writeln!(out, "\n--- AI Context Dry Run ---");
                let _ = writeln!(out, "Estimated Tokens: {}", dry.estimated_tokens);
                let _ = writeln!(
                    out,
                    "Included Sections ({}): {}",
                    dry.included.len(),
                    dry.included.join(", ")
                );
                let _ = writeln!(
                    out,
                    "Dropped Sections ({}): {}",
                    dry.dropped.len(),
                    if dry.dropped.is_empty() {
                        "none".to_string()
                    } else {
                        dry.dropped.join(", ")
                    }
                );
                let _ = writeln!(
                    out,
                    "Prompt Injection Signals ({}): {}",
                    dry.injection_signals.len(),
                    if dry.injection_signals.is_empty() {
                        "none".to_string()
                    } else {
                        dry.injection_signals.join(", ")
                    }
                );
            }

            if let Some(ai) = &report.ai {
                let _ = writeln!(out, "\n--- AI Architectural Interpretation ---");
                let _ = writeln!(
                    out,
                    "Model: {}/{} (Tokens: {}, Cached: {})",
                    ai.provider,
                    ai.model,
                    ai.token_usage.total(),
                    ai.cached
                );
                let _ = writeln!(out, "Summary: {}", ai.answer.summary);
                let _ = writeln!(out, "\nResponsibilities:");
                for r in &ai.answer.responsibilities {
                    let _ = writeln!(out, "  - {r}");
                }
                let _ = writeln!(out, "\nKey Dependencies:");
                for d in &ai.answer.key_dependencies {
                    let _ = writeln!(out, "  - {d}");
                }
                let _ = writeln!(out, "\nArchitectural Risks:");
                for risk in &ai.answer.risks {
                    let tag = if risk.verified {
                        "verified"
                    } else {
                        "unverified"
                    };
                    let _ = writeln!(out, "  - [{tag}] {}: {}", risk.source, risk.text);
                }
                let _ = writeln!(out, "\nSources: {}", ai.answer.sources.join(", "));
            }

            Ok(out)
        }
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
    fn explain_module_resolves_files_imports_dependents_and_findings() {
        let root = fixture_next_app();
        let report = execute_explain(&root, Some("src/services"));
        assert!(report.is_ok());
        if let Ok(rep) = report {
            assert_eq!(rep.target, "src/services");
            assert!(rep.files.iter().any(|f| f == "src/services/userService.ts"));
            assert!(rep.imports.iter().any(|m| m == "src/db"));
            assert!(rep.imported_by.iter().any(|m| m == "src/db"));

            let md = render_explain_report(&rep, OutputFormat::Markdown);
            assert!(md.is_ok());
            if let Ok(text) = md {
                assert!(text.contains("# ELCODE Forge — Explain `src/services`"));
            }
        }
    }

    #[test]
    fn explain_with_ai_dry_run_produces_context_pack_without_network() {
        let root = fixture_next_app();
        let ai_opts = AiExplainOptions {
            ai: false,
            provider: None,
            model: None,
            dry_run: true,
            no_cache: true,
        };
        let report = execute_explain_with_ai(&root, Some("src/services"), &ai_opts);
        assert!(report.is_ok());
        if let Ok(rep) = report {
            assert!(rep.dry_run.is_some());
            assert!(rep.ai.is_none());
            if let Some(ref pack) = rep.dry_run {
                assert!(pack.estimated_tokens > 0);
                assert!(!pack.included.is_empty());
                assert!(pack.system_prompt.contains("ELCODE Forge"));
            }

            let term = render_explain_report(&rep, OutputFormat::Terminal);
            assert!(term.is_ok());
            if let Ok(rendered) = term {
                assert!(rendered.contains("--- AI Context Dry Run ---"));
                assert!(rendered.contains("Estimated Tokens:"));
            }
        }
    }

    #[test]
    fn explain_with_ai_mock_provider_executes_successfully() {
        let root = fixture_next_app();
        let ai_opts = AiExplainOptions {
            ai: true,
            provider: Some("mock".to_string()),
            model: Some("mock-v1".to_string()),
            dry_run: false,
            no_cache: true,
        };
        let report = execute_explain_with_ai(&root, Some("src/services"), &ai_opts);
        assert!(report.is_ok());
        if let Ok(rep) = report {
            assert!(rep.dry_run.is_none());
            assert!(rep.ai.is_some());
            if let Some(ref ai) = rep.ai {
                assert_eq!(ai.provider, "mock");
                assert_eq!(ai.model, "mock-v1");
                assert!(!ai.cached);
                assert!(!ai.answer.summary.is_empty());
                assert_eq!(ai.answer.risks.len(), 1);
                assert!(ai.answer.risks[0].verified);
            }

            let term = render_explain_report(&rep, OutputFormat::Terminal);
            assert!(term.is_ok());
            if let Ok(rendered) = term {
                assert!(rendered.contains("--- AI Architectural Interpretation ---"));
                assert!(rendered.contains("Model: mock/mock-v1"));
            }
        }
    }
}
