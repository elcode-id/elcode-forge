//! `eforge security` command — focused secret and unsafe pattern security scan.

use std::path::Path;

use forge_analyzer::scanner::scan_repository;
use forge_core::{config::load_config, ForgeError, Repository};
use forge_security::SecurityScanner;

use crate::cli::OutputFormat;
use crate::exit_code::ExitCode;
use crate::formatters::render_repository_report;

/// Runs the dedicated security & secret scanner on `repo_root` and renders findings in `format`.
pub fn run_security(
    repo_root: &Path,
    format: OutputFormat,
) -> Result<(String, ExitCode), ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let findings = SecurityScanner::scan_repository_files(repo_root, &discovery.files)?;
    let exit_code = ExitCode::evaluate_findings(&findings, config.fail_on_severity);

    let repo = Repository {
        root_path: repo_root.to_string_lossy().replace('\\', "/"),
        name: config.project_name.unwrap_or_else(|| {
            repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repository")
                .to_string()
        }),
        detected_languages: discovery.languages,
        detected_frameworks: discovery.frameworks,
        package_managers: discovery.package_managers,
        files: discovery.files,
        modules: Vec::new(),
        dependencies: Vec::new(),
        findings,
        health: None,
    };

    let rendered = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&repo.findings).map_err(|err| {
            ForgeError::Serialization {
                message: err.to_string(),
            }
        })?,
        _ => render_repository_report(&repo, format)?,
    };

    Ok((rendered, exit_code))
}
