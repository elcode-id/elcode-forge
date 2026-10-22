//! CycloneDX v1.5 JSON Software Bill of Materials (SBOM) generator.

use forge_core::{Dependency, ForgeError};
use serde_json::json;

use super::purl::generate_purl;

/// Generates an OASIS / OWASP CycloneDX v1.5 JSON document for `project_name` and `dependencies`.
pub fn generate_cyclonedx_sbom(
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

    let mut components = Vec::with_capacity(sorted_deps.len());

    for dep in &sorted_deps {
        let purl = generate_purl(dep);
        let version_str = dep
            .resolved_version
            .as_deref()
            .unwrap_or(dep.version_req.as_str());
        let clean_version = version_str.trim_start_matches(['=', '^', '~', '>', '<', ' ']);

        let component = json!({
            "type": "library",
            "name": dep.name,
            "version": clean_version,
            "purl": purl,
            "scope": if dep.is_dev { "optional" } else { "required" },
            "properties": [
                {
                    "name": "forge:ecosystem",
                    "value": dep.ecosystem
                },
                {
                    "name": "forge:is_direct",
                    "value": dep.is_direct.to_string()
                }
            ]
        });

        components.push(component);
    }

    // Generate a deterministic serial number based on project name and sorted purls
    let mut hasher = blake3::Hasher::new();
    hasher.update(project_name.as_bytes());
    for dep in &sorted_deps {
        hasher.update(dep.name.as_bytes());
        hasher.update(dep.version_req.as_bytes());
    }
    let hash_bytes = hasher.finalize();
    let hash_hex = hash_bytes.to_hex();
    let serial_uuid = format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hash_hex[0..8],
        &hash_hex[8..12],
        &hash_hex[12..16],
        &hash_hex[16..20],
        &hash_hex[20..32]
    );

    let doc = json!({
        "$schema": "http://cyclonedx.org/schema/bom-1.5.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": serial_uuid,
        "version": 1,
        "metadata": {
            "tools": [
                {
                    "vendor": "ELCODE",
                    "name": "eforge",
                    "version": env!("CARGO_PKG_VERSION")
                }
            ],
            "component": {
                "type": "application",
                "name": project_name,
                "version": project_version.unwrap_or("1.0.0")
            }
        },
        "components": components
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
    fn generates_valid_cyclonedx_structure() -> Result<(), Box<dyn std::error::Error>> {
        let deps = vec![
            Dependency {
                name: "lodash".to_string(),
                version_req: "4.17.15".to_string(),
                resolved_version: Some("4.17.15".to_string()),
                ecosystem: "npm".to_string(),
                is_dev: false,
                is_direct: true,
            },
            Dependency {
                name: "requests".to_string(),
                version_req: "2.19.1".to_string(),
                resolved_version: Some("2.19.1".to_string()),
                ecosystem: "pypi".to_string(),
                is_dev: true,
                is_direct: true,
            },
        ];

        let sbom = generate_cyclonedx_sbom("my-app", Some("1.2.0"), &deps)?;
        let parsed: Value = serde_json::from_str(&sbom)?;

        assert_eq!(parsed["bomFormat"], "CycloneDX");
        assert_eq!(parsed["specVersion"], "1.5");
        assert_eq!(parsed["metadata"]["component"]["name"], "my-app");
        assert_eq!(parsed["metadata"]["component"]["version"], "1.2.0");

        let Some(comps) = parsed["components"].as_array() else {
            return Err("Components is not an array".into());
        };
        assert_eq!(comps.len(), 2);
        assert_eq!(comps[0]["name"], "lodash");
        assert_eq!(comps[0]["purl"], "pkg:npm/lodash@4.17.15");
        assert_eq!(comps[0]["scope"], "required");
        assert_eq!(comps[1]["name"], "requests");
        assert_eq!(comps[1]["purl"], "pkg:pypi/requests@2.19.1");
        assert_eq!(comps[1]["scope"], "optional");
        Ok(())
    }
}
