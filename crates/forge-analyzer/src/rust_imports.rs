use std::collections::BTreeSet;
use std::path::Path;

/// Standard library crates in Rust.
pub const RUST_STD_CRATES: &[&str] = &["std", "core", "alloc", "proc_macro", "test"];

/// Checks if an import or crate name belongs to the Rust standard library ecosystem.
pub fn is_rust_std(crate_name: &str) -> bool {
    let root = match crate_name.split("::").next() {
        Some(r) => r.trim(),
        None => crate_name.trim(),
    };
    RUST_STD_CRATES.contains(&root)
}

/// Strips `//` line comments (including `///`, `//!`) and nested `/* ... */` block comments from Rust source,
/// preserving string literals and newlines.
pub fn strip_rust_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // 1. Raw string literals: r#"..."# or r"..."
        if chars[i] == 'r' && (i + 1 < n && (chars[i + 1] == '"' || chars[i + 1] == '#')) {
            let mut j = i + 1;
            let mut hash_count = 0;
            while j < n && chars[j] == '#' {
                hash_count += 1;
                j += 1;
            }
            if j < n && chars[j] == '"' {
                result.push('r');
                for _ in 0..hash_count {
                    result.push('#');
                }
                result.push('"');
                i = j + 1;
                while i < n {
                    if chars[i] == '"' {
                        let mut matches = true;
                        for k in 0..hash_count {
                            if i + 1 + k >= n || chars[i + 1 + k] != '#' {
                                matches = false;
                                break;
                            }
                        }
                        if matches {
                            result.push('"');
                            for _ in 0..hash_count {
                                result.push('#');
                            }
                            i += 1 + hash_count;
                            break;
                        }
                    }
                    result.push(chars[i]);
                    i += 1;
                }
                continue;
            }
        }

        // 2. Normal string literals
        if chars[i] == '"' {
            result.push('"');
            i += 1;
            while i < n {
                if chars[i] == '\\' && i + 1 < n {
                    result.push('\\');
                    result.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    result.push('"');
                    i += 1;
                    break;
                }
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // 3. Line comment: //
        if chars[i] == '/' && i + 1 < n && chars[i + 1] == '/' {
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            if i < n && chars[i] == '\n' {
                result.push('\n');
                i += 1;
            }
            continue;
        }

        // 4. Block comment: /* ... */ with nesting support
        if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            let mut depth = 1;
            while i < n && depth > 0 {
                if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if chars[i] == '\n' {
                        result.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Strips visibility modifiers (`pub`, `pub(crate)`, `pub(super)`, `pub(in ...)`) from a line.
fn strip_visibility(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("pub") {
        let rest = rest.trim_start();
        if rest.starts_with('(') {
            if let Some(idx) = rest.find(')') {
                return rest[idx + 1..].trim_start();
            }
        }
        return rest;
    }
    trimmed
}

/// Recursively expands a Rust `use` tree into a list of normalized use paths.
///
/// Examples:
/// - `"crate::foo::{bar, baz}"` -> `["crate::foo::bar", "crate::foo::baz"]`
/// - `"std::{collections::{BTreeMap, BTreeSet}, sync::Arc}"` -> `["std::collections::BTreeMap", "std::collections::BTreeSet", "std::sync::Arc"]`
pub fn expand_use_tree(prefix: &str, tree: &str) -> Vec<String> {
    let tree = tree.trim();
    if tree.is_empty() {
        return Vec::new();
    }

    if let Some(open_idx) = tree.find('{') {
        let before_brace = tree[..open_idx].trim();
        let full_prefix = if before_brace.is_empty() {
            prefix.to_string()
        } else if prefix.is_empty() {
            before_brace.to_string()
        } else if prefix.ends_with("::") {
            format!("{prefix}{before_brace}")
        } else {
            format!("{prefix}::{before_brace}")
        };

        let mut depth = 0;
        let mut close_idx = None;
        for (i, c) in tree[open_idx..].char_indices() {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    close_idx = Some(open_idx + i);
                    break;
                }
            }
        }

        let Some(end_brace) = close_idx else {
            return Vec::new();
        };

        let inside = &tree[open_idx + 1..end_brace];
        let mut results = Vec::new();

        let mut current = String::new();
        let mut inner_depth = 0;
        for c in inside.chars() {
            if c == '{' {
                inner_depth += 1;
                current.push(c);
            } else if c == '}' {
                inner_depth -= 1;
                current.push(c);
            } else if c == ',' && inner_depth == 0 {
                let chunk = current.trim();
                if !chunk.is_empty() {
                    let sub_prefix = if full_prefix.is_empty() || full_prefix.ends_with("::") {
                        full_prefix.clone()
                    } else {
                        format!("{full_prefix}::")
                    };
                    results.extend(expand_use_tree(&sub_prefix, chunk));
                }
                current.clear();
            } else {
                current.push(c);
            }
        }

        let chunk = current.trim();
        if !chunk.is_empty() {
            let sub_prefix = if full_prefix.is_empty() || full_prefix.ends_with("::") {
                full_prefix.clone()
            } else {
                format!("{full_prefix}::")
            };
            results.extend(expand_use_tree(&sub_prefix, chunk));
        }

        results
    } else {
        let mut item = tree;
        if let Some((before_as, _)) = item.split_once(" as ") {
            item = before_as.trim();
        }
        let item = item.trim();

        if item == "self" {
            let clean_prefix = prefix.trim_end_matches("::");
            if !clean_prefix.is_empty() {
                vec![clean_prefix.to_string()]
            } else {
                Vec::new()
            }
        } else if !item.is_empty() {
            let full = if prefix.is_empty() {
                item.to_string()
            } else if prefix.ends_with("::") {
                format!("{prefix}{item}")
            } else {
                format!("{prefix}::{item}")
            };
            vec![full]
        } else {
            Vec::new()
        }
    }
}

/// Extracts declared submodule names (`mod <name>;`) and imported use paths from Rust source code.
pub fn extract_rust_mods_and_uses(source: &str) -> (Vec<String>, Vec<String>) {
    let cleaned = strip_rust_comments(source);
    let mut declared_mods = BTreeSet::new();
    let mut use_statements = BTreeSet::new();

    let mut current_use: Option<String> = None;

    for raw_line in cleaned.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 1. If currently accumulating a multiline use statement
        if let Some(ref mut acc) = current_use {
            if let Some(semi_pos) = trimmed.find(';') {
                acc.push(' ');
                acc.push_str(&trimmed[..semi_pos]);
                let full = acc.clone();
                current_use = None;

                let tree = full.trim().trim_start_matches("use ").trim();
                for expanded in expand_use_tree("", tree) {
                    use_statements.insert(expanded);
                }
            } else {
                acc.push(' ');
                acc.push_str(trimmed);
            }
            continue;
        }

        let without_vis = strip_visibility(trimmed);

        // 2. Mod declaration: `mod <name>;` (must end with ';' and NOT contain '{')
        if let Some(rest) = without_vis.strip_prefix("mod ") {
            if rest.contains(';') && !rest.contains('{') {
                let semi_pos = rest.find(';').unwrap_or(rest.len());
                let mod_name = rest[..semi_pos].trim();
                let ident: String = mod_name
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !ident.is_empty() {
                    declared_mods.insert(ident);
                }
                continue;
            }
        }

        // 3. Use statement: `use ...;` or start of multiline use
        if let Some(rest) = without_vis.strip_prefix("use ") {
            if let Some(semi_pos) = rest.find(';') {
                let tree = rest[..semi_pos].trim();
                for expanded in expand_use_tree("", tree) {
                    use_statements.insert(expanded);
                }
            } else {
                current_use = Some(format!("use {rest}"));
            }
        }
    }

    (
        declared_mods.into_iter().collect(),
        use_statements.into_iter().collect(),
    )
}

/// Resolves a Rust import or submodule path against internal repository `known_files`.
///
/// Returns `Some(target_file_path)` if it resolves to an internal file within the repository,
/// or `None` if it belongs to standard library or external crate.
pub fn resolve_rust_import(
    importer_rel_path: &str,
    raw_use: &str,
    known_files: &BTreeSet<String>,
) -> Option<String> {
    if is_rust_std(raw_use) {
        return None;
    }

    // 1. Resolve declared submodule: `mod::<name>`
    if let Some(mod_name) = raw_use.strip_prefix("mod::") {
        let mod_name = mod_name.trim();
        let importer_path = Path::new(importer_rel_path);
        let parent_dir = importer_path
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let file_stem = importer_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let mut candidate_bases = Vec::new();
        if file_stem == "mod" || file_stem == "lib" || file_stem == "main" {
            candidate_bases.push(parent_dir.clone());
        } else {
            if !parent_dir.is_empty() {
                candidate_bases.push(format!("{parent_dir}/{file_stem}"));
            } else {
                candidate_bases.push(file_stem.to_string());
            }
            candidate_bases.push(parent_dir);
        }

        for base in candidate_bases {
            let prefix = if base.is_empty() {
                mod_name.to_string()
            } else {
                format!("{base}/{mod_name}")
            };
            let c1 = format!("{prefix}.rs");
            if known_files.contains(&c1) {
                return Some(c1);
            }
            let c2 = format!("{prefix}/mod.rs");
            if known_files.contains(&c2) {
                return Some(c2);
            }
        }
        return None;
    }

    // Determine the crate root directory of importer
    let crate_root_dir = if importer_rel_path.starts_with("crates/") {
        let parts: Vec<&str> = importer_rel_path.split('/').collect();
        if parts.len() >= 3 && parts[2] == "src" {
            format!("{}/{}/src", parts[0], parts[1])
        } else if parts.len() >= 2 {
            format!("{}/{}", parts[0], parts[1])
        } else {
            "src".to_string()
        }
    } else if let Some(idx) = importer_rel_path.find("/src/") {
        importer_rel_path[..idx + 4].to_string()
    } else if importer_rel_path.starts_with("src/") {
        "src".to_string()
    } else {
        Path::new(importer_rel_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    };

    // 2. Resolve `crate::...`
    if let Some(rest) = raw_use.strip_prefix("crate::") {
        let clean_rest = rest.trim();
        let segments: Vec<&str> = clean_rest.split("::").collect();
        for i in (1..=segments.len()).rev() {
            let sub = segments[..i].join("/");
            let c1 = if crate_root_dir.is_empty() {
                format!("{sub}.rs")
            } else {
                format!("{crate_root_dir}/{sub}.rs")
            };
            if known_files.contains(&c1) {
                return Some(c1);
            }
            let c2 = if crate_root_dir.is_empty() {
                format!("{sub}/mod.rs")
            } else {
                format!("{crate_root_dir}/{sub}/mod.rs")
            };
            if known_files.contains(&c2) {
                return Some(c2);
            }
        }
        return None;
    }

    // 3. Resolve `super::...`
    if raw_use.starts_with("super::") {
        let mut count = 0;
        let mut rem = raw_use;
        while let Some(r) = rem.strip_prefix("super::") {
            count += 1;
            rem = r;
        }

        let importer_path = Path::new(importer_rel_path);
        let mut current_dir = importer_path.parent();
        let file_stem = importer_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if file_stem == "mod" {
            current_dir = current_dir.and_then(|p| p.parent());
        }

        for _ in 1..count {
            if let Some(dir) = current_dir {
                current_dir = dir.parent();
            }
        }

        let base_dir = current_dir
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let segments: Vec<&str> = rem.split("::").collect();
        for i in (1..=segments.len()).rev() {
            let sub = segments[..i].join("/");
            let c1 = if base_dir.is_empty() {
                format!("{sub}.rs")
            } else {
                format!("{base_dir}/{sub}.rs")
            };
            if known_files.contains(&c1) {
                return Some(c1);
            }
            let c2 = if base_dir.is_empty() {
                format!("{sub}/mod.rs")
            } else {
                format!("{base_dir}/{sub}/mod.rs")
            };
            if known_files.contains(&c2) {
                return Some(c2);
            }
        }
        return None;
    }

    // 4. Resolve `self::...`
    if let Some(rest) = raw_use.strip_prefix("self::") {
        let importer_path = Path::new(importer_rel_path);
        let parent_dir = importer_path
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let file_stem = importer_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let base_dir = if file_stem == "mod" || file_stem == "lib" || file_stem == "main" {
            parent_dir
        } else if parent_dir.is_empty() {
            file_stem.to_string()
        } else {
            format!("{parent_dir}/{file_stem}")
        };

        let segments: Vec<&str> = rest.split("::").collect();
        for i in (1..=segments.len()).rev() {
            let sub = segments[..i].join("/");
            let c1 = if base_dir.is_empty() {
                format!("{sub}.rs")
            } else {
                format!("{base_dir}/{sub}.rs")
            };
            if known_files.contains(&c1) {
                return Some(c1);
            }
            let c2 = if base_dir.is_empty() {
                format!("{sub}/mod.rs")
            } else {
                format!("{base_dir}/{sub}/mod.rs")
            };
            if known_files.contains(&c2) {
                return Some(c2);
            }
        }
        return None;
    }

    // 5. Check if `raw_use` is an internal submodule of the current crate
    // E.g. in `src/lib.rs`: `use dep_health::...;`
    let segments: Vec<&str> = raw_use.split("::").collect();
    for i in (1..=segments.len()).rev() {
        let sub = segments[..i].join("/");
        let c1 = if crate_root_dir.is_empty() {
            format!("{sub}.rs")
        } else {
            format!("{crate_root_dir}/{sub}.rs")
        };
        if known_files.contains(&c1) {
            return Some(c1);
        }
        let c2 = if crate_root_dir.is_empty() {
            format!("{sub}/mod.rs")
        } else {
            format!("{crate_root_dir}/{sub}/mod.rs")
        };
        if known_files.contains(&c2) {
            return Some(c2);
        }
    }

    // 6. Check if `raw_use` targets another crate in the repository workspace
    // E.g. `use forge_core::ForgeError;` -> `crates/forge-core/src/lib.rs`
    let first_seg = segments.first().copied().unwrap_or(raw_use).trim();
    let hyphenated = first_seg.replace('_', "-");
    let candidates = [
        format!("crates/{first_seg}/src/lib.rs"),
        format!("crates/{hyphenated}/src/lib.rs"),
        format!("crates/{first_seg}/src/main.rs"),
        format!("crates/{hyphenated}/src/main.rs"),
        format!("packages/{first_seg}/src/lib.rs"),
        format!("packages/{hyphenated}/src/lib.rs"),
        format!("{first_seg}/src/lib.rs"),
        format!("{hyphenated}/src/lib.rs"),
    ];

    candidates.into_iter().find(|c| known_files.contains(c))
}

/// Extracts the root third-party crate name if it is not stdlib or internal.
///
/// For example:
/// - `serde::Deserialize` -> `Some("serde")`
/// - `anyhow::Result` -> `Some("anyhow")`
/// - `std::path::Path` -> `None`
/// - `crate::foo::bar` -> `None`
pub fn extract_rust_external_crate(raw_use: &str) -> Option<String> {
    let first = match raw_use.split("::").next() {
        Some(s) => s.trim(),
        None => raw_use.trim(),
    };

    if first.is_empty()
        || first == "crate"
        || first == "super"
        || first == "self"
        || is_rust_std(first)
    {
        return None;
    }

    let clean: String = first
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if clean.is_empty() || is_rust_std(&clean) {
        None
    } else {
        Some(clean)
    }
}
