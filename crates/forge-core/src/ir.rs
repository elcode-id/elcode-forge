use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Normalized finding severity levels ordered from lowest (`Info`) to highest (`Critical`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Normalized confidence levels ordered from lowest (`Low`) to highest (`Certain`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
    Certain,
}

/// Source code location for a finding or symbol within the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocation {
    pub file_path: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

/// Normalized finding emitted by deterministic analyzers, rules, or security scanners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    pub id: String,
    pub category: String,
    pub severity: Severity,
    pub confidence: Confidence,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub location: Option<SourceLocation>,
    pub recommendation: String,
    pub rule_id: String,
}

/// Normalized file metadata node in the repository graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileNode {
    pub relative_path: String,
    pub language: Option<String>,
    pub size_bytes: u64,
    pub line_count: u32,
    pub content_hash: String,
    pub is_test: bool,
}

/// Logical module boundary inferred from repository structure and import relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub files: Vec<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// External package dependency metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub resolved_version: Option<String>,
    pub ecosystem: String,
    pub is_dev: bool,
    pub is_direct: bool,
}

/// Explainable health score for an individual engineering dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DimensionScore {
    pub dimension: String,
    pub score: u8,
    pub weight_pct: u8,
    pub reasons: Vec<String>,
}

/// Transparent overall codebase health score (`0..=100`) and breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HealthScore {
    pub overall: u8,
    pub dimensions: Vec<DimensionScore>,
}

/// Language-neutral Intermediate Representation (IR) of an analyzed repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Repository {
    pub root_path: String,
    pub name: String,
    pub detected_languages: Vec<String>,
    pub detected_frameworks: Vec<String>,
    pub package_managers: Vec<String>,
    pub files: Vec<FileNode>,
    pub modules: Vec<Module>,
    pub dependencies: Vec<Dependency>,
    pub findings: Vec<Finding>,
    pub health: Option<HealthScore>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ForgeError;
    use crate::traits::Analyzer;

    struct DummyAnalyzer;

    impl Analyzer for DummyAnalyzer {
        fn name(&self) -> &'static str {
            "dummy-analyzer"
        }

        fn analyze(&self, repo: &Repository) -> Result<Vec<Finding>, ForgeError> {
            Ok(vec![Finding {
                id: "F-001".to_string(),
                category: "architecture".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                title: format!("Checked {}", repo.name),
                description: "Component bypasses service layer".to_string(),
                evidence: "import { db } from '../db/client'".to_string(),
                location: Some(SourceLocation {
                    file_path: "src/components/UserCard.tsx".to_string(),
                    start_line: Some(4),
                    end_line: Some(4),
                }),
                recommendation: "Route data access through a service module".to_string(),
                rule_id: "arch/no-direct-db-in-ui".to_string(),
            }])
        }
    }

    #[test]
    fn severity_and_confidence_ordering_is_deterministic() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);

        assert!(Confidence::Certain > Confidence::High);
        assert!(Confidence::High > Confidence::Medium);
        assert!(Confidence::Medium > Confidence::Low);
    }

    #[test]
    fn repository_ir_roundtrips_deterministically_through_json() {
        let repo = Repository {
            root_path: "/workspace/app".to_string(),
            name: "app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec!["nextjs".to_string()],
            package_managers: vec!["pnpm".to_string()],
            files: vec![FileNode {
                relative_path: "src/index.ts".to_string(),
                language: Some("typescript".to_string()),
                size_bytes: 128,
                line_count: 10,
                content_hash: "blake3_abc123".to_string(),
                is_test: false,
            }],
            modules: vec![Module {
                id: "mod:src".to_string(),
                name: "src".to_string(),
                root_path: "src".to_string(),
                files: vec!["src/index.ts".to_string()],
                imports: vec![],
                exports: vec!["main".to_string()],
            }],
            dependencies: vec![Dependency {
                name: "next".to_string(),
                version_req: "^15.0.0".to_string(),
                resolved_version: Some("15.0.2".to_string()),
                ecosystem: "npm".to_string(),
                is_dev: false,
                is_direct: true,
            }],
            findings: vec![],
            health: Some(HealthScore {
                overall: 92,
                dimensions: vec![DimensionScore {
                    dimension: "Architecture".to_string(),
                    score: 92,
                    weight_pct: 20,
                    reasons: vec!["0 circular dependencies detected".to_string()],
                }],
            }),
        };

        let analyzer = DummyAnalyzer;
        let findings = analyzer.analyze(&repo);
        assert!(findings.is_ok());

        let json_a = serde_json::to_string(&repo);
        let json_b = serde_json::to_string(&repo);
        assert!(json_a.is_ok());
        assert_eq!(json_a.as_ref().ok(), json_b.as_ref().ok());

        if let Ok(serialized) = json_a {
            let decoded: Result<Repository, _> = serde_json::from_str(&serialized);
            assert_eq!(decoded.ok().as_ref(), Some(&repo));
        }
    }
}
