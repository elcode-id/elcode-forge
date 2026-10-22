//! Shannon entropy calculation engine and token extraction for credential and secret detection.
//!
//! Shannon entropy measures the information density / randomness of a candidate string:
//! \[
//!   H(X) = -\sum_{x \in X} P(x) \log_2 P(x)
//! \]
//!
//! High entropy (typically \(\ge 4.5\) for base64 / alphanumeric tokens of length \(\ge 20\))
//! strongly correlates with cryptographically generated API keys, private tokens, and passwords.

use std::collections::BTreeSet;

/// Default minimum Shannon entropy threshold for candidate secret strings.
pub const DEFAULT_ENTROPY_THRESHOLD: f64 = 4.5;

/// Default minimum character length for candidate secret strings.
pub const DEFAULT_MIN_TOKEN_LEN: usize = 20;

/// Calculates the Shannon entropy \(H(X)\) of `token` in bits per symbol.
///
/// Returns `0.0` for empty strings or strings with only a single repeated character.
#[must_use]
pub fn calculate_shannon_entropy(token: &str) -> f64 {
    if token.is_empty() {
        return 0.0;
    }

    let mut freq = [0usize; 256];
    let mut total = 0usize;

    for byte in token.bytes() {
        freq[byte as usize] += 1;
        total += 1;
    }

    if total == 0 {
        return 0.0;
    }

    let total_f = total as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / total_f;
            entropy -= p * p.log2();
        }
    }

    // Guard against potential IEEE-754 negative zero floating point artifact
    if entropy.abs() < 1e-10 {
        0.0
    } else {
        entropy
    }
}

/// Checks whether `token` is composed strictly of hexadecimal characters (0-9, a-f, A-F).
#[must_use]
pub fn is_hex(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|c| c.is_ascii_hexdigit())
}

/// Evaluates whether `token` has sufficiently high entropy to be considered a candidate secret.
///
/// Pure hexadecimal strings have a theoretical maximum entropy of \(\log_2(16) = 4.0\).
/// For hex tokens, the threshold is scaled proportionally (\(\approx \text{threshold} \times 4/6\))
/// so that high-entropy 128-bit / 256-bit hex keys (e.g. 32-char / 64-char API secrets) are also
/// detected with high fidelity.
#[must_use]
pub fn is_high_entropy(token: &str, threshold: f64) -> bool {
    let entropy = calculate_shannon_entropy(token);
    if is_hex(token) {
        // Hex charset size is 16 vs 64 for Base64 (ratio 4/6 = 0.667).
        let hex_threshold = (threshold * (4.0 / 6.0)).min(3.8);
        entropy >= hex_threshold
    } else {
        entropy >= threshold
    }
}

/// Extracts candidate secret tokens from a raw line of text.
///
/// Extracts string literals enclosed in single/double/backtick quotes, as well as standalone
/// alphanumeric and base64/hex tokens meeting `min_len`.
#[must_use]
pub fn tokenize_candidate_secrets(line: &str, min_len: usize) -> Vec<String> {
    let mut candidates = BTreeSet::new();
    let bytes = line.as_bytes();
    let mut remainder = bytes.to_vec();

    // 1. Quoted literals (e.g., "secret", 'secret', `secret`)
    let mut i = 0;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote == b'"' || quote == b'\'' || quote == b'`' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != quote {
                if bytes[end] == b'\\' && end + 1 < bytes.len() {
                    end += 2;
                } else {
                    end += 1;
                }
            }
            if end < bytes.len() {
                let literal = &line[start..end];
                if literal.len() >= min_len && is_candidate_charset(literal) {
                    candidates.insert(literal.to_string());
                }
                // Mask out quoted region in remainder so unquoted scanner does not double-count
                remainder[i..=end].fill(b' ');
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    // 2. Standalone unquoted tokens
    if let Ok(unquoted_str) = std::str::from_utf8(&remainder) {
        for part in unquoted_str.split(|c: char| c.is_whitespace() || c == ';' || c == ',') {
            let mut val = part.trim();
            if let Some((_, after_eq)) = val.split_once('=') {
                val = after_eq.trim();
            }
            let cleaned = val.trim_matches(|c: char| {
                c == '"'
                    || c == '\''
                    || c == '`'
                    || c == '('
                    || c == ')'
                    || c == '{'
                    || c == '}'
                    || c == '['
                    || c == ']'
            });
            if cleaned.len() >= min_len && is_candidate_charset(cleaned) {
                candidates.insert(cleaned.to_string());
            }
        }
    }

    candidates.into_iter().collect()
}

/// Checks if a string contains characters typical of API keys, tokens, or hashes,
/// and does not look like a plain URL or path.
fn is_candidate_charset(s: &str) -> bool {
    // Exclude URLs, file paths, and namespaces
    if s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("file://")
        || s.contains("://")
        || s.starts_with('/')
        || s.starts_with("./")
        || s.starts_with("../")
        || s.contains('\\')
    {
        return false;
    }

    // Allowed secret token characters: alphanumeric, _, -, +, /, = (base64 & hex symbols)
    let valid_chars = s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '+' || c == '/' || c == '='
    });

    if !valid_chars {
        return false;
    }

    // Exclude tokens that are purely lowercase letters and underscores (e.g. snake_case identifier)
    let only_lower_snake = s.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    if only_lower_snake {
        return false;
    }

    true
}

/// Scans `line` for candidate tokens exceeding `threshold` Shannon entropy and `min_len`.
///
/// Returns a list of `(candidate_token, entropy)` pairs.
#[must_use]
pub fn find_high_entropy_tokens(line: &str, threshold: f64, min_len: usize) -> Vec<(String, f64)> {
    let tokens = tokenize_candidate_secrets(line, min_len);
    let mut high_entropy = Vec::new();

    for token in tokens {
        let entropy = calculate_shannon_entropy(&token);
        if is_high_entropy(&token, threshold) {
            high_entropy.push((token, entropy));
        }
    }

    high_entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_zero_for_empty_and_repeated_chars() {
        assert_eq!(calculate_shannon_entropy(""), 0.0);
        assert_eq!(calculate_shannon_entropy("aaaaaaaaaaaa"), 0.0);
        assert_eq!(calculate_shannon_entropy("11111111111111111111"), 0.0);
    }

    #[test]
    fn test_entropy_high_for_random_strings() {
        // High entropy random token
        let token = "dGhpcy1pcy1hLXZlcnktc2VjcmV0LXRva2VuLXZhbHVlCg==";
        let entropy = calculate_shannon_entropy(token);
        assert!(entropy >= 4.0, "Expected entropy >= 4.0, got {entropy}");
        assert!(is_high_entropy(token, 4.0));
    }

    #[test]
    fn test_hex_entropy_scaling() {
        // 64-char random hex key
        let hex_key = "4f8b2d1c9e7a3f0b8c6e4d2a1f9e8b7c6d5a4f3e2b1a0f9e8d7c6b5a4f3e2d1c";
        assert!(is_hex(hex_key));
        let entropy = calculate_shannon_entropy(hex_key);
        assert!(entropy >= 3.5, "Expected hex entropy >= 3.5, got {entropy}");
        assert!(is_high_entropy(hex_key, 4.5));
    }

    #[test]
    fn test_tokenize_candidate_secrets() {
        let line = r#"const API_KEY = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";"#;
        let candidates = tokenize_candidate_secrets(line, 20);
        assert!(candidates.contains(&"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()));
    }
}
