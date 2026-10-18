use forge_core::{Confidence, Finding, Severity, SourceLocation};

/// Rule ID for unsafe `eval(...)` execution.
pub const RULE_NO_EVAL: &str = "security/no-eval";

/// Rule ID for `new Function(...)` dynamic code execution.
pub const RULE_NO_IMPLIED_EVAL: &str = "security/no-implied-eval";

/// Rule ID for React `dangerouslySetInnerHTML` usage.
pub const RULE_NO_DANGEROUS_INNER_HTML: &str = "security/no-dangerous-inner-html";

/// Rule ID for unsafe shell execution via `child_process.exec`.
pub const RULE_UNSAFE_SHELL_EXEC: &str = "security/unsafe-shell-exec";

/// Rule ID for overly permissive wildcard CORS headers.
pub const RULE_PERMISSIVE_CORS: &str = "security/permissive-cors";

fn is_js_ts_or_sfc_file(relative_path: &str) -> bool {
    let lower = relative_path.to_ascii_lowercase();
    [
        ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", ".vue", ".svelte",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

/// Scans `content` for unsafe JavaScript/TypeScript execution and configuration patterns.
pub fn scan_unsafe_patterns(relative_path: &str, content: &str) -> Vec<Finding> {
    if !is_js_ts_or_sfc_file(relative_path) {
        return Vec::new();
    }
    let mut findings = Vec::new();
    let imports_child_process = content.contains("child_process");

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }

        if contains_eval_call(trimmed) {
            findings.push(Finding {
                id: format!("SEC-EVAL-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Unsafe dynamic code execution via eval()".to_string(),
                description: format!(
                    "File '{relative_path}' calls eval() at line {line_no}, enabling arbitrary code injection if untrusted input reaches this call."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Replace eval() with safe structured parsing (e.g., JSON.parse) or explicit dispatch tables."
                        .to_string(),
                rule_id: RULE_NO_EVAL.to_string(),
            });
        }

        if trimmed.contains("new Function(") {
            findings.push(Finding {
                id: format!("SEC-NEW-FUNC-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: "Unsafe dynamic code construction via new Function()".to_string(),
                description: format!(
                    "File '{relative_path}' constructs a function from a string at line {line_no}."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation: "Avoid dynamic function compilation from strings.".to_string(),
                rule_id: RULE_NO_IMPLIED_EVAL.to_string(),
            });
        }

        if trimmed.contains("dangerouslySetInnerHTML") {
            findings.push(Finding {
                id: format!("SEC-INNER-HTML-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::Medium,
                confidence: Confidence::High,
                title: "Potential XSS vector via dangerouslySetInnerHTML".to_string(),
                description: format!(
                    "File '{relative_path}' uses dangerouslySetInnerHTML at line {line_no}."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Sanitize HTML content with a strict allowlist sanitizer (e.g., DOMPurify) before rendering."
                        .to_string(),
                rule_id: RULE_NO_DANGEROUS_INNER_HTML.to_string(),
            });
        }

        if imports_child_process && (trimmed.contains("exec(") || trimmed.contains("execSync(")) {
            findings.push(Finding {
                id: format!("SEC-SHELL-EXEC-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::High,
                title: "Unsafe shell command execution via child_process.exec".to_string(),
                description: format!(
                    "File '{relative_path}' invokes shell execution at line {line_no}, risking command injection."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Use child_process.execFile or spawn with an explicit argument array and shell: false."
                        .to_string(),
                rule_id: RULE_UNSAFE_SHELL_EXEC.to_string(),
            });
        }

        if trimmed.contains("Access-Control-Allow-Origin")
            && (trimmed.contains("'*'") || trimmed.contains("\"*\""))
        {
            findings.push(Finding {
                id: format!("SEC-CORS-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::Medium,
                confidence: Confidence::High,
                title: "Permissive wildcard CORS policy ('*')".to_string(),
                description: format!(
                    "File '{relative_path}' configures Access-Control-Allow-Origin: '*' at line {line_no}."
                ),
                evidence: trimmed.to_string(),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Restrict Access-Control-Allow-Origin to trusted origin allowlists."
                        .to_string(),
                rule_id: RULE_PERMISSIVE_CORS.to_string(),
            });
        }
    }

    findings
}

fn contains_eval_call(line: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = line[start..].find("eval(") {
        let abs = start + pos;
        let prev_char = if abs > 0 {
            line[..abs].chars().next_back()
        } else {
            None
        };
        let is_part_of_ident = prev_char
            .map(|c| c.is_ascii_alphanumeric() || c == '_')
            .unwrap_or(false);
        if !is_part_of_ident {
            return true;
        }
        start = abs + 5;
    }
    false
}
