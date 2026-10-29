//! Role-based access control (RBAC) definitions and permission checks.

use serde::{Deserialize, Serialize};

/// System authorization roles ordered by ascending privilege level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only access to repository findings, metrics, and trends.
    Viewer = 1,
    /// Access to compliance reports, tech debt metrics, and audit event logs.
    Auditor = 2,
    /// Permission to ingest snapshots, update baselines, and manage projects.
    Maintainer = 3,
    /// Full permissions including organization configuration, API keys, and webhooks.
    Admin = 4,
}

impl Role {
    /// Parses a role from a string safely, defaulting to `Viewer` on unrecognized input.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "admin" | "administrator" | "root" => Self::Admin,
            "maintainer" | "developer" | "engineer" | "ci" | "agent" | "api_client" => {
                Self::Maintainer
            }
            "auditor" | "security" | "compliance" => Self::Auditor,
            _ => Self::Viewer,
        }
    }

    /// Whether this role can ingest repository snapshots.
    pub fn can_write_snapshots(&self) -> bool {
        matches!(self, Self::Maintainer | Self::Admin)
    }

    /// Whether this role can manage organizations, webhooks, and API keys.
    pub fn can_manage_orgs(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// Whether this role can view audit event records.
    pub fn can_read_audit(&self) -> bool {
        matches!(self, Self::Auditor | Self::Admin)
    }

    /// Whether this role can view repositories, findings, and trends.
    pub fn can_read_findings(&self) -> bool {
        true
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Viewer => write!(f, "viewer"),
            Self::Auditor => write!(f, "auditor"),
            Self::Maintainer => write!(f, "maintainer"),
            Self::Admin => write!(f, "admin"),
        }
    }
}
