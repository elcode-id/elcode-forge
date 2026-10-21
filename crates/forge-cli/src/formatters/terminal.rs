//! Deterministic human-readable terminal formatter for [`Repository`] reports.

use forge_core::{Finding, Repository};

use crate::theme::Theme;

/// Formats a finding into a structured terminal card with severity badge, location, and snippet evidence.
pub fn render_finding_card(finding: &Finding, theme: &Theme) -> String {
    let mut out = String::new();
    let badge = theme.badge_severity(finding.severity);
    out.push_str(&format!("  {} {}\n", badge, theme.bold(&finding.rule_id)));
    out.push_str(&format!("  {}\n", theme.bold(&finding.title)));

    let loc = finding
        .location
        .as_ref()
        .map(|l| match l.start_line {
            Some(line) => format!("{}:{}", l.file_path, line),
            None => l.file_path.clone(),
        })
        .unwrap_or_else(|| "-".to_string());
    out.push_str(&format!("  Location: {}\n", theme.cyan(&loc)));

    if !finding.description.is_empty() {
        out.push_str(&format!("  {}\n", finding.description));
    }
    if !finding.recommendation.is_empty() {
        out.push_str(&format!("  Recommendation: {}\n", finding.recommendation));
    }
    if !finding.evidence.is_empty() {
        out.push_str("  Evidence:\n");
        let border = if theme.unicode_enabled { "│" } else { "|" };
        for line in finding.evidence.lines() {
            out.push_str(&format!("    {} {}\n", border, line));
        }
    }
    out
}

/// Renders a human-readable terminal report for a [`Repository`] using the default detected theme.
pub fn render_terminal(repo: &Repository) -> String {
    render_terminal_with_theme(repo, &Theme::default())
}

/// Renders a human-readable terminal report for a [`Repository`] with an explicit [`Theme`].
pub fn render_terminal_with_theme(repo: &Repository, theme: &Theme) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str(&format!(
        " {}\n",
        theme.bold(&format!("ELCODE Forge Report — {}", repo.name))
    ));
    out.push_str("============================================================\n");
    out.push_str(&format!("Root Path:         {}\n", repo.root_path));
    out.push_str(&format!(
        "Inventory:         {} file(s), {} module(s), {} dependency(ies)\n\n",
        repo.files.len(),
        repo.modules.len(),
        repo.dependencies.len()
    ));

    out.push_str("Pipeline Progress:\n");
    let langs = if repo.detected_languages.is_empty() {
        "none".to_string()
    } else {
        repo.detected_languages.join(", ")
    };
    let frameworks = if repo.detected_frameworks.is_empty() {
        String::new()
    } else {
        format!(" ({})", repo.detected_frameworks.join(", "))
    };
    out.push_str(&format!(
        "  {}\n",
        theme.status_ok(&format!("Languages:     {langs}{frameworks}"))
    ));

    let pkg_mgrs = if repo.package_managers.is_empty() {
        "none".to_string()
    } else {
        repo.package_managers.join(", ")
    };
    let deps_desc = format!("{} packages analyzed ({pkg_mgrs})", repo.dependencies.len());
    out.push_str(&format!(
        "  {}\n",
        theme.status_ok(&format!("Dependencies:  {deps_desc}"))
    ));

    let arch_desc = format!(
        "{} file(s), {} module(s) mapped",
        repo.files.len(),
        repo.modules.len()
    );
    out.push_str(&format!(
        "  {}\n",
        theme.status_ok(&format!("Architecture:  {arch_desc}"))
    ));

    let rules_desc = format!("{} finding(s) evaluated", repo.findings.len());
    out.push_str(&format!(
        "  {}\n\n",
        theme.status_ok(&format!("Rules:         {rules_desc}"))
    ));

    if let Some(health) = &repo.health {
        out.push_str(&format!(
            "Overall Health: {}\n\n",
            theme.render_score_meter(health.overall)
        ));
        if !health.dimensions.is_empty() {
            out.push_str("Dimension Breakdown:\n");
            if theme.unicode_enabled {
                out.push_str("  ┌──────────────────────┬───────┬────────┐\n");
                out.push_str("  │ Dimension            │ Score │ Weight │\n");
                out.push_str("  ├──────────────────────┼───────┼────────┤\n");
                for dim in &health.dimensions {
                    out.push_str(&format!(
                        "  │ {:<20} │  {:>3}  │  {:>3}%  │\n",
                        dim.dimension, dim.score, dim.weight_pct
                    ));
                }
                out.push_str("  └──────────────────────┴───────┴────────┘\n\n");
            } else {
                out.push_str("  +----------------------+-------+--------+\n");
                out.push_str("  | Dimension            | Score | Weight |\n");
                out.push_str("  +----------------------+-------+--------+\n");
                for dim in &health.dimensions {
                    out.push_str(&format!(
                        "  | {:<20} |  {:>3}  |  {:>3}%  |\n",
                        dim.dimension, dim.score, dim.weight_pct
                    ));
                }
                out.push_str("  +----------------------+-------+--------+\n\n");
            }
        }
    }

    out.push_str(&format!("Findings ({}):\n", repo.findings.len()));
    if repo.findings.is_empty() {
        out.push_str("  No findings detected.\n");
    } else {
        for finding in &repo.findings {
            out.push_str(&render_finding_card(finding, theme));
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use forge_core::{
        Confidence, DimensionScore, Finding, HealthScore, Repository, Severity, SourceLocation,
    };

    use super::*;

    fn test_repo() -> Repository {
        Repository {
            name: "test-app".to_string(),
            root_path: "/workspace/test-app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec!["nextjs".to_string()],
            package_managers: vec!["npm".to_string()],
            files: Vec::new(),
            modules: Vec::new(),
            dependencies: Vec::new(),
            findings: vec![Finding {
                id: "F-001".to_string(),
                rule_id: "arch/no-direct-db-in-ui".to_string(),
                title: "Direct DB Import in UI".to_string(),
                description: "UI component imports database client directly.".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                category: "architecture".to_string(),
                location: Some(SourceLocation {
                    file_path: "src/pages/index.tsx".to_string(),
                    start_line: Some(4),
                    end_line: Some(4),
                }),
                evidence: "import { db } from '../db/client';\nconst data = db.query();"
                    .to_string(),
                recommendation: "Delegate database queries to server actions or API routes."
                    .to_string(),
            }],
            health: Some(HealthScore {
                overall: 85,
                dimensions: vec![DimensionScore {
                    dimension: "Architecture".to_string(),
                    score: 85,
                    weight_pct: 20,
                    reasons: vec!["1 architecture violation detected".to_string()],
                }],
            }),
        }
    }

    #[test]
    fn test_render_terminal_plain() {
        let repo = test_repo();
        let plain = Theme::plain();
        let output = render_terminal_with_theme(&repo, &plain);

        // Header & inventory
        assert!(output.contains("ELCODE Forge Report — test-app"));
        assert!(output.contains("Root Path:         /workspace/test-app"));

        // Pipeline progress checklist
        assert!(output.contains("[OK] Languages:     typescript (nextjs)"));
        assert!(output.contains("[OK] Dependencies:"));
        assert!(output.contains("[OK] Architecture:"));
        assert!(output.contains("[OK] Rules:"));

        // Health meter & dimension table
        assert!(output.contains("Overall Health: [========..] 85/100 (Good)"));
        assert!(output.contains("+----------------------+-------+--------+"));
        assert!(output.contains("| Architecture         |   85  |   20%  |"));

        // Finding card
        assert!(output.contains("[HIGH] arch/no-direct-db-in-ui"));
        assert!(output.contains("Direct DB Import in UI"));
        assert!(output.contains("Location: src/pages/index.tsx:4"));
        assert!(output.contains("Evidence:"));
        assert!(output.contains("| import { db } from '../db/client';"));
        assert!(output.contains("| const data = db.query();"));

        // Zero ANSI escape codes in plain mode
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn test_render_terminal_colored() {
        let repo = test_repo();
        let colored = Theme::colored();
        let output = render_terminal_with_theme(&repo, &colored);

        // Contains ANSI escapes
        assert!(output.contains("\x1b["));
        // Unicode checkmark in progress
        assert!(output.contains("✓"));
        // Box drawing table border
        assert!(output.contains("┌──────────────────────┬───────┬────────┐"));
        // Snippet vertical border
        assert!(output.contains("│ import { db } from '../db/client';"));
        // High severity badge
        assert!(output.contains("\x1b[31m[HIGH]\x1b[0m"));
    }
}
