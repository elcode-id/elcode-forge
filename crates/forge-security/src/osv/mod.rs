//! OSV (Open Source Vulnerabilities) schema definitions and ecosystem normalization.

pub mod cache;
pub mod matcher;
pub mod version;

use forge_core::Severity;
use serde::{Deserialize, Serialize};

pub use cache::LocalAdvisoryCache;
pub use matcher::{VulnerabilityMatcher, RULE_VULNERABLE_DEPENDENCY};
pub use version::Version;

/// Standard OSV (Open Source Vulnerabilities) Advisory Schema v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvAdvisory {
    /// Primary vulnerability identifier (e.g. `GHSA-p6mc-m468-83gw` or `CVE-2020-8203`).
    pub id: String,
    /// Vulnerability aliases (e.g. `["CVE-2020-8203"]`).
    #[serde(default)]
    pub aliases: Vec<String>,
    /// One-line human-readable summary of the vulnerability.
    #[serde(default)]
    pub summary: String,
    /// Longer markdown description detailing vulnerability impact.
    #[serde(default)]
    pub details: String,
    /// Severity ratings including CVSS metrics.
    #[serde(default)]
    pub severity: Vec<OsvSeverity>,
    /// Affected packages and vulnerable version ranges.
    #[serde(default)]
    pub affected: Vec<OsvAffected>,
    /// Upstream references and advisories.
    #[serde(default)]
    pub references: Vec<OsvReference>,
    /// Ecosystem-specific database fields.
    #[serde(default)]
    pub database_specific: Option<serde_json::Value>,
}

impl OsvAdvisory {
    /// Returns the most widely recognized identifier (preferring CVE over GHSA if available).
    pub fn display_id(&self) -> &str {
        if let Some(cve) = self.aliases.iter().find(|a| a.starts_with("CVE-")) {
            return cve.as_str();
        }
        &self.id
    }

    /// Derives the normalized [`Severity`] level based on CVSS scores or database-specific labels.
    pub fn compute_severity(&self) -> Severity {
        // 1. Check database_specific severity string (e.g. "CRITICAL", "HIGH", "MODERATE", "LOW")
        if let Some(db_spec) = &self.database_specific {
            if let Some(sev_str) = db_spec.get("severity").and_then(|v| v.as_str()) {
                match sev_str.to_ascii_uppercase().as_str() {
                    "CRITICAL" => return Severity::Critical,
                    "HIGH" => return Severity::High,
                    "MODERATE" | "MEDIUM" => return Severity::Medium,
                    "LOW" => return Severity::Low,
                    _ => {}
                }
            }
        }

        // 2. Check CVSS numeric score if present in severity list
        for sev in &self.severity {
            // If the score string is a direct numeric score (e.g. "9.8")
            if let Ok(score) = sev.score.parse::<f32>() {
                return cvss_score_to_severity(score);
            }
        }

        // 3. Fallback to High severity to ensure safety
        Severity::High
    }
}

/// Helper mapping numerical CVSS v3 score (0.0 ..= 10.0) to normalized [`Severity`].
pub fn cvss_score_to_severity(score: f32) -> Severity {
    if score >= 9.0 {
        Severity::Critical
    } else if score >= 7.0 {
        Severity::High
    } else if score >= 4.0 {
        Severity::Medium
    } else {
        Severity::Low
    }
}

/// OSV severity descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvSeverity {
    /// Severity type, e.g. `"CVSS_V3"` or `"CVSS_V4"`.
    #[serde(rename = "type")]
    pub severity_type: String,
    /// Score vector (e.g. `"CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:H/A:H"`) or numeric string.
    pub score: String,
}

/// Affected package block in an OSV advisory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvAffected {
    /// Package identification.
    pub package: OsvPackage,
    /// Vulnerability version ranges.
    #[serde(default)]
    pub ranges: Vec<OsvRange>,
    /// List of explicitly affected versions.
    #[serde(default)]
    pub versions: Vec<String>,
    /// Ecosystem-specific database fields.
    #[serde(default)]
    pub database_specific: Option<serde_json::Value>,
}

/// Package identity within an ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvPackage {
    /// Normalized package name (e.g. `"lodash"`, `"requests"`).
    pub name: String,
    /// Target ecosystem (e.g. `"npm"`, `"PyPI"`, `"Go"`, `"crates.io"`, `"Packagist"`).
    pub ecosystem: String,
    /// Package URL (PURL).
    #[serde(default)]
    pub purl: Option<String>,
}

/// Version range block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvRange {
    /// Range type (`"SEMVER"`, `"ECOSYSTEM"`, etc.).
    #[serde(rename = "type")]
    pub range_type: String,
    /// Sequential version boundary events.
    #[serde(default)]
    pub events: Vec<OsvEvent>,
}

/// Event point within a range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvEvent {
    #[serde(default)]
    pub introduced: Option<String>,
    #[serde(default)]
    pub fixed: Option<String>,
    #[serde(default)]
    pub last_affected: Option<String>,
    #[serde(default)]
    pub limit: Option<String>,
}

/// External reference link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvReference {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub url: String,
}

/// Normalizes ecosystem identifiers across Forge and OSV conventions.
pub fn normalize_ecosystem(ecosystem: &str) -> &'static str {
    match ecosystem.to_ascii_lowercase().as_str() {
        "npm" | "node" | "javascript" | "typescript" => "npm",
        "pypi" | "python" | "pip" => "pypi",
        "go" | "golang" => "go",
        "cargo" | "crates.io" | "rust" => "cargo",
        "composer" | "packagist" | "php" => "composer",
        _ => "unknown",
    }
}

/// Normalizes package names for deterministic, case-insensitive comparison.
pub fn normalize_package_name(ecosystem: &str, name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    match normalize_ecosystem(ecosystem) {
        // PyPI package names normalize underscores to dashes
        "pypi" => lower.replace('_', "-"),
        // Go packages strip scheme if present
        "go" => lower.strip_prefix("https://").unwrap_or(&lower).to_string(),
        _ => lower,
    }
}
