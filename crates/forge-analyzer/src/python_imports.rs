use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Immutable catalog of standard library root modules across Python 3.8 - 3.13.
/// Imports originating from these modules are treated as standard library, not external PyPI packages.
pub const PYTHON_STDLIB_MODULES: &[&str] = &[
    "__future__",
    "_thread",
    "abc",
    "aifc",
    "argparse",
    "array",
    "ast",
    "asynchat",
    "asyncio",
    "asyncore",
    "atexit",
    "audioop",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cgi",
    "cgitb",
    "chunk",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "cProfile",
    "crypt",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "grp",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "idlelib",
    "imaplib",
    "imghdr",
    "imp",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "mailcap",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "msilib",
    "msvcrt",
    "multiprocessing",
    "netrc",
    "nis",
    "nntplib",
    "numbers",
    "operator",
    "optparse",
    "os",
    "ossaudiodev",
    "parser",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pipes",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "posix",
    "posixpath",
    "pprint",
    "profile",
    "pstats",
    "pty",
    "pwd",
    "py_compile",
    "pyclbr",
    "pydoc",
    "queue",
    "quopri",
    "random",
    "re",
    "readline",
    "reprlib",
    "resource",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtpd",
    "smtplib",
    "sndhdr",
    "socket",
    "socketserver",
    "spwd",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "sunau",
    "symtable",
    "sys",
    "sysconfig",
    "syslog",
    "tabnanny",
    "tarfile",
    "telnetlib",
    "tempfile",
    "termios",
    "test",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "tomllib",
    "trace",
    "traceback",
    "tracemalloc",
    "tty",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uu",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "winreg",
    "winsound",
    "wsgiref",
    "xdrlib",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "zoneinfo",
];

/// Checks if a root module name belongs to the Python standard library.
pub fn is_python_stdlib(module_name: &str) -> bool {
    let root = match module_name.split('.').next() {
        Some(r) => r,
        None => module_name,
    };
    PYTHON_STDLIB_MODULES.contains(&root)
}

/// Strips `#` comments and `"""..."""` / `'''...'''` triple-quoted docstrings from Python source.
pub fn strip_python_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_single_doc = false;
    let mut in_double_doc = false;

    while let Some(c) = chars.next() {
        // Check for triple quotes
        if !in_single_doc && !in_double_doc && c == '"' {
            if chars.clone().take(2).collect::<String>() == "\"\"" {
                chars.next();
                chars.next();
                in_double_doc = true;
                continue;
            }
        } else if in_double_doc && c == '"' {
            if chars.clone().take(2).collect::<String>() == "\"\"" {
                chars.next();
                chars.next();
                in_double_doc = false;
                continue;
            }
            continue;
        }

        if !in_single_doc && !in_double_doc && c == '\'' {
            if chars.clone().take(2).collect::<String>() == "''" {
                chars.next();
                chars.next();
                in_single_doc = true;
                continue;
            }
        } else if in_single_doc && c == '\'' {
            if chars.clone().take(2).collect::<String>() == "''" {
                chars.next();
                chars.next();
                in_single_doc = false;
                continue;
            }
            continue;
        }

        if in_single_doc || in_double_doc {
            if c == '\n' {
                result.push('\n');
            }
            continue;
        }

        // Inline comment
        if c == '#' {
            for next_c in chars.by_ref() {
                if next_c == '\n' {
                    result.push('\n');
                    break;
                }
            }
            continue;
        }

        result.push(c);
    }

    result
}

/// Extracts raw import statements and exported top-level symbols from Python source.
///
/// Returns `(imports, exports)` where:
/// - `imports`: raw import module strings (e.g. `fastapi`, `app.models`, `..services.user`)
/// - `exports`: top-level function names, class names, or `__all__` entries.
pub fn extract_python_imports_and_exports(source: &str) -> (Vec<String>, Vec<String>) {
    let cleaned = strip_python_comments(source);
    let mut imports = BTreeSet::new();
    let mut exports = BTreeSet::new();

    let mut lines = cleaned.lines().map(str::trim).peekable();

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }

        // 1. `from <module> import ...`
        if let Some(rest) = line.strip_prefix("from ") {
            let mut full_line = rest.to_string();

            // Handle multiline parenthesized import: `from x import (\n a,\n b\n)`
            if full_line.contains('(') && !full_line.contains(')') {
                for next_line in lines.by_ref() {
                    full_line.push(' ');
                    full_line.push_str(next_line);
                    if next_line.contains(')') {
                        break;
                    }
                }
            }

            if let Some((module_part, symbols_part)) = full_line.split_once(" import ") {
                let module_name = module_part.trim();
                if !module_name.is_empty() {
                    imports.insert(module_name.to_string());
                }

                // Symbols might also be submodule names: e.g. `from . import utils`, `from ..services import user_service`
                let clean_symbols = symbols_part.replace(['(', ')'], "");
                for raw_sym in clean_symbols.split(',') {
                    let sym = raw_sym.split_whitespace().next().unwrap_or("").trim();
                    if !sym.is_empty() && sym != "*" {
                        let combined = if module_name.ends_with('.') {
                            format!("{module_name}{sym}")
                        } else if module_name.is_empty() {
                            sym.to_string()
                        } else {
                            format!("{module_name}.{sym}")
                        };
                        imports.insert(combined);
                    }
                }
            }
            continue;
        }

        // 2. `import <module1>, <module2> as <alias>`
        if let Some(rest) = line.strip_prefix("import ") {
            for item in rest.split(',') {
                let clean = item.trim();
                let mod_name = clean.split_whitespace().next().unwrap_or("").trim();
                if !mod_name.is_empty() {
                    imports.insert(mod_name.to_string());
                }
            }
            continue;
        }

        // 3. Top-level exports: `def func_name`, `async def func_name`, `class ClassName`
        if let Some(after_def) = line
            .strip_prefix("def ")
            .or_else(|| line.strip_prefix("async def "))
        {
            let ident: String = after_def
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() && !ident.starts_with('_') {
                exports.insert(ident);
            }
            continue;
        }

        if let Some(after_class) = line.strip_prefix("class ") {
            let ident: String = after_class
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() && !ident.starts_with('_') {
                exports.insert(ident);
            }
            continue;
        }

        // 4. `__all__ = ["A", "B"]`
        if let Some(rest) = line.strip_prefix("__all__") {
            let trimmed = rest.trim_start();
            if let Some(after_eq) = trimmed.strip_prefix('=') {
                for part in after_eq.split([',', '[', ']', '(', ')']) {
                    let sym = part.trim().trim_matches(['\'', '"']);
                    if !sym.is_empty() {
                        exports.insert(sym.to_string());
                    }
                }
            }
        }
    }

    (imports.into_iter().collect(), exports.into_iter().collect())
}

/// Resolves a raw Python import specifier from `importer_rel_path` against `known_files`.
///
/// Returns `Some(target_file_path)` if it matches an internal repository file, or `None` if it is
/// external or standard library.
pub fn resolve_python_import(
    importer_rel_path: &str,
    raw_import: &str,
    known_files: &BTreeSet<String>,
) -> Option<String> {
    if is_python_stdlib(raw_import) {
        return None;
    }

    let importer_dir = Path::new(importer_rel_path)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();

    // 1. Relative import (starts with one or more dots)
    if raw_import.starts_with('.') {
        let dot_count = raw_import.chars().take_while(|c| *c == '.').count();
        let remainder = raw_import[dot_count..].trim_start_matches('.');

        let mut current_dir = PathBuf::from(&importer_dir);
        // dot_count == 1: current directory
        // dot_count == 2: parent directory (pop 1)
        // dot_count == 3: grandparent directory (pop 2)
        for _ in 1..dot_count {
            current_dir.pop();
        }

        let base_rel = current_dir.to_string_lossy().replace('\\', "/");
        let sub_path = remainder.replace('.', "/");

        let candidate_base = if base_rel.is_empty() {
            sub_path
        } else if remainder.is_empty() {
            base_rel
        } else {
            format!("{base_rel}/{sub_path}")
        };

        if let Some(resolved) = check_python_file_candidates(&candidate_base, known_files) {
            return Some(resolved);
        }
    }

    // 2. Absolute / internal package import: e.g. `app.services.user_service` -> `app/services/user_service`
    let as_path = raw_import.replace('.', "/");
    if let Some(resolved) = check_python_file_candidates(&as_path, known_files) {
        return Some(resolved);
    }

    // Try relative to importer_dir: e.g. importer is `app/routers/items.py` importing `schemas` -> `app/routers/schemas.py`
    if !importer_dir.is_empty() {
        let nested = format!("{importer_dir}/{as_path}");
        if let Some(resolved) = check_python_file_candidates(&nested, known_files) {
            return Some(resolved);
        }
    }

    // Try root subdirectories: e.g. `from services.user import ...` inside `src/` or `app/`
    for prefix in ["src", "app"] {
        let prefixed = format!("{prefix}/{as_path}");
        if let Some(resolved) = check_python_file_candidates(&prefixed, known_files) {
            return Some(resolved);
        }
    }

    None
}

fn check_python_file_candidates(base: &str, known_files: &BTreeSet<String>) -> Option<String> {
    // 1. Exact `<base>.py`
    let direct_py = format!("{base}.py");
    if known_files.contains(&direct_py) {
        return Some(direct_py);
    }

    // 2. Package module `<base>/__init__.py`
    let init_py = format!("{base}/__init__.py");
    if known_files.contains(&init_py) {
        return Some(init_py);
    }

    // 3. Exact `<base>` (if already ending in .py)
    if base.ends_with(".py") && known_files.contains(base) {
        return Some(base.to_string());
    }

    // 4. If base has a submodule part that is a symbol: e.g. `app/services/user/get_user`
    if let Some((parent, _)) = base.rsplit_once('/') {
        let parent_py = format!("{parent}.py");
        if known_files.contains(&parent_py) {
            return Some(parent_py);
        }
    }

    None
}

/// Extracts the root third-party PyPI package name from an import string if it is not internal or stdlib.
pub fn extract_pypi_package_name(raw_import: &str) -> Option<String> {
    if raw_import.starts_with('.') || is_python_stdlib(raw_import) {
        return None;
    }

    let root = match raw_import.split('.').next() {
        Some(r) => r.trim(),
        None => raw_import.trim(),
    };

    if root.is_empty() {
        None
    } else {
        Some(crate::python_manifest::normalize_python_package_name(root))
    }
}
