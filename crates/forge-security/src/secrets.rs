use forge_core::config::SecurityConfig;
use forge_core::{Confidence, Finding, Severity, SourceLocation};

use crate::entropy::{find_high_entropy_tokens, DEFAULT_ENTROPY_THRESHOLD, DEFAULT_MIN_TOKEN_LEN};

/// Rule ID for hardcoded secret / token detection.
pub const RULE_HARDCODED_SECRET: &str = "security/hardcoded-secret";

/// Rule ID for committed private key headers.
pub const RULE_PRIVATE_KEY: &str = "security/private-key";

/// Rule ID for generic high-entropy secret detection.
pub const RULE_HIGH_ENTROPY_SECRET: &str = "security/high-entropy-secret";

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
        prefix: "sk-",
        min_token_len: 24,
        label: "OpenAI API Key",
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
        prefix: "xoxb-",
        min_token_len: 20,
        label: "Slack Bot Token",
    },
    SecretPattern {
        prefix: "xoxp-",
        min_token_len: 20,
        label: "Slack User Token",
    },
    SecretPattern {
        prefix: "AKIA",
        min_token_len: 20,
        label: "AWS Access Key ID",
    },
];

/// Checks if a file path is a standard package manager lockfile.
#[must_use]
pub fn is_lockfile(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    let filename = norm.split('/').next_back().unwrap_or(&norm);
    matches!(
        filename,
        "Cargo.lock"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "composer.lock"
            | "poetry.lock"
            | "Pipfile.lock"
            | "Gemfile.lock"
    )
}

/// Checks whether `path` matches any pattern in `allowlist`.
///
/// Supports exact path matching, directory prefix (`fixtures/` or `fixtures/**`),
/// suffix matching (`*.mock.ts`), or substring matching.
#[must_use]
pub fn is_path_allowlisted(path: &str, allowlist: &[String]) -> bool {
    if allowlist.is_empty() {
        return false;
    }

    let norm_path = path.replace('\\', "/");
    let norm_path = norm_path.strip_prefix("./").unwrap_or(&norm_path);

    for pattern in allowlist {
        let norm_pat = pattern.replace('\\', "/");
        let norm_pat = norm_pat.strip_prefix("./").unwrap_or(&norm_pat);

        // Exact match
        if norm_path == norm_pat {
            return true;
        }

        // Wildcard directory prefix: "fixtures/**" or "fixtures/*"
        if let Some(prefix) = norm_pat.strip_suffix("/**") {
            if norm_path.starts_with(prefix) {
                return true;
            }
        }
        if let Some(prefix) = norm_pat.strip_suffix("/*") {
            if norm_path.starts_with(prefix) {
                return true;
            }
        }

        // Directory prefix ending with '/': "fixtures/"
        if norm_pat.ends_with('/') && norm_path.starts_with(norm_pat) {
            return true;
        }

        // Substring / component directory match: e.g. "fixtures" matches "fixtures/app/secrets.ts"
        if norm_path.starts_with(&format!("{norm_pat}/"))
            || norm_path.contains(&format!("/{norm_pat}/"))
            || norm_path.ends_with(&format!("/{norm_pat}"))
        {
            return true;
        }

        // File extension / suffix match: e.g. "*.mock.ts" or "*.env"
        if let Some(suffix) = norm_pat.strip_prefix('*') {
            if norm_path.ends_with(suffix) {
                return true;
            }
        }
    }

    false
}

/// Scans `content` line-by-line for hardcoded secrets, verified credentials, private keys,
/// and high-entropy secret tokens using default configuration.
#[must_use]
pub fn scan_secrets(relative_path: &str, content: &str) -> Vec<Finding> {
    scan_secrets_with_config(relative_path, content, None)
}

/// Scans `content` line-by-line for hardcoded secrets, verified credentials, private keys,
/// and high-entropy secret tokens with configurable thresholds and allowlists.
///
/// All detected secret values are strictly redacted (`****REDACTED****`) in the emitted
/// [`Finding::evidence`] field so raw credentials are never logged or persisted in reports.
#[must_use]
pub fn scan_secrets_with_config(
    relative_path: &str,
    content: &str,
    config: Option<&SecurityConfig>,
) -> Vec<Finding> {
    // If the path is a lockfile or in the security allowlist, skip secret scanning
    if is_lockfile(relative_path) {
        return Vec::new();
    }

    if let Some(cfg) = config {
        if is_path_allowlisted(relative_path, &cfg.allowlist) {
            return Vec::new();
        }
    }

    let entropy_threshold = config
        .map(|c| c.entropy_threshold)
        .unwrap_or(DEFAULT_ENTROPY_THRESHOLD);
    let entropy_min_len = config
        .map(|c| c.entropy_min_len)
        .unwrap_or(DEFAULT_MIN_TOKEN_LEN);

    let mut findings = Vec::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.contains("****REDACTED****") {
            continue;
        }

        // 1. Check for committed PEM private key headers
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

        let mut matched_tokens = Vec::new();

        // 2. Check for verified credential patterns
        for pat in SECRET_PREFIX_PATTERNS {
            let mut search_idx = 0;
            while let Some(rel_pos) = trimmed[search_idx..].find(pat.prefix) {
                let pos = search_idx + rel_pos;
                let candidate = &trimmed[pos..];

                // If checking generic "sk-", ensure it doesn't duplicate "sk-proj-" or "sk-ant-"
                if pat.prefix == "sk-"
                    && (candidate.starts_with("sk-proj-") || candidate.starts_with("sk-ant-"))
                {
                    search_idx = pos + pat.prefix.len();
                    continue;
                }

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
                    matched_tokens.push(token);
                }
                search_idx = pos + pat.prefix.len();
            }
        }

        // 3. Shannon entropy scanner for unverified high-entropy candidate secrets
        let candidate_high_entropy =
            find_high_entropy_tokens(trimmed, entropy_threshold, entropy_min_len);
        for (token, entropy) in candidate_high_entropy {
            // Do not emit duplicate finding if token was already identified by verified pattern
            if matched_tokens
                .iter()
                .any(|m| m.contains(&token) || token.contains(m.as_str()))
            {
                continue;
            }

            let token_prefix = if token.len() >= 4 {
                &token[..4]
            } else {
                &token[..]
            };
            findings.push(Finding {
                id: format!("SEC-ENTROPY-{relative_path}-{line_no}-{token_prefix}"),
                category: "security".to_string(),
                severity: Severity::High,
                confidence: Confidence::Medium,
                title: "High-entropy secret token detected".to_string(),
                description: format!(
                    "Candidate high-entropy token (Shannon entropy: {entropy:.2} >= {entropy_threshold:.1}) found in '{relative_path}' at line {line_no}."
                ),
                evidence: format!("****REDACTED**** (entropy: {entropy:.2})"),
                location: Some(SourceLocation {
                    file_path: relative_path.to_string(),
                    start_line: Some(line_no),
                    end_line: Some(line_no),
                }),
                recommendation:
                    "Inspect candidate token. If it is a credential or API token, rotate it and move it to an environment variable. Otherwise, add the path or pattern to 'security.allowlist' in eforge.config.yaml."
                        .to_string(),
                rule_id: RULE_HIGH_ENTROPY_SECRET.to_string(),
            });
        }
    }

    findings
}

/// Redacts detected secret patterns and private keys in the given text, replacing them with
/// `****REDACTED****` markers to ensure safe inclusion in AI prompts and logs.
#[must_use]
pub fn redact_secrets(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let pem_begin = "-----BEGIN ";
    let pem_privkey_end = "PRIVATE KEY-----";

    for (i, line) in content.lines().enumerate() {
        if i > 0 {
            output.push('\n');
        }

        if line.contains(pem_begin) && line.contains(pem_privkey_end) {
            output.push_str("-----BEGIN ****REDACTED**** PRIVATE KEY-----");
            continue;
        }

        let mut line_buf = line.to_string();

        // 1. Redact known prefix patterns
        for pat in SECRET_PREFIX_PATTERNS {
            let mut search_idx = 0;
            while let Some(rel_pos) = line_buf[search_idx..].find(pat.prefix) {
                let pos = search_idx + rel_pos;
                let candidate = &line_buf[pos..];

                if pat.prefix == "sk-"
                    && (candidate.starts_with("sk-proj-") || candidate.starts_with("sk-ant-"))
                {
                    search_idx = pos + pat.prefix.len();
                    continue;
                }

                let token: String = candidate
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();

                if token.len() >= pat.min_token_len {
                    let redacted = format!("{}****REDACTED****", pat.prefix);
                    line_buf.replace_range(pos..pos + token.len(), &redacted);
                    search_idx = pos + redacted.len();
                } else {
                    search_idx = pos + pat.prefix.len();
                }
            }
        }

        // 2. Redact remaining candidate high-entropy tokens
        let candidate_entropy =
            find_high_entropy_tokens(&line_buf, DEFAULT_ENTROPY_THRESHOLD, DEFAULT_MIN_TOKEN_LEN);
        for (token, _) in candidate_entropy {
            if token.contains("****REDACTED****") {
                continue;
            }
            if let Some(pos) = line_buf.find(&token) {
                line_buf.replace_range(pos..pos + token.len(), "****REDACTED****");
            }
        }

        output.push_str(&line_buf);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_secrets_masks_known_tokens_and_pem_keys() {
        let input = concat!(
            "const key = \"sk-proj-",
            "1234567890abcdefghijklmnop\";\n",
            "-----BEGIN ",
            "RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----"
        );
        let redacted = redact_secrets(input);
        assert!(!redacted.contains(concat!("sk-proj-", "1234567890abcdefghijklmnop")));
        assert!(redacted.contains("sk-proj-****REDACTED****"));
        assert!(redacted.contains("-----BEGIN ****REDACTED**** PRIVATE KEY-----"));
    }

    #[test]
    fn test_lockfile_detection() {
        assert!(is_lockfile("Cargo.lock"));
        assert!(is_lockfile("frontend/package-lock.json"));
        assert!(is_lockfile(".eforge/pnpm-lock.yaml"));
        assert!(!is_lockfile("src/config.rs"));
    }

    #[test]
    fn test_path_allowlist() {
        let allowlist = vec![
            "fixtures/**".to_string(),
            "tests/mocks/".to_string(),
            "*.mock.ts".to_string(),
        ];

        assert!(is_path_allowlisted(
            "fixtures/vulnerable-ts-app/src/app.ts",
            &allowlist
        ));
        assert!(is_path_allowlisted(
            "tests/mocks/credentials.json",
            &allowlist
        ));
        assert!(is_path_allowlisted("src/auth.mock.ts", &allowlist));
        assert!(!is_path_allowlisted("src/auth.ts", &allowlist));
    }
}
