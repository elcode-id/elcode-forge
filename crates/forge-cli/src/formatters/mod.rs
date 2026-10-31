//! Output formatters (`terminal`, `json`, `markdown`, `sarif`, `github`) for `eforge`.

pub mod codeframe;
pub mod json;
pub mod markdown;
pub mod sarif;
pub mod terminal;

pub use crate::cli::OutputFormat;
pub use sarif::render_sarif;

use forge_core::{ForgeError, Repository};

/// Renders a [`Repository`] report into the requested [`OutputFormat`].
pub fn render_repository_report(
    repo: &Repository,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Terminal => Ok(terminal::render_terminal(repo)),
        OutputFormat::Json => json::render_json(repo),
        OutputFormat::Markdown => Ok(markdown::render_markdown(repo)),
        OutputFormat::Sarif => sarif::render_sarif(repo),
        OutputFormat::Github => Ok(markdown::render_github_annotations(repo)),
    }
}

#[cfg(test)]
mod tests {
    use forge_core::{
        Confidence, DimensionScore, Finding, HealthScore, Repository, Severity, SourceLocation,
    };

    use super::{render_repository_report, OutputFormat};
    use crate::exit_code::ExitCode;

    fn sample_repo() -> Repository {
        Repository {
            name: "fixture-next-app".to_string(),
            root_path: "/repo/fixture-next-app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec!["nextjs".to_string(), "react".to_string()],
            package_managers: vec!["npm".to_string()],
            files: Vec::new(),
            modules: Vec::new(),
            dependencies: Vec::new(),
            findings: vec![Finding {
                id: "F-001".to_string(),
                rule_id: "arch/no-direct-db-in-ui".to_string(),
                title: "UI imports DB directly".to_string(),
                description: "Component imports database client directly.".to_string(),
                severity: Severity::High,
                confidence: Confidence::Certain,
                category: "architecture".to_string(),
                location: Some(SourceLocation {
                    file_path: "src/components/UserCard.tsx".to_string(),
                    start_line: Some(1),
                    end_line: Some(1),
                }),
                evidence: "src/components -> src/db".to_string(),
                recommendation: "Route data fetching through src/services.".to_string(),
            }],
            health: Some(HealthScore {
                overall: 85,
                dimensions: vec![DimensionScore {
                    dimension: "architecture".to_string(),
                    score: 85,
                    weight_pct: 18,
                    reasons: vec!["1 architecture boundary violation".to_string()],
                }],
            }),
        }
    }

    #[test]
    fn renders_all_output_formats_deterministically() {
        let repo = sample_repo();

        let term_res = render_repository_report(&repo, OutputFormat::Terminal);
        assert!(term_res.is_ok());
        if let Ok(text) = term_res {
            assert!(text.contains("ELCODE Forge Report — fixture-next-app"));
            assert!(text.contains("arch/no-direct-db-in-ui"));
        }

        let json_res = render_repository_report(&repo, OutputFormat::Json);
        assert!(json_res.is_ok());
        if let Ok(json_text) = json_res {
            assert!(json_text.contains("\"name\": \"fixture-next-app\""));
        }

        let md_res = render_repository_report(&repo, OutputFormat::Markdown);
        assert!(md_res.is_ok());
        if let Ok(md_text) = md_res {
            assert!(md_text.contains("# ELCODE Forge Report — `fixture-next-app`"));
            assert!(md_text.contains("| **High** | `arch/no-direct-db-in-ui` |"));
        }

        let gh_res = render_repository_report(&repo, OutputFormat::Github);
        assert!(gh_res.is_ok());
        if let Ok(gh_text) = gh_res {
            assert!(gh_text.contains(
                "::error file=src/components/UserCard.tsx,line=1,title=arch/no-direct-db-in-ui::"
            ));
        }

        let sarif_res = render_repository_report(&repo, OutputFormat::Sarif);
        assert!(sarif_res.is_ok());
        if let Ok(sarif_text) = sarif_res {
            assert!(sarif_text.contains("\"version\": \"2.1.0\""));
            assert!(sarif_text.contains("\"ruleId\": \"arch/no-direct-db-in-ui\""));
            assert!(sarif_text.contains("\"level\": \"error\""));
        }
    }

    #[test]
    fn evaluates_exit_codes_for_general_and_security_findings() {
        let repo = sample_repo();
        assert_eq!(
            ExitCode::evaluate_findings(&repo.findings, Some(Severity::High)),
            ExitCode::FindingsAboveThreshold
        );
        assert_eq!(
            ExitCode::evaluate_findings(&repo.findings, Some(Severity::Critical)),
            ExitCode::Success
        );

        let sec_finding = Finding {
            id: "SEC-001".to_string(),
            rule_id: "security/hardcoded-secret".to_string(),
            title: "Secret detected".to_string(),
            description: "Hardcoded secret".to_string(),
            severity: Severity::Critical,
            confidence: Confidence::Certain,
            category: "security".to_string(),
            location: None,
            evidence: "****REDACTED****".to_string(),
            recommendation: "Use env vars".to_string(),
        };
        assert_eq!(
            ExitCode::evaluate_findings(&[sec_finding], Some(Severity::High)),
            ExitCode::SecurityPolicyFailure
        );
    }
}
