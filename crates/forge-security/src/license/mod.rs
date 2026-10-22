//! Open-source license detection and policy compliance evaluation engine.

pub mod category;
pub mod checker;
pub mod detector;

pub use category::{categorize_license, LicenseCategory};
pub use checker::{
    evaluate_license_compliance, RULE_DENIED_LICENSE, RULE_DISALLOWED_COPYLEFT,
    RULE_MISSING_PROJECT_LICENSE, RULE_UNAPPROVED_LICENSE,
};
pub use detector::{detect_dependency_license, detect_project_license, ProjectLicenseInfo};
