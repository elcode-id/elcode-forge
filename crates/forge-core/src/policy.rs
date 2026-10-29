//! Policy-as-Code data models, waiver validation, and schema definitions.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ForgeError;
use crate::ir::Severity;

/// Top-level Policy-as-Code document controlling enterprise engineering quality gates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct PolicyDocument {
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub constraints: PolicyConstraints,
    #[serde(default)]
    pub waivers: Vec<PolicyWaiver>,
    #[serde(default)]
    pub overrides: Vec<PolicyRuleOverride>,
}

/// Strict constraints evaluated against repository health scores, findings, and change impact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct PolicyConstraints {
    pub min_health_score: Option<u8>,
    #[serde(default)]
    pub min_dimension_scores: BTreeMap<String, u8>,
    #[serde(default)]
    pub disallowed_severities: Vec<Severity>,
    #[serde(default)]
    pub forbidden_licenses: Vec<String>,
    pub max_circular_dependencies: Option<usize>,
    pub max_blast_radius_modules: Option<usize>,
    pub max_modified_lines_without_tests: Option<usize>,
    #[serde(default)]
    pub required_rule_packs: Vec<String>,
    #[serde(default)]
    pub enforce_secret_zero_tolerance: bool,
}

/// Time-bounded, path-scoped waiver granting temporary exemption for a specific finding or rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PolicyWaiver {
    pub rule_id: String,
    #[serde(default)]
    pub path_pattern: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>, // RFC 3339 UTC or YYYY-MM-DD
    pub justification: String,
}

impl PolicyWaiver {
    /// Determines whether the waiver has expired relative to a given timestamp.
    pub fn is_expired_at(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        let Some(ref expires_str) = self.expires_at else {
            return false;
        };
        match chrono::DateTime::parse_from_rfc3339(expires_str) {
            Ok(dt) => now >= dt.with_timezone(&chrono::Utc),
            Err(_) => {
                if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(expires_str, "%Y-%m-%d") {
                    if let Some(naive_dt) = naive_date.and_hms_opt(23, 59, 59) {
                        let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                            naive_dt,
                            chrono::Utc,
                        );
                        return now >= dt;
                    }
                }
                // Safety: invalid date format is treated as expired
                true
            }
        }
    }

    /// Determines whether the waiver has expired as of current UTC time.
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(chrono::Utc::now())
    }

    /// Checks if this waiver matches a specific rule ID and optional file path.
    pub fn matches(&self, rule_id: &str, file_path: Option<&str>) -> bool {
        if self.rule_id != "*" && self.rule_id != rule_id {
            return false;
        }

        if let Some(ref pattern) = self.path_pattern {
            let Some(path) = file_path else {
                return false;
            };
            glob_path_match(pattern, path)
        } else {
            true
        }
    }
}

/// Simple glob-style path pattern matcher supporting `*`, `**`, and path prefixes/suffixes.
pub fn glob_path_match(pattern: &str, path: &str) -> bool {
    let norm_pattern = pattern.replace('\\', "/");
    let norm_path = path.replace('\\', "/");

    if norm_pattern == "*" || norm_pattern == "**" || norm_pattern == norm_path {
        return true;
    }

    if let Some(prefix) = norm_pattern.strip_suffix("/**") {
        return norm_path == prefix || norm_path.starts_with(&format!("{prefix}/"));
    }

    if let Some(suffix) = norm_pattern.strip_prefix("**/") {
        return norm_path == suffix || norm_path.ends_with(&format!("/{suffix}"));
    }

    if let Some(prefix) = norm_pattern.strip_suffix("/*") {
        if norm_path.starts_with(&format!("{prefix}/")) {
            let remainder = &norm_path[prefix.len() + 1..];
            return !remainder.contains('/');
        }
        return false;
    }

    if norm_pattern.contains('*') {
        let parts: Vec<&str> = norm_pattern.split('*').collect();
        if parts.len() == 2 {
            return norm_path.starts_with(parts[0]) && norm_path.ends_with(parts[1]);
        }
    }

    norm_path == norm_pattern
}

/// Rule severity/enablement override declared at the organization policy level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PolicyRuleOverride {
    pub rule_id: String,
    #[serde(default)]
    pub severity: Option<Severity>,
    pub enabled: bool,
}

/// Overall verdict result of policy gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PolicyVerdict {
    Passed,
    Warning,
    Violation,
}

/// Explainable report produced by policy gate evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PolicyReport {
    pub verdict: PolicyVerdict,
    pub policy_name: String,
    pub passed_checks: Vec<String>,
    pub warnings: Vec<String>,
    pub violations: Vec<String>,
    pub active_waivers: Vec<PolicyWaiver>,
}

impl PolicyReport {
    pub fn is_passed(&self) -> bool {
        matches!(self.verdict, PolicyVerdict::Passed)
    }

    pub fn is_violation(&self) -> bool {
        matches!(self.verdict, PolicyVerdict::Violation)
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.verdict, PolicyVerdict::Warning)
    }
}

/// Normalized change impact metrics for policy evaluation without depending on `forge-git`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct PolicyImpactSummary {
    pub affected_module_count: usize,
    pub untested_changed_files: usize,
    pub total_modified_lines: usize,
    #[serde(default)]
    pub untested_modified_lines: usize,
}

/// Loads a policy document from a `.json` or `.yaml`/`.yml` file.
pub fn load_policy(path: &Path) -> Result<PolicyDocument, ForgeError> {
    let content = fs::read_to_string(path).map_err(|source| ForgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let is_json = path.extension().and_then(|e| e.to_str()) == Some("json")
        || content.trim_start().starts_with('{');

    if is_json {
        serde_json::from_str(&content).map_err(|err| ForgeError::Serialization {
            message: format!("Failed to parse policy JSON at {}: {err}", path.display()),
        })
    } else {
        parse_policy_yaml(&content)
    }
}

/// Parses a Policy-as-Code YAML string into [`PolicyDocument`].
pub fn parse_policy_yaml(content: &str) -> Result<PolicyDocument, ForgeError> {
    let mut policy = PolicyDocument::default();
    let mut current_section: Option<&str> = None;
    let mut active_list: Option<&str> = None;

    let mut current_waiver: Option<PolicyWaiver> = None;
    let mut current_override: Option<PolicyRuleOverride> = None;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
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

        // Check for list items
        if let Some(item) = trimmed.strip_prefix('-') {
            let item_trimmed = item.trim();

            if current_section == Some("waivers") {
                if let Some(w) = current_waiver.take() {
                    policy.waivers.push(w);
                }
                current_waiver = Some(PolicyWaiver {
                    rule_id: String::new(),
                    path_pattern: None,
                    expires_at: None,
                    justification: String::new(),
                });
                if !item_trimmed.is_empty() {
                    if let Some((k, v)) = item_trimmed.split_once(':') {
                        apply_waiver_field(
                            current_waiver.as_mut(),
                            unquote(k.trim()),
                            unquote(v.trim()),
                        );
                    }
                }
                continue;
            }

            if current_section == Some("overrides") {
                if let Some(o) = current_override.take() {
                    policy.overrides.push(o);
                }
                current_override = Some(PolicyRuleOverride {
                    rule_id: String::new(),
                    severity: None,
                    enabled: true,
                });
                if !item_trimmed.is_empty() {
                    if let Some((k, v)) = item_trimmed.split_once(':') {
                        apply_override_field(
                            current_override.as_mut(),
                            unquote(k.trim()),
                            unquote(v.trim()),
                            line_no,
                        )?;
                    }
                }
                continue;
            }

            let value = unquote(item_trimmed);
            match active_list {
                Some("disallowed_severities") => {
                    let sev = parse_severity(value, line_no)?;
                    policy.constraints.disallowed_severities.push(sev);
                }
                Some("forbidden_licenses") => {
                    policy
                        .constraints
                        .forbidden_licenses
                        .push(value.to_string());
                }
                Some("required_rule_packs") => {
                    policy
                        .constraints
                        .required_rule_packs
                        .push(value.to_string());
                }
                _ => {}
            }
            continue;
        }

        let Some((raw_key, raw_val)) = trimmed.split_once(':') else {
            return Err(ForgeError::Config {
                message: format!("line {line_no}: expected 'key: value' syntax in policy"),
            });
        };

        let key = unquote(raw_key.trim());
        let val = unquote(raw_val.trim());

        if indent == 0 {
            // Commit any pending waiver or override
            if let Some(w) = current_waiver.take() {
                policy.waivers.push(w);
            }
            if let Some(o) = current_override.take() {
                policy.overrides.push(o);
            }

            current_section = None;
            active_list = None;

            match key {
                "name" => policy.name = val.to_string(),
                "version" => {
                    let v = val.parse::<u32>().map_err(|_| ForgeError::Config {
                        message: format!("line {line_no}: invalid policy version '{val}'"),
                    })?;
                    policy.version = v;
                }
                "organization" => {
                    if !val.is_empty() {
                        policy.organization = Some(val.to_string());
                    }
                }
                "constraints" => current_section = Some("constraints"),
                "waivers" => current_section = Some("waivers"),
                "overrides" => current_section = Some("overrides"),
                _ => {
                    return Err(ForgeError::Config {
                        message: format!("line {line_no}: unknown policy root key '{key}'"),
                    });
                }
            }
        } else if current_section == Some("waivers") {
            if let Some(ref mut waiver) = current_waiver {
                apply_waiver_field(Some(waiver), key, val);
            }
        } else if current_section == Some("overrides") {
            if let Some(ref mut over) = current_override {
                apply_override_field(Some(over), key, val, line_no)?;
            }
        } else if current_section == Some("constraints") {
            match key {
                "min_health_score" => {
                    let score = val.parse::<u8>().map_err(|_| ForgeError::Config {
                        message: format!("line {line_no}: invalid min_health_score '{val}'"),
                    })?;
                    policy.constraints.min_health_score = Some(score);
                }
                "min_dimension_scores" => {
                    active_list = Some("min_dimension_scores");
                }
                "disallowed_severities" => {
                    if let Some(items) = parse_inline_array(val) {
                        for item in items {
                            policy
                                .constraints
                                .disallowed_severities
                                .push(parse_severity(&item, line_no)?);
                        }
                    } else {
                        active_list = Some("disallowed_severities");
                    }
                }
                "forbidden_licenses" => {
                    if let Some(items) = parse_inline_array(val) {
                        policy.constraints.forbidden_licenses.extend(items);
                    } else {
                        active_list = Some("forbidden_licenses");
                    }
                }
                "max_circular_dependencies" => {
                    let count = val.parse::<usize>().map_err(|_| ForgeError::Config {
                        message: format!(
                            "line {line_no}: invalid max_circular_dependencies '{val}'"
                        ),
                    })?;
                    policy.constraints.max_circular_dependencies = Some(count);
                }
                "max_blast_radius_modules" => {
                    let count = val.parse::<usize>().map_err(|_| ForgeError::Config {
                        message: format!(
                            "line {line_no}: invalid max_blast_radius_modules '{val}'"
                        ),
                    })?;
                    policy.constraints.max_blast_radius_modules = Some(count);
                }
                "max_modified_lines_without_tests" => {
                    let count = val.parse::<usize>().map_err(|_| ForgeError::Config {
                        message: format!(
                            "line {line_no}: invalid max_modified_lines_without_tests '{val}'"
                        ),
                    })?;
                    policy.constraints.max_modified_lines_without_tests = Some(count);
                }
                "required_rule_packs" => {
                    if let Some(items) = parse_inline_array(val) {
                        policy.constraints.required_rule_packs.extend(items);
                    } else {
                        active_list = Some("required_rule_packs");
                    }
                }
                "enforce_secret_zero_tolerance" => {
                    policy.constraints.enforce_secret_zero_tolerance = parse_bool(val, line_no)?;
                }
                dim if active_list == Some("min_dimension_scores") => {
                    let score = val.parse::<u8>().map_err(|_| ForgeError::Config {
                        message: format!("line {line_no}: invalid score for dimension '{dim}'"),
                    })?;
                    policy
                        .constraints
                        .min_dimension_scores
                        .insert(dim.to_string(), score);
                }
                _ => {
                    return Err(ForgeError::Config {
                        message: format!("line {line_no}: unknown constraint key '{key}'"),
                    });
                }
            }
        }
    }

    if let Some(w) = current_waiver {
        policy.waivers.push(w);
    }
    if let Some(o) = current_override {
        policy.overrides.push(o);
    }

    Ok(policy)
}

fn apply_waiver_field(waiver: Option<&mut PolicyWaiver>, key: &str, val: &str) {
    let Some(w) = waiver else { return };
    match key {
        "rule_id" => w.rule_id = val.to_string(),
        "path_pattern" => {
            if !val.is_empty() {
                w.path_pattern = Some(val.to_string());
            }
        }
        "expires_at" => {
            if !val.is_empty() {
                w.expires_at = Some(val.to_string());
            }
        }
        "justification" => w.justification = val.to_string(),
        _ => {}
    }
}

fn apply_override_field(
    over: Option<&mut PolicyRuleOverride>,
    key: &str,
    val: &str,
    line_no: usize,
) -> Result<(), ForgeError> {
    let Some(o) = over else { return Ok(()) };
    match key {
        "rule_id" => o.rule_id = val.to_string(),
        "severity" => {
            if !val.is_empty() {
                o.severity = Some(parse_severity(val, line_no)?);
            }
        }
        "enabled" => o.enabled = parse_bool(val, line_no)?,
        _ => {}
    }
    Ok(())
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            &s[1..s.len() - 1]
        } else {
            s
        }
    } else {
        s
    }
}

fn parse_bool(val: &str, line_no: usize) -> Result<bool, ForgeError> {
    match val.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => Err(ForgeError::Config {
            message: format!("line {line_no}: invalid boolean '{val}'"),
        }),
    }
}

fn parse_severity(val: &str, line_no: usize) -> Result<Severity, ForgeError> {
    match val.trim().to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "high" => Ok(Severity::High),
        "medium" => Ok(Severity::Medium),
        "low" => Ok(Severity::Low),
        "info" => Ok(Severity::Info),
        _ => Err(ForgeError::Config {
            message: format!("line {line_no}: invalid severity '{val}'"),
        }),
    }
}

fn parse_inline_array(val: &str) -> Option<Vec<String>> {
    let trimmed = val.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.trim().is_empty() {
            return Some(Vec::new());
        }
        let items = inner
            .split(',')
            .map(|s| unquote(s.trim()).to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(items)
    } else {
        None
    }
}
