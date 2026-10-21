use std::collections::BTreeSet;
use std::path::Path;

/// Immutable catalog of standard library root packages in Go 1.18 - 1.23+.
pub const GO_STDLIB_ROOT_PACKAGES: &[&str] = &[
    "archive",
    "bufio",
    "builtin",
    "bytes",
    "cmd",
    "compress",
    "container",
    "context",
    "crypto",
    "database",
    "debug",
    "embed",
    "encoding",
    "errors",
    "expvar",
    "flag",
    "fmt",
    "go",
    "hash",
    "html",
    "image",
    "index",
    "internal",
    "io",
    "log",
    "math",
    "mime",
    "net",
    "os",
    "path",
    "plugin",
    "reflect",
    "regexp",
    "runtime",
    "sort",
    "strconv",
    "strings",
    "sync",
    "syscall",
    "test2json",
    "testing",
    "text",
    "time",
    "unicode",
    "unsafe",
];

/// Checks if an import path belongs to the official Go standard library.
pub fn is_go_stdlib(import_path: &str) -> bool {
    let root = match import_path.split('/').next() {
        Some(r) => r.trim(),
        None => import_path.trim(),
    };

    // Third-party packages have domain names in the first segment (e.g. github.com, golang.org)
    if root.contains('.') {
        return false;
    }

    GO_STDLIB_ROOT_PACKAGES.contains(&root)
}

/// Strips `//` line comments and `/* ... */` block comments from Go source, preserving newlines.
pub fn strip_go_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '/' {
            if let Some(&next) = chars.peek() {
                if next == '/' {
                    // Line comment
                    chars.next();
                    for next_c in chars.by_ref() {
                        if next_c == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                    continue;
                } else if next == '*' {
                    // Block comment
                    chars.next();
                    while let Some(b) = chars.next() {
                        if b == '\n' {
                            result.push('\n');
                        } else if b == '*' {
                            if let Some(&slash) = chars.peek() {
                                if slash == '/' {
                                    chars.next();
                                    break;
                                }
                            }
                        }
                    }
                    continue;
                }
            }
        }

        result.push(c);
    }

    result
}

/// Extracts all imported package paths from Go source code.
///
/// Handles single-line imports (`import "fmt"`, `import alias "path"`) and multiline blocks (`import (\n ... \n)`).
pub fn extract_go_imports(source: &str) -> Vec<String> {
    let cleaned = strip_go_comments(source);
    let mut imports = BTreeSet::new();
    let mut in_import_block = false;

    for raw_line in cleaned.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Start/end of multiline import block: `import (`
        if trimmed.starts_with("import (") || trimmed == "import (" || trimmed == "import(" {
            in_import_block = true;
            continue;
        }

        if in_import_block {
            if trimmed.starts_with(')') {
                in_import_block = false;
                continue;
            }

            if let Some(pkg) = extract_import_path_from_line(trimmed) {
                imports.insert(pkg);
            }
            continue;
        }

        // Single-line import: `import "fmt"` or `import alias "path"`
        if let Some(rest) = trimmed.strip_prefix("import ") {
            if let Some(pkg) = extract_import_path_from_line(rest.trim()) {
                imports.insert(pkg);
            }
        }
    }

    imports.into_iter().collect()
}

/// Extracts the quoted import path from a line item.
fn extract_import_path_from_line(line: &str) -> Option<String> {
    let start_quote = line.find('"')?;
    let remainder = &line[start_quote + 1..];
    let end_quote = remainder.find('"')?;
    let path = remainder[..end_quote].trim();

    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Resolves a raw Go import specifier against internal repository `known_files`.
///
/// Returns `Some(target_file_path)` if it resolves to an internal package within the repository,
/// or `None` if it is external or standard library.
pub fn resolve_go_import(
    importer_rel_path: &str,
    raw_import: &str,
    module_path: Option<&str>,
    known_files: &BTreeSet<String>,
) -> Option<String> {
    if is_go_stdlib(raw_import) {
        return None;
    }

    // 1. Internal module package matching declared `module_path`
    if let Some(mod_root) = module_path {
        if raw_import == mod_root {
            // Root package
            for file in known_files {
                if !file.contains('/') {
                    return Some(file.clone());
                }
            }
        } else if let Some(sub) = raw_import.strip_prefix(mod_root) {
            let clean_sub = sub.trim_start_matches('/');
            let prefix = format!("{clean_sub}/");
            for file in known_files {
                if file.starts_with(&prefix) {
                    return Some(file.clone());
                }
            }
        }
    }

    // 2. Relative import (e.g. `./subpkg` or `../services`)
    if raw_import.starts_with('.') {
        let importer_dir = Path::new(importer_rel_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let combined = if importer_dir.is_empty() {
            raw_import.trim_start_matches("./").to_string()
        } else {
            format!("{}/{}", importer_dir, raw_import.trim_start_matches("./"))
        };

        let prefix = format!("{combined}/");
        for file in known_files {
            if file.starts_with(&prefix) {
                return Some(file.clone());
            }
        }
    }

    None
}

/// Extracts the root third-party module name from an import path if it is not stdlib or internal.
///
/// For example:
/// - `github.com/gin-gonic/gin/binding` -> `github.com/gin-gonic/gin`
/// - `golang.org/x/crypto/bcrypt` -> `golang.org/x/crypto`
pub fn extract_go_external_module(raw_import: &str, module_path: Option<&str>) -> Option<String> {
    if is_go_stdlib(raw_import) {
        return None;
    }

    if let Some(mod_root) = module_path {
        if raw_import == mod_root || raw_import.starts_with(&format!("{mod_root}/")) {
            return None;
        }
    }

    let parts: Vec<&str> = raw_import.split('/').collect();
    if parts.len() >= 3 && parts[0].contains('.') {
        // e.g. github.com/user/repo
        Some(format!("{}/{}/{}", parts[0], parts[1], parts[2]))
    } else if !parts.is_empty() && parts[0].contains('.') {
        Some(parts[0].to_string())
    } else {
        Some(raw_import.to_string())
    }
}
