use forge_core::config::SecurityConfig;
use forge_core::{Confidence, Severity};
use forge_security::{
    calculate_shannon_entropy, is_hex, is_high_entropy, is_lockfile, is_path_allowlisted,
    redact_secrets, scan_secrets, scan_secrets_with_config, tokenize_candidate_secrets,
    RULE_HARDCODED_SECRET, RULE_HIGH_ENTROPY_SECRET, RULE_PRIVATE_KEY,
};

#[test]
fn test_shannon_entropy_math() {
    // 0 entropy
    assert_eq!(calculate_shannon_entropy(""), 0.0);
    assert_eq!(calculate_shannon_entropy("AAAAAAAAAAAAAAAA"), 0.0);

    // Low entropy
    let english = "hello world hello world hello world";
    let english_entropy = calculate_shannon_entropy(english);
    assert!(english_entropy < 3.2);

    let code_ident = "user_account_profile_settings_controller";
    let ident_entropy = calculate_shannon_entropy(code_ident);
    assert!(ident_entropy < 3.8);

    // High entropy base64 token
    let secret_token = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let entropy = calculate_shannon_entropy(secret_token);
    assert!(entropy >= 4.5, "Expected entropy >= 4.5, got {entropy}");
    assert!(is_high_entropy(secret_token, 4.5));

    // Pure hex entropy
    let hex_key = "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0";
    assert!(is_hex(hex_key));
    let hex_entropy = calculate_shannon_entropy(hex_key);
    assert!(
        hex_entropy >= 3.5,
        "Expected hex entropy >= 3.5, got {hex_entropy}"
    );
    assert!(is_high_entropy(hex_key, 4.5));
}

#[test]
fn test_verified_credential_patterns() {
    let content = format!(
        "// AWS\nconst aws_id = \"AKIA{}\";\n// GitHub\nconst gh_token = \"ghp_{}\";\n// Slack\nconst slack_bot = \"xoxb-{}\";\n// OpenAI\nconst openai_key = \"sk-proj-{}\";\n// Anthropic\nconst claude_key = \"sk-ant-{}\";\n",
        "IOSFODNN7EXAMPLE",
        "1234567890abcdef1234567890abcdef123456",
        "123456789012-123456789012-abcdefghijklmnopqrstuvwx",
        "1234567890abcdefghijklmnop1234567890abcdef",
        "api03-abcdef1234567890abcdef1234567890abcdef",
    );

    let findings = scan_secrets("src/credentials.ts", &content);
    assert_eq!(findings.len(), 5);

    for f in &findings {
        assert_eq!(f.rule_id, RULE_HARDCODED_SECRET);
        assert_eq!(f.severity, Severity::Critical);
        assert_eq!(f.confidence, Confidence::Certain);
        assert!(f.evidence.contains("****REDACTED****"));
        assert!(!f.evidence.contains("1234567890abcdef"));
    }
}

#[test]
fn test_generic_high_entropy_secret_detection() {
    let content = r#"
        export const ENCRYPTION_SECRET = "bGFyZ2Utc2VjcmV0LWtleS13aXRoLWxvbmctaGlnaC1lbnRyb3B5LXZhbHVlCg==";
    "#;

    let findings = scan_secrets("src/config.ts", content);
    assert_eq!(findings.len(), 1);

    let finding = &findings[0];
    assert_eq!(finding.rule_id, RULE_HIGH_ENTROPY_SECRET);
    assert_eq!(finding.severity, Severity::High);
    assert_eq!(finding.confidence, Confidence::Medium);
    assert!(finding.evidence.contains("****REDACTED****"));
    assert!(!finding.evidence.contains("bGFyZ2Utc2VjcmV0"));
}

#[test]
fn test_path_allowlist_bypass() {
    let content = format!(
        "const token = \"ghp_{}\";\nconst custom = \"bGFyZ2Utc2VjcmV0LWtleS13aXRoLWxvbmctaGlnaC1lbnRyb3B5LXZhbHVlCg==\";\n",
        "1234567890abcdef1234567890abcdef123456"
    );

    let config = SecurityConfig {
        allowlist: vec!["fixtures/**".to_string(), "tests/mocks/".to_string()],
        entropy_threshold: 4.5,
        entropy_min_len: 20,
    };

    // Inside allowlisted path -> 0 findings
    let findings_allowed =
        scan_secrets_with_config("fixtures/app/dummy.ts", &content, Some(&config));
    assert!(findings_allowed.is_empty());

    let findings_mock = scan_secrets_with_config("tests/mocks/secrets.ts", &content, Some(&config));
    assert!(findings_mock.is_empty());

    // Non-allowlisted path -> findings emitted
    let findings_prod = scan_secrets_with_config("src/secrets.ts", &content, Some(&config));
    assert!(!findings_prod.is_empty());
}

#[test]
fn test_lockfiles_automatically_ignored() {
    let lockfile_content = r#"
        "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
        "integrity": "sha512-v2kDEe57lecTulaDIuNTPy3Ry4gLGJ6Z1O3vE1krgXZNrsQ+LFTGHVxVjcXPs17LhbZVGedAJv8XZ1tvj5FvSg=="
    "#;

    // Both package-lock.json and Cargo.lock are ignored
    let findings_npm = scan_secrets("package-lock.json", lockfile_content);
    assert!(findings_npm.is_empty());

    let findings_cargo = scan_secrets("Cargo.lock", lockfile_content);
    assert!(findings_cargo.is_empty());
}

#[test]
fn test_redact_secrets_masks_credentials_and_entropy_tokens() {
    let input = format!(
        "const pat = \"ghp_{}\";\nconst slack = \"xoxb-{}\";\nconst custom = \"bGFyZ2Utc2VjcmV0LWtleS13aXRoLWxvbmctaGlnaC1lbnRyb3B5LXZhbHVlCg==\";\n-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----",
        "1234567890abcdef1234567890abcdef123456",
        "123456789012-123456789012-abcdefghijklmnopqrstuvwx",
    );

    let redacted = redact_secrets(&input);
    assert!(!redacted.contains("ghp_1234567890abcdef"));
    assert!(!redacted.contains("xoxb-123456789012"));
    assert!(!redacted.contains("bGFyZ2Utc2VjcmV0"));
    assert!(redacted.contains("ghp_****REDACTED****"));
    assert!(redacted.contains("xoxb-****REDACTED****"));
    assert!(redacted.contains("****REDACTED****"));
    assert!(redacted.contains("-----BEGIN ****REDACTED**** PRIVATE KEY-----"));
}

#[test]
fn test_allowlist_and_lockfile_helpers() {
    assert!(is_lockfile("nested/path/Cargo.lock"));
    assert!(!is_lockfile("src/main.rs"));

    let allowlist = vec!["vendor/**".to_string(), "fixtures/".to_string()];
    assert!(is_path_allowlisted("vendor/bundle/secret.txt", &allowlist));
    assert!(is_path_allowlisted("fixtures/test.ts", &allowlist));
    assert!(!is_path_allowlisted("src/app.ts", &allowlist));
}

#[test]
fn test_tokenizer_and_private_key_rule() {
    let line = r#"const API_TOKEN = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";"#;
    let tokens = tokenize_candidate_secrets(line, 20);
    assert!(!tokens.is_empty());
    assert_eq!(RULE_PRIVATE_KEY, "security/private-key");
}
