//! Package URL (PURL) specification implementation for polyglot dependencies.
//! Follows standard https://github.com/package-url/purl-spec.

use forge_core::Dependency;

use crate::osv::normalize_ecosystem;

/// Generates a canonical Package URL (PURL) for a given [`Dependency`].
pub fn generate_purl(dep: &Dependency) -> String {
    let eco = normalize_ecosystem(&dep.ecosystem);
    let version_str = dep
        .resolved_version
        .as_deref()
        .unwrap_or(dep.version_req.as_str());
    let clean_version = clean_version_for_purl(version_str);

    match eco {
        "npm" => {
            // Check for scoped package e.g. @scope/pkg
            if let Some((scope, name)) = dep.name.split_once('/') {
                let clean_scope = scope.strip_prefix('@').unwrap_or(scope);
                format!("pkg:npm/%40{clean_scope}/{name}@{clean_version}")
            } else {
                format!("pkg:npm/{}@{clean_version}", dep.name)
            }
        }
        "pypi" => {
            let normalized_name = dep.name.to_ascii_lowercase().replace('_', "-");
            format!("pkg:pypi/{normalized_name}@{clean_version}")
        }
        "go" => {
            // Go packages are often module paths like github.com/gin-gonic/gin
            format!("pkg:golang/{}@{clean_version}", dep.name)
        }
        "cargo" => {
            format!("pkg:cargo/{}@{clean_version}", dep.name)
        }
        "composer" => {
            format!("pkg:composer/{}@{clean_version}", dep.name)
        }
        _ => {
            format!("pkg:generic/{}@{clean_version}", dep.name)
        }
    }
}

/// Cleans version strings by removing operators like `^`, `~`, `==`, etc.
fn clean_version_for_purl(raw: &str) -> &str {
    let trimmed = raw.trim();
    let cleaned = trimmed.trim_start_matches(|c: char| {
        c == '=' || c == '^' || c == '~' || c == '>' || c == '<' || c == ' '
    });
    if cleaned.is_empty() {
        "0.0.0"
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_purls_across_ecosystems() {
        let npm_dep = Dependency {
            name: "lodash".to_string(),
            version_req: "^4.17.15".to_string(),
            resolved_version: Some("4.17.15".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(generate_purl(&npm_dep), "pkg:npm/lodash@4.17.15");

        let scoped_npm = Dependency {
            name: "@elcode/forge".to_string(),
            version_req: "1.0.0".to_string(),
            resolved_version: Some("1.0.0".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(generate_purl(&scoped_npm), "pkg:npm/%40elcode/forge@1.0.0");

        let pypi_dep = Dependency {
            name: "My_Package".to_string(),
            version_req: "==2.1.0".to_string(),
            resolved_version: None,
            ecosystem: "pypi".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(generate_purl(&pypi_dep), "pkg:pypi/my-package@2.1.0");

        let go_dep = Dependency {
            name: "github.com/gin-gonic/gin".to_string(),
            version_req: "v1.6.0".to_string(),
            resolved_version: Some("v1.6.0".to_string()),
            ecosystem: "go".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(
            generate_purl(&go_dep),
            "pkg:golang/github.com/gin-gonic/gin@v1.6.0"
        );

        let cargo_dep = Dependency {
            name: "serde".to_string(),
            version_req: "1.0.210".to_string(),
            resolved_version: Some("1.0.210".to_string()),
            ecosystem: "cargo".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(generate_purl(&cargo_dep), "pkg:cargo/serde@1.0.210");

        let composer_dep = Dependency {
            name: "guzzlehttp/guzzle".to_string(),
            version_req: "^7.4.0".to_string(),
            resolved_version: Some("7.4.0".to_string()),
            ecosystem: "composer".to_string(),
            is_dev: false,
            is_direct: true,
        };
        assert_eq!(
            generate_purl(&composer_dep),
            "pkg:composer/guzzlehttp/guzzle@7.4.0"
        );
    }
}
