//! `forge-core` — Foundational Intermediate Representation (IR), error model, configuration
//! loader, canonical path guard, `.eforge/` incremental cache, explainable scoring engine,
//! remote synchronization protocol, and core traits for ELCODE Forge (`eforge`).
//!
//! This crate is the leaf of the workspace dependency DAG and MUST NOT depend on any other
//! internal `forge-*` crate.

pub mod cache;
pub mod config;
pub mod error;
pub mod ir;
pub mod path_guard;
pub mod scoring;
pub mod sync;
pub mod traits;
pub mod untrusted;

pub use cache::{CacheMetadata, ForgeCache, EFORGE_DIR};
pub use config::{
    load_config, parse_config_yaml, AiConfig, AiProviderKind, ForgeConfig, LicenseConfig,
    RuleOverrideConfig, SecurityConfig, FALLBACK_CONFIG_FILE, PRIMARY_CONFIG_FILE,
};
pub use error::ForgeError;
pub use ir::{
    Confidence, Dependency, DimensionScore, FileNode, Finding, HealthScore, Module, Repository,
    Severity, SourceLocation,
};
pub use path_guard::resolve_safe_path;
pub use scoring::{calculate_change_risk, calculate_health_score, RiskLevel, RiskReport};
pub use sync::{
    compute_sync_content_hash, default_credentials_path, home_dir, load_credentials,
    load_credentials_from_path, resolve_token, save_credentials, save_credentials_to_path,
    CredentialsConfig, DeltaSummary, ServerCredential, SyncManifest, SyncPullResponse,
    SyncPushPayload, SyncStatus,
};
pub use traits::Analyzer;
pub use untrusted::{
    detect_prompt_injection_signals, wrap_untrusted_payload, UntrustedEnvelope,
    UNTRUSTED_REPO_DATA_LEVEL, UNTRUSTED_SECURITY_NOTICE,
};
