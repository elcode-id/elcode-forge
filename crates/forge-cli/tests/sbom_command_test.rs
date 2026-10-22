use std::error::Error;
use std::path::PathBuf;

use forge_cli::commands::sbom::run_sbom;
use forge_security::SbomFormat;
use serde_json::Value;

#[test]
fn test_sbom_generation_on_polyglot_fixture() -> Result<(), Box<dyn Error>> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("vulnerable-dependencies");

    // 1. CycloneDX v1.5 JSON
    let cyclonedx_out = run_sbom(&fixture_dir, SbomFormat::CycloneDx)?;
    let cdx_json: Value = serde_json::from_str(&cyclonedx_out)?;

    assert_eq!(cdx_json["bomFormat"], "CycloneDX");
    assert_eq!(cdx_json["specVersion"], "1.5");
    assert!(cdx_json["serialNumber"]
        .as_str()
        .unwrap_or("")
        .starts_with("urn:uuid:"));
    assert_eq!(
        cdx_json["metadata"]["component"]["name"],
        "vulnerable-dependencies-fixture"
    );
    assert_eq!(cdx_json["metadata"]["tools"][0]["name"], "eforge");

    let Some(components) = cdx_json["components"].as_array() else {
        return Err("CycloneDX components must be an array".into());
    };
    assert!(!components.is_empty());

    // Verify PURLs across ecosystems
    let purls: Vec<&str> = components
        .iter()
        .filter_map(|c| c["purl"].as_str())
        .collect();

    assert!(purls.iter().any(|p| p.starts_with("pkg:npm/lodash@")));
    assert!(purls.iter().any(|p| p.starts_with("pkg:pypi/requests@")));
    assert!(purls
        .iter()
        .any(|p| p.starts_with("pkg:golang/github.com/gin-gonic/gin@")));
    assert!(purls
        .iter()
        .any(|p| p.starts_with("pkg:composer/guzzlehttp/guzzle@")));

    // 2. SPDX v2.3 JSON
    let spdx_out = run_sbom(&fixture_dir, SbomFormat::Spdx)?;
    let spdx_json: Value = serde_json::from_str(&spdx_out)?;

    assert_eq!(spdx_json["spdxVersion"], "SPDX-2.3");
    assert_eq!(spdx_json["dataLicense"], "CC0-1.0");
    assert_eq!(spdx_json["name"], "vulnerable-dependencies-fixture");
    assert!(spdx_json["documentNamespace"]
        .as_str()
        .unwrap_or("")
        .starts_with("https://spdx.org/spdxdocs/"));

    let Some(packages) = spdx_json["packages"].as_array() else {
        return Err("SPDX packages must be an array".into());
    };
    // Root package + dependencies
    assert!(packages.len() >= 5);
    assert_eq!(packages[0]["SPDXID"], "SPDXRef-RootPackage");
    assert_eq!(packages[0]["name"], "vulnerable-dependencies-fixture");

    let Some(relationships) = spdx_json["relationships"].as_array() else {
        return Err("SPDX relationships must be an array".into());
    };
    assert!(relationships
        .iter()
        .any(|r| r["relationshipType"] == "DESCRIBES"
            && r["relatedSpdxElement"] == "SPDXRef-RootPackage"));
    assert!(relationships
        .iter()
        .any(|r| r["relationshipType"] == "DEPENDS_ON"));

    Ok(())
}
