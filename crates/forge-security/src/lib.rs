//! `forge-security` — Secret detection (with mandatory redaction & Shannon entropy analysis),
//! unsafe code pattern analysis, and polyglot OSV dependency vulnerability intelligence.

use std::fs;
use std::path::Path;

use forge_core::config::SecurityConfig;
use forge_core::{Dependency, FileNode, Finding, ForgeError};

pub mod entropy;
pub mod license;
pub mod osv;
pub mod patterns;
pub mod php_patterns;
pub mod python_patterns;
pub mod sbom;
pub mod secrets;

pub use entropy::{
    calculate_shannon_entropy, find_high_entropy_tokens, is_hex, is_high_entropy,
    tokenize_candidate_secrets, DEFAULT_ENTROPY_THRESHOLD, DEFAULT_MIN_TOKEN_LEN,
};
pub use forge_core::Analyzer;
pub use license::{
    categorize_license, detect_dependency_license, detect_project_license,
    evaluate_license_compliance, LicenseCategory, ProjectLicenseInfo, RULE_DENIED_LICENSE,
    RULE_DISALLOWED_COPYLEFT, RULE_MISSING_PROJECT_LICENSE, RULE_UNAPPROVED_LICENSE,
};
pub use osv::{
    normalize_ecosystem, normalize_package_name, LocalAdvisoryCache, OsvAdvisory, Version,
    VulnerabilityMatcher, RULE_VULNERABLE_DEPENDENCY,
};
pub use patterns::{
    scan_unsafe_patterns, RULE_NO_DANGEROUS_INNER_HTML, RULE_NO_EVAL, RULE_NO_IMPLIED_EVAL,
    RULE_PERMISSIVE_CORS, RULE_UNSAFE_SHELL_EXEC,
};
pub use php_patterns::{
    scan_php_unsafe_patterns, RULE_PHP_COMMAND_INJECTION, RULE_PHP_CREATE_FUNCTION,
    RULE_PHP_INSECURE_UNSERIALIZE, RULE_PHP_UNSAFE_EVAL,
};
pub use python_patterns::{
    scan_python_unsafe_patterns, RULE_PYTHON_INSECURE_SUBPROCESS,
    RULE_PYTHON_PICKLE_DESERIALIZATION, RULE_PYTHON_UNSAFE_EVAL,
};
pub use sbom::{
    generate_cyclonedx_sbom, generate_purl, generate_sbom, generate_spdx_sbom, SbomFormat,
};
pub use secrets::{
    is_lockfile, is_path_allowlisted, redact_secrets, scan_secrets, scan_secrets_with_config,
    RULE_HARDCODED_SECRET, RULE_HIGH_ENTROPY_SECRET, RULE_PRIVATE_KEY,
};

/// Deterministic security scanner combining redacted secret detection, Shannon entropy scanning,
/// unsafe pattern checks, and OSV dependency vulnerability intelligence.
#[derive(Debug, Default, Clone, Copy)]
pub struct SecurityScanner;

impl SecurityScanner {
    /// Scans an individual file's `content` for secrets and unsafe patterns using default configuration.
    pub fn scan_file_content(relative_path: &str, content: &str) -> Vec<Finding> {
        Self::scan_file_content_with_config(relative_path, content, None)
    }

    /// Scans an individual file's `content` for secrets and unsafe patterns with configurable settings.
    pub fn scan_file_content_with_config(
        relative_path: &str,
        content: &str,
        config: Option<&SecurityConfig>,
    ) -> Vec<Finding> {
        let mut findings = scan_secrets_with_config(relative_path, content, config);
        findings.extend(scan_unsafe_patterns(relative_path, content));
        findings.extend(scan_python_unsafe_patterns(relative_path, content));
        findings.extend(scan_php_unsafe_patterns(relative_path, content));
        findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        findings
    }

    /// Scans all text source files in `files` within `repo_root` using default configuration and returns deterministically
    /// sorted security [`Finding`]s.
    pub fn scan_repository_files(
        repo_root: &Path,
        files: &[FileNode],
    ) -> Result<Vec<Finding>, ForgeError> {
        Self::scan_repository_files_with_config(repo_root, files, None)
    }

    /// Scans all text source files in `files` within `repo_root` with configurable security policy and returns deterministically
    /// sorted security [`Finding`]s.
    pub fn scan_repository_files_with_config(
        repo_root: &Path,
        files: &[FileNode],
        config: Option<&SecurityConfig>,
    ) -> Result<Vec<Finding>, ForgeError> {
        let mut all_findings = Vec::new();

        for file in files {
            if is_lockfile(&file.relative_path) {
                continue;
            }
            if let Some(cfg) = config {
                if is_path_allowlisted(&file.relative_path, &cfg.allowlist) {
                    continue;
                }
            }

            let full_path = repo_root.join(&file.relative_path);
            let Ok(content) = fs::read_to_string(&full_path) else {
                continue;
            };
            all_findings.extend(Self::scan_file_content_with_config(
                &file.relative_path,
                &content,
                config,
            ));
        }

        all_findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        Ok(all_findings)
    }

    /// Evaluates resolved repository dependencies against OSV advisories and returns
    /// deterministically sorted vulnerability [`Finding`]s.
    pub fn scan_dependencies(repo_root: &Path, dependencies: &[Dependency]) -> Vec<Finding> {
        let matcher = VulnerabilityMatcher::load(repo_root);
        matcher.match_dependencies(dependencies)
    }

    /// Evaluates project and dependency license compliance against policies.
    pub fn scan_license_compliance(
        repo_root: &Path,
        policy: Option<&forge_core::config::LicenseConfig>,
        dependencies: &[Dependency],
    ) -> Vec<Finding> {
        evaluate_license_compliance(repo_root, policy, dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_and_redacts_secrets_and_unsafe_patterns_in_vulnerable_fixture() {
        let fixture_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("vulnerable-ts-app")
            .join("src")
            .join("unsafe.ts");

        let content = fs::read_to_string(&fixture_file);
        assert!(content.is_ok());

        if let Ok(src) = content {
            let findings = SecurityScanner::scan_file_content("src/unsafe.ts", &src);
            let rule_ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();

            assert!(rule_ids.contains(&RULE_HARDCODED_SECRET));
            assert!(rule_ids.contains(&RULE_NO_EVAL));
            assert!(rule_ids.contains(&RULE_UNSAFE_SHELL_EXEC));
            assert!(rule_ids.contains(&RULE_PERMISSIVE_CORS));

            // Verify that raw secret strings are NEVER exposed in finding evidence
            for f in &findings {
                if f.rule_id == RULE_HARDCODED_SECRET || f.rule_id == RULE_HIGH_ENTROPY_SECRET {
                    assert!(f.evidence.contains("****REDACTED****"));
                    assert!(!f.evidence.contains("1234567890abcdefghijklmnop"));
                }
            }
        }
    }
}
