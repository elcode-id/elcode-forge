//! Open-source license categorization according to standard SPDX definitions.

use serde::{Deserialize, Serialize};

/// High-level legal categorization of software licenses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseCategory {
    /// Permissive licenses with minimal restrictions (e.g. MIT, Apache-2.0, BSD).
    Permissive,
    /// Weak copyleft licenses requiring source disclosure for modified files only (e.g. LGPL, MPL).
    WeakCopyleft,
    /// Strong copyleft licenses requiring derivative works to be open-sourced under the same terms (e.g. GPL, AGPL).
    StrongCopyleft,
    /// Proprietary, commercial, or explicitly unlicensed software.
    Proprietary,
    /// Unknown, unrecognized, or unspecified license terms.
    Unknown,
}

impl std::fmt::Display for LicenseCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permissive => write!(f, "Permissive"),
            Self::WeakCopyleft => write!(f, "WeakCopyleft"),
            Self::StrongCopyleft => write!(f, "StrongCopyleft"),
            Self::Proprietary => write!(f, "Proprietary"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Categorizes a raw SPDX license identifier or expression into a [`LicenseCategory`].
pub fn categorize_license(raw: &str) -> LicenseCategory {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return LicenseCategory::Unknown;
    }

    // Check for multi-license expressions like "MIT OR Apache-2.0"
    if trimmed.contains(" OR ") {
        // If any option is Permissive, developers can choose the permissive option
        let parts = trimmed.split(" OR ");
        let categories: Vec<LicenseCategory> = parts.map(categorize_license).collect();
        if categories.contains(&LicenseCategory::Permissive) {
            return LicenseCategory::Permissive;
        }
        if categories.contains(&LicenseCategory::WeakCopyleft) {
            return LicenseCategory::WeakCopyleft;
        }
    }

    if trimmed.contains(" AND ") {
        // Under AND, the most restrictive license governs compliance
        let parts = trimmed.split(" AND ");
        let mut most_restrictive = LicenseCategory::Permissive;
        for part in parts {
            let cat = categorize_license(part);
            if cat > most_restrictive {
                most_restrictive = cat;
            }
        }
        return most_restrictive;
    }

    let upper = trimmed.to_ascii_uppercase();

    // 1. Weak Copyleft (must check before GPL because LGPL contains "GPL")
    if upper.starts_with("LGPL")
        || upper.contains("LGPL")
        || upper.starts_with("MPL")
        || upper.starts_with("EPL")
        || upper.starts_with("CDDL")
    {
        return LicenseCategory::WeakCopyleft;
    }

    // 2. Strong Copyleft
    if upper.starts_with("GPL")
        || upper.starts_with("AGPL")
        || upper.contains("GPL-2")
        || upper.contains("GPL-3")
        || upper.contains("AGPL-3")
        || upper.starts_with("OSL")
        || upper.starts_with("SSPL")
        || upper.starts_with("EUPL")
    {
        return LicenseCategory::StrongCopyleft;
    }

    // 3. Permissive
    if upper == "MIT"
        || upper.starts_with("APACHE")
        || upper.starts_with("BSD")
        || upper == "ISC"
        || upper == "UNLICENSE"
        || upper.starts_with("CC0")
        || upper == "0BSD"
        || upper == "ZLIB"
        || upper == "WTFPL"
        || upper.starts_with("BSL")
        || upper == "BOOST"
    {
        return LicenseCategory::Permissive;
    }

    // 4. Proprietary
    if upper == "PROPRIETARY"
        || upper == "UNLICENSED"
        || upper.contains("ALL-RIGHTS-RESERVED")
        || upper == "COMMERCIAL"
    {
        return LicenseCategory::Proprietary;
    }

    LicenseCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_standard_spdx_licenses() {
        assert_eq!(categorize_license("MIT"), LicenseCategory::Permissive);
        assert_eq!(
            categorize_license("Apache-2.0"),
            LicenseCategory::Permissive
        );
        assert_eq!(
            categorize_license("BSD-3-Clause"),
            LicenseCategory::Permissive
        );
        assert_eq!(categorize_license("ISC"), LicenseCategory::Permissive);
        assert_eq!(categorize_license("CC0-1.0"), LicenseCategory::Permissive);

        assert_eq!(
            categorize_license("LGPL-2.1"),
            LicenseCategory::WeakCopyleft
        );
        assert_eq!(
            categorize_license("LGPL-3.0"),
            LicenseCategory::WeakCopyleft
        );
        assert_eq!(categorize_license("MPL-2.0"), LicenseCategory::WeakCopyleft);

        assert_eq!(
            categorize_license("GPL-2.0"),
            LicenseCategory::StrongCopyleft
        );
        assert_eq!(
            categorize_license("GPL-3.0"),
            LicenseCategory::StrongCopyleft
        );
        assert_eq!(
            categorize_license("AGPL-3.0"),
            LicenseCategory::StrongCopyleft
        );

        assert_eq!(
            categorize_license("UNLICENSED"),
            LicenseCategory::Proprietary
        );
        assert_eq!(
            categorize_license("Proprietary"),
            LicenseCategory::Proprietary
        );

        assert_eq!(
            categorize_license("Custom-Non-Standard"),
            LicenseCategory::Unknown
        );
    }

    #[test]
    fn handles_spdx_expressions() {
        assert_eq!(
            categorize_license("MIT OR Apache-2.0"),
            LicenseCategory::Permissive
        );
        assert_eq!(
            categorize_license("MIT OR GPL-3.0"),
            LicenseCategory::Permissive
        );
        assert_eq!(
            categorize_license("MIT AND GPL-3.0"),
            LicenseCategory::StrongCopyleft
        );
    }
}
