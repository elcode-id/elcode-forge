//! OASIS SARIF v2.1.0 JSON formatter for [`Repository`] findings.

use std::collections::BTreeMap;

use forge_core::{ForgeError, Repository, Severity};
use serde_json::{json, Value};

/// Renders a [`Repository`] report into a deterministic OASIS SARIF v2.1.0 JSON document.
pub fn render_sarif(repo: &Repository) -> Result<String, ForgeError> {
    let mut rules_map: BTreeMap<String, Value> = BTreeMap::new();
    let mut results: Vec<Value> = Vec::with_capacity(repo.findings.len());

    for finding in &repo.findings {
        rules_map.entry(finding.rule_id.clone()).or_insert_with(|| {
            let mut tags = vec!["security".to_string()];
            if finding.rule_id.contains("vulnerab") || finding.id.contains("vulnerab") {
                tags.push("vulnerability".to_string());
                tags.push("supply-chain".to_string());
            }

            let security_severity = match finding.severity {
                Severity::Critical => "9.0",
                Severity::High => "7.0",
                Severity::Medium => "4.0",
                Severity::Low => "2.0",
                Severity::Info => "0.0",
            };

            let help_uri = if finding.id.starts_with("security/vulnerability/") {
                if let Some(adv_id) = finding.id.strip_prefix("security/vulnerability/") {
                    if adv_id.starts_with("cve-") {
                        format!(
                            "https://nvd.nist.gov/vuln/detail/{}",
                            adv_id.to_ascii_uppercase()
                        )
                    } else if adv_id.starts_with("ghsa-") {
                        format!(
                            "https://github.com/advisories/{}",
                            adv_id.to_ascii_uppercase()
                        )
                    } else {
                        "https://github.com/elcode-id/elcode-forge".to_string()
                    }
                } else {
                    "https://github.com/elcode-id/elcode-forge".to_string()
                }
            } else {
                "https://github.com/elcode-id/elcode-forge".to_string()
            };

            json!({
                "id": finding.rule_id,
                "name": finding.title,
                "shortDescription": {
                    "text": finding.title
                },
                "fullDescription": {
                    "text": finding.description
                },
                "help": {
                    "text": finding.recommendation
                },
                "helpUri": help_uri,
                "properties": {
                    "category": finding.category,
                    "tags": tags,
                    "security-severity": security_severity
                }
            })
        });

        let sarif_level = match finding.severity {
            Severity::Critical | Severity::High => "error",
            Severity::Medium | Severity::Low => "warning",
            Severity::Info => "note",
        };

        let locations = if let Some(loc) = &finding.location {
            let mut region = serde_json::Map::new();
            region.insert(
                "startLine".to_string(),
                Value::from(loc.start_line.unwrap_or(1).max(1)),
            );
            if let Some(end_line) = loc.end_line {
                region.insert("endLine".to_string(), Value::from(end_line.max(1)));
            }
            vec![json!({
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": loc.file_path
                    },
                    "region": Value::Object(region)
                }
            })]
        } else {
            Vec::new()
        };

        results.push(json!({
            "ruleId": finding.rule_id,
            "level": sarif_level,
            "message": {
                "text": format!("{} — {} (Evidence: {})", finding.title, finding.description, finding.evidence)
            },
            "locations": locations
        }));
    }

    let rules: Vec<Value> = rules_map.into_values().collect();
    let sarif_doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "ELCODE Forge (eforge)",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/elcode-id/elcode-forge",
                        "rules": rules
                    }
                },
                "results": results
            }
        ]
    });

    serde_json::to_string_pretty(&sarif_doc).map_err(|err| ForgeError::Serialization {
        message: err.to_string(),
    })
}
