use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Candidate TypeScript/JavaScript/SFC extensions tried when resolving extensionless imports.
const TS_JS_EXTENSIONS: &[&str] = &[
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".vue",
    ".svelte",
    ".mts",
    ".cts",
    ".mjs",
    ".cjs",
    "/index.ts",
    "/index.tsx",
    "/index.js",
    "/index.jsx",
];

/// Extracts raw import module specifiers and exported symbol names from TypeScript/JavaScript source code.
///
/// Strips `//` line comments and `/* ... */` block comments before scanning so commented-out code
/// never produces false edges.
pub fn extract_ts_imports_and_exports(source: &str) -> (Vec<String>, Vec<String>) {
    let cleaned = strip_comments(source);
    let mut imports = BTreeSet::new();
    let mut exports = BTreeSet::new();

    for raw_line in cleaned.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        // 1. Static `import ... from '...'` or `export ... from '...'`
        if (line.starts_with("import ") || line.starts_with("export ")) && line.contains(" from ") {
            if let Some((_, after_from)) = line.rsplit_once(" from ") {
                if let Some(spec) = extract_quoted_literal(after_from.trim()) {
                    imports.insert(spec);
                }
            }
        } else if let Some(rest) = line.strip_prefix("import ") {
            // Side-effect import: `import 'polyfill';`
            let trimmed = rest.trim();
            if trimmed.starts_with('\'') || trimmed.starts_with('"') {
                if let Some(spec) = extract_quoted_literal(trimmed) {
                    imports.insert(spec);
                }
            }
        }

        // 2. Dynamic `import('...')` and CommonJS `require('...')`
        extract_call_specifiers(line, "import(", &mut imports);
        extract_call_specifiers(line, "require(", &mut imports);

        // 3. Exported symbols
        if let Some(rest) = line.strip_prefix("export ") {
            let rest = rest.trim_start();
            if rest.starts_with("default ") {
                exports.insert("default".to_string());
            } else {
                let rest = rest.strip_prefix("async ").unwrap_or(rest).trim_start();
                for kw in [
                    "function ",
                    "const ",
                    "let ",
                    "var ",
                    "class ",
                    "interface ",
                    "type ",
                    "enum ",
                ] {
                    if let Some(after_kw) = rest.strip_prefix(kw) {
                        let ident: String = after_kw
                            .trim_start()
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                            .collect();
                        if !ident.is_empty() {
                            exports.insert(ident);
                        }
                    }
                }
            }
        }
    }

    (imports.into_iter().collect(), exports.into_iter().collect())
}

/// Resolves a relative (`./`, `../`) or `@/` aliased specifier from `importer_rel_path` against
/// the set of `known_files` in the repository.
pub fn resolve_import_specifier(
    importer_rel_path: &str,
    specifier: &str,
    known_files: &BTreeSet<String>,
) -> Option<String> {
    let base_candidate = if let Some(stripped) = specifier.strip_prefix("@/") {
        format!("src/{stripped}")
    } else if specifier.starts_with("./") || specifier.starts_with("../") {
        let importer_dir = Path::new(importer_rel_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        normalize_joined_path(importer_dir, Path::new(specifier))?
    } else {
        return None;
    };

    if known_files.contains(&base_candidate) {
        return Some(base_candidate);
    }

    for suffix in TS_JS_EXTENSIONS {
        let candidate = format!("{base_candidate}{suffix}");
        if known_files.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

/// Extracts the external npm package name from a non-relative, non-alias import specifier.
///
/// Examples:
/// - `"react"` -> `Some("react")`
/// - `"next/navigation"` -> `Some("next")`
/// - `"@sveltejs/kit/hooks"` -> `Some("@sveltejs/kit")`
/// - `"../db/client"` -> `None`
pub fn extract_npm_package_name(specifier: &str) -> Option<String> {
    let trimmed = specifier.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('.')
        || trimmed.starts_with('/')
        || trimmed.starts_with("@/")
        || trimmed.starts_with("node:")
    {
        return None;
    }

    if trimmed.starts_with('@') {
        let mut parts = trimmed.split('/');
        let scope = parts.next()?;
        let name = parts.next()?;
        if scope.len() > 1 && !name.is_empty() {
            return Some(format!("{scope}/{name}"));
        }
        return None;
    }

    let pkg = trimmed.split('/').next()?;
    if pkg.is_empty() {
        None
    } else {
        Some(pkg.to_string())
    }
}

fn normalize_joined_path(base_dir: &Path, rel: &Path) -> Option<String> {
    let joined = base_dir.join(rel);
    let mut stack: Vec<String> = Vec::new();

    for comp in joined.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(part) => {
                stack.push(part.to_str()?.to_string());
            }
            Component::ParentDir => {
                stack.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if stack.is_empty() {
        None
    } else {
        let mut out = PathBuf::new();
        for item in &stack {
            out.push(item);
        }
        Some(stack.join("/"))
    }
}

fn extract_call_specifiers(line: &str, pattern: &str, out: &mut BTreeSet<String>) {
    let mut search_from = 0;
    while let Some(pos) = line[search_from..].find(pattern) {
        let start = search_from + pos + pattern.len();
        let rest = line[start..].trim_start();
        if let Some(spec) = extract_quoted_literal(rest) {
            out.insert(spec);
        }
        search_from = start;
    }
}

fn extract_quoted_literal(input: &str) -> Option<String> {
    let first = input.chars().next()?;
    if first != '\'' && first != '"' {
        return None;
    }
    let after_quote = &input[first.len_utf8()..];
    let end = after_quote.find(first)?;
    let literal = &after_quote[..end];
    if literal.is_empty() {
        None
    } else {
        Some(literal.to_string())
    }
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < bytes.len() {
        let b = bytes[i];
        if !in_single && !in_double && i + 1 < bytes.len() {
            if b == b'/' && bytes[i + 1] == b'/' {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if b == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                continue;
            }
        }

        if b == b'\'' && !in_double {
            in_single = !in_single;
        } else if b == b'"' && !in_single {
            in_double = !in_double;
        }

        out.push(b as char);
        i += 1;
    }

    out
}

/// Extracts the contents of all `<script ...>...</script>` blocks from a `.vue` or `.svelte`
/// Single-File Component (SFC) so TypeScript/JavaScript imports and exports can be analyzed.
pub fn extract_sfc_script_content(raw_sfc: &str) -> String {
    let mut extracted = String::new();
    let mut remaining = raw_sfc;

    while let Some(open_idx) = remaining.find("<script") {
        let after_open = &remaining[open_idx + "<script".len()..];
        let Some(tag_end_idx) = after_open.find('>') else {
            break;
        };
        let body_start = &after_open[tag_end_idx + 1..];
        let Some(close_idx) = body_start.find("</script>") else {
            break;
        };
        if !extracted.is_empty() {
            extracted.push('\n');
        }
        extracted.push_str(&body_start[..close_idx]);
        remaining = &body_start[close_idx + "</script>".len()..];
    }

    extracted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sfc_script_blocks_from_vue_and_svelte() {
        let vue_sfc = r#"
<template>
  <div>{{ title }}</div>
</template>
<script setup lang="ts">
import { ref } from 'vue';
import { fetchUser } from '../services/userService';
export const title = ref('Hello');
</script>
"#;
        let script = extract_sfc_script_content(vue_sfc);
        let (imports, exports) = extract_ts_imports_and_exports(&script);
        assert_eq!(imports, vec!["../services/userService", "vue"]);
        assert_eq!(exports, vec!["title"]);
    }

    #[test]
    fn extracts_imports_exports_and_ignores_comments() {
        let code = r#"
// import { fake } from 'commented-out';
/* import { blockFake } from 'block-commented'; */
import React from 'react';
import { queryDb } from '../db/client';
const dyn = require('@sveltejs/kit/hooks');

export interface UserCardProps { userId: string; }
export function UserCard() { return null; }
export default UserCard;
"#;
        let (imports, exports) = extract_ts_imports_and_exports(code);
        assert_eq!(
            imports,
            vec!["../db/client", "@sveltejs/kit/hooks", "react"]
        );
        assert_eq!(exports, vec!["UserCard", "UserCardProps", "default"]);
    }

    #[test]
    fn resolves_relative_and_alias_specifiers_and_extracts_npm_packages() {
        let mut known = BTreeSet::new();
        known.insert("src/components/UserCard.tsx".to_string());
        known.insert("src/db/client.ts".to_string());
        known.insert("src/services/userService.ts".to_string());

        assert_eq!(
            resolve_import_specifier("src/components/UserCard.tsx", "../db/client", &known)
                .as_deref(),
            Some("src/db/client.ts")
        );
        assert_eq!(
            resolve_import_specifier("src/pages/index.tsx", "@/services/userService", &known)
                .as_deref(),
            Some("src/services/userService.ts")
        );

        assert_eq!(extract_npm_package_name("react").as_deref(), Some("react"));
        assert_eq!(
            extract_npm_package_name("@sveltejs/kit/hooks").as_deref(),
            Some("@sveltejs/kit")
        );
        assert_eq!(extract_npm_package_name("../db/client"), None);
        assert_eq!(extract_npm_package_name("@/services/userService"), None);
    }
}
