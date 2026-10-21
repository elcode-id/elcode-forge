use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use forge_core::{Confidence, Dependency, FileNode, Finding, ForgeError, Severity, SourceLocation};
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

/// Implicit Python runners, frameworks, or linters that are executed via CLI rather than direct source import.
const PYTHON_IMPLICIT_RUNNERS: &[&str] = &[
    "uvicorn",
    "gunicorn",
    "hypercorn",
    "daphne",
    "waitress",
    "celery",
    "pytest",
    "mypy",
    "ruff",
    "black",
    "flake8",
];

/// Aggregated dependency health report for a repository.
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

/// Analyzes Python manifests (`pyproject.toml`, `requirements.txt`, and lockfiles)
/// against the set of `imported_packages` observed in Python source files.
pub fn analyze_python_dependencies(
    repo_root: &Path,
    imported_packages: &BTreeSet<String>,
) -> Result<DependencyReport, ForgeError> {
    let manifest = crate::python_manifest::parse_python_manifests(repo_root)?;

    let mut unused = BTreeSet::new();
    let mut wildcard_or_unpinned = BTreeSet::new();
    let mut findings = Vec::new();

    let manifest_file = if repo_root.join("pyproject.toml").is_file() {
        "pyproject.toml"
    } else {
        "requirements.txt"
    };

    for dep in &manifest.dependencies {
        if crate::python_manifest::is_python_unpinned(&dep.version_req) {
            wildcard_or_unpinned.insert(dep.name.clone());
            findings.push(Finding {
                id: format!("DEP-PY-UNPINNED-{}", dep.name),
                category: "dependencies".to_string(),
                severity: Severity::Medium,
                confidence: Confidence::Certain,
                title: format!("Unpinned or wildcard dependency version for '{}'", dep.name),
                description: format!(
                    "Package '{}' uses non-deterministic version specifier '{}' in {manifest_file}.",
                    dep.name, dep.version_req
                ),
                evidence: format!("{} = \"{}\"", dep.name, dep.version_req),
                location: Some(SourceLocation {
                    file_path: manifest_file.to_string(),
                    start_line: None,
                    end_line: None,
                }),
                recommendation: format!(
                    "Pin '{}' to an explicit semver range (e.g., '>=0.100.0, <1.0.0' or exact version).",
                    dep.name
                ),
                rule_id: "deps/unpinned-version".to_string(),
            });
        }

        let normalized_name = crate::python_manifest::normalize_python_package_name(&dep.name);
        if !dep.is_dev
            && !PYTHON_IMPLICIT_RUNNERS.contains(&normalized_name.as_str())
            && !imported_packages.contains(&normalized_name)
            && !imported_packages.contains(&dep.name)
        {
            unused.insert(dep.name.clone());
            findings.push(Finding {
                id: format!("DEP-PY-UNUSED-{}", dep.name),
                category: "dependencies".to_string(),
                severity: Severity::Low,
                confidence: Confidence::High,
                title: format!("Unused production dependency '{}'", dep.name),
                description: format!(
                    "Package '{}' is declared in {manifest_file} but never imported in scanned source files.",
                    dep.name
                ),
                evidence: dep.name.clone(),
                location: Some(SourceLocation {
                    file_path: manifest_file.to_string(),
                    start_line: None,
                    end_line: None,
                }),
                recommendation: format!(
                    "Remove '{}' or move it to dev-dependencies if only needed for tooling.",
                    dep.name
                ),
                rule_id: "deps/unused-dependency".to_string(),
            });
        }
    }

    for dup in &manifest.duplicate_packages {
        findings.push(Finding {
            id: format!("DEP-PY-DUP-{dup}"),
            category: "dependencies".to_string(),
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            title: format!("Duplicate dependency declaration for '{dup}'"),
            description: format!(
                "Package '{dup}' is declared multiple times across Python manifests."
            ),
            evidence: format!("Duplicate key '{dup}' in {manifest_file}"),
            location: Some(SourceLocation {
                file_path: manifest_file.to_string(),
                start_line: None,
                end_line: None,
            }),
            recommendation: format!("Keep '{dup}' in only one section or requirements file."),
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

/// Analyzes all project dependency manifests across ecosystems (npm, PyPI, Go, and Cargo)
/// in `repo_root` (and subdirectories identified from `files`) against `external_imports`.
///
/// Emits unified dependency health findings for unpinned versions, wildcards, unused dependencies,
/// and duplicate declarations.
pub fn analyze_polyglot_dependencies(
    repo_root: &Path,
    files: &[FileNode],
    external_imports: &BTreeSet<String>,
) -> Result<DependencyReport, ForgeError> {
    let mut all_dependencies: Vec<Dependency> = Vec::new();
    let mut all_unused: BTreeSet<String> = BTreeSet::new();
    let mut all_duplicates: BTreeSet<String> = BTreeSet::new();
    let mut all_wildcards: BTreeSet<String> = BTreeSet::new();
    let mut all_findings: Vec<Finding> = Vec::new();

    // 1. Locate manifest root directories
    let mut npm_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut py_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut go_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut rust_dirs: BTreeSet<PathBuf> = BTreeSet::new();

    if repo_root.join("package.json").is_file() {
        npm_dirs.insert(repo_root.to_path_buf());
    }
    if repo_root.join("pyproject.toml").is_file()
        || repo_root.join("requirements.txt").is_file()
        || repo_root.join("poetry.lock").is_file()
        || repo_root.join("uv.lock").is_file()
    {
        py_dirs.insert(repo_root.to_path_buf());
    }
    if repo_root.join("go.mod").is_file() {
        go_dirs.insert(repo_root.to_path_buf());
    }
    if repo_root.join("Cargo.toml").is_file() {
        rust_dirs.insert(repo_root.to_path_buf());
    }

    for f in files {
        let p = &f.relative_path;
        if p.ends_with("package.json") {
            let full = repo_root.join(p);
            if let Some(parent) = full.parent() {
                npm_dirs.insert(parent.to_path_buf());
            }
        } else if p.ends_with("pyproject.toml") || p.ends_with("requirements.txt") {
            let full = repo_root.join(p);
            if let Some(parent) = full.parent() {
                py_dirs.insert(parent.to_path_buf());
            }
        } else if p.ends_with("go.mod") {
            let full = repo_root.join(p);
            if let Some(parent) = full.parent() {
                go_dirs.insert(parent.to_path_buf());
            }
        } else if p.ends_with("Cargo.toml") && !rust_dirs.contains(repo_root) {
            let full = repo_root.join(p);
            if let Some(parent) = full.parent() {
                rust_dirs.insert(parent.to_path_buf());
            }
        }
    }

    // 2. Analyze NPM manifests
    for dir in npm_dirs {
        if let Ok(npm_report) = analyze_npm_dependencies(&dir, external_imports) {
            all_dependencies.extend(npm_report.dependencies);
            all_unused.extend(npm_report.unused_dependencies);
            all_duplicates.extend(npm_report.duplicate_packages);
            all_wildcards.extend(npm_report.wildcard_or_unpinned);
            all_findings.extend(npm_report.findings);
        }
    }

    // 3. Analyze Python manifests
    for dir in py_dirs {
        if let Ok(py_report) = analyze_python_dependencies(&dir, external_imports) {
            all_dependencies.extend(py_report.dependencies);
            all_unused.extend(py_report.unused_dependencies);
            all_duplicates.extend(py_report.duplicate_packages);
            all_wildcards.extend(py_report.wildcard_or_unpinned);
            all_findings.extend(py_report.findings);
        }
    }

    // 4. Analyze Go manifests
    for dir in go_dirs {
        if let Ok(go_manifest) = crate::go_manifest::parse_go_manifest(&dir) {
            all_dependencies.extend(go_manifest.dependencies);
        }
    }

    // 5. Analyze Rust workspaces
    for dir in rust_dirs {
        if let Ok(rust_ws) = crate::rust_manifest::parse_rust_workspace(&dir) {
            all_dependencies.extend(rust_ws.dependencies);
            for wild in &rust_ws.wildcard_dependencies {
                all_wildcards.insert(wild.clone());
                all_findings.push(Finding {
                    id: format!("dep/cargo-wildcard-{wild}"),
                    category: "dependencies".to_string(),
                    severity: Severity::High,
                    confidence: Confidence::Certain,
                    title: format!("Wildcard dependency constraint on '{wild}'"),
                    description: format!(
                        "Package '{wild}' uses wildcard constraint '*' in Cargo manifest. This allows untested breaking versions to be pulled in."
                    ),
                    evidence: format!("Cargo.toml: {wild} = \"*\""),
                    location: None,
                    recommendation: "Pin the dependency to a specific version requirement.".to_string(),
                    rule_id: "deps/unpinned-version".to_string(),
                });
            }
        }
    }

    // Deterministic deduplication by (name, ecosystem)
    all_dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.ecosystem.cmp(&b.ecosystem))
    });
    all_dependencies.dedup_by(|a, b| a.name == b.name && a.ecosystem == b.ecosystem);

    Ok(DependencyReport {
        dependencies: all_dependencies,
        unused_dependencies: all_unused.into_iter().collect(),
        duplicate_packages: all_duplicates.into_iter().collect(),
        wildcard_or_unpinned: all_wildcards.into_iter().collect(),
        findings: all_findings,
    })
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
