//! Deterministic human-readable terminal formatter for [`Repository`] reports.

use forge_core::Repository;

/// Renders a human-readable terminal report for a [`Repository`].
pub fn render_terminal(repo: &Repository) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str(&format!(" ELCODE Forge Report — {}\n", repo.name));
    out.push_str("============================================================\n");
    out.push_str(&format!("Root Path:         {}\n", repo.root_path));
    out.push_str(&format!(
        "Languages:         {}\n",
        if repo.detected_languages.is_empty() {
            "none".to_string()
        } else {
            repo.detected_languages.join(", ")
        }
    ));
    out.push_str(&format!(
        "Frameworks:        {}\n",
        if repo.detected_frameworks.is_empty() {
            "none".to_string()
        } else {
            repo.detected_frameworks.join(", ")
        }
    ));
    out.push_str(&format!(
        "Package Managers:  {}\n",
        if repo.package_managers.is_empty() {
            "none".to_string()
        } else {
            repo.package_managers.join(", ")
        }
    ));
    out.push_str(&format!(
        "Inventory:         {} file(s), {} module(s), {} dependency(ies)\n\n",
        repo.files.len(),
        repo.modules.len(),
        repo.dependencies.len()
    ));

    if let Some(health) = &repo.health {
        out.push_str(&format!("Overall Health Score: {}/100\n", health.overall));
        if !health.dimensions.is_empty() {
            out.push_str("Dimensions:\n");
            for dim in &health.dimensions {
                out.push_str(&format!(
                    "  - {:<20} {:>3}/100 (weight {:>2}%)\n",
                    dim.dimension, dim.score, dim.weight_pct
                ));
            }
            out.push('\n');
        }
    }

    out.push_str(&format!("Findings ({}):\n", repo.findings.len()));
    if repo.findings.is_empty() {
        out.push_str("  No findings detected.\n");
    } else {
        for finding in &repo.findings {
            let loc = finding
                .location
                .as_ref()
                .map(|l| match l.start_line {
                    Some(line) => format!("{}:{}", l.file_path, line),
                    None => l.file_path.clone(),
                })
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "  [{:?}] {} ({}) at {}\n    {}\n    Evidence: {}\n",
                finding.severity,
                finding.title,
                finding.rule_id,
                loc,
                finding.description,
                finding.evidence
            ));
        }
    }

    out
}
