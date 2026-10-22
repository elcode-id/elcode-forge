//! `forge-security` — Secret detection (with mandatory redaction), unsafe code pattern analysis,
//! and polyglot OSV dependency vulnerability intelligence.

use std::fs;
use std::path::Path;

use forge_core::{Dependency, FileNode, Finding, ForgeError};

pub mod osv;
pub mod patterns;
pub mod php_patterns;
pub mod python_patterns;
pub mod secrets;

pub use forge_core::Analyzer;
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
pub use secrets::{redact_secrets, scan_secrets, RULE_HARDCODED_SECRET, RULE_PRIVATE_KEY};

/// Deterministic security scanner combining redacted secret detection, unsafe pattern checks,
/// and OSV dependency vulnerability intelligence.
#[derive(Debug, Default, Clone, Copy)]
pub struct SecurityScanner;

impl SecurityScanner {
    /// Scans an individual file's `content` for secrets and unsafe patterns (JS/TS and Python).
    pub fn scan_file_content(relative_path: &str, content: &str) -> Vec<Finding> {
        let mut findings = scan_secrets(relative_path, content);
        findings.extend(scan_unsafe_patterns(relative_path, content));
        findings.extend(scan_python_unsafe_patterns(relative_path, content));
        findings.extend(scan_php_unsafe_patterns(relative_path, content));
        findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        findings
    }

    /// Scans all text source files in `files` within `repo_root` and returns deterministically
    /// sorted security [`Finding`]s.
    pub fn scan_repository_files(
        repo_root: &Path,
        files: &[FileNode],
    ) -> Result<Vec<Finding>, ForgeError> {
        let mut all_findings = Vec::new();

        for file in files {
            let full_path = repo_root.join(&file.relative_path);
            let Ok(content) = fs::read_to_string(&full_path) else {
                continue;
            };
            all_findings.extend(Self::scan_file_content(&file.relative_path, &content));
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
                if f.rule_id == RULE_HARDCODED_SECRET {
                    assert!(f.evidence.contains("****REDACTED****"));
                    assert!(!f.evidence.contains("1234567890abcdefghijklmnop"));
                }
            }
        }
    }
}
