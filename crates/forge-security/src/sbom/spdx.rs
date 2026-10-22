//! SPDX v2.3 JSON Software Bill of Materials (SBOM) generator.

use forge_core::{Dependency, ForgeError};
use serde_json::json;

use super::purl::generate_purl;

/// Generates an ISO/IEC 5962:2021 SPDX v2.3 JSON document for `project_name` and `dependencies`.
pub fn generate_spdx_sbom(
    project_name: &str,
    project_version: Option<&str>,
    dependencies: &[Dependency],
) -> Result<String, ForgeError> {
    let mut sorted_deps = dependencies.to_vec();
    sorted_deps.sort_by(|a, b| {
        a.ecosystem
            .cmp(&b.ecosystem)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.version_req.cmp(&b.version_req))
    });

    let mut hasher = blake3::Hasher::new();
    hasher.update(project_name.as_bytes());
    for dep in &sorted_deps {
        hasher.update(dep.name.as_bytes());
        hasher.update(dep.version_req.as_bytes());
    }
    let hash_hex = hasher.finalize().to_hex();
    let doc_namespace = format!(
        "https://spdx.org/spdxdocs/{project_name}-{}",
        &hash_hex[0..16]
    );

    let mut packages = Vec::with_capacity(sorted_deps.len() + 1);
    let mut relationships = Vec::with_capacity(sorted_deps.len() + 1);

    // Root project package
    packages.push(json!({
        "SPDXID": "SPDXRef-RootPackage",
        "name": project_name,
        "versionInfo": project_version.unwrap_or("1.0.0"),
        "downloadLocation": "NOASSERTION",
        "filesAnalyzed": false,
        "primaryPackagePurpose": "APPLICATION"
    }));

    relationships.push(json!({
        "spdxElementId": "SPDXRef-DOCUMENT",
        "relationshipType": "DESCRIBES",
        "relatedSpdxElement": "SPDXRef-RootPackage"
    }));

    for (idx, dep) in sorted_deps.iter().enumerate() {
        let purl = generate_purl(dep);
        let version_str = dep
            .resolved_version
            .as_deref()
            .unwrap_or(dep.version_req.as_str());
        let clean_version = version_str.trim_start_matches(['=', '^', '~', '>', '<', ' ']);
        let spdx_id = format!("SPDXRef-Package-{}", idx + 1);

        let pkg = json!({
            "SPDXID": spdx_id,
            "name": dep.name,
            "versionInfo": clean_version,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": false,
            "primaryPackagePurpose": "LIBRARY",
            "externalRefs": [
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": purl
                }
            ]
        });

        relationships.push(json!({
            "spdxElementId": "SPDXRef-RootPackage",
            "relationshipType": "DEPENDS_ON",
            "relatedSpdxElement": spdx_id
        }));

        packages.push(pkg);
    }

    let doc = json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": project_name,
        "documentNamespace": doc_namespace,
        "creationInfo": {
            "creators": [
                format!("Tool: elcode-forge-{}", env!("CARGO_PKG_VERSION")),
                "Organization: ELCODE".to_string()
            ],
            "created": "2024-01-01T00:00:00Z"
        },
        "packages": packages,
        "relationships": relationships
    });

    serde_json::to_string_pretty(&doc).map_err(|e| ForgeError::Serialization {
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn generates_valid_spdx_structure() -> Result<(), Box<dyn std::error::Error>> {
        let deps = vec![Dependency {
            name: "lodash".to_string(),
            version_req: "4.17.15".to_string(),
            resolved_version: Some("4.17.15".to_string()),
            ecosystem: "npm".to_string(),
            is_dev: false,
            is_direct: true,
        }];

        let sbom = generate_spdx_sbom("my-app", Some("2.0.0"), &deps)?;
        let parsed: Value = serde_json::from_str(&sbom)?;

        assert_eq!(parsed["spdxVersion"], "SPDX-2.3");
        assert_eq!(parsed["dataLicense"], "CC0-1.0");
        assert_eq!(parsed["name"], "my-app");

        let Some(pkgs) = parsed["packages"].as_array() else {
            return Err("Packages is not an array".into());
        };
        assert_eq!(pkgs.len(), 2); // Root + lodash
        assert_eq!(pkgs[0]["name"], "my-app");
        assert_eq!(pkgs[1]["name"], "lodash");
        assert_eq!(pkgs[1]["versionInfo"], "4.17.15");

        let Some(rels) = parsed["relationships"].as_array() else {
            return Err("Relationships is not an array".into());
        };
        assert_eq!(rels.len(), 2);
        Ok(())
    }
}
