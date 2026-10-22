//! Software Bill of Materials (SBOM) generation module.
//! Supports CycloneDX v1.5 and SPDX v2.3 JSON formats with canonical PURL package references.

pub mod cyclonedx;
pub mod purl;
pub mod spdx;

pub use cyclonedx::generate_cyclonedx_sbom;
pub use purl::generate_purl;
pub use spdx::generate_spdx_sbom;

use forge_core::{Dependency, ForgeError};
use serde::{Deserialize, Serialize};

/// Supported SBOM export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SbomFormat {
    /// OASIS / OWASP CycloneDX v1.5 JSON
    CycloneDx,
    /// ISO/IEC 5962:2021 SPDX v2.3 JSON
    Spdx,
}

/// Dispatches SBOM generation for the specified [`SbomFormat`].
pub fn generate_sbom(
    format: SbomFormat,
    project_name: &str,
    project_version: Option<&str>,
    dependencies: &[Dependency],
) -> Result<String, ForgeError> {
    match format {
        SbomFormat::CycloneDx => {
            generate_cyclonedx_sbom(project_name, project_version, dependencies)
        }
        SbomFormat::Spdx => generate_spdx_sbom(project_name, project_version, dependencies),
    }
}
