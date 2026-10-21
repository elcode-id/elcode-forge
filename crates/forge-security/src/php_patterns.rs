use forge_core::{Confidence, Finding, Severity, SourceLocation};

/// Rule ID for PHP unsafe dynamic code execution via `eval(...)`.
pub const RULE_PHP_UNSAFE_EVAL: &str = "security/php-unsafe-eval";

/// Rule ID for PHP untrusted object deserialization via `unserialize(...)`.
pub const RULE_PHP_INSECURE_UNSERIALIZE: &str = "security/php-insecure-unserialize";

/// Rule ID for PHP shell command execution (`exec`, `passthru`, `shell_exec`, `system`, `proc_open`, `popen`).
pub const RULE_PHP_COMMAND_INJECTION: &str = "security/php-command-injection";

/// Rule ID for deprecated and unsafe `create_function(...)`.
pub const RULE_PHP_CREATE_FUNCTION: &str = "security/php-create-function";

fn is_php_file(relative_path: &str) -> bool {
    relative_path.to_ascii_lowercase().ends_with(".php")
}

/// Checks if a trimmed line contains an invocation of `eval(...)`.
fn contains_php_eval(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if let Some(idx) = lower.find("eval") {
        let rest = lower[idx + 4..].trim_start();
        if rest.starts_with('(') {
            // Ensure not part of another word (e.g. `evaluate`, `timeval`)
            if idx == 0 {
                return true;
            }
            let prev = lower.as_bytes()[idx - 1];
            return !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$';
        }
    }
    false
}

/// Checks if a trimmed line contains an invocation of `unserialize(...)`.
fn contains_php_unserialize(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if let Some(idx) = lower.find("unserialize") {
        let rest = lower[idx + 11..].trim_start();
        if rest.starts_with('(') {
            if idx == 0 {
                return true;
            }
            let prev = lower.as_bytes()[idx - 1];
            return !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$' && prev != b'>';
        }
    }
    false
}

/// Checks if a trimmed line contains a shell command execution function.
fn contains_php_command_exec(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    let targets: &[(&str, &str)] = &[
        ("shell_exec", "shell_exec()"),
        ("passthru", "passthru()"),
        ("proc_open", "proc_open()"),
        ("popen", "popen()"),
        ("system", "system()"),
        ("exec", "exec()"),
    ];

    for &(fn_name, label) in targets {
        if let Some(idx) = lower.find(fn_name) {
            let rest = lower[idx + fn_name.len()..].trim_start();
            if rest.starts_with('(') {
                if idx == 0 {
                    return Some(label);
                }
                let prev = lower.as_bytes()[idx - 1];
                if !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$' && prev != b'>' {
                    return Some(label);
                }
            }
        }
    }

    None
}

/// Checks if a trimmed line contains an invocation of `create_function(...)`.
fn contains_php_create_function(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if let Some(idx) = lower.find("create_function") {
        let rest = lower[idx + 15..].trim_start();
        if rest.starts_with('(') {
            if idx == 0 {
                return true;
            }
            let prev = lower.as_bytes()[idx - 1];
            return !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'$';
        }
    }
    false
}

/// Scans `content` for unsafe PHP execution, deserialization, command execution, and deprecated dynamic code.
pub fn scan_php_unsafe_patterns(relative_path: &str, content: &str) -> Vec<Finding> {
    if !is_php_file(relative_path) {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let mut in_block_comment = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = raw_line.trim();

        // Handle block comments across lines: /* ... */
        if in_block_comment {
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }

        if trimmed.starts_with("/*") {
            if !trimmed.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }

        // Single-line comments
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
        {
            continue;
        }

        // 1. Unsafe dynamic evaluation: `eval(...)`
        if contains_php_eval(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PHP-EVAL-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Unsafe dynamic code execution via eval()".to_string(),
                description: format!(
                    "File '{relative_path}' calls eval() at line {line_no}, enabling arbitrary remote code execution if untrusted input reaches this call."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation: "Avoid dynamic code execution with eval(); replace with structured logic or explicit dispatch tables.".to_string(),
                rule_id: RULE_PHP_UNSAFE_EVAL.to_string(),
            });
        }

        // 2. Insecure deserialization: `unserialize(...)`
        if contains_php_unserialize(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PHP-UNSERIALIZE-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Insecure PHP object deserialization via unserialize()".to_string(),
                description: format!(
                    "File '{relative_path}' calls unserialize() at line {line_no}. Untrusted deserialization can instantiate arbitrary classes and trigger magic methods, leading to remote code execution."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation: "Use json_decode() with JSON format or safe structured serializers instead of native PHP unserialize().".to_string(),
                rule_id: RULE_PHP_INSECURE_UNSERIALIZE.to_string(),
            });
        }

        // 3. Command execution functions
        if let Some(cmd_fn) = contains_php_command_exec(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PHP-CMD-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::High,
                title: format!("Potentially insecure shell command execution via {cmd_fn}"),
                description: format!(
                    "File '{relative_path}' executes external shell commands via {cmd_fn} at line {line_no}."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation: "Avoid invoking shell processes directly. If necessary, escape arguments using escapeshellarg() or use a hardened process execution library (e.g. symfony/process).".to_string(),
                rule_id: RULE_PHP_COMMAND_INJECTION.to_string(),
            });
        }

        // 4. Deprecated create_function()
        if contains_php_create_function(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PHP-CREATE-FUNC-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Deprecated and unsafe dynamic function creation via create_function()".to_string(),
                description: format!(
                    "File '{relative_path}' calls deprecated create_function() at line {line_no}. create_function() uses eval() internally and was removed in PHP 8.0."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation: "Replace create_function() with native anonymous functions (closures) or arrow functions `fn(...) => ...`.".to_string(),
                rule_id: RULE_PHP_CREATE_FUNCTION.to_string(),
            });
        }
    }

    findings
}
