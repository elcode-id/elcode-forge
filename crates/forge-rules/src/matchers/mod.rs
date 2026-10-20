//! Matchers for evaluating declarative architectural and quality rules.

pub mod dependency_restriction;
pub mod file_naming;
pub mod file_size;
pub mod forbidden_imports;
pub mod regex_pattern;
pub mod required_companion;

use forge_core::{Finding, ForgeError, Repository};

use crate::declarative::{DeclarativeRule, MatcherKind};

/// Evaluates a single [`DeclarativeRule`] against `repo` and returns any findings.
pub fn evaluate_declarative_rule(
    rule: &DeclarativeRule,
    repo: &Repository,
) -> Result<Vec<Finding>, ForgeError> {
    if !rule.enabled {
        return Ok(Vec::new());
    }

    let findings = match rule.matcher.kind {
        MatcherKind::ForbiddenImports => forbidden_imports::evaluate(rule, repo),
        MatcherKind::RequiredCompanion => required_companion::evaluate(rule, repo),
        MatcherKind::FileNaming => file_naming::evaluate(rule, repo),
        MatcherKind::FileSize => file_size::evaluate(rule, repo),
        MatcherKind::RegexPattern => regex_pattern::evaluate(rule, repo),
        MatcherKind::DependencyRestriction => dependency_restriction::evaluate(rule, repo),
        MatcherKind::Unknown => Vec::new(),
    };

    Ok(findings)
}

/// Matches a glob pattern against a relative path with normalized `/` separators.
///
/// Supports:
/// - `**` (recursive directory match)
/// - `*` (single segment wildcard)
/// - Exact substring / prefix / suffix matches
#[must_use]
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pat = pattern.replace('\\', "/");
    let target = path.replace('\\', "/");

    if pat == "*" || pat == "**" || pat == "**/*" {
        return true;
    }

    if let Some(prefix) = pat.strip_suffix("/**") {
        return target == prefix || target.starts_with(&format!("{prefix}/"));
    }

    if let Some(suffix) = pat.strip_prefix("**/") {
        if let Some(ext) = suffix.strip_prefix('*') {
            return target.ends_with(ext);
        }
        return target.ends_with(suffix) || target.contains(&format!("/{suffix}"));
    }

    if pat.contains('*') {
        let parts: Vec<&str> = pat.split('*').collect();
        if parts.len() == 2 {
            let start = parts[0];
            let end = parts[1];
            return target.starts_with(start)
                && target.ends_with(end)
                && target.len() >= start.len() + end.len();
        }
    }

    pat == target
}

/// Checks if `path` matches any pattern in `patterns`.
/// Returns `true` if `patterns` is empty (default allow).
#[must_use]
pub fn matches_any_pattern(path: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }
    patterns.iter().any(|pat| glob_match(pat, path))
}

/// Helper converting arbitrary string into an alphanumeric identifier slug.
#[must_use]
pub fn slug_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_patterns() {
        assert!(glob_match("src/**", "src/ui/Card.tsx"));
        assert!(glob_match("src/**", "src/db/client.ts"));
        assert!(!glob_match("src/ui/**", "src/db/client.ts"));
        assert!(glob_match("src/ui/**", "src/ui/components/Button.tsx"));
        assert!(glob_match("**/*.test.ts", "src/services/user.test.ts"));
        assert!(!glob_match("**/*.test.ts", "src/services/user.ts"));
        assert!(glob_match(
            "*Controller.ts",
            "src/controllers/UserController.ts"
        ));
        assert!(!glob_match("*Controller.ts", "src/controllers/user.ts"));
    }

    #[test]
    fn test_slug_id() {
        assert_eq!(slug_id("arch/no-db-in-ui"), "ARCH_NO_DB_IN_UI");
        assert_eq!(slug_id("src/ui/Card.tsx"), "SRC_UI_CARD_TSX");
    }
}
