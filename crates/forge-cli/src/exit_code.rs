//! Deterministic CLI exit codes (`0..=4`) for CI/CD policy gates.

use forge_core::{Finding, Severity};

/// Exit code returned by `eforge` commands.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Execution succeeded with no findings violating thresholds (`0`).
    Success = 0,
    /// Architecture or quality findings met or exceeded the configured `--fail-on` threshold (`1`).
    FindingsAboveThreshold = 1,
    /// Invalid CLI arguments or malformed `eforge.config.yaml` (`2`).
    ConfigurationError = 2,
    /// Unexpected I/O, Git, or runtime error (`3`).
    ExecutionError = 3,
    /// Security policy failure (e.g., critical secret or vulnerability exceeding threshold) (`4`).
    SecurityPolicyFailure = 4,
}

impl ExitCode {
    /// Returns the numeric process exit code (`0..=4`).
    pub const fn code(self) -> i32 {
        self as i32
    }

    /// Evaluates a list of findings against an optional minimum failure severity threshold.
    pub fn evaluate_findings(findings: &[Finding], fail_on: Option<Severity>) -> Self {
        let Some(threshold) = fail_on else {
            return Self::Success;
        };

        let mut has_security_violation = false;
        let mut has_general_violation = false;

        for finding in findings {
            if finding.severity >= threshold {
                if finding.category == "security" || finding.rule_id.starts_with("security/") {
                    has_security_violation = true;
                } else {
                    has_general_violation = true;
                }
            }
        }

        if has_security_violation {
            Self::SecurityPolicyFailure
        } else if has_general_violation {
            Self::FindingsAboveThreshold
        } else {
            Self::Success
        }
    }
}

/// Parses a `--fail-on` severity string (`critical|high|medium|low|info`) into [`Severity`].
pub fn parse_severity_threshold(raw: &str) -> Option<Severity> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        "medium" => Some(Severity::Medium),
        "low" => Some(Severity::Low),
        "info" => Some(Severity::Info),
        _ => None,
    }
}
