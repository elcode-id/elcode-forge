use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{Confidence, Dependency, Finding, ForgeError, Severity, SourceLocation};
use serde_json::Value;

use crate::dep_health::DependencyReport;

/// Implicit PHP tools, testing frameworks, and runners that are invoked via CLI
/// rather than direct source imports.
pub const PHP_IMPLICIT_RUNNERS: &[&str] = &[
    "php",
    "hhvm",
    "phpunit/phpunit",
    "pestphp/pest",
    "phpstan/phpstan",
    "vimeo/psalm",
    "rector/rector",
    "friendsofphp/php-cs-fixer",
    "laravel/pint",
    "laravel/sail",
    "fakerphp/faker",
    "mockery/mockery",
    "nunomaduro/collision",
    "spatie/laravel-ignition",
];

/// Parsed results of a PHP Composer manifest (`composer.json`) and lockfile (`composer.lock`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedComposerManifest {
    pub name: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub psr4_mappings: BTreeMap<String, String>,
    pub unpinned_packages: Vec<String>,
    pub duplicate_packages: Vec<String>,
    pub has_composer_lock: bool,
}

/// Reads `<repo_root>/composer.lock` and maps package names to exact resolved versions.
pub fn read_composer_lock_versions(repo_root: &Path) -> BTreeMap<String, String> {
    let lock_path = repo_root.join("composer.lock");
    let mut map = BTreeMap::new();

    let Ok(content) = fs::read_to_string(&lock_path) else {
        return map;
    };

    let Ok(json) = serde_json::from_str::<Value>(&content) else {
        return map;
    };

    for section in ["packages", "packages-dev"] {
        if let Some(packages) = json.get(section).and_then(Value::as_array) {
            for pkg in packages {
                if let (Some(name), Some(version)) = (
                    pkg.get("name").and_then(Value::as_str),
                    pkg.get("version").and_then(Value::as_str),
                ) {
                    map.insert(name.to_lowercase(), version.to_string());
                }
            }
        }
    }

    map
}

/// Determines whether a Composer version requirement is unpinned or allows unbounded breaking changes.
pub fn is_composer_unpinned(version_req: &str) -> bool {
    let v = version_req.trim();
    if v == "*" || v.is_empty() {
        return true;
    }
    // Open-ended lower-bound only without upper bound constraint (e.g. ">=8.0" or ">1.0")
    if (v.starts_with(">=") || v.starts_with('>')) && !v.contains('<') && !v.contains(',') {
        return true;
    }
    // Unpinned dev branches without commit hash
    if v == "dev-master" || v == "dev-main" {
        return true;
    }
    false
}

/// Parses the contents of a `composer.json` file into a [`ParsedComposerManifest`].
pub fn parse_composer_json_content(
    content: &str,
    has_composer_lock: bool,
    lock_versions: &BTreeMap<String, String>,
) -> Result<ParsedComposerManifest, ForgeError> {
    let json: Value = serde_json::from_str(content).map_err(|err| ForgeError::Serialization {
        message: format!("Failed to parse composer.json: {err}"),
    })?;

    let name = json
        .get("name")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let mut dependencies = Vec::new();
    let mut prod_names = BTreeSet::new();
    let mut dev_names = BTreeSet::new();
    let mut unpinned_packages = BTreeSet::new();
    let mut duplicate_packages = BTreeSet::new();

    // 1. Production dependencies (`require`)
    if let Some(req_obj) = json.get("require").and_then(Value::as_object) {
        for (pkg_name, ver_val) in req_obj {
            let pkg_lower = pkg_name.to_lowercase();
            prod_names.insert(pkg_lower.clone());

            let ver_req = ver_val.as_str().unwrap_or("*").trim().to_string();
            if is_composer_unpinned(&ver_req)
                && !pkg_lower.starts_with("ext-")
                && pkg_lower != "php"
            {
                unpinned_packages.insert(pkg_lower.clone());
            }

            let resolved = lock_versions.get(&pkg_lower).cloned();
            dependencies.push(Dependency {
                name: pkg_name.clone(),
                version_req: ver_req,
                resolved_version: resolved,
                ecosystem: "composer".to_string(),
                is_dev: false,
                is_direct: true,
            });
        }
    }

    // 2. Dev dependencies (`require-dev`)
    if let Some(dev_obj) = json.get("require-dev").and_then(Value::as_object) {
        for (pkg_name, ver_val) in dev_obj {
            let pkg_lower = pkg_name.to_lowercase();
            dev_names.insert(pkg_lower.clone());

            if prod_names.contains(&pkg_lower) {
                duplicate_packages.insert(pkg_lower.clone());
            }

            let ver_req = ver_val.as_str().unwrap_or("*").trim().to_string();
            if is_composer_unpinned(&ver_req)
                && !pkg_lower.starts_with("ext-")
                && pkg_lower != "php"
            {
                unpinned_packages.insert(pkg_lower.clone());
            }

            let resolved = lock_versions.get(&pkg_lower).cloned();
            dependencies.push(Dependency {
                name: pkg_name.clone(),
                version_req: ver_req,
                resolved_version: resolved,
                ecosystem: "composer".to_string(),
                is_dev: true,
                is_direct: true,
            });
        }
    }

    // 3. PSR-4 Autoload mappings (`autoload.psr-4` and `autoload-dev.psr-4`)
    let mut psr4_mappings = BTreeMap::new();
    for section in ["autoload", "autoload-dev"] {
        if let Some(psr4_obj) = json
            .get(section)
            .and_then(|v| v.get("psr-4"))
            .and_then(Value::as_object)
        {
            for (ns, path_val) in psr4_obj {
                if let Some(path_str) = path_val.as_str() {
                    let clean_path = path_str.trim().replace('\\', "/");
                    let normalized_path = if clean_path.ends_with('/') {
                        clean_path[..clean_path.len() - 1].to_string()
                    } else {
                        clean_path
                    };
                    psr4_mappings.insert(ns.clone(), normalized_path);
                }
            }
        }
    }

    // Deterministic dependency ordering
    dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.is_dev.cmp(&b.is_dev)));

    Ok(ParsedComposerManifest {
        name,
        dependencies,
        psr4_mappings,
        unpinned_packages: unpinned_packages.into_iter().collect(),
        duplicate_packages: duplicate_packages.into_iter().collect(),
        has_composer_lock,
    })
}

/// Parses the Composer manifest (`composer.json`) and resolves lockfile versions in `repo_root`.
pub fn parse_composer_manifest(repo_root: &Path) -> Result<ParsedComposerManifest, ForgeError> {
    let manifest_path = repo_root.join("composer.json");
    if !manifest_path.is_file() {
        return Err(ForgeError::Analysis {
            analyzer: "composer_manifest",
            message: format!(
                "No composer.json found at '{}'",
                repo_root.to_string_lossy()
            ),
        });
    }

    let content = fs::read_to_string(&manifest_path).map_err(|err| ForgeError::Io {
        path: manifest_path.clone(),
        source: err,
    })?;

    let lock_versions = read_composer_lock_versions(repo_root);
    let has_composer_lock = repo_root.join("composer.lock").is_file();

    parse_composer_json_content(&content, has_composer_lock, &lock_versions)
}

/// Evaluates Composer dependencies in `repo_root` against `external_imports` to detect
/// unused dependencies, unpinned versions, missing lockfiles, and duplicate declarations.
pub fn analyze_composer_dependencies(
    repo_root: &Path,
    external_imports: &BTreeSet<String>,
) -> Result<DependencyReport, ForgeError> {
    let manifest = parse_composer_manifest(repo_root)?;

    let mut unused = BTreeSet::new();
    let mut wildcard_or_unpinned = BTreeSet::new();
    let mut duplicate_packages = BTreeSet::new();
    let mut findings = Vec::new();

    // 1. Missing composer.lock check
    if !manifest.has_composer_lock {
        findings.push(Finding {
            id: "dep/composer-missing-lockfile".to_string(),
            category: "dependencies".to_string(),
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            title: "Missing composer.lock lockfile".to_string(),
            description: "Repository has composer.json but lacks composer.lock. Builds may resolve non-deterministic dependency versions across environments.".to_string(),
            evidence: "composer.lock not found".to_string(),
            location: Some(SourceLocation {
                file_path: "composer.json".to_string(),
                start_line: Some(1),
                end_line: Some(1),
            }),
            recommendation: "Run 'composer update' or 'composer install' locally to generate and commit composer.lock.".to_string(),
            rule_id: "deps/missing-lockfile".to_string(),
        });
    }

    // 2. Unpinned or wildcard versions
    for unpinned in &manifest.unpinned_packages {
        wildcard_or_unpinned.insert(unpinned.clone());
        findings.push(Finding {
            id: format!("dep/composer-unpinned-{unpinned}"),
            category: "dependencies".to_string(),
            severity: Severity::High,
            confidence: Confidence::Certain,
            title: format!("Unpinned or wildcard version constraint on '{unpinned}'"),
            description: format!(
                "Package '{unpinned}' uses an unpinned or open-ended version requirement in composer.json. This allows unintended breaking changes to be pulled in."
            ),
            evidence: format!("composer.json: \"{unpinned}\" constraint is unpinned"),
            location: Some(SourceLocation {
                file_path: "composer.json".to_string(),
                start_line: None,
                end_line: None,
            }),
            recommendation: "Pin the dependency with caret (`^`), tilde (`~`), or exact version constraint.".to_string(),
            rule_id: "deps/unpinned-version".to_string(),
        });
    }

    // 3. Duplicate packages across require and require-dev
    for dup in &manifest.duplicate_packages {
        duplicate_packages.insert(dup.clone());
        findings.push(Finding {
            id: format!("dep/composer-duplicate-{dup}"),
            category: "dependencies".to_string(),
            severity: Severity::Low,
            confidence: Confidence::Certain,
            title: format!("Duplicate package declaration for '{dup}'"),
            description: format!(
                "Package '{dup}' is declared in both 'require' and 'require-dev' in composer.json."
            ),
            evidence: format!("composer.json: \"{dup}\" declared in require and require-dev"),
            location: Some(SourceLocation {
                file_path: "composer.json".to_string(),
                start_line: None,
                end_line: None,
            }),
            recommendation: "Remove the duplicate declaration from 'require-dev'.".to_string(),
            rule_id: "deps/duplicate-dependency".to_string(),
        });
    }

    // 4. Unused production dependencies
    for dep in &manifest.dependencies {
        if dep.is_dev {
            continue;
        }

        let name_lower = dep.name.to_lowercase();
        // Skip PHP language runtime and native C extensions
        if name_lower == "php" || name_lower == "hhvm" || name_lower.starts_with("ext-") {
            continue;
        }

        // Skip implicit CLI runners and tools
        if PHP_IMPLICIT_RUNNERS.contains(&name_lower.as_str()) {
            continue;
        }

        // Check against external_imports (e.g. "guzzlehttp/guzzle" or vendor prefix "guzzlehttp")
        let is_used = external_imports.contains(&name_lower)
            || external_imports.iter().any(|imp| {
                let imp_lower = imp.to_lowercase();
                imp_lower == name_lower
                    || imp_lower.starts_with(&format!("{name_lower}/"))
                    || name_lower.starts_with(&imp_lower)
            });

        if !is_used && !external_imports.is_empty() {
            unused.insert(dep.name.clone());
            findings.push(Finding {
                id: format!("dep/composer-unused-{}", dep.name),
                category: "dependencies".to_string(),
                severity: Severity::Low,
                confidence: Confidence::Medium,
                title: format!("Unused production dependency '{}'", dep.name),
                description: format!(
                    "Package '{}' is declared in 'require' in composer.json but never referenced in scanned PHP source files.",
                    dep.name
                ),
                evidence: format!("composer.json: \"{}\"", dep.name),
                location: Some(SourceLocation {
                    file_path: "composer.json".to_string(),
                    start_line: None,
                    end_line: None,
                }),
                recommendation: format!(
                    "Remove '{}' from composer.json if it is no longer needed, or move to 'require-dev'.",
                    dep.name
                ),
                rule_id: "deps/unused-dependency".to_string(),
            });
        }
    }

    // Sort findings deterministically by (severity desc, id asc)
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));

    Ok(DependencyReport {
        dependencies: manifest.dependencies,
        unused_dependencies: unused.into_iter().collect(),
        duplicate_packages: duplicate_packages.into_iter().collect(),
        wildcard_or_unpinned: wildcard_or_unpinned.into_iter().collect(),
        findings,
    })
}
