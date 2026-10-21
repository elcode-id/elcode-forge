use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use forge_core::FileNode;
use serde_json::Value;

/// Detected ecosystem signals for a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcosystemSignals {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub package_managers: Vec<String>,
    pub has_git: bool,
    pub has_docker: bool,
    pub has_github_actions: bool,
    pub has_tests: bool,
}

/// Maps a normalized relative file path to its canonical programming language name.
pub fn detect_language(relative_path: &str) -> Option<String> {
    let lower = relative_path.to_ascii_lowercase();
    let ext = Path::new(&lower).extension()?.to_str()?;

    match ext {
        "ts" | "tsx" | "mts" | "cts" => Some("typescript".to_string()),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript".to_string()),
        "py" => Some("python".to_string()),
        "rs" => Some("rust".to_string()),
        "go" => Some("go".to_string()),
        "php" => Some("php".to_string()),
        _ => None,
    }
}

/// Determines whether a normalized relative file path represents a test file.
pub fn is_test_file(relative_path: &str) -> bool {
    let lower = relative_path.to_ascii_lowercase();
    lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.contains("test_")
        || lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
}

/// Infers languages, frameworks, package managers, and infrastructure signals from `repo_root`
/// and the scanned `files`.
pub fn detect_ecosystem(repo_root: &Path, files: &[FileNode]) -> EcosystemSignals {
    let mut languages = BTreeSet::new();
    let mut frameworks = BTreeSet::new();
    let mut package_managers = BTreeSet::new();
    let mut has_docker = false;
    let mut has_github_actions = repo_root.join(".github").join("workflows").is_dir();
    let mut has_tests = false;

    for file in files {
        if let Some(lang) = &file.language {
            languages.insert(lang.clone());
        }
        if file.is_test {
            has_tests = true;
        }

        let path = file.relative_path.as_str();
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name.eq_ignore_ascii_case("dockerfile")
            || file_name.eq_ignore_ascii_case("docker-compose.yml")
            || file_name.eq_ignore_ascii_case("docker-compose.yaml")
            || file_name.eq_ignore_ascii_case("compose.yml")
            || file_name.eq_ignore_ascii_case("compose.yaml")
        {
            has_docker = true;
        }

        if path.starts_with(".github/workflows/") {
            has_github_actions = true;
        }

        if file_name.starts_with("next.config.") {
            frameworks.insert("nextjs".to_string());
        } else if file_name.starts_with("nuxt.config.") {
            frameworks.insert("nuxt".to_string());
        } else if file_name.starts_with("svelte.config.") {
            frameworks.insert("sveltekit".to_string());
        } else if file_name == "pytest.ini" || file_name == "conftest.py" {
            frameworks.insert("pytest".to_string());
            has_tests = true;
        }
    }

    // Detect package managers from lockfiles / manifests at repo_root
    let has_pnpm_lock = repo_root.join("pnpm-lock.yaml").is_file();
    let has_yarn_lock = repo_root.join("yarn.lock").is_file();
    let has_bun_lock =
        repo_root.join("bun.lockb").is_file() || repo_root.join("bun.lock").is_file();
    let has_npm_lock = repo_root.join("package-lock.json").is_file();
    let pkg_json_path = repo_root.join("package.json");

    if has_pnpm_lock {
        package_managers.insert("pnpm".to_string());
    }
    if has_yarn_lock {
        package_managers.insert("yarn".to_string());
    }
    if has_bun_lock {
        package_managers.insert("bun".to_string());
    }
    if has_npm_lock
        || (pkg_json_path.is_file() && !has_pnpm_lock && !has_yarn_lock && !has_bun_lock)
    {
        package_managers.insert("npm".to_string());
    }
    if repo_root.join("Cargo.toml").is_file() {
        package_managers.insert("cargo".to_string());
    }
    if repo_root.join("poetry.lock").is_file() {
        package_managers.insert("poetry".to_string());
    }
    if repo_root.join("uv.lock").is_file() {
        package_managers.insert("uv".to_string());
    }
    if repo_root.join("requirements.txt").is_file() {
        package_managers.insert("pip".to_string());
    }

    // Inspect package.json dependencies for JS/TS frameworks and test runners
    if pkg_json_path.is_file() {
        if let Ok(raw) = fs::read_to_string(&pkg_json_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&raw) {
                for section in ["dependencies", "devDependencies"] {
                    if let Some(obj) = json.get(section).and_then(Value::as_object) {
                        for dep in obj.keys() {
                            match dep.as_str() {
                                "next" => {
                                    frameworks.insert("nextjs".to_string());
                                }
                                "nuxt" => {
                                    frameworks.insert("nuxt".to_string());
                                }
                                "@sveltejs/kit" => {
                                    frameworks.insert("sveltekit".to_string());
                                }
                                "svelte" => {
                                    frameworks.insert("svelte".to_string());
                                }
                                "react" => {
                                    frameworks.insert("react".to_string());
                                }
                                "vue" => {
                                    frameworks.insert("vue".to_string());
                                }
                                "express" => {
                                    frameworks.insert("express".to_string());
                                }
                                "@nestjs/core" => {
                                    frameworks.insert("nestjs".to_string());
                                }
                                "typescript" => {
                                    languages.insert("typescript".to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    // Inspect Python manifests and lockfiles for frameworks (FastAPI, Flask, Django) and pytest
    if repo_root.join("pyproject.toml").is_file()
        || repo_root.join("requirements.txt").is_file()
        || repo_root.join("poetry.lock").is_file()
        || repo_root.join("uv.lock").is_file()
    {
        if let Ok(manifest) = crate::python_manifest::parse_python_manifests(repo_root) {
            for dep in &manifest.dependencies {
                match dep.name.as_str() {
                    "fastapi" => {
                        frameworks.insert("fastapi".to_string());
                    }
                    "flask" => {
                        frameworks.insert("flask".to_string());
                    }
                    "django" => {
                        frameworks.insert("django".to_string());
                    }
                    "pytest" => {
                        frameworks.insert("pytest".to_string());
                        has_tests = true;
                    }
                    _ => {}
                }
            }
        }
    }

    // Inspect Python source imports if framework not yet found
    if !frameworks.contains("fastapi")
        || !frameworks.contains("flask")
        || !frameworks.contains("django")
    {
        for file in files {
            if file.language.as_deref() == Some("python") || file.relative_path.ends_with(".py") {
                let full_path = repo_root.join(&file.relative_path);
                if let Ok(content) = fs::read_to_string(&full_path) {
                    if content.contains("fastapi")
                        && (content.contains("import fastapi")
                            || content.contains("from fastapi import"))
                    {
                        frameworks.insert("fastapi".to_string());
                    }
                    if content.contains("flask")
                        && (content.contains("import flask")
                            || content.contains("from flask import"))
                    {
                        frameworks.insert("flask".to_string());
                    }
                    if content.contains("django")
                        && (content.contains("import django")
                            || content.contains("from django import"))
                    {
                        frameworks.insert("django".to_string());
                    }
                    if content.contains("pytest")
                        && (content.contains("import pytest")
                            || content.contains("from pytest import"))
                    {
                        frameworks.insert("pytest".to_string());
                        has_tests = true;
                    }
                }
            }
        }
    }

    EcosystemSignals {
        languages: languages.into_iter().collect(),
        frameworks: frameworks.into_iter().collect(),
        package_managers: package_managers.into_iter().collect(),
        has_git: repo_root.join(".git").exists(),
        has_docker,
        has_github_actions,
        has_tests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn maps_extensions_to_languages_and_identifies_test_files() {
        assert_eq!(
            detect_language("src/pages/index.tsx").as_deref(),
            Some("typescript")
        );
        assert_eq!(
            detect_language("src/utils/helper.mjs").as_deref(),
            Some("javascript")
        );
        assert_eq!(detect_language("app/main.py").as_deref(), Some("python"));
        assert_eq!(detect_language("src/lib.rs").as_deref(), Some("rust"));
        assert_eq!(detect_language("README.md"), None);

        assert!(is_test_file("src/services/userService.test.ts"));
        assert!(is_test_file("src/__tests__/auth.ts"));
        assert!(is_test_file("tests/test_api.py"));
        assert!(is_test_file("app/routers/test_items.py"));
        assert!(!is_test_file("src/services/userService.ts"));
    }

    #[test]
    fn detects_nextjs_and_react_in_fixture_next_app() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let signals = detect_ecosystem(&fixture, &[]);
        assert!(signals.languages.contains(&"typescript".to_string()));
        assert!(signals.frameworks.contains(&"nextjs".to_string()));
        assert!(signals.frameworks.contains(&"react".to_string()));
        assert!(signals.package_managers.contains(&"npm".to_string()));
    }
}
