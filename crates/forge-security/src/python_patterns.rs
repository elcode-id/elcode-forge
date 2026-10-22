use forge_core::{Confidence, Finding, Severity, SourceLocation};

/// Rule ID for Python unsafe `eval(...)` and `exec(...)` execution.
pub const RULE_PYTHON_UNSAFE_EVAL: &str = "security/python-unsafe-eval";

/// Rule ID for Python insecure subprocess invocation (e.g. `shell=True`, `os.system`, `os.popen`).
pub const RULE_PYTHON_INSECURE_SUBPROCESS: &str = "security/python-insecure-subprocess";

/// Rule ID for untrusted Python pickle deserialization (`pickle.loads`, `_pickle.loads`).
pub const RULE_PYTHON_PICKLE_DESERIALIZATION: &str = "security/python-pickle-deserialization";

fn is_python_file(relative_path: &str) -> bool {
    relative_path.to_ascii_lowercase().ends_with(".py")
}

/// Scans `content` for unsafe Python execution, subprocess, and deserialization patterns.
pub fn scan_python_unsafe_patterns(relative_path: &str, content: &str) -> Vec<Finding> {
    if !is_python_file(relative_path) {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let mut in_multiline_doc = false;
    let mut doc_delimiter = "";

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = raw_line.trim();

        // Handle docstrings across lines
        if in_multiline_doc {
            if trimmed.contains(doc_delimiter) {
                in_multiline_doc = false;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
            if !rest.contains("\"\"\"") {
                in_multiline_doc = true;
                doc_delimiter = "\"\"\"";
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("'''") {
            if !rest.contains("'''") {
                in_multiline_doc = true;
                doc_delimiter = "'''";
            }
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 1. Unsafe dynamic evaluation: `eval(...)` or `exec(...)`
        if contains_python_eval_or_exec(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PY-EVAL-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Unsafe dynamic code execution via eval() or exec()".to_string(),
                description: format!(
                    "File '{relative_path}' invokes eval() or exec() at line {line_no}, allowing arbitrary code execution if dynamic input is passed."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Avoid eval() or exec(). Use safe parsers (e.g. ast.literal_eval) or explicit mapping tables."
                        .to_string(),
                rule_id: RULE_PYTHON_UNSAFE_EVAL.to_string(),
            });
        }

        // 2. Insecure subprocess invocation: `shell=True` or `os.system(...)` or `os.popen(...)`
        if contains_insecure_subprocess(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PY-SUBPROCESS-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::High,
                title: "Insecure shell execution risking command injection".to_string(),
                description: format!(
                    "File '{relative_path}' executes commands with shell=True or legacy os.system/popen at line {line_no}."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Set shell=False and pass arguments as an explicit list (e.g. ['executable', 'arg1']), or use subprocess.run without a shell."
                        .to_string(),
                rule_id: RULE_PYTHON_INSECURE_SUBPROCESS.to_string(),
            });
        }

        // 3. Untrusted pickle deserialization: `pickle.loads`, `_pickle.loads`, `pickle.load`
        if contains_pickle_deserialization(trimmed) {
            findings.push(Finding {
                id: format!("SEC-PY-PICKLE-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Unsafe deserialization via pickle".to_string(),
                description: format!(
                    "File '{relative_path}' deserializes data with pickle at line {line_no}, which can execute arbitrary code during unpickling."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Never deserialize untrusted data with pickle. Use safe structured serialization formats such as JSON or Protocol Buffers."
                        .to_string(),
                rule_id: RULE_PYTHON_PICKLE_DESERIALIZATION.to_string(),
            });
        }
    }

    findings
}

fn contains_python_eval_or_exec(line: &str) -> bool {
    for target in ["eval(", "exec("] {
        let mut start = 0;
        while let Some(pos) = line[start..].find(target) {
            let abs = start + pos;
            let prev_char = if abs > 0 {
                line[..abs].chars().next_back()
            } else {
                None
            };
            let is_part_of_ident = prev_char
                .map(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                .unwrap_or(false);
            if !is_part_of_ident {
                return true;
            }
            start = abs + target.len();
        }
    }
    false
}

fn contains_insecure_subprocess(line: &str) -> bool {
    // Check for `shell=True` or `shell = True`
    if let Some(pos) = line.find("shell") {
        let after = line[pos + 5..].trim_start();
        if let Some(after_eq) = after.strip_prefix('=') {
            let val = after_eq.trim_start();
            if val.starts_with("True") {
                return true;
            }
        }
    }

    // Check for `os.system(` or `os.popen(`
    for target in ["os.system(", "os.popen("] {
        if line.contains(target) {
            return true;
        }
    }

    false
}

fn contains_pickle_deserialization(line: &str) -> bool {
    for target in [
        "pickle.loads(",
        "_pickle.loads(",
        "pickle.load(",
        "_pickle.load(",
    ] {
        if line.contains(target) {
            return true;
        }
    }
    false
}
