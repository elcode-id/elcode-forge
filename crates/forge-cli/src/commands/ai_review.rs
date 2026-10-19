//! `eforge ai-review` command — PR/diff review combining deterministic risk & blast radius
//! with grounded AI review items classified as `Confirmed | Probable | Suggestion`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use forge_ai::{
    append_usage, build_context, build_provider, compute_cost_usd, enforce_daily_budget,
    ensure_ai_enabled, estimate_tokens, preflight_check, summarize_usage, AiMessage, AiProvider,
    AiRequest, AiTask, CodeSnippet, ContextInput, ContextPack, MockProvider, ResponseCache,
    SectionKind, TokenUsage, UsageRecord,
};
use forge_core::config::{load_config, AiProviderKind};
use forge_core::error::ForgeError;
use forge_security::redact_secrets;
use serde::{Deserialize, Serialize};

use crate::ai::orchestrate::{current_utc_date, is_sensitive_path};
use crate::cli::OutputFormat;
use crate::commands::review::{execute_review, render_review_report, ReviewReport};
use crate::exit_code::{parse_severity_threshold, ExitCode};

/// Grounded classification level of an AI-generated code review insight (PRD §21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiClassification {
    /// High-confidence issue verified against an existing deterministic finding.
    Confirmed,
    /// Plausible behavioral, performance, or edge-case defect inferred from diff context.
    Probable,
    /// Stylistic, maintainability, documentation, or non-blocking design recommendation.
    Suggestion,
}

/// A structured code review item produced by the AI model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiReviewItem {
    /// Grounded classification level.
    pub classification: AiClassification,
    /// Concise summary of the review feedback.
    pub title: String,
    /// Detailed rationale explaining why this issue or improvement matters.
    pub rationale: String,
    /// File path referenced by this item, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Specific line number referenced, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// ID of the deterministic finding backing this item, if `Confirmed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_finding_id: Option<String>,
}

/// Complete report emitted by `eforge ai-review`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiReviewReport {
    /// Deterministic change risk assessment, blast radius, and rule findings.
    pub deterministic: ReviewReport,
    /// Grounded and guarded AI review insights.
    pub ai_items: Vec<AiReviewItem>,
    /// Name of the AI provider used.
    pub provider: String,
    /// Model used for completion.
    pub model: String,
    /// Token accounting metrics.
    pub usage: TokenUsage,
    /// Whether the AI response was served from cache.
    pub cached: bool,
    /// Optional dry run context pack inspection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<ContextPack>,
}

/// CLI execution options for `eforge ai-review`.
#[derive(Debug, Clone, Default)]
pub struct AiReviewOptions {
    /// Run fast diff-scoped mode.
    pub fast: bool,
    /// Base Git reference for diff comparison.
    pub base: Option<String>,
    /// Head Git reference for diff comparison.
    pub head: Option<String>,
    /// Minimum finding severity to return non-zero exit code.
    pub fail_on: Option<String>,
    /// AI provider override (`openai`, `anthropic`, `gemini`, `mock`).
    pub provider: Option<String>,
    /// Model identifier override.
    pub model: Option<String>,
    /// Output context pack without making network calls.
    pub dry_run: bool,
    /// Bypass AI response cache.
    pub no_cache: bool,
}

/// Applies deterministic grounding rules to raw AI review items (PRD §21).
///
/// Invariants:
/// 1. Items referencing unknown/hallucinated files are discarded.
/// 2. `Confirmed` items without a valid matching finding in `det.findings` are downgraded to `Probable`.
/// 3. Deterministic findings are never removed, altered, or superseded.
#[must_use]
pub fn apply_deterministic_guard(
    det: &ReviewReport,
    known_files: &BTreeSet<String>,
    items: Vec<AiReviewItem>,
) -> Vec<AiReviewItem> {
    let finding_ids: BTreeSet<&str> = det.findings.iter().map(|f| f.id.as_str()).collect();
    let mut guarded = Vec::new();

    for mut item in items {
        // Discard item if it references a file not in known repository / diff files
        if let Some(ref file_path) = item.file {
            let normalized = file_path.replace('\\', "/");
            if !known_files.contains(&normalized) {
                continue;
            }
        }

        // Confirmed items must link to a real deterministic finding ID
        if item.classification == AiClassification::Confirmed {
            let has_valid_finding = match &item.related_finding_id {
                Some(fid) => finding_ids.contains(fid.as_str()),
                None => false,
            };
            if !has_valid_finding {
                item.classification = AiClassification::Probable;
            }
        }

        guarded.push(item);
    }

    guarded
}

/// JSON Schema definition for AI review response.
#[must_use]
pub fn review_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "classification": {
                            "type": "string",
                            "enum": ["confirmed", "probable", "suggestion"]
                        },
                        "title": { "type": "string" },
                        "rationale": { "type": "string" },
                        "file": { "type": ["string", "null"] },
                        "line": { "type": ["integer", "null"] },
                        "related_finding_id": { "type": ["string", "null"] }
                    },
                    "required": ["classification", "title", "rationale"]
                }
            }
        },
        "required": ["items"]
    })
}

#[derive(Debug, Deserialize)]
struct RawReviewResponse {
    items: Vec<AiReviewItem>,
}

/// Executes `eforge ai-review` against `repo_root`.
pub fn execute_ai_review(
    repo_root: &Path,
    diff_input: Option<&str>,
    opts: &AiReviewOptions,
) -> Result<(AiReviewReport, ExitCode), ForgeError> {
    // 1. Run deterministic review first
    let (det, mut exit_code) = execute_review(repo_root, diff_input, opts.fast)?;

    // 2. Evaluate --fail-on strictly against deterministic findings
    if let Some(ref raw_thresh) = opts.fail_on {
        if let Some(sev) = parse_severity_threshold(raw_thresh) {
            exit_code = ExitCode::evaluate_findings(&det.findings, Some(sev));
        } else {
            return Err(ForgeError::Config {
                message: format!("invalid --fail-on severity: {raw_thresh}"),
            });
        }
    }

    // 3. Assemble known files set (for hallucination rejection)
    let mut known_files = BTreeSet::new();
    for cf in &det.impact.changed_files {
        known_files.insert(cf.path.replace('\\', "/"));
    }
    // Also include files present in the repo
    if let Ok(entries) = fs::read_dir(repo_root) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                known_files.insert(name);
            }
        }
    }

    // 4. Assemble context pack for review
    let mut sections = Vec::new();

    // Section: Blast Radius & Impact
    let mut impact_body = format!(
        "Directly Affected Modules: {}\nTransitively Affected Modules: {}\nFiles Missing Tests: {}\nChanged Files:\n",
        det.impact.directly_affected_modules.join(", "),
        det.impact.transitively_affected_modules.join(", "),
        det.impact.changed_files_without_tests.join(", ")
    );
    for cf in &det.impact.changed_files {
        let _ = writeln!(
            impact_body,
            "- {} (+{} -{})",
            cf.path, cf.added_lines, cf.deleted_lines
        );
    }
    sections.push(forge_ai::ContextSection::new(
        SectionKind::DiffImpact,
        "Diff Blast Radius & Impact",
        impact_body,
        90,
    ));

    // Section: Deterministic Findings
    if !det.findings.is_empty() {
        let mut findings_body = String::new();
        for f in &det.findings {
            let loc = f
                .location
                .as_ref()
                .map(|l| format!("{}:{}", l.file_path, l.start_line.unwrap_or(1)))
                .unwrap_or_else(|| "global".to_string());
            let _ = writeln!(
                findings_body,
                "- [{:?}] ID: `{}` at {} ({})\n  Title: {}\n  Evidence: {}",
                f.severity, f.id, loc, f.rule_id, f.title, f.evidence
            );
        }
        sections.push(forge_ai::ContextSection::new(
            SectionKind::SecurityFindings,
            "Deterministic Rule & Security Findings",
            findings_body,
            100,
        ));
    }

    // Section: Change Risk Assessment
    let mut risk_body = format!(
        "Risk Level: {:?}\nRisk Score: {}/100\nReasons:\n",
        det.risk.level, det.risk.score
    );
    for reason in &det.risk.reasons {
        let _ = writeln!(risk_body, "- {reason}");
    }
    sections.push(forge_ai::ContextSection::new(
        SectionKind::QualityFindings,
        "Change Risk Assessment",
        risk_body,
        80,
    ));

    // Snippets: collect changed files safely with secret redaction
    let mut snippets = Vec::new();
    let mut count = 0;
    for cf in &det.impact.changed_files {
        if is_sensitive_path(&cf.path) {
            continue;
        }
        let full_path = repo_root.join(&cf.path);
        if let Ok(content) = fs::read_to_string(&full_path) {
            let redacted = redact_secrets(&content);
            let lines: Vec<&str> = redacted.lines().take(100).collect();
            let line_count = lines.len() as u32;
            snippets.push(CodeSnippet::new(
                cf.path.clone(),
                1,
                line_count,
                lines.join("\n"),
            ));
            known_files.insert(cf.path.replace('\\', "/"));
            count += 1;
            if count >= 5 {
                break;
            }
        }
    }

    let forge_cfg = load_config(repo_root).unwrap_or_default();
    let mut ai_cfg = forge_cfg.ai;

    let is_mock = opts
        .provider
        .as_deref()
        .map(|p| p.eq_ignore_ascii_case("mock"))
        .unwrap_or(false);

    if let Some(ref p) = opts.provider {
        if is_mock {
            ai_cfg.enabled = true;
        } else {
            match p.to_lowercase().as_str() {
                "openai" | "openai-compatible" => {
                    ai_cfg.provider = Some(AiProviderKind::OpenAiCompatible)
                }
                "anthropic" => ai_cfg.provider = Some(AiProviderKind::Anthropic),
                "gemini" => ai_cfg.provider = Some(AiProviderKind::Gemini),
                other => {
                    return Err(ForgeError::Config {
                        message: format!("unsupported AI provider override '{other}'"),
                    })
                }
            }
        }
    }
    if let Some(ref m) = opts.model {
        ai_cfg.model = Some(m.clone());
    }

    let input = ContextInput {
        task: AiTask::ReviewDiff,
        sections,
        snippets,
    };

    let pack = build_context(&input, ai_cfg.max_context_tokens)?;

    // Handle Dry Run
    if opts.dry_run {
        let report = AiReviewReport {
            deterministic: det,
            ai_items: Vec::new(),
            provider: "dry-run".to_string(),
            model: "dry-run".to_string(),
            usage: TokenUsage::new(0, 0),
            cached: false,
            dry_run: Some(pack),
        };
        return Ok((report, exit_code));
    }

    // Verify AI capability is enabled
    ensure_ai_enabled(&ai_cfg)?;

    let today = current_utc_date();
    let provider_name = if is_mock {
        "mock".to_string()
    } else {
        match ai_cfg.provider {
            Some(AiProviderKind::OpenAiCompatible) => "openai-compatible".to_string(),
            Some(AiProviderKind::Anthropic) => "anthropic".to_string(),
            Some(AiProviderKind::Gemini) => "gemini".to_string(),
            None => "openai-compatible".to_string(),
        }
    };
    let model_name = ai_cfg
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    let schema = review_response_schema();
    let schema_str = serde_json::to_string(&schema).map_err(|e| ForgeError::Serialization {
        message: e.to_string(),
    })?;

    let cache = ResponseCache::new(repo_root);
    let cache_key = ResponseCache::key(
        &provider_name,
        &model_name,
        &pack.system_prompt,
        &pack.user_payload,
        Some(&schema_str),
    );

    // Check response cache
    if !opts.no_cache {
        if let Ok(Some(cached_val)) = cache.get(&cache_key) {
            if let Ok(raw_res) = serde_json::from_value::<RawReviewResponse>(cached_val) {
                let guarded_items = apply_deterministic_guard(&det, &known_files, raw_res.items);

                let usage = UsageRecord {
                    date: today,
                    provider: provider_name.clone(),
                    model: model_name.clone(),
                    command: "ai-review".to_string(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: Some(0.0),
                    cached: true,
                };
                let _ = append_usage(repo_root, &usage);

                let report = AiReviewReport {
                    deterministic: det,
                    ai_items: guarded_items,
                    provider: provider_name,
                    model: model_name,
                    usage: TokenUsage::new(0, 0),
                    cached: true,
                    dry_run: None,
                };
                return Ok((report, exit_code));
            }
        }
    }

    // Enforce cost controls
    let summary = summarize_usage(repo_root, &today)?;
    enforce_daily_budget(&summary, &ai_cfg)?;
    preflight_check(&pack, &ai_cfg)?;

    // Build provider
    let provider: Box<dyn AiProvider> = if is_mock {
        let sample_items = vec![
            AiReviewItem {
                classification: AiClassification::Confirmed,
                title: "Circular dependency detected".to_string(),
                rationale: "Service directly loops back into DB layer.".to_string(),
                file: det.impact.changed_files.first().map(|f| f.path.clone()),
                line: Some(1),
                related_finding_id: det.findings.first().map(|f| f.id.clone()),
            },
            AiReviewItem {
                classification: AiClassification::Probable,
                title: "Potential missing error handling on DB query".to_string(),
                rationale: "Query call might throw unhandled exception on timeout.".to_string(),
                file: det.impact.changed_files.first().map(|f| f.path.clone()),
                line: Some(8),
                related_finding_id: None,
            },
            AiReviewItem {
                classification: AiClassification::Suggestion,
                title: "Add unit test for userService audit tag helper".to_string(),
                rationale: "Helper function lacks dedicated unit test.".to_string(),
                file: det.impact.changed_files.first().map(|f| f.path.clone()),
                line: Some(12),
                related_finding_id: None,
            },
        ];
        let mock_json = serde_json::json!({
            "items": sample_items
        });
        Box::new(
            MockProvider::new()
                .with_structured_output(mock_json)
                .with_model(&model_name),
        )
    } else {
        build_provider(&ai_cfg, &|k| std::env::var(k).ok())?
    };

    let req = AiRequest::new(
        &pack.system_prompt,
        vec![AiMessage::user(&pack.user_payload)],
    );
    let json_val = provider.structured_output(&req, &schema)?;

    let raw_res: RawReviewResponse =
        serde_json::from_value(json_val.clone()).map_err(|e| ForgeError::Ai {
            message: format!("failed to parse structured model response as RawReviewResponse: {e}"),
        })?;

    // Apply deterministic guard to enforce grounding invariants
    let guarded_items = apply_deterministic_guard(&det, &known_files, raw_res.items);

    if !opts.no_cache {
        let _ = cache.put(&cache_key, &json_val);
    }

    let out_json_str = serde_json::to_string(&json_val).unwrap_or_default();
    let output_tokens = estimate_tokens(&out_json_str);
    let token_usage = TokenUsage::new(pack.estimated_tokens, output_tokens);

    let cost_usd = compute_cost_usd(token_usage.input_tokens, token_usage.output_tokens, &ai_cfg);
    let usage = UsageRecord {
        date: today,
        provider: provider_name.clone(),
        model: model_name.clone(),
        command: "ai-review".to_string(),
        input_tokens: token_usage.input_tokens,
        output_tokens: token_usage.output_tokens,
        cost_usd,
        cached: false,
    };
    let _ = append_usage(repo_root, &usage);

    let report = AiReviewReport {
        deterministic: det,
        ai_items: guarded_items,
        provider: provider_name,
        model: model_name,
        usage: token_usage,
        cached: false,
        dry_run: None,
    };

    Ok((report, exit_code))
}

/// Renders an [`AiReviewReport`] in Terminal, JSON, Markdown, or GitHub PR comment format.
pub fn render_ai_review_report(
    report: &AiReviewReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut out = render_review_report(&report.deterministic, OutputFormat::Markdown)?;

            if let Some(dry) = &report.dry_run {
                let _ = writeln!(out, "\n## AI Context Dry Run");
                let _ = writeln!(out, "- **Estimated Tokens:** {}", dry.estimated_tokens);
                let _ = writeln!(
                    out,
                    "- **Included Sections ({}):** {}",
                    dry.included.len(),
                    dry.included.join(", ")
                );
                let _ = writeln!(
                    out,
                    "- **Dropped Sections ({}):** {}",
                    dry.dropped.len(),
                    if dry.dropped.is_empty() {
                        "none".to_string()
                    } else {
                        dry.dropped.join(", ")
                    }
                );
            }

            if !report.ai_items.is_empty() {
                let _ = writeln!(out, "\n## AI Code Review Insights");
                let _ = writeln!(
                    out,
                    "> Generated by `{}/{}` (Tokens: {}, Cached: {})\n",
                    report.provider,
                    report.model,
                    report.usage.total(),
                    report.cached
                );

                let confirmed: Vec<&AiReviewItem> = report
                    .ai_items
                    .iter()
                    .filter(|i| i.classification == AiClassification::Confirmed)
                    .collect();
                let probable: Vec<&AiReviewItem> = report
                    .ai_items
                    .iter()
                    .filter(|i| i.classification == AiClassification::Probable)
                    .collect();
                let suggestions: Vec<&AiReviewItem> = report
                    .ai_items
                    .iter()
                    .filter(|i| i.classification == AiClassification::Suggestion)
                    .collect();

                if !confirmed.is_empty() {
                    let _ = writeln!(
                        out,
                        "### Confirmed Defects (Backed by Deterministic Engine)"
                    );
                    for item in confirmed {
                        let loc = item
                            .file
                            .as_ref()
                            .map(|f| format!("`{}:{}`", f, item.line.unwrap_or(1)))
                            .unwrap_or_else(|| "`global`".to_string());
                        let fid = item.related_finding_id.as_deref().unwrap_or("verified");
                        let _ = writeln!(
                            out,
                            "- **[Confirmed: `{fid}`]** {} ({})\n  > {}",
                            item.title, loc, item.rationale
                        );
                    }
                    let _ = writeln!(out);
                }

                if !probable.is_empty() {
                    let _ = writeln!(out, "### Probable Concerns");
                    for item in probable {
                        let loc = item
                            .file
                            .as_ref()
                            .map(|f| format!("`{}:{}`", f, item.line.unwrap_or(1)))
                            .unwrap_or_else(|| "`global`".to_string());
                        let _ = writeln!(
                            out,
                            "- **[Probable]** {} ({})\n  > {}",
                            item.title, loc, item.rationale
                        );
                    }
                    let _ = writeln!(out);
                }

                if !suggestions.is_empty() {
                    let _ = writeln!(out, "### Suggestions & Improvements");
                    for item in suggestions {
                        let loc = item
                            .file
                            .as_ref()
                            .map(|f| format!("`{}:{}`", f, item.line.unwrap_or(1)))
                            .unwrap_or_else(|| "`global`".to_string());
                        let _ = writeln!(
                            out,
                            "- **[Suggestion]** {} ({})\n  > {}",
                            item.title, loc, item.rationale
                        );
                    }
                }
            }

            Ok(out)
        }
        OutputFormat::Terminal => {
            let mut out = render_review_report(&report.deterministic, OutputFormat::Terminal)?;

            if let Some(dry) = &report.dry_run {
                let _ = writeln!(out, "\n--- AI Context Dry Run ---");
                let _ = writeln!(out, "Estimated Tokens: {}", dry.estimated_tokens);
                let _ = writeln!(
                    out,
                    "Included Sections ({}): {}",
                    dry.included.len(),
                    dry.included.join(", ")
                );
                let _ = writeln!(
                    out,
                    "Dropped Sections ({}): {}",
                    dry.dropped.len(),
                    if dry.dropped.is_empty() {
                        "none".to_string()
                    } else {
                        dry.dropped.join(", ")
                    }
                );
            }

            if !report.ai_items.is_empty() {
                let _ = writeln!(
                    out,
                    "\n--- AI Code Review Insights (Model: {}/{}, Tokens: {}, Cached: {}) ---",
                    report.provider,
                    report.model,
                    report.usage.total(),
                    report.cached
                );
                for item in &report.ai_items {
                    let tag = match item.classification {
                        AiClassification::Confirmed => "[confirmed]",
                        AiClassification::Probable => "[probable]",
                        AiClassification::Suggestion => "[suggestion]",
                    };
                    let loc = item
                        .file
                        .as_ref()
                        .map(|f| format!(" ({}:{})", f, item.line.unwrap_or(1)))
                        .unwrap_or_default();
                    let _ = writeln!(out, "{tag} {}{loc}", item.title);
                    let _ = writeln!(out, "  Rationale: {}", item.rationale);
                    if let Some(ref fid) = item.related_finding_id {
                        let _ = writeln!(out, "  Linked Finding: {fid}");
                    }
                }
            }

            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::scoring::{RiskLevel, RiskReport};
    use forge_core::Finding;
    use forge_git::ChangeImpactReport;
    use std::path::PathBuf;

    fn fixture_next_app() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn deterministic_guard_downgrades_unverified_confirmed_and_discards_hallucinations() {
        let det = ReviewReport {
            risk: RiskReport {
                level: RiskLevel::Low,
                score: 15,
                reasons: Vec::new(),
            },
            impact: ChangeImpactReport {
                changed_files: Vec::new(),
                directly_affected_modules: Vec::new(),
                transitively_affected_modules: Vec::new(),
                changed_files_without_tests: Vec::new(),
            },
            findings: vec![Finding {
                id: "ARCH-001".to_string(),
                category: "architecture".to_string(),
                severity: forge_core::Severity::High,
                confidence: forge_core::Confidence::High,
                title: "Circular dependency".to_string(),
                description: "desc".to_string(),
                evidence: "ev".to_string(),
                location: None,
                recommendation: "rec".to_string(),
                rule_id: "arch/circular".to_string(),
            }],
        };

        let mut known_files = BTreeSet::new();
        known_files.insert("src/services/user.ts".to_string());

        let items = vec![
            // Valid Confirmed
            AiReviewItem {
                classification: AiClassification::Confirmed,
                title: "Confirmed cycle".to_string(),
                rationale: "r1".to_string(),
                file: Some("src/services/user.ts".to_string()),
                line: Some(10),
                related_finding_id: Some("ARCH-001".to_string()),
            },
            // Unverified Confirmed (hallucinated finding ID -> downgraded to Probable)
            AiReviewItem {
                classification: AiClassification::Confirmed,
                title: "Fake confirmed".to_string(),
                rationale: "r2".to_string(),
                file: Some("src/services/user.ts".to_string()),
                line: Some(15),
                related_finding_id: Some("NONEXISTENT-FINDING".to_string()),
            },
            // Hallucinated File -> must be discarded
            AiReviewItem {
                classification: AiClassification::Suggestion,
                title: "Hallucinated file item".to_string(),
                rationale: "r3".to_string(),
                file: Some("src/nonexistent/fake.ts".to_string()),
                line: Some(1),
                related_finding_id: None,
            },
        ];

        let guarded = apply_deterministic_guard(&det, &known_files, items);
        assert_eq!(guarded.len(), 2);

        assert_eq!(guarded[0].classification, AiClassification::Confirmed);
        assert_eq!(guarded[0].title, "Confirmed cycle");

        // Downgraded to Probable
        assert_eq!(guarded[1].classification, AiClassification::Probable);
        assert_eq!(guarded[1].title, "Fake confirmed");
    }

    #[test]
    fn execute_ai_review_dry_run_produces_context_pack() {
        let root = fixture_next_app();
        let opts = AiReviewOptions {
            fast: false,
            base: None,
            head: None,
            fail_on: None,
            provider: None,
            model: None,
            dry_run: true,
            no_cache: true,
        };
        let res = execute_ai_review(&root, None, &opts);
        assert!(res.is_ok());
        if let Ok((rep, exit_code)) = res {
            assert_eq!(exit_code, ExitCode::Success);
            assert!(rep.dry_run.is_some());
            assert!(rep.ai_items.is_empty());

            let rendered_res = render_ai_review_report(&rep, OutputFormat::Terminal);
            assert!(rendered_res.is_ok());
            if let Ok(rendered) = rendered_res {
                assert!(rendered.contains("--- AI Context Dry Run ---"));
                assert!(rendered.contains("Estimated Tokens:"));
            }
        }
    }

    #[test]
    fn execute_ai_review_mock_provider_produces_guarded_items() {
        let root = fixture_next_app();
        let opts = AiReviewOptions {
            fast: false,
            base: None,
            head: None,
            fail_on: None,
            provider: Some("mock".to_string()),
            model: Some("mock-v1".to_string()),
            dry_run: false,
            no_cache: true,
        };
        let res = execute_ai_review(&root, None, &opts);
        assert!(res.is_ok());
        if let Ok((rep, _)) = res {
            assert!(rep.dry_run.is_none());
            assert!(!rep.ai_items.is_empty());
            assert_eq!(rep.provider, "mock");

            let md_res = render_ai_review_report(&rep, OutputFormat::Markdown);
            assert!(md_res.is_ok());
            if let Ok(md) = md_res {
                assert!(md.contains("## AI Code Review Insights"));
            }

            let json_res = render_ai_review_report(&rep, OutputFormat::Json);
            assert!(json_res.is_ok());
            if let Ok(json_str) = json_res {
                assert!(json_str.contains("\"ai_items\":"));
            }
        }
    }
}
