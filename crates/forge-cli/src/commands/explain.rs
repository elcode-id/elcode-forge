//! `eforge explain [target]` command — deterministic structural explanation of a project, module, or file.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use forge_core::{Finding, ForgeError, Module};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};

/// Deterministic structural explanation of a repository, module, or file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainReport {
    pub target: String,
    pub summary: String,
    pub files: Vec<String>,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub exports: Vec<String>,
    pub related_findings: Vec<Finding>,
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
        .into_iter()
        .filter(|f| {
            if effective_target == "." {
                return true;
            }
            f.location
                .as_ref()
                .is_some_and(|loc| matches_target(&loc.file_path, &effective_target))
        })
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

    Ok(ExplainReport {
        target: effective_target,
        summary,
        files,
        imports,
        imported_by,
        exports,
        related_findings,
    })
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
}
