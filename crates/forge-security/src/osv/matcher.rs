//! High-throughput polyglot vulnerability matcher against OSV advisories.

use std::collections::BTreeMap;
use std::path::Path;

use forge_core::{Confidence, Dependency, Finding, SourceLocation};

use super::cache::LocalAdvisoryCache;
use super::version::{is_version_in_range, Version};
use super::{normalize_ecosystem, normalize_package_name, OsvAdvisory};

/// Rule ID for all CVE/GHSA dependency vulnerabilities.
pub const RULE_VULNERABLE_DEPENDENCY: &str = "security/vulnerable-dependency";

/// Indexed in-memory vulnerability matcher.
#[derive(Debug, Default, Clone)]
pub struct VulnerabilityMatcher {
    /// In-memory indexed advisories keyed by `(normalized_ecosystem, normalized_package_name)`.
    advisories: BTreeMap<(String, String), Vec<OsvAdvisory>>,
}

impl VulnerabilityMatcher {
    /// Constructs a matcher from an existing collection of [`OsvAdvisory`] items.
    pub fn from_advisories(advisories: Vec<OsvAdvisory>) -> Self {
        let mut index: BTreeMap<(String, String), Vec<OsvAdvisory>> = BTreeMap::new();

        for advisory in advisories {
            for affected in &advisory.affected {
                let eco = normalize_ecosystem(&affected.package.ecosystem).to_string();
                let name = normalize_package_name(&eco, &affected.package.name);
                index.entry((eco, name)).or_default().push(advisory.clone());
            }
        }

        Self { advisories: index }
    }

    /// Loads advisories from local `.eforge/cache/osv/` and the embedded offline catalog.
    pub fn load(repo_root: &Path) -> Self {
        let advisories = LocalAdvisoryCache::load_all_advisories(repo_root);
        Self::from_advisories(advisories)
    }

    /// Evaluates an individual [`Dependency`] and returns all matching vulnerability [`Finding`]s.
    pub fn match_dependency(&self, dep: &Dependency) -> Vec<Finding> {
        let eco = normalize_ecosystem(&dep.ecosystem).to_string();
        let name = normalize_package_name(&eco, &dep.name);

        let Some(advisories) = self.advisories.get(&(eco, name)) else {
            return Vec::new();
        };

        let raw_ver_str = dep
            .resolved_version
            .as_deref()
            .unwrap_or(dep.version_req.as_str());

        let cleaned_ver_str = clean_version_requirement(raw_ver_str);
        let candidate_ver = Version::parse(cleaned_ver_str);

        let mut findings = Vec::new();

        for adv in advisories {
            for affected in &adv.affected {
                if !is_package_affected(affected, &candidate_ver) {
                    continue;
                }

                let display_id = adv.display_id();
                let fixed_target = extract_fixed_version(affected).unwrap_or("latest");

                let finding = Finding {
                    id: format!("security/vulnerability/{}", display_id.to_ascii_lowercase()),
                    category: "security".to_string(),
                    severity: adv.compute_severity(),
                    confidence: Confidence::Certain,
                    title: format!(
                        "Vulnerable dependency: {}@{} ({})",
                        dep.name, candidate_ver.raw, display_id
                    ),
                    description: if adv.summary.is_empty() {
                        adv.details.clone()
                    } else {
                        adv.summary.clone()
                    },
                    evidence: format!(
                        "Package: {} | Installed: {} | Advisory: {} | Fixed in: {}",
                        dep.name, candidate_ver.raw, display_id, fixed_target
                    ),
                    location: Some(SourceLocation {
                        file_path: default_manifest_for_ecosystem(&dep.ecosystem).to_string(),
                        start_line: None,
                        end_line: None,
                    }),
                    recommendation: format!(
                        "Upgrade {} to version {} or later",
                        dep.name, fixed_target
                    ),
                    rule_id: RULE_VULNERABLE_DEPENDENCY.to_string(),
                };

                findings.push(finding);
                // Break out of inner loop so we don't emit duplicate findings for the same advisory
                break;
            }
        }

        findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        findings
    }

    /// Evaluates a collection of [`Dependency`] items and returns deterministically sorted [`Finding`]s.
    pub fn match_dependencies(&self, deps: &[Dependency]) -> Vec<Finding> {
        let mut all_findings = Vec::new();
        for dep in deps {
            all_findings.extend(self.match_dependency(dep));
        }

        all_findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        all_findings
    }
}

/// Checks if `candidate_ver` satisfies the affected range or versions in `affected`.
fn is_package_affected(affected: &super::OsvAffected, candidate_ver: &Version) -> bool {
    // 1. Check explicit affected versions
    if affected
        .versions
        .iter()
        .any(|v| Version::parse(v) == *candidate_ver)
    {
        return true;
    }

    // 2. Check range events
    for range in &affected.ranges {
        let mut events: Vec<(Option<String>, Option<String>, Option<String>)> = Vec::new();
        for ev in &range.events {
            events.push((
                ev.introduced.clone(),
                ev.fixed.clone(),
                ev.last_affected.clone(),
            ));
        }

        if is_version_in_range(candidate_ver, &events) {
            return true;
        }
    }

    false
}

/// Extracts the first fixed version string from range events if present.
fn extract_fixed_version(affected: &super::OsvAffected) -> Option<&str> {
    for range in &affected.ranges {
        for ev in &range.events {
            if let Some(fixed) = &ev.fixed {
                return Some(fixed.as_str());
            }
        }
    }
    None
}

/// Strips version requirement prefixes like `==`, `=`, `^`, `~`, `>=`, `<=`, etc.
fn clean_version_requirement(raw: &str) -> &str {
    let trimmed = raw.trim();
    let cleaned = trimmed.trim_start_matches(|c: char| {
        c == '=' || c == '^' || c == '~' || c == '>' || c == '<' || c == ' '
    });
    cleaned.trim()
}

/// Returns the idiomatic manifest file for an ecosystem.
fn default_manifest_for_ecosystem(ecosystem: &str) -> &'static str {
    match normalize_ecosystem(ecosystem) {
        "npm" => "package.json",
        "pypi" => "requirements.txt",
        "go" => "go.mod",
        "cargo" => "Cargo.toml",
        "composer" => "composer.json",
        _ => "manifest",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::Severity;

    #[test]
    fn matches_lodash_prototype_pollution() {
        let matcher = VulnerabilityMatcher::from_advisories(
            super::super::cache::LocalAdvisoryCache::load_advisories(Path::new("."), "npm"),
        );

        let dep = Dependency {
            name: "lodash".to_string(),
            version_req: "^4.17.15".to_string(),
            resolved_version: Some("4.17.15".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        };

        let findings = matcher.match_dependency(&dep);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
        assert!(findings[0].evidence.contains("CVE-2020-8203"));
        assert!(findings[0].recommendation.contains("4.17.19"));
    }

    #[test]
    fn ignores_safe_version_of_lodash() {
        let matcher = VulnerabilityMatcher::from_advisories(
            super::super::cache::LocalAdvisoryCache::load_advisories(Path::new("."), "npm"),
        );

        let dep = Dependency {
            name: "lodash".to_string(),
            version_req: "^4.17.21".to_string(),
            resolved_version: Some("4.17.21".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        };

        let findings = matcher.match_dependency(&dep);
        assert!(findings.is_empty());
    }
}
