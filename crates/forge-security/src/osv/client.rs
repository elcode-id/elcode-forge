//! OSV.dev live HTTP batch client, query builder, and advisory hydrator.
//!
//! Enables on-demand vulnerability synchronization against `https://api.osv.dev/v1/querybatch`
//! and `https://api.osv.dev/v1/vulns/<id>` with bounded response sizes and graceful offline fallback.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use forge_core::{Dependency, ForgeError};
use serde::{Deserialize, Serialize};

use super::{normalize_ecosystem, LocalAdvisoryCache, OsvAdvisory};

/// Maximum response body allowed when reading OSV API responses (4 MiB).
pub const MAX_OSV_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

/// Default OSV cache freshness TTL in seconds (24 hours).
pub const DEFAULT_OSV_CACHE_TTL_SECS: u64 = 24 * 3600;

/// Maps a normalized internal ecosystem name to the canonical OSV API ecosystem identifier.
pub fn osv_api_ecosystem_name(norm_eco: &str) -> &'static str {
    match norm_eco {
        "npm" => "npm",
        "pypi" => "PyPI",
        "go" => "Go",
        "cargo" => "crates.io",
        "composer" => "Packagist",
        _ => "npm",
    }
}

/// Cleans raw version strings (e.g. `^4.17.15`, `>= 1.2.0`, `v0.4.0`) to a concrete version for OSV querying.
pub fn clean_version_for_query(raw_version: &str) -> Option<String> {
    let trimmed = raw_version.trim().trim_start_matches(|c| {
        c == '^' || c == '~' || c == '=' || c == '>' || c == '<' || c == 'v'
    });
    let candidate = trimmed.split_whitespace().next().unwrap_or(trimmed);
    if candidate.is_empty() || candidate == "*" || candidate.contains('*') {
        None
    } else {
        Some(candidate.to_string())
    }
}

/// Single package descriptor within an OSV batch query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvQueryPackage {
    pub name: String,
    pub ecosystem: String,
}

/// Single item within an OSV batch query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvQueryItem {
    pub package: OsvQueryPackage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Batch query request body sent to `https://api.osv.dev/v1/querybatch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvBatchRequest {
    pub queries: Vec<OsvQueryItem>,
}

/// Single result item within an OSV batch query response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvBatchResultItem {
    #[serde(default)]
    pub vulns: Vec<OsvAdvisory>,
}

/// Response returned from `https://api.osv.dev/v1/querybatch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsvBatchResponse {
    #[serde(default)]
    pub results: Vec<OsvBatchResultItem>,
}

/// Builds an [`OsvBatchRequest`] for dependencies belonging to `ecosystem`.
pub fn build_osv_batch_queries(ecosystem: &str, dependencies: &[Dependency]) -> OsvBatchRequest {
    let norm_eco = normalize_ecosystem(ecosystem);
    let api_eco = osv_api_ecosystem_name(norm_eco);
    let mut queries = Vec::new();
    let mut seen = BTreeSet::new();

    for dep in dependencies {
        if normalize_ecosystem(&dep.ecosystem) != norm_eco {
            continue;
        }
        let raw_ver = dep.resolved_version.as_deref().unwrap_or(&dep.version_req);
        let clean_ver = clean_version_for_query(raw_ver);
        let key = (dep.name.clone(), clean_ver.clone());
        if seen.insert(key) {
            queries.push(OsvQueryItem {
                package: OsvQueryPackage {
                    name: dep.name.clone(),
                    ecosystem: api_eco.to_string(),
                },
                version: clean_ver,
            });
        }
    }

    OsvBatchRequest { queries }
}

/// Deserializes a batch query response, returning fully hydrated advisories and IDs that require GET hydration.
pub fn parse_batch_query_response(
    json_text: &str,
) -> Result<(Vec<OsvAdvisory>, Vec<String>), ForgeError> {
    let parsed: OsvBatchResponse =
        serde_json::from_str(json_text).map_err(|e| ForgeError::Serialization {
            message: e.to_string(),
        })?;

    let mut full = Vec::new();
    let mut ids_to_hydrate = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for res in parsed.results {
        for adv in res.vulns {
            if seen_ids.insert(adv.id.clone()) {
                if adv.affected.is_empty() {
                    ids_to_hydrate.push(adv.id);
                } else {
                    full.push(adv);
                }
            }
        }
    }

    Ok((full, ids_to_hydrate))
}

/// Synchronizes stale advisories from OSV.dev for dependencies across all relevant ecosystems.
pub fn sync_dependencies_advisories(
    repo_root: &Path,
    dependencies: &[Dependency],
    agent: Option<&ureq::Agent>,
) -> Result<usize, ForgeError> {
    let mut ecosystems = BTreeSet::new();
    for dep in dependencies {
        let norm = normalize_ecosystem(&dep.ecosystem);
        if norm != "unknown" {
            ecosystems.insert(norm);
        }
    }

    let mut total_synced = 0;
    for eco in ecosystems {
        if let Ok(count) = sync_ecosystem_advisories(repo_root, eco, dependencies, agent) {
            total_synced += count;
        }
    }

    Ok(total_synced)
}

/// Synchronizes advisories from OSV.dev for `dependencies` within `ecosystem`.
pub fn sync_ecosystem_advisories(
    repo_root: &Path,
    ecosystem: &str,
    dependencies: &[Dependency],
    agent: Option<&ureq::Agent>,
) -> Result<usize, ForgeError> {
    let norm_eco = normalize_ecosystem(ecosystem);
    if LocalAdvisoryCache::is_cache_fresh(repo_root, norm_eco, DEFAULT_OSV_CACHE_TTL_SECS) {
        return Ok(0);
    }

    let batch_req = build_osv_batch_queries(norm_eco, dependencies);
    if batch_req.queries.is_empty() {
        return Ok(0);
    }

    let default_agent;
    let client = match agent {
        Some(a) => a,
        None => {
            default_agent = ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(5))
                .timeout_read(std::time::Duration::from_secs(10))
                .build();
            &default_agent
        }
    };

    let response = match client
        .post("https://api.osv.dev/v1/querybatch")
        .set("Content-Type", "application/json")
        .set("User-Agent", "elcode-forge/1.2.0")
        .send_json(&batch_req)
    {
        Ok(resp) => resp,
        Err(_) => return Ok(0),
    };

    let mut reader = response.into_reader().take(MAX_OSV_RESPONSE_BYTES);
    let mut body = String::new();
    if reader.read_to_string(&mut body).is_err() {
        return Ok(0);
    }

    let (mut full_advisories, ids_to_hydrate) = match parse_batch_query_response(&body) {
        Ok(res) => res,
        Err(_) => return Ok(0),
    };

    // Hydrate stub IDs via GET https://api.osv.dev/v1/vulns/<id> (capped to 50 requests)
    for id in ids_to_hydrate.into_iter().take(50) {
        let url = format!("https://api.osv.dev/v1/vulns/{id}");
        if let Ok(resp) = client
            .get(&url)
            .set("User-Agent", "elcode-forge/1.2.0")
            .call()
        {
            let mut r = resp.into_reader().take(MAX_OSV_RESPONSE_BYTES);
            let mut v_body = String::new();
            if r.read_to_string(&mut v_body).is_ok() {
                if let Ok(adv) = serde_json::from_str::<OsvAdvisory>(&v_body) {
                    full_advisories.push(adv);
                }
            }
        }
    }

    let mut merged_map: BTreeMap<String, OsvAdvisory> = BTreeMap::new();
    for adv in LocalAdvisoryCache::load_advisories(repo_root, norm_eco) {
        merged_map.insert(adv.id.clone(), adv);
    }
    for adv in full_advisories {
        merged_map.insert(adv.id.clone(), adv);
    }

    let merged: Vec<OsvAdvisory> = merged_map.into_values().collect();
    let _ = LocalAdvisoryCache::save_advisories(repo_root, norm_eco, &merged);
    Ok(merged.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_version_for_query() {
        assert_eq!(
            clean_version_for_query("^4.17.15"),
            Some("4.17.15".to_string())
        );
        assert_eq!(clean_version_for_query("~1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(clean_version_for_query("v2.0.1"), Some("2.0.1".to_string()));
        assert_eq!(
            clean_version_for_query("= 3.4.5"),
            Some("3.4.5".to_string())
        );
        assert_eq!(clean_version_for_query("*"), None);
        assert_eq!(
            clean_version_for_query("latest"),
            Some("latest".to_string())
        );
    }

    #[test]
    fn test_build_osv_batch_queries_and_ecosystem_mapping() {
        let deps = vec![
            Dependency {
                name: "lodash".to_string(),
                version_req: "^4.17.15".to_string(),
                resolved_version: None,
                ecosystem: "npm".to_string(),
                is_dev: false,
                is_direct: true,
            },
            Dependency {
                name: "lodash".to_string(),
                version_req: "^4.17.15".to_string(),
                resolved_version: None,
                ecosystem: "npm".to_string(),
                is_dev: true,
                is_direct: true,
            },
            Dependency {
                name: "requests".to_string(),
                version_req: "2.28.1".to_string(),
                resolved_version: None,
                ecosystem: "pypi".to_string(),
                is_dev: false,
                is_direct: true,
            },
        ];

        let npm_batch = build_osv_batch_queries("npm", &deps);
        assert_eq!(npm_batch.queries.len(), 1);
        assert_eq!(npm_batch.queries[0].package.name, "lodash");
        assert_eq!(npm_batch.queries[0].package.ecosystem, "npm");
        assert_eq!(npm_batch.queries[0].version.as_deref(), Some("4.17.15"));

        let pypi_batch = build_osv_batch_queries("pypi", &deps);
        assert_eq!(pypi_batch.queries.len(), 1);
        assert_eq!(pypi_batch.queries[0].package.name, "requests");
        assert_eq!(pypi_batch.queries[0].package.ecosystem, "PyPI");
    }

    #[test]
    fn test_parse_batch_query_response() {
        let json = r#"{
            "results": [
                {
                    "vulns": [
                        {
                            "id": "GHSA-p6mc-m468-83gw",
                            "summary": "Prototype pollution",
                            "affected": [
                                {
                                    "package": {
                                        "name": "lodash",
                                        "ecosystem": "npm"
                                    }
                                }
                            ]
                        },
                        {
                            "id": "CVE-2023-1234"
                        }
                    ]
                }
            ]
        }"#;

        let parsed = parse_batch_query_response(json);
        assert!(parsed.is_ok());
        let (full, to_hydrate) = match parsed {
            Ok(res) => res,
            Err(_) => return,
        };
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].id, "GHSA-p6mc-m468-83gw");
        assert_eq!(to_hydrate, vec!["CVE-2023-1234"]);
    }
}
