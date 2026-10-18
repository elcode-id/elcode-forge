//! GitHub-flavored Markdown and GitHub Actions workflow formatter for [`Repository`] reports.

use forge_core::{Repository, Severity};

/// Renders a GitHub-flavored Markdown report for a [`Repository`].
pub fn render_markdown(repo: &Repository) -> String {
    let mut md = String::new();
    md.push_str(&format!("# ELCODE Forge Report — `{}`\n\n", repo.name));
    md.push_str(&format!(
        "- **Languages:** `{}`\n- **Frameworks:** `{}`\n- **Package Managers:** `{}`\n",
        repo.detected_languages.join(", "),
        repo.detected_frameworks.join(", "),
        repo.package_managers.join(", "),
    ));
    if let Some(health) = &repo.health {
        md.push_str(&format!(
            "- **Overall Health Score:** **{}/100**\n\n",
            health.overall
        ));
        if !health.dimensions.is_empty() {
            md.push_str("## Health Dimensions\n\n");
            md.push_str("| Dimension | Score | Weight |\n| :--- | :---: | :---: |\n");
            for dim in &health.dimensions {
                md.push_str(&format!(
                    "| `{}` | {}/100 | {}% |\n",
                    dim.dimension, dim.score, dim.weight_pct
                ));
            }
            md.push('\n');
        }
    } else {
        md.push('\n');
    }

    md.push_str(&format!("## Findings ({})\n\n", repo.findings.len()));
    if repo.findings.is_empty() {
        md.push_str("_No findings detected._\n");
    } else {
        md.push_str("| Severity | Rule ID | Location | Title |\n| :--- | :--- | :--- | :--- |\n");
        for finding in &repo.findings {
            let loc = finding
                .location
                .as_ref()
                .map(|l| match l.start_line {
                    Some(line) => format!("`{}:{}`", l.file_path, line),
                    None => format!("`{}`", l.file_path),
                })
                .unwrap_or_else(|| "`-`".to_string());
            md.push_str(&format!(
                "| **{:?}** | `{}` | {} | {} |\n",
                finding.severity, finding.rule_id, loc, finding.title
            ));
        }
    }

    md
}

/// Renders GitHub Actions workflow annotations (`::error`, `::warning`, `::notice`) followed by the Markdown report.
pub fn render_github_annotations(repo: &Repository) -> String {
    let mut out = String::new();
    for finding in &repo.findings {
        let level = match finding.severity {
            Severity::Critical | Severity::High => "error",
            Severity::Medium | Severity::Low => "warning",
            Severity::Info => "notice",
        };
        if let Some(loc) = &finding.location {
            let line = loc.start_line.unwrap_or(1);
            out.push_str(&format!(
                "::{level} file={},line={line},title={}::{} ({})\n",
                loc.file_path, finding.rule_id, finding.title, finding.description
            ));
        } else {
            out.push_str(&format!(
                "::{level} title={}::{} ({})\n",
                finding.rule_id, finding.title, finding.description
            ));
        }
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&render_markdown(repo));
    out
}
