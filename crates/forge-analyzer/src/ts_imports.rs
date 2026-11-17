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

    let statements = collect_logical_ts_statements(&cleaned);

    for statement in &statements {
        let line = statement.trim();
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
            } else if rest.starts_with('{') {
                if let Some(close_brace) = rest.find('}') {
                    let inside = &rest[1..close_brace];
                    extract_named_export_list(inside, &mut exports);
                }
            } else if let Some(after_type) = rest.strip_prefix("type ") {
                let after_type = after_type.trim_start();
                if after_type.starts_with('{') {
                    if let Some(close_brace) = after_type.find('}') {
                        let inside = &after_type[1..close_brace];
                        extract_named_export_list(inside, &mut exports);
                    }
                } else {
                    let ident: String = after_type
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                        .collect();
                    if !ident.is_empty() {
                        exports.insert(ident);
                    }
                }
            } else {
                let rest = rest.strip_prefix("async ").unwrap_or(rest).trim_start();
                for kw in [
                    "function ",
                    "const ",
                    "let ",
                    "var ",
                    "class ",
                    "interface ",
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

fn extract_named_export_list(list_content: &str, exports: &mut BTreeSet<String>) {
    for part in list_content.split(',') {
        let item = part.trim();
        if item.is_empty() {
            continue;
        }
        let item = item.strip_prefix("type ").unwrap_or(item).trim();
        let target = if let Some((_, alias)) = item.split_once(" as ") {
            alias.trim()
        } else {
            item
        };
        let ident: String = target
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
            .collect();
        if !ident.is_empty() {
            exports.insert(ident);
        }
    }
}

fn collect_logical_ts_statements(cleaned: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current_stmt = String::new();
    let mut brace_depth: i32 = 0;
    let mut in_import_or_export = false;

    for raw_line in cleaned.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if in_import_or_export {
            current_stmt.push(' ');
            current_stmt.push_str(line);

            for c in line.chars() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                }
            }

            let has_from = current_stmt.contains(" from ")
                || current_stmt.contains(" from'")
                || current_stmt.contains(" from\"");
            let ends_with_semicolon = line.ends_with(';');

            if brace_depth <= 0 {
                if current_stmt.starts_with("import ") || current_stmt.starts_with("import\t") {
                    if has_from || ends_with_semicolon || line.starts_with("from ") {
                        statements.push(current_stmt.trim().to_string());
                        current_stmt.clear();
                        brace_depth = 0;
                        in_import_or_export = false;
                    }
                } else {
                    statements.push(current_stmt.trim().to_string());
                    current_stmt.clear();
                    brace_depth = 0;
                    in_import_or_export = false;
                }
            }
        } else {
            let starts_import = line.starts_with("import ") || line.starts_with("import\t");
            let starts_export = line.starts_with("export ") || line.starts_with("export\t");

            if starts_import || starts_export {
                let mut line_braces = 0;
                for c in line.chars() {
                    if c == '{' {
                        line_braces += 1;
                    } else if c == '}' {
                        line_braces -= 1;
                    }
                }

                let has_from =
                    line.contains(" from ") || line.contains(" from'") || line.contains(" from\"");
                let ends_with_semicolon = line.ends_with(';');

                if line_braces > 0 || (starts_import && !has_from && !ends_with_semicolon) {
                    current_stmt = line.to_string();
                    brace_depth = line_braces;
                    in_import_or_export = true;
                    continue;
                }
            }

            statements.push(line.to_string());
        }
    }

    if !current_stmt.is_empty() {
        statements.push(current_stmt.trim().to_string());
    }

    statements
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if !in_single && !in_double && !in_template && c == '/' {
            if let Some(&'/') = chars.peek() {
                chars.next();
                while let Some(&next_c) = chars.peek() {
                    if next_c == '\n' {
                        break;
                    }
                    chars.next();
                }
                continue;
            } else if let Some(&'*') = chars.peek() {
                chars.next();
                while let Some(bc) = chars.next() {
                    if bc == '\n' {
                        out.push('\n');
                    } else if bc == '*' {
                        if let Some(&'/') = chars.peek() {
                            chars.next();
                            break;
                        }
                    }
                }
                continue;
            }
        }

        if !escaped {
            if c == '\\' && (in_single || in_double || in_template) {
                escaped = true;
            } else if c == '\'' && !in_double && !in_template {
                in_single = !in_single;
            } else if c == '"' && !in_single && !in_template {
                in_double = !in_double;
            } else if c == '`' && !in_single && !in_double {
                in_template = !in_template;
            }
        } else {
            escaped = false;
        }

        out.push(c);
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

    #[test]
    fn extracts_multiline_imports_named_exports_and_preserves_utf8() {
        let code = r#"
// Single line comment with UTF-8: Halo Dunia 🇮🇩
/* Multiline block comment
   eval(forbidden);
   with emoji 🚀 and umlauts äöü
*/
import {
  UserService,
  AuthGuard,
} from '../services/userService';

import type {
  UserRole,
  Permission,
} from '@/types/auth';

export {
  UserService,
  AuthGuard as Guard,
};

export type {
  UserRole,
  Permission as Perm,
};

export { helper } from './helpers';

const apiUrl = `https://api.example.com/v1/auth`;
const greeting = "Selamat Datang 🇮🇩";
"#;

        let (imports, exports) = extract_ts_imports_and_exports(code);
        assert_eq!(
            imports,
            vec!["../services/userService", "./helpers", "@/types/auth"]
        );
        assert_eq!(
            exports,
            vec!["Guard", "Perm", "UserRole", "UserService", "helper"]
        );

        // Verify UTF-8 stripped comment integrity
        let stripped = strip_comments(code);
        assert!(!stripped.contains("Halo Dunia 🇮🇩"));
        assert!(stripped.contains("https://api.example.com/v1/auth"));
        assert!(stripped.contains("Selamat Datang 🇮🇩"));
    }
}
