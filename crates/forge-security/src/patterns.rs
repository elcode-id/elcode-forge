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
    let cleaned = strip_js_comments_preserving_lines(content);
    let mut findings = Vec::new();
    let imports_child_process = cleaned.contains("child_process");

    for (idx, (cleaned_line, original_line)) in cleaned.lines().zip(content.lines()).enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = cleaned_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let orig_evidence = original_line.trim().to_string();

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
                evidence: orig_evidence.clone(),
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
                evidence: orig_evidence.clone(),
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
                evidence: orig_evidence.clone(),
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
                evidence: orig_evidence.clone(),
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
                evidence: orig_evidence,
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

fn strip_js_comments_preserving_lines(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiline_block_comments_do_not_trigger_unsafe_patterns() {
        let code = r#"
/*
eval("commented untrusted code");
new Function("commented");
dangerouslySetInnerHTML = { __html: "unsafe" };
const { exec } = require('child_process');
exec("rm -rf /");
res.setHeader("Access-Control-Allow-Origin", "*");
*/

// Line comment: eval(ignored);

function safeHandler() {
    return "clean";
}
"#;

        let findings = scan_unsafe_patterns("src/safe.ts", code);
        assert!(
            findings.is_empty(),
            "Expected no findings in commented code, got: {:?}",
            findings
        );
    }

    #[test]
    fn test_detects_real_unsafe_patterns_after_block_comments_with_exact_line_number() {
        let code = r#"
/*
comment line 1
comment line 2
*/
const res = eval(user_input);
"#;

        let findings = scan_unsafe_patterns("src/controller.ts", code);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_NO_EVAL);
        assert_eq!(
            findings[0].location.as_ref().and_then(|l| l.start_line),
            Some(6)
        );
        assert_eq!(findings[0].evidence, "const res = eval(user_input);");
    }
}
