//! Project and dependency license detection across polyglot ecosystems.

use std::fs;
use std::path::Path;

use forge_core::Dependency;
use serde_json::Value;

/// Detected project license metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLicenseInfo {
    /// SPDX license identifier (e.g. `MIT`, `Apache-2.0`, `GPL-3.0`).
    pub spdx_id: String,
    /// Origin of discovery (e.g. `LICENSE`, `package.json`, `Cargo.toml`).
    pub source: String,
    /// Whether project is marked as private/unlicensed.
    pub is_private: bool,
}

/// Detects the root repository's license information.
pub fn detect_project_license(repo_root: &Path) -> Option<ProjectLicenseInfo> {
    // 1. Check license files in repo root
    let license_filenames = [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENCE",
        "LICENCE.md",
        "LICENCE.txt",
        "COPYING",
        "COPYING.txt",
    ];

    for name in &license_filenames {
        let path = repo_root.join(name);
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(spdx) = identify_license_text(&content) {
                    return Some(ProjectLicenseInfo {
                        spdx_id: spdx.to_string(),
                        source: (*name).to_string(),
                        is_private: false,
                    });
                }
            }
        }
    }

    // 2. Check package.json
    let pkg_json_path = repo_root.join("package.json");
    if pkg_json_path.is_file() {
        if let Ok(content) = fs::read_to_string(&pkg_json_path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                let is_private = val.get("private").and_then(Value::as_bool).unwrap_or(false);

                if let Some(license_str) = val.get("license").and_then(Value::as_str) {
                    return Some(ProjectLicenseInfo {
                        spdx_id: license_str.to_string(),
                        source: "package.json".to_string(),
                        is_private,
                    });
                }
                if is_private {
                    return Some(ProjectLicenseInfo {
                        spdx_id: "UNLICENSED".to_string(),
                        source: "package.json (private)".to_string(),
                        is_private: true,
                    });
                }
            }
        }
    }

    // 3. Check Cargo.toml
    let cargo_toml_path = repo_root.join("Cargo.toml");
    if cargo_toml_path.is_file() {
        if let Ok(content) = fs::read_to_string(&cargo_toml_path) {
            if let Ok(val) = toml::from_str::<toml::Value>(&content) {
                let pkg = val
                    .get("package")
                    .or_else(|| val.get("workspace").and_then(|w| w.get("package")));
                if let Some(p) = pkg {
                    if let Some(license_str) = p.get("license").and_then(toml::Value::as_str) {
                        return Some(ProjectLicenseInfo {
                            spdx_id: license_str.to_string(),
                            source: "Cargo.toml".to_string(),
                            is_private: false,
                        });
                    }
                }
            }
        }
    }

    // 4. Check composer.json
    let composer_path = repo_root.join("composer.json");
    if composer_path.is_file() {
        if let Ok(content) = fs::read_to_string(&composer_path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if let Some(license_str) = val.get("license").and_then(Value::as_str) {
                    return Some(ProjectLicenseInfo {
                        spdx_id: license_str.to_string(),
                        source: "composer.json".to_string(),
                        is_private: false,
                    });
                }
            }
        }
    }

    None
}

/// Identifies an SPDX license identifier from the body of a license text file.
fn identify_license_text(content: &str) -> Option<&'static str> {
    let lower = content.to_ascii_lowercase();

    if lower.contains("apache license") && lower.contains("version 2.0") {
        Some("Apache-2.0")
    } else if lower.contains("mit license")
        || lower.contains(
            "permission is hereby granted, free of charge, to any person obtaining a copy",
        )
    {
        Some("MIT")
    } else if lower.contains("gnu affero general public license") {
        Some("AGPL-3.0")
    } else if lower.contains("gnu general public license") {
        if lower.contains("version 3") || lower.contains("gplv3") {
            Some("GPL-3.0")
        } else if lower.contains("version 2") || lower.contains("gplv2") {
            Some("GPL-2.0")
        } else {
            Some("GPL-3.0")
        }
    } else if lower.contains("gnu lesser general public license") {
        if lower.contains("version 3") || lower.contains("lgplv3") {
            Some("LGPL-3.0")
        } else {
            Some("LGPL-2.1")
        }
    } else if lower.contains("redistribution and use in source and binary forms") {
        if lower.contains("neither the name") {
            Some("BSD-3-Clause")
        } else {
            Some("BSD-2-Clause")
        }
    } else if lower.contains("mozilla public license") {
        Some("MPL-2.0")
    } else if lower.contains("the unlicense") {
        Some("Unlicense")
    } else if lower.contains("isc license")
        || lower.contains("permission to use, copy, modify, and/or distribute this software")
    {
        Some("ISC")
    } else {
        None
    }
}

/// Detects the license of an individual [`Dependency`], inspecting installed packages or known catalogs.
pub fn detect_dependency_license(repo_root: &Path, dep: &Dependency) -> String {
    // 1. Try local node_modules
    if dep.ecosystem == "npm" {
        let pkg_json = repo_root
            .join("node_modules")
            .join(&dep.name)
            .join("package.json");
        if pkg_json.is_file() {
            if let Ok(content) = fs::read_to_string(&pkg_json) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    if let Some(lic) = val.get("license").and_then(Value::as_str) {
                        return lic.to_string();
                    }
                    if let Some(lic_obj) = val.get("license").and_then(Value::as_object) {
                        if let Some(t) = lic_obj.get("type").and_then(Value::as_str) {
                            return t.to_string();
                        }
                    }
                }
            }
        }
    }

    // 2. Try vendor directory for composer
    if dep.ecosystem == "composer" {
        let composer_json = repo_root
            .join("vendor")
            .join(&dep.name)
            .join("composer.json");
        if composer_json.is_file() {
            if let Ok(content) = fs::read_to_string(&composer_json) {
                if let Ok(val) = serde_json::from_str::<Value>(&content) {
                    if let Some(lic) = val.get("license").and_then(Value::as_str) {
                        return lic.to_string();
                    }
                    if let Some(arr) = val.get("license").and_then(Value::as_array) {
                        if let Some(first) = arr.first().and_then(Value::as_str) {
                            return first.to_string();
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback catalog of known popular packages
    catalog_lookup(&dep.ecosystem, &dep.name)
        .unwrap_or("Unknown")
        .to_string()
}

/// Fast deterministic catalog lookup for common polyglot libraries when offline or dependencies aren't pre-installed.
fn catalog_lookup(ecosystem: &str, name: &str) -> Option<&'static str> {
    match ecosystem {
        "npm" => match name {
            "lodash" | "minimist" | "express" | "react" | "react-dom" | "vue" | "axios"
            | "chalk" | "debug" | "commander" | "tslib" | "glob" | "semver" | "dotenv" | "uuid"
            | "moment" | "prettier" | "eslint" | "jest" | "mocha" => Some("MIT"),
            "typescript" => Some("Apache-2.0"),
            _ => None,
        },
        "pypi" => match name.to_ascii_lowercase().as_str() {
            "requests" | "urllib3" | "cryptography" | "pytest" => Some("Apache-2.0"),
            "flask" | "django" | "werkzeug" | "click" | "jinja2" | "numpy" | "pandas" => {
                Some("BSD-3-Clause")
            }
            "pydantic" | "fastapi" | "black" | "six" => Some("MIT"),
            _ => None,
        },
        "go" => match name {
            "github.com/gin-gonic/gin" | "github.com/stretchr/testify" | "golang.org/x/sync" => {
                Some("MIT")
            }
            "github.com/gorilla/mux" => Some("BSD-3-Clause"),
            "google.golang.org/grpc" => Some("Apache-2.0"),
            _ => None,
        },
        "composer" => match name {
            "guzzlehttp/guzzle" | "guzzlehttp/psr7" | "guzzlehttp/promises" | "monolog/monolog" => {
                Some("MIT")
            }
            _ => {
                if name.starts_with("symfony/") || name.starts_with("laravel/") {
                    Some("MIT")
                } else {
                    None
                }
            }
        },
        "cargo" => match name {
            "serde" | "serde_json" | "tokio" | "syn" | "quote" | "regex" | "anyhow"
            | "thiserror" => Some("MIT OR Apache-2.0"),
            "blake3" => Some("Apache-2.0 OR CC0-1.0"),
            _ => None,
        },
        _ => None,
    }
}
