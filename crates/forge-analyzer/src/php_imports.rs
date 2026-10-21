use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{FileNode, ForgeError, Module};

use crate::graph::ArchitectureGraph;

/// Catalog of standard PHP built-in classes, interfaces, and exceptions across PHP 8.0 - 8.3+.
pub const PHP_BUILTIN_CLASSES: &[&str] = &[
    "stdClass",
    "DateTime",
    "DateTimeImmutable",
    "DateTimeZone",
    "DateInterval",
    "DatePeriod",
    "Exception",
    "ErrorException",
    "Error",
    "TypeError",
    "ParseError",
    "ArithmeticError",
    "DivisionByZeroError",
    "UnhandledMatchError",
    "ValueError",
    "Throwable",
    "Closure",
    "Generator",
    "Fiber",
    "WeakReference",
    "WeakMap",
    "ArrayAccess",
    "Iterator",
    "IteratorAggregate",
    "Traversable",
    "Countable",
    "Serializable",
    "JsonSerializable",
    "Stringable",
    "PDO",
    "PDOStatement",
    "PDOException",
    "SimpleXMLElement",
    "DOMDocument",
    "XMLReader",
    "XMLWriter",
    "CURLFile",
    "SplFileInfo",
    "SplFileObject",
    "SplQueue",
    "SplStack",
    "SplHeap",
    "SplPriorityQueue",
];

/// Checks if a class or interface name is a built-in PHP standard type.
pub fn is_php_builtin(class_name: &str) -> bool {
    let clean = class_name.trim().trim_start_matches('\\');
    // If it has no namespace separator and matches built-in
    if !clean.contains('\\') && PHP_BUILTIN_CLASSES.contains(&clean) {
        return true;
    }
    false
}

/// Strips `//` and `#` line comments and `/* ... */` block comments from PHP source code,
/// preserving string literals and newlines.
pub fn strip_php_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // 1. Single-quoted strings: '...' (escaped \')
        if chars[i] == '\'' {
            result.push('\'');
            i += 1;
            while i < n {
                if chars[i] == '\\' && i + 1 < n {
                    result.push(chars[i]);
                    result.push(chars[i + 1]);
                    i += 2;
                } else if chars[i] == '\'' {
                    result.push('\'');
                    i += 1;
                    break;
                } else {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            continue;
        }

        // 2. Double-quoted strings: "..." (escaped \")
        if chars[i] == '"' {
            result.push('"');
            i += 1;
            while i < n {
                if chars[i] == '\\' && i + 1 < n {
                    result.push(chars[i]);
                    result.push(chars[i + 1]);
                    i += 2;
                } else if chars[i] == '"' {
                    result.push('"');
                    i += 1;
                    break;
                } else {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            continue;
        }

        // 3. Multi-line block comments: /* ... */
        if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    result.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }

        // 4. Single-line comments: // or #
        if (chars[i] == '/' && i + 1 < n && chars[i + 1] == '/') || chars[i] == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Extracts the declared `namespace` and all imported `use` statements from PHP source code.
///
/// Handles single use statements, aliased use statements (`use Foo as Bar`), and group
/// use statements (`use App\Services\{Foo, Bar as Baz}`).
pub fn extract_php_namespace_and_uses(source: &str) -> (Option<String>, Vec<String>) {
    let clean = strip_php_comments(source);
    let mut declared_namespace = None;
    let mut uses = Vec::new();

    let mut in_use_statement = false;
    let mut current_use = String::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detect declared namespace: `namespace App\Http\Controllers;` or `namespace App\Http\Controllers {`
        if declared_namespace.is_none()
            && (trimmed.starts_with("namespace ") || trimmed.starts_with("namespace\t"))
        {
            let rest = trimmed.strip_prefix("namespace").unwrap_or("").trim();
            let ns_name = rest.trim_end_matches(';').trim_end_matches('{').trim();
            if !ns_name.is_empty() {
                declared_namespace = Some(ns_name.trim_start_matches('\\').to_string());
            }
            continue;
        }

        // Accumulate multi-line use statements (e.g. group use statements spanning lines)
        if in_use_statement {
            current_use.push(' ');
            current_use.push_str(trimmed);
            if trimmed.contains(';') {
                in_use_statement = false;
                parse_use_statement(&current_use, &mut uses);
                current_use.clear();
            }
            continue;
        }

        // Check for start of `use ` statement outside class body (ignore `use function` and `use const` prefixes if any)
        if trimmed.starts_with("use ") || trimmed.starts_with("use\t") {
            // Distinguish from trait use inside a method/class if possible
            // In PHP, top-level use statements always have a namespace or are before class
            if trimmed.contains(';') {
                parse_use_statement(trimmed, &mut uses);
            } else {
                in_use_statement = true;
                current_use = trimmed.to_string();
            }
        }
    }

    uses.sort();
    uses.dedup();

    (declared_namespace, uses)
}

/// Helper that parses a single or grouped `use ...;` statement string and appends canonical class names to `uses`.
fn parse_use_statement(stmt: &str, uses: &mut Vec<String>) {
    let raw = stmt.trim();
    let Some(rest) = raw.strip_prefix("use ") else {
        return;
    };
    let clean = rest.trim_end_matches(';').trim();

    // Skip `use function ...` or `use const ...`
    if clean.starts_with("function ") || clean.starts_with("const ") {
        return;
    }

    // Check for group use syntax: `prefix\{Item1, Item2 as Alias}`
    if let Some(brace_idx) = clean.find('{') {
        if let Some(end_brace) = clean.rfind('}') {
            let prefix = clean[..brace_idx].trim().trim_end_matches('\\');
            let inner = &clean[brace_idx + 1..end_brace];
            for part in inner.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                // Strip alias if any: `Foo as Bar`
                let item_name = p.split_whitespace().next().unwrap_or(p);
                let full = if prefix.is_empty() {
                    item_name.trim_start_matches('\\').to_string()
                } else {
                    format!("{prefix}\\{}", item_name.trim_start_matches('\\'))
                };
                if !is_php_builtin(&full) {
                    uses.push(full);
                }
            }
            return;
        }
    }

    // Handle multiple comma-separated imports: `use Foo, Bar;`
    for item in clean.split(',') {
        let it = item.trim();
        if it.is_empty() {
            continue;
        }
        // Strip alias: `App\Models\User as UserModel` -> `App\Models\User`
        let canonical = it.split_whitespace().next().unwrap_or(it);
        let clean_class = canonical.trim_start_matches('\\').to_string();
        if !clean_class.is_empty() && !is_php_builtin(&clean_class) {
            uses.push(clean_class);
        }
    }
}

/// Infers the logical module `(id, name)` from a normalized relative PHP file path.
///
/// Supports Laravel conventions (`app/Http/Controllers`, `app/Models`, `app/Services`, `app/Providers`),
/// standard PSR-4 paths (`src/Domain`, `src/Infrastructure`), and test suites (`tests/Feature`, `tests/Unit`).
pub fn infer_php_module_boundary(relative_path: &str) -> (String, String) {
    let normalized = relative_path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();

    // Laravel application directory (`app/...`)
    if parts.len() >= 3 && parts[0] == "app" {
        // e.g. "app/Http/Controllers/UserController.php" -> ("app/Http/Controllers", "Controllers")
        if parts[1] == "Http" && parts.len() >= 4 {
            let id = parts[..3].join("/");
            let name = parts[2].to_string();
            return (id, name);
        }
        // e.g. "app/Models/User.php" -> ("app/Models", "Models")
        // e.g. "app/Services/UserService.php" -> ("app/Services", "Services")
        let id = parts[..2].join("/");
        let name = parts[1].to_string();
        return (id, name);
    }

    // Tests directory (`tests/...`)
    if parts.len() >= 3 && parts[0] == "tests" {
        // e.g. "tests/Feature/UserTest.php" -> ("tests/Feature", "Feature")
        let id = parts[..2].join("/");
        let name = parts[1].to_string();
        return (id, name);
    }

    // Standard PSR-4 `src/` directory
    if parts.len() >= 3 && parts[0] == "src" {
        let id = parts[..2].join("/");
        let name = parts[1].to_string();
        return (id, name);
    }

    // Fallback: parent directory
    if parts.len() >= 2 {
        let dir_parts = &parts[..parts.len() - 1];
        let id = dir_parts.join("/");
        let name = dir_parts.last().unwrap_or(&"root").to_string();
        (id, name)
    } else {
        ("root".to_string(), "root".to_string())
    }
}

/// Resolves a PHP class use statement to an internal repository file path using PSR-4 mappings
/// and known project files.
///
/// Returns `Some(relative_path)` if resolved to a file within the project, or `None` if it refers
/// to an external third-party package or non-existent class.
pub fn resolve_php_import(
    raw_use: &str,
    psr4_mappings: &BTreeMap<String, String>,
    known_files: &BTreeSet<String>,
) -> Option<String> {
    let clean_use = raw_use.trim().trim_start_matches('\\');

    // 1. Try explicit PSR-4 autoload mappings from composer.json
    for (prefix, base_dir) in psr4_mappings {
        let clean_prefix = prefix.trim_start_matches('\\');
        if let Some(suffix) = clean_use.strip_prefix(clean_prefix) {
            let file_subpath = suffix.replace('\\', "/");
            let clean_subpath = file_subpath.trim_start_matches('/');
            let candidate = if base_dir.is_empty() {
                format!("{clean_subpath}.php")
            } else {
                format!("{base_dir}/{clean_subpath}.php")
            };
            if known_files.contains(&candidate) {
                return Some(candidate);
            }
        }
    }

    // 2. Default Laravel / standard convention: `App\` -> `app/`
    if let Some(sub) = clean_use.strip_prefix("App\\") {
        let candidate = format!("app/{}.php", sub.replace('\\', "/"));
        if known_files.contains(&candidate) {
            return Some(candidate);
        }
    }

    // 3. Fallback: direct match under known files
    let direct_candidate = format!("{}.php", clean_use.replace('\\', "/"));
    if known_files.contains(&direct_candidate) {
        return Some(direct_candidate);
    }

    None
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.extend(ch.to_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Extracts a probable third-party Composer package name from an external class namespace.
///
/// Examples:
/// - `"Illuminate\\Support\\Facades\\DB"` -> `Some("laravel/framework")`
/// - `"GuzzleHttp\\Client"` -> `Some("guzzlehttp/guzzle")`
/// - `"Symfony\\Component\\HttpFoundation\\Request"` -> `Some("symfony/http-foundation")`
/// - `"Monolog\\Logger"` -> `Some("monolog/monolog")`
pub fn extract_composer_package_name(raw_use: &str) -> Option<String> {
    let clean = raw_use.trim().trim_start_matches('\\');
    let parts: Vec<&str> = clean.split('\\').collect();
    if parts.is_empty() {
        return None;
    }

    let vendor = parts[0].to_lowercase();
    match vendor.as_str() {
        "illuminate" => Some("laravel/framework".to_string()),
        "guzzlehttp" => {
            if parts.len() >= 2 {
                let p1 = parts[1].to_lowercase();
                if p1 == "psr7" || p1 == "promises" {
                    Some(format!("guzzlehttp/{p1}"))
                } else {
                    Some("guzzlehttp/guzzle".to_string())
                }
            } else {
                Some("guzzlehttp/guzzle".to_string())
            }
        }
        "symfony" => {
            if parts.len() >= 3 && parts[1].eq_ignore_ascii_case("component") {
                Some(format!("symfony/{}", to_kebab_case(parts[2])))
            } else {
                Some("symfony/symfony".to_string())
            }
        }
        "monolog" => Some("monolog/monolog".to_string()),
        "carbon" => Some("nesbot/carbon".to_string()),
        "spatie" => {
            if parts.len() >= 2 {
                Some(format!("spatie/{}", parts[1].to_lowercase()))
            } else {
                Some("spatie".to_string())
            }
        }
        "psr" => {
            if parts.len() >= 2 {
                Some(format!("psr/{}-implementation", parts[1].to_lowercase()))
            } else {
                Some("psr".to_string())
            }
        }
        _ => {
            // General two-level vendor: `Vendor\Package` -> `vendor/package`
            if parts.len() >= 2 {
                Some(format!("{}/{}", vendor, parts[1].to_lowercase()))
            } else {
                Some(vendor)
            }
        }
    }
}

/// Builds an [`ArchitectureGraph`] for all PHP files in `files`.
pub fn build_php_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<ArchitectureGraph, ForgeError> {
    let php_files: Vec<&FileNode> = files
        .iter()
        .filter(|f| {
            matches!(f.language.as_deref(), Some("php")) || f.relative_path.ends_with(".php")
        })
        .collect();

    if php_files.is_empty() {
        return Ok(ArchitectureGraph {
            modules: Vec::new(),
            circular_dependencies: Vec::new(),
            high_centrality_modules: Vec::new(),
            external_imports: BTreeSet::new(),
        });
    }

    let known_files: BTreeSet<String> = php_files.iter().map(|f| f.relative_path.clone()).collect();

    // Load PSR-4 mappings from composer.json if present
    let psr4_mappings =
        if let Ok(manifest) = crate::composer_manifest::parse_composer_manifest(repo_root) {
            manifest.psr4_mappings
        } else {
            let mut def = BTreeMap::new();
            def.insert("App\\".to_string(), "app".to_string());
            def
        };

    let mut module_map: BTreeMap<String, Module> = BTreeMap::new();
    let mut file_to_module: BTreeMap<String, String> = BTreeMap::new();

    // 1. Initialize modules from file locations
    for file in &php_files {
        let (mod_id, mod_name) = infer_php_module_boundary(&file.relative_path);
        file_to_module.insert(file.relative_path.clone(), mod_id.clone());

        module_map
            .entry(mod_id.clone())
            .and_modify(|m| m.files.push(file.relative_path.clone()))
            .or_insert_with(|| Module {
                id: mod_id,
                name: mod_name,
                root_path: infer_php_module_boundary(&file.relative_path).0,
                files: vec![file.relative_path.clone()],
                imports: Vec::new(),
                exports: Vec::new(),
            });
    }

    let mut module_imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut all_external_imports: BTreeSet<String> = BTreeSet::new();

    // 2. Parse use statements and resolve internal/external dependencies
    for file in &php_files {
        let full_path = repo_root.join(&file.relative_path);
        let Ok(content) = fs::read_to_string(&full_path) else {
            continue;
        };

        let current_mod_id = match file_to_module.get(&file.relative_path) {
            Some(id) => id.clone(),
            None => continue,
        };

        let (_, uses) = extract_php_namespace_and_uses(&content);

        for u in uses {
            if let Some(target_file) = resolve_php_import(&u, &psr4_mappings, &known_files) {
                if let Some(target_mod_id) = file_to_module.get(&target_file) {
                    if target_mod_id != &current_mod_id {
                        module_imports
                            .entry(current_mod_id.clone())
                            .or_default()
                            .insert(target_mod_id.clone());
                    }
                }
            } else if let Some(pkg) = extract_composer_package_name(&u) {
                all_external_imports.insert(pkg);
            }
        }
    }

    // 3. Populate module imports and sort deterministically
    for (mod_id, imports) in module_imports {
        if let Some(m) = module_map.get_mut(&mod_id) {
            m.imports = imports.into_iter().collect();
        }
    }

    for m in module_map.values_mut() {
        m.files.sort();
        m.imports.sort();
        m.exports.sort();
    }

    let modules: Vec<Module> = module_map.into_values().collect();
    let circular_dependencies = crate::graph::detect_circular_dependencies(&modules);
    let high_centrality_modules = crate::graph::compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports: all_external_imports,
    })
}
