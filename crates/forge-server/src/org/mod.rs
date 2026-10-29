//! Organization management, multi-repository aggregation, and materialized rollups.

pub mod aggregator;

pub use aggregator::{
    compute_organization_rollup, get_organization_summary, recompute_all_orgs_for_project,
    recompute_org_rollup,
};
