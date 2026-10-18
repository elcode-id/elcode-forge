use std::collections::BTreeSet;
use std::path::Path;

use forge_core::{Confidence, Dependency, Finding, ForgeError, Severity, SourceLocation};
use serde::{Deserialize, Serialize};

use crate::package_json::parse_package_json;

/// Implicit framework/runtime packages that are often invoked by build tools or framework runtimes
/// without an explicit `import` statement in every source file.
const IMPLICIT_RUNTIME_PACKAGES: &[&str] = &[
    "next",
    "nuxt",
    "react-dom",
    "@sveltejs/kit",
    "svelte",
    "vue",
    "typescript",
];

/// Aggregated dependency health report for a Node/TypeScript repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyReport {
    pub dependencies: Vec<Dependency>,
    pub unused_dependencies: Vec<String>,
    pub duplicate_packages: Vec<String>,
    pub wildcard_or_unpinned: Vec<String>,
    pub findings: Vec<Finding>,
}

/// Analyzes `<repo_root>/package.json` against the set of `imported_packages` observed in source
/// files to detect unused dependencies, unpinned/wildcard version specifiers, and duplicate
/// dependency declarations.
pub fn analyze_npm_dependencies(
    repo_root: &Path,
    imported_packages: &BTreeSet<String>,
) -> Result<DependencyReport, ForgeError> {
    let manifest = parse_package_json(repo_root)?;

    let mut unused = BTreeSet::new();
    let mut wildcard_or_unpinned = BTreeSet::new();
    let mut findings = Vec::new();

    for dep in &manifest.dependencies {
        if is_unpinned_or_wildcard(&dep.version_req) {
            wildcard_or_unpinned.insert(dep.name.clone());
            findings.push(Finding {
                id: format!("DEP-UNPINNED-{}", dep.name),
                category: "dependencies".to_string(),
                severity: Severity::Medium,
                confidence: Confidence::Certain,
                title: format!("Unpinned or wildcard dependency version for '{}'", dep.name),
                description: format!(
                    "Package '{}' uses non-deterministic version specifier '{}' in package.json.",
                    dep.name, dep.version_req
                ),
                evidence: format!("\"{}\": \"{}\"", dep.name, dep.version_req),
                location: Some(SourceLocation {
                    file_path: "package.json".to_string(),
                    start_line: None,
                    end_line: None,
                }),
                recommendation: format!(
                    "Pin '{}' to an explicit semver range (e.g., '^1.0.0' or exact version).",
                    dep.name
                ),
                rule_id: "deps/unpinned-version".to_string(),
            });
        }

        if !dep.is_dev
            && !IMPLICIT_RUNTIME_PACKAGES.contains(&dep.name.as_str())
            && !imported_packages.contains(&dep.name)
        {
            unused.insert(dep.name.clone());
            findings.push(Finding {
                id: format!("DEP-UNUSED-{}", dep.name),
                category: "dependencies".to_string(),
                severity: Severity::Low,
                confidence: Confidence::High,
                title: format!("Unused production dependency '{}'", dep.name),
                description: format!(
                    "Package '{}' is declared in 'dependencies' but never imported in scanned source files.",
                    dep.name
                ),
                evidence: format!("\"{}\": \"{}\"", dep.name, dep.version_req),
                location: Some(SourceLocation {
                    file_path: "package.json".to_string(),
                    start_line: None,
                    end_line: None,
                }),
                recommendation: format!(
                    "Remove '{}' from package.json if it is no longer needed.",
                    dep.name
                ),
                rule_id: "deps/unused-dependency".to_string(),
            });
        }
    }

    for dup in &manifest.duplicate_packages {
        findings.push(Finding {
            id: format!("DEP-DUPLICATE-{dup}"),
            category: "dependencies".to_string(),
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            title: format!("Duplicate dependency declaration for '{dup}'"),
            description: format!(
                "Package '{dup}' is declared in both 'dependencies' and 'devDependencies'."
            ),
            evidence: format!("Duplicate key '{dup}' in package.json"),
            location: Some(SourceLocation {
                file_path: "package.json".to_string(),
                start_line: None,
                end_line: None,
            }),
            recommendation: format!(
                "Keep '{dup}' in only one section ('dependencies' or 'devDependencies')."
            ),
            rule_id: "deps/duplicate-dependency".to_string(),
        });
    }

    Ok(DependencyReport {
        dependencies: manifest.dependencies,
        unused_dependencies: unused.into_iter().collect(),
        duplicate_packages: manifest.duplicate_packages,
        wildcard_or_unpinned: wildcard_or_unpinned.into_iter().collect(),
        findings,
    })
}

fn is_unpinned_or_wildcard(version_req: &str) -> bool {
    let trimmed = version_req.trim().to_ascii_lowercase();
    trimmed.is_empty()
        || trimmed == "*"
        || trimmed == "latest"
        || trimmed == "x"
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("git+")
        || trimmed.starts_with("github:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_unused_and_wildcard_dependencies_in_fixture_next_app() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let mut imported = BTreeSet::new();
        imported.insert("react".to_string());

        let report = analyze_npm_dependencies(&fixture, &imported);
        assert!(report.is_ok());

        if let Ok(r) = report {
            assert_eq!(
                r.unused_dependencies,
                vec!["legacy-unpinned-pkg", "unused-analytics-sdk"]
            );
            assert_eq!(r.wildcard_or_unpinned, vec!["legacy-unpinned-pkg"]);

            let rule_ids: Vec<&str> = r.findings.iter().map(|f| f.rule_id.as_str()).collect();
            assert!(rule_ids.contains(&"deps/unpinned-version"));
            assert!(rule_ids.contains(&"deps/unused-dependency"));
        }
    }
}
