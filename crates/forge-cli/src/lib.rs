//! `forge-cli` — Shared CLI execution library for `eforge` and `elcode-forge`.

/// Returns the formatted version banner for the CLI.
pub fn version_banner() -> String {
    format!("ELCODE Forge (eforge) v{}", env!("CARGO_PKG_VERSION"))
}
