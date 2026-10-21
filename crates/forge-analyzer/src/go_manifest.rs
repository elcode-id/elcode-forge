use std::fs;
use std::path::Path;

use forge_core::{Dependency, ForgeError};

/// Parsed results of a Go module's `go.mod` manifest and `go.sum` lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGoManifest {
    pub module_path: Option<String>,
    pub go_version: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub has_go_sum: bool,
}

/// Parses the contents of a `go.mod` file into a [`ParsedGoManifest`].
pub fn parse_go_mod_content(
    content: &str,
    has_go_sum: bool,
) -> Result<ParsedGoManifest, ForgeError> {
    let mut module_path = None;
    let mut go_version = None;
    let mut dependencies = Vec::new();
    let mut in_require_block = false;

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle start/end of multiline require block
        if trimmed.starts_with("require (") || (trimmed == "require (" || trimmed == "require(") {
            in_require_block = true;
            continue;
        }

        if in_require_block {
            if trimmed.starts_with(')') {
                in_require_block = false;
                continue;
            }

            if let Some(dep) = parse_require_item(trimmed) {
                dependencies.push(dep);
            }
            continue;
        }

        // Module directive: `module github.com/user/repo`
        if let Some(rest) = trimmed.strip_prefix("module ") {
            let clean = rest.trim();
            if !clean.is_empty() {
                module_path = Some(clean.split_whitespace().next().unwrap_or(clean).to_string());
            }
            continue;
        }

        // Go version directive: `go 1.22` or `go 1.22.0`
        if let Some(rest) = trimmed.strip_prefix("go ") {
            let clean = rest.trim();
            if !clean.is_empty() {
                go_version = Some(clean.split_whitespace().next().unwrap_or(clean).to_string());
            }
            continue;
        }

        // Single-line require: `require github.com/gin-gonic/gin v1.9.1 [// indirect]`
        if let Some(rest) = trimmed.strip_prefix("require ") {
            if let Some(dep) = parse_require_item(rest.trim()) {
                dependencies.push(dep);
            }
        }
    }

    dependencies.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.is_direct.cmp(&b.is_direct))
    });

    Ok(ParsedGoManifest {
        module_path,
        go_version,
        dependencies,
        has_go_sum,
    })
}

/// Parses an individual `require` line item: `<package> <version> [// indirect]`
fn parse_require_item(line: &str) -> Option<Dependency> {
    let is_indirect = line.contains("// indirect");
    let clean = if let Some(idx) = line.find("//") {
        line[..idx].trim()
    } else {
        line.trim()
    };

    let parts: Vec<&str> = clean.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let name = parts[0].trim().trim_matches('"').to_string();
    let version = parts[1].trim().to_string();

    if name.is_empty() || version.is_empty() {
        return None;
    }

    Some(Dependency {
        name,
        version_req: version.clone(),
        resolved_version: Some(version),
        ecosystem: "go".to_string(),
        is_dev: is_indirect,
        is_direct: !is_indirect,
    })
}

/// Discovers and parses `go.mod` (and checks for `go.sum`) within `repo_root`.
pub fn parse_go_manifest(repo_root: &Path) -> Result<ParsedGoManifest, ForgeError> {
    let go_mod_path = repo_root.join("go.mod");
    if !go_mod_path.is_file() {
        return Ok(ParsedGoManifest {
            module_path: None,
            go_version: None,
            dependencies: Vec::new(),
            has_go_sum: false,
        });
    }

    let has_go_sum = repo_root.join("go.sum").is_file();
    let content = fs::read_to_string(&go_mod_path).map_err(|source| ForgeError::Io {
        path: go_mod_path,
        source,
    })?;

    parse_go_mod_content(&content, has_go_sum)
}
