//! `forge-git` — Git history, unified diff parser, hotspot, and blast-radius impact analysis.

pub mod diff;
pub mod hotspot;
pub mod impact;

pub use diff::{collect_working_tree_diff, parse_unified_diff, FileChange};
pub use forge_core::Analyzer;
pub use hotspot::{compute_hotspots_from_log, HotspotEntry};
pub use impact::{compute_change_impact, ChangeImpactReport};
