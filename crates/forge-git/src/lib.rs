//! `forge-git` — Git history, unified diff parser, hotspot, blast-radius impact analysis, and hooks.

pub mod diff;
pub mod hooks;
pub mod hotspot;
pub mod impact;
pub mod log;

pub use diff::{collect_working_tree_diff, parse_unified_diff, FileChange};
pub use forge_core::Analyzer;
pub use hooks::{
    get_hooks_status, install_hooks, uninstall_hooks, HookEntryStatus, HookInstallResult,
    HookStatusReport, HookType, HookUninstallResult,
};
pub use hotspot::{compute_hotspots_from_log, HotspotEntry};
pub use impact::{compute_change_impact, ChangeImpactReport};
pub use log::{collect_commit_log, read_head_revision, GitRevision};
