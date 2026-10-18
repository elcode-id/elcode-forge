use forge_core::{Confidence, Finding, Severity, SourceLocation};

/// Rule ID for hardcoded secret / token detection.
pub const RULE_HARDCODED_SECRET: &str = "security/hardcoded-secret";

/// Rule ID for committed private key headers.
pub const RULE_PRIVATE_KEY: &str = "security/private-key";

struct SecretPattern {
    prefix: &'static str,
    min_token_len: usize,
    label: &'static str,
}

const SECRET_PREFIX_PATTERNS: &[SecretPattern] = &[
    SecretPattern {
        prefix: "sk-proj-",
        min_token_len: 20,
        label: "OpenAI Project API Key",
    },
    SecretPattern {
        prefix: "sk-ant-",
        min_token_len: 20,
        label: "Anthropic API Key",
    },
    SecretPattern {
        prefix: "ghp_",
        min_token_len: 20,
        label: "GitHub Personal Access Token",
    },
    SecretPattern {
        prefix: "github_pat_",
        min_token_len: 20,
        label: "GitHub Fine-Grained Personal Access Token",
    },
    SecretPattern {
        prefix: "AKIA",
        min_token_len: 16,
        label: "AWS Access Key ID",
    },
];

/// Scans `content` line-by-line for hardcoded secrets and private keys.
///
/// All detected secret values are strictly redacted (`****REDACTED****`) in the emitted
/// [`Finding::evidence`] field so raw credentials are never logged or persisted in reports.
pub fn scan_secrets(relative_path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let pem_begin = "-----BEGIN ";
        let pem_privkey_end = "PRIVATE KEY-----";
        if trimmed.contains(pem_begin) && trimmed.contains(pem_privkey_end) {
            findings.push(Finding {
                id: format!("SEC-PRIVKEY-{relative_path}-{line_no}"),
                category: "security".to_string(),
                severity: Severity::Critical,
                confidence: Confidence::Certain,
                title: "Committed private key material detected".to_string(),
                description: format!(
                    "File '{relative_path}' contains a PEM private key header on line {line_no}."
                ),
                evidence: format!("{pem_begin}****REDACTED**** {pem_privkey_end}"),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Revoke the exposed key immediately and load private keys from a secret manager or environment variable."
                        .to_string(),
                rule_id: RULE_PRIVATE_KEY.to_string(),
            });
            continue;
        }

        for pat in SECRET_PREFIX_PATTERNS {
            if let Some(pos) = trimmed.find(pat.prefix) {
                let candidate = &trimmed[pos..];
                let token: String = candidate
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();

                if token.len() >= pat.min_token_len {
                    let redacted = format!("{}****REDACTED****", pat.prefix);
                    findings.push(Finding {
                        id: format!("SEC-SECRET-{relative_path}-{line_no}-{}", pat.prefix),
                        category: "security".to_string(),
                        severity: Severity::Critical,
                        confidence: Confidence::Certain,
                        title: format!("Hardcoded {} detected", pat.label),
                        description: format!(
                            "Potential {} found in '{}' at line {}.",
                            pat.label, relative_path, line_no
                        ),
                        evidence: redacted,
                        location: Some(SourceLocation {
                            file_path: relative_path.to_string(),
                            start_line: Some(line_no),
                            end_line: Some(line_no),
                        }),
                        recommendation:
                            "Rotate the exposed credential and move secrets to environment variables."
                                .to_string(),
                        rule_id: RULE_HARDCODED_SECRET.to_string(),
                    });
                }
            }
        }
    }

    findings
}
