//! Declarative custom rule models, schema validation, and rule loader.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use forge_core::path_guard::resolve_safe_path;
use forge_core::{Confidence, ForgeError, Severity};
use serde::{Deserialize, Serialize};

/// High-level category of an architectural or quality rule.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum RuleCategory {
    #[default]
    #[serde(rename = "architecture", alias = "arch")]
    Architecture,
    #[serde(rename = "quality")]
    Quality,
    #[serde(rename = "security", alias = "sec")]
    Security,
    #[serde(rename = "dependencies", alias = "deps")]
    Dependencies,
}

impl RuleCategory {
    /// Returns the canonical kebab-case string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Architecture => "architecture",
            Self::Quality => "quality",
            Self::Security => "security",
            Self::Dependencies => "dependencies",
        }
    }
}

impl fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RuleCategory {
    type Err = ForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "arch" | "architecture" => Ok(Self::Architecture),
            "quality" => Ok(Self::Quality),
            "security" | "sec" => Ok(Self::Security),
            "deps" | "dependencies" => Ok(Self::Dependencies),
            other => Err(ForgeError::Config {
                message: format!(
                    "unknown rule category '{other}': expected architecture, quality, security, or dependencies"
                ),
            }),
        }
    }
}

/// Provenance of a rule definition (built-in, packaged, or local custom).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "name")]
pub enum RuleSource {
    Builtin(String),
    Pack(String),
    Custom(String),
}

impl Default for RuleSource {
    fn default() -> Self {
        Self::Custom("custom".to_string())
    }
}

impl fmt::Display for RuleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(name) => write!(f, "builtin:{name}"),
            Self::Pack(name) => write!(f, "pack:{name}"),
            Self::Custom(path) => write!(f, "custom:{path}"),
        }
    }
}

/// Supported matcher algorithms for declarative rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatcherKind {
    #[default]
    ForbiddenImports,
    RequiredCompanion,
    FileNaming,
    FileSize,
    RegexPattern,
    DependencyRestriction,
    #[serde(other)]
    Unknown,
}

/// Matcher parameters defining when a declarative rule fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuleMatcher {
    #[serde(rename = "type")]
    pub kind: MatcherKind,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub companion: Option<String>,
    #[serde(default)]
    pub convention: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub max_lines: Option<u32>,
    #[serde(default)]
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub disallowed_packages: Vec<String>,
}

fn default_confidence() -> Confidence {
    Confidence::High
}

fn default_true() -> bool {
    true
}

/// Fully-qualified declarative rule specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarativeRule {
    pub id: String,
    pub name: String,
    pub category: RuleCategory,
    pub severity: Severity,
    #[serde(default = "default_confidence")]
    pub confidence: Confidence,
    pub description: String,
    #[serde(alias = "match")]
    pub matcher: RuleMatcher,
    #[serde(default)]
    pub remediation: String,
    #[serde(default)]
    pub source: RuleSource,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_version() -> u32 {
    1
}

/// Schema for a declarative rule file (`.eforge/rules/*.yaml` or `.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub pack: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub rules: Vec<DeclarativeRule>,
}

/// Validates canonical rule ID format: `<category>/<slug>` (e.g. `arch/no-db-in-handlers`).
pub fn validate_rule_id(id: &str) -> Result<(), ForgeError> {
    let Some((cat, slug)) = id.split_once('/') else {
        return Err(ForgeError::Config {
            message: format!(
                "invalid rule id '{id}': must follow '<category>/<slug>' format (e.g. 'arch/no-db-in-ui')"
            ),
        });
    };

    match cat.to_ascii_lowercase().as_str() {
        "arch" | "architecture" | "quality" | "security" | "sec" | "deps" | "dependencies" => {}
        _ => {
            return Err(ForgeError::Config {
                message: format!(
                    "invalid rule category '{cat}' in '{id}': must be one of arch, quality, security, or deps"
                ),
            });
        }
    }

    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::Config {
            message: format!(
                "invalid rule slug '{slug}' in '{id}': must contain only alphanumeric characters, hyphens, or underscores"
            ),
        });
    }

    Ok(())
}

/// Validates that a [`DeclarativeRule`] satisfies all integrity invariants.
pub fn validate_rule(rule: &DeclarativeRule) -> Result<(), ForgeError> {
    validate_rule_id(&rule.id)?;

    if rule.name.trim().is_empty() {
        return Err(ForgeError::Config {
            message: format!("rule '{}' must have a non-empty 'name'", rule.id),
        });
    }

    if rule.description.trim().is_empty() {
        return Err(ForgeError::Config {
            message: format!("rule '{}' must have a non-empty 'description'", rule.id),
        });
    }

    match rule.matcher.kind {
        MatcherKind::ForbiddenImports => {
            if rule.matcher.forbidden.is_empty() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'forbidden_imports' must declare at least one 'forbidden' pattern",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::RequiredCompanion => {
            if rule.matcher.companion.is_none() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'required_companion' must declare 'companion' pattern",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::FileNaming => {
            if rule.matcher.convention.is_none() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'file_naming' must declare 'convention' pattern",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::FileSize => {
            if rule.matcher.max_lines.is_none() && rule.matcher.max_bytes.is_none() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'file_size' must declare 'max_lines' or 'max_bytes'",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::RegexPattern => {
            if rule.matcher.pattern.is_none() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'regex_pattern' must declare 'pattern'",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::DependencyRestriction => {
            if rule.matcher.disallowed_packages.is_empty() {
                return Err(ForgeError::Config {
                    message: format!(
                        "rule '{}' with matcher 'dependency_restriction' must declare at least one 'disallowed_packages' entry",
                        rule.id
                    ),
                });
            }
        }
        MatcherKind::Unknown => {
            return Err(ForgeError::Config {
                message: format!(
                    "rule '{}' specifies an unknown or unsupported matcher type",
                    rule.id
                ),
            });
        }
    }

    Ok(())
}

/// Parses a JSON or YAML rule specification document into a list of [`DeclarativeRule`]s.
pub fn parse_rule_file(
    content: &str,
    source: RuleSource,
) -> Result<Vec<DeclarativeRule>, ForgeError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // Try direct JSON deserialization if payload looks like JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(file) = serde_json::from_str::<RuleFile>(trimmed) {
            let mut rules = file.rules;
            for r in &mut rules {
                r.source = source.clone();
                validate_rule(r)?;
            }
            return Ok(rules);
        }

        if let Ok(mut rules) = serde_json::from_str::<Vec<DeclarativeRule>>(trimmed) {
            for r in &mut rules {
                r.source = source.clone();
                validate_rule(r)?;
            }
            return Ok(rules);
        }

        if let Ok(mut single) = serde_json::from_str::<DeclarativeRule>(trimmed) {
            single.source = source;
            validate_rule(&single)?;
            return Ok(vec![single]);
        }
    }

    // Convert YAML into serde_json::Value
    let json_val = yaml_to_json_value(trimmed)?;

    let mut rules = if let Ok(file) = serde_json::from_value::<RuleFile>(json_val.clone()) {
        file.rules
    } else if let Ok(rules_vec) = serde_json::from_value::<Vec<DeclarativeRule>>(json_val.clone()) {
        rules_vec
    } else if let Ok(single) = serde_json::from_value::<DeclarativeRule>(json_val.clone()) {
        vec![single]
    } else {
        return Err(ForgeError::Config {
            message: format!(
                "failed to deserialize rule specification into RuleFile or DeclarativeRule: parsed JSON = {}",
                serde_json::to_string(&json_val).unwrap_or_default()
            ),
        });
    };

    for r in &mut rules {
        r.source = source.clone();
        validate_rule(r)?;
    }

    Ok(rules)
}

/// Converts simple-to-medium YAML string content into a [`serde_json::Value`] without external yaml dependencies.
fn yaml_to_json_value(yaml_str: &str) -> Result<serde_json::Value, ForgeError> {
    let mut lines = Vec::new();
    for (idx, raw_line) in yaml_str.lines().enumerate() {
        let without_comment = match raw_line.find('#') {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };
        let trimmed = without_comment.trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        let indent = without_comment
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();
        lines.push((idx + 1, indent, trimmed.to_string()));
    }

    if lines.is_empty() {
        return Ok(serde_json::Value::Null);
    }

    let mut index = 0;
    parse_yaml_block(&lines, &mut index, 0)
}

fn parse_yaml_block(
    lines: &[(usize, usize, String)],
    idx: &mut usize,
    current_indent: usize,
) -> Result<serde_json::Value, ForgeError> {
    if *idx >= lines.len() {
        return Ok(serde_json::Value::Null);
    }

    let is_sequence = lines[*idx].2.starts_with('-');
    if is_sequence {
        let mut arr = Vec::new();
        while *idx < lines.len() {
            let (_, indent, text) = &lines[*idx];
            if *indent < current_indent {
                break;
            }
            if !text.starts_with('-') {
                break;
            }

            let item_text = text[1..].trim();
            if item_text.is_empty() {
                // Item on next lines
                *idx += 1;
                if *idx < lines.len() && lines[*idx].1 > *indent {
                    let next_indent = lines[*idx].1;
                    let val = parse_yaml_block(lines, idx, next_indent)?;
                    arr.push(val);
                } else {
                    arr.push(serde_json::Value::Null);
                }
            } else if let Some((k, v)) = split_key_val(item_text) {
                // Inline map start: - key: value
                let mut map = serde_json::Map::new();
                let sub_indent = *indent + 2;
                if v.trim().is_empty() {
                    *idx += 1;
                    let val = if *idx < lines.len() && lines[*idx].1 >= sub_indent {
                        parse_yaml_block(lines, idx, lines[*idx].1)?
                    } else {
                        serde_json::Value::Null
                    };
                    map.insert(k.to_string(), val);
                } else {
                    map.insert(k.to_string(), parse_scalar(v));
                    *idx += 1;
                }

                // Collect remaining keys at sub_indent
                while *idx < lines.len() {
                    let (_, key_indent, key_text) = &lines[*idx];
                    if *key_indent < sub_indent || key_text.starts_with('-') {
                        break;
                    }
                    if let Some((sub_k, sub_v)) = split_key_val(key_text) {
                        if sub_v.trim().is_empty() {
                            *idx += 1;
                            let val = if *idx < lines.len() && lines[*idx].1 > *key_indent {
                                parse_yaml_block(lines, idx, lines[*idx].1)?
                            } else {
                                serde_json::Value::Null
                            };
                            map.insert(sub_k.to_string(), val);
                        } else {
                            map.insert(sub_k.to_string(), parse_scalar(sub_v));
                            *idx += 1;
                        }
                    } else {
                        break;
                    }
                }
                arr.push(serde_json::Value::Object(map));
            } else {
                arr.push(parse_scalar(item_text));
                *idx += 1;
            }
        }
        Ok(serde_json::Value::Array(arr))
    } else {
        let mut map = serde_json::Map::new();
        while *idx < lines.len() {
            let (_, indent, text) = &lines[*idx];
            if *indent < current_indent {
                break;
            }
            if text.starts_with('-') {
                break;
            }

            if let Some((k, v)) = split_key_val(text) {
                if v.trim().is_empty() {
                    *idx += 1;
                    let val = if *idx < lines.len() && lines[*idx].1 > *indent {
                        parse_yaml_block(lines, idx, lines[*idx].1)?
                    } else {
                        serde_json::Value::Null
                    };
                    map.insert(k.to_string(), val);
                } else {
                    map.insert(k.to_string(), parse_scalar(v));
                    *idx += 1;
                }
            } else {
                *idx += 1;
            }
        }
        Ok(serde_json::Value::Object(map))
    }
}

fn split_key_val(text: &str) -> Option<(&str, &str)> {
    let (k, v) = text.split_once(':')?;
    let trimmed_k = k.trim();
    if trimmed_k.is_empty() {
        return None;
    }
    Some((trimmed_k, v.trim()))
}

fn parse_scalar(text: &str) -> serde_json::Value {
    let trimmed = text.trim();
    if (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return v;
        }
    }

    let unquoted = unquote(trimmed);
    if unquoted == "true" {
        return serde_json::Value::Bool(true);
    }
    if unquoted == "false" {
        return serde_json::Value::Bool(false);
    }
    if unquoted == "null" || unquoted == "~" {
        return serde_json::Value::Null;
    }
    if let Ok(i) = unquoted.parse::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(u) = unquoted.parse::<u64>() {
        return serde_json::Value::Number(u.into());
    }
    if let Ok(f) = unquoted.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(num);
        }
    }
    serde_json::Value::String(unquoted.to_string())
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Discovers, validates, and loads all declarative custom rules across `repo_root`.
///
/// Automatically inspects:
/// 1. `.eforge/rules/*.{yaml,yml,json}`
/// 2. Explicit paths declared in `custom_paths` (e.g. from `eforge.config.yaml`).
///
/// All paths are secured against path-traversal escapes via [`resolve_safe_path`].
pub fn load_declarative_rules(
    repo_root: &Path,
    custom_paths: &[String],
) -> Result<Vec<DeclarativeRule>, ForgeError> {
    let mut collected: Vec<DeclarativeRule> = Vec::new();
    let default_rules_dir = repo_root.join(".eforge").join("rules");

    if default_rules_dir.is_dir() {
        let entries = fs::read_dir(&default_rules_dir).map_err(|source| ForgeError::Io {
            path: default_rules_dir.clone(),
            source,
        })?;

        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("yaml")
                        || ext.eq_ignore_ascii_case("yml")
                        || ext.eq_ignore_ascii_case("json")
                    {
                        paths.push(path);
                    }
                }
            }
        }
        paths.sort();

        for p in paths {
            let rel = p.strip_prefix(repo_root).unwrap_or(&p);
            let safe_p = resolve_safe_path(repo_root, rel)?;
            let content = fs::read_to_string(&safe_p).map_err(|source| ForgeError::Io {
                path: safe_p.clone(),
                source,
            })?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let rules = parse_rule_file(&content, RuleSource::Custom(rel_str))?;
            collected.extend(rules);
        }
    }

    for custom in custom_paths {
        let cand = Path::new(custom);
        let safe_p = resolve_safe_path(repo_root, cand)?;
        if safe_p.is_file() {
            let content = fs::read_to_string(&safe_p).map_err(|source| ForgeError::Io {
                path: safe_p.clone(),
                source,
            })?;
            let rules = parse_rule_file(&content, RuleSource::Custom(custom.clone()))?;
            collected.extend(rules);
        } else if safe_p.is_dir() {
            let entries = fs::read_dir(&safe_p).map_err(|source| ForgeError::Io {
                path: safe_p.clone(),
                source,
            })?;
            let mut sub_paths: Vec<PathBuf> = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("yaml")
                            || ext.eq_ignore_ascii_case("yml")
                            || ext.eq_ignore_ascii_case("json")
                        {
                            sub_paths.push(p);
                        }
                    }
                }
            }
            sub_paths.sort();
            for p in sub_paths {
                let content = fs::read_to_string(&p).map_err(|source| ForgeError::Io {
                    path: p.clone(),
                    source,
                })?;
                let rel = p
                    .strip_prefix(repo_root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                let rules = parse_rule_file(&content, RuleSource::Custom(rel))?;
                collected.extend(rules);
            }
        }
    }

    // Deduplicate rules by ID, deterministically sorting by ID ascending
    let mut dedup_map: std::collections::BTreeMap<String, DeclarativeRule> =
        std::collections::BTreeMap::new();
    for r in collected {
        dedup_map.insert(r.id.clone(), r);
    }

    Ok(dedup_map.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_declarative_yaml_rule_file() {
        let yaml = r#"
version: 1
pack: "api-boundaries"
description: "Rules governing boundary isolation between layers"
rules:
  - id: "arch/no-db-in-handlers"
    name: "Forbid Database in Handlers"
    category: "architecture"
    severity: "high"
    confidence: "certain"
    description: "API handlers must not import database drivers directly"
    match:
      type: "forbidden_imports"
      files:
        - "src/api/**"
      forbidden:
        - "src/db/**"
        - "@prisma/client"
    remediation: "Route queries through service abstractions."
"#;
        let rules_res = parse_rule_file(yaml, RuleSource::Custom("api.yaml".to_string()));
        assert!(rules_res.is_ok());
        if let Ok(rules) = rules_res {
            assert_eq!(rules.len(), 1);
            let rule = &rules[0];
            assert_eq!(rule.id, "arch/no-db-in-handlers");
            assert_eq!(rule.category, RuleCategory::Architecture);
            assert_eq!(rule.severity, Severity::High);
            assert_eq!(rule.confidence, Confidence::Certain);
            assert_eq!(rule.matcher.kind, MatcherKind::ForbiddenImports);
            assert_eq!(rule.matcher.files, vec!["src/api/**"]);
            assert_eq!(rule.matcher.forbidden, vec!["src/db/**", "@prisma/client"]);
            assert_eq!(
                rule.remediation,
                "Route queries through service abstractions."
            );
            assert_eq!(rule.source, RuleSource::Custom("api.yaml".to_string()));
        }
    }

    #[test]
    fn parses_declarative_json_rule_file() {
        let json = r#"{
  "version": 1,
  "rules": [
    {
      "id": "quality/require-controller-tests",
      "name": "Require Controller Tests",
      "category": "quality",
      "severity": "medium",
      "description": "Every controller must have a matching unit test",
      "match": {
        "type": "required_companion",
        "files": ["src/controllers/**/*.ts"],
        "companion": "tests/{name}.test.ts"
      },
      "remediation": "Create companion test file."
    }
  ]
}"#;
        let rules_res = parse_rule_file(json, RuleSource::Pack("strict".to_string()));
        assert!(rules_res.is_ok());
        if let Ok(rules) = rules_res {
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].id, "quality/require-controller-tests");
            assert_eq!(rules[0].category, RuleCategory::Quality);
            assert_eq!(rules[0].matcher.kind, MatcherKind::RequiredCompanion);
            assert_eq!(
                rules[0].matcher.companion.as_deref(),
                Some("tests/{name}.test.ts")
            );
        }
    }

    #[test]
    fn rejects_invalid_rule_ids() {
        assert!(validate_rule_id("no-category-prefix").is_err());
        assert!(validate_rule_id("unknowncat/rule-name").is_err());
        assert!(validate_rule_id("arch/").is_err());
        assert!(validate_rule_id("arch/invalid whitespace").is_err());
        assert!(validate_rule_id("arch/valid-rule_id").is_ok());
        assert!(validate_rule_id("quality/max-file-lines").is_ok());
        assert!(validate_rule_id("security/no-eval").is_ok());
        assert!(validate_rule_id("deps/no-unpinned").is_ok());
    }

    #[test]
    fn rejects_incomplete_rule_matchers() {
        let bad_rule = DeclarativeRule {
            id: "arch/empty-forbidden".to_string(),
            name: "Empty forbidden".to_string(),
            category: RuleCategory::Architecture,
            severity: Severity::High,
            confidence: Confidence::Certain,
            description: "Some description".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::ForbiddenImports,
                files: vec!["src/**".to_string()],
                forbidden: Vec::new(),
                ..Default::default()
            },
            remediation: "Fix it".to_string(),
            source: RuleSource::Builtin("test".to_string()),
            enabled: true,
        };
        assert!(validate_rule(&bad_rule).is_err());
    }

    #[test]
    fn prevents_path_traversal_when_loading_rules() {
        let temp_dir = std::env::temp_dir().join("forge-rules-traversal-test");
        let _ = fs::create_dir_all(&temp_dir);
        let malicious_path = vec!["../../outside-rules.yaml".to_string()];
        let res = load_declarative_rules(&temp_dir, &malicious_path);
        assert!(matches!(res, Err(ForgeError::PathTraversal { .. })));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn loads_declarative_rules_from_directory_and_custom_paths() {
        let raw_temp = std::env::temp_dir().join("forge-rules-load-test");
        let _ = fs::create_dir_all(&raw_temp);
        let temp_dir = match raw_temp.canonicalize() {
            Ok(c) => c,
            Err(_) => raw_temp,
        };
        let rules_dir = temp_dir.join(".eforge").join("rules");
        let _ = fs::create_dir_all(&rules_dir);

        let yaml1 = r#"
rules:
  - id: "arch/no-db-in-ui"
    name: "No DB in UI"
    category: "arch"
    severity: "high"
    description: "UI cannot touch DB directly"
    match:
      type: "forbidden_imports"
      files: ["src/ui/**"]
      forbidden: ["src/db/**"]
"#;
        let _ = fs::write(rules_dir.join("arch.yaml"), yaml1);

        let json2 = r#"{
  "rules": [
    {
      "id": "quality/max-lines",
      "name": "Max Lines",
      "category": "quality",
      "severity": "low",
      "description": "Keep files small",
      "match": {
        "type": "file_size",
        "max_lines": 350
      }
    }
  ]
}"#;
        let custom_dir = temp_dir.join("custom");
        let _ = fs::create_dir_all(&custom_dir);
        let custom_file = custom_dir.join("quality.json");
        let _ = fs::write(&custom_file, json2);

        let custom_paths = vec!["custom/quality.json".to_string()];
        let loaded_res = load_declarative_rules(&temp_dir, &custom_paths);
        assert!(loaded_res.is_ok());
        if let Ok(loaded) = loaded_res {
            assert_eq!(loaded.len(), 2);
            assert_eq!(loaded[0].id, "arch/no-db-in-ui");
            assert_eq!(loaded[1].id, "quality/max-lines");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn validates_all_matcher_kinds() {
        let naming_rule = DeclarativeRule {
            id: "quality/controller-suffix".to_string(),
            name: "Controller Suffix".to_string(),
            category: RuleCategory::Quality,
            severity: Severity::Low,
            confidence: Confidence::High,
            description: "Controllers must end in Controller".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::FileNaming,
                convention: Some("*Controller.ts".to_string()),
                ..Default::default()
            },
            remediation: "Rename file".to_string(),
            source: RuleSource::Builtin("pack".to_string()),
            enabled: true,
        };
        assert!(validate_rule(&naming_rule).is_ok());

        let regex_rule = DeclarativeRule {
            id: "security/no-eval".to_string(),
            name: "No Eval".to_string(),
            category: RuleCategory::Security,
            severity: Severity::Critical,
            confidence: Confidence::Certain,
            description: "eval is dangerous".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::RegexPattern,
                pattern: Some(r"eval\(".to_string()),
                ..Default::default()
            },
            remediation: "Avoid dynamic code execution".to_string(),
            source: RuleSource::Builtin("security".to_string()),
            enabled: true,
        };
        assert!(validate_rule(&regex_rule).is_ok());

        let dep_rule = DeclarativeRule {
            id: "deps/no-moment".to_string(),
            name: "No Moment".to_string(),
            category: RuleCategory::Dependencies,
            severity: Severity::Medium,
            confidence: Confidence::High,
            description: "Moment is legacy".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::DependencyRestriction,
                disallowed_packages: vec!["moment".to_string()],
                ..Default::default()
            },
            remediation: "Use date-fns".to_string(),
            source: RuleSource::Builtin("pack".to_string()),
            enabled: true,
        };
        assert!(validate_rule(&dep_rule).is_ok());
    }

    #[test]
    fn rule_category_parsing_and_display() {
        assert_eq!(
            RuleCategory::from_str("architecture").ok(),
            Some(RuleCategory::Architecture)
        );
        assert_eq!(
            RuleCategory::from_str("arch").ok(),
            Some(RuleCategory::Architecture)
        );
        assert_eq!(
            RuleCategory::from_str("quality").ok(),
            Some(RuleCategory::Quality)
        );
        assert_eq!(
            RuleCategory::from_str("security").ok(),
            Some(RuleCategory::Security)
        );
        assert_eq!(
            RuleCategory::from_str("sec").ok(),
            Some(RuleCategory::Security)
        );
        assert_eq!(
            RuleCategory::from_str("deps").ok(),
            Some(RuleCategory::Dependencies)
        );
        assert_eq!(
            RuleCategory::from_str("dependencies").ok(),
            Some(RuleCategory::Dependencies)
        );
        assert!(RuleCategory::from_str("invalid").is_err());
        assert_eq!(RuleCategory::Architecture.to_string(), "architecture");
    }
}
