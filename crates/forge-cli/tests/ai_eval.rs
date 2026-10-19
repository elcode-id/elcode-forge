//! AI quality evaluation harness (PRD §79): deterministic retrieval recall,
//! grounding guard verification, report determinism, and optional live eval.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use forge_cli::ai::orchestrate::AiExplainOptions;
use forge_cli::commands::ai_review::{
    apply_deterministic_guard, execute_ai_review, AiClassification, AiReviewItem, AiReviewOptions,
};
use forge_cli::commands::explain::execute_explain_with_ai;
use forge_cli::commands::report::{build_report_bundle, ReportOptions};
use forge_cli::commands::review::ReviewReport;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::OutputFormat;
use forge_core::scoring::{RiskLevel, RiskReport};
use forge_core::{Finding, Severity};
use forge_git::ChangeImpactReport;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedFacts {
    #[serde(default)]
    modules: Vec<String>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    exports: Vec<String>,
    #[serde(default)]
    findings: Vec<String>,
    #[serde(default)]
    finding_rules: Vec<String>,
    #[serde(default)]
    circular_cycle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkQuestion {
    id: String,
    fixture: String,
    target: String,
    task: String,
    question: String,
    expected_grounding_facts: ExpectedFacts,
}

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn ai_retrieval_recall_matches_benchmark_grounding_facts_deterministically() {
    let questions_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("ai-eval")
        .join("questions.json");

    let content_res = fs::read_to_string(&questions_file);
    assert!(content_res.is_ok());

    let Ok(content) = content_res else {
        return;
    };

    let questions_res = serde_json::from_str::<Vec<BenchmarkQuestion>>(&content);
    assert!(questions_res.is_ok(), "failed to parse questions.json");

    let Ok(questions) = questions_res else {
        return;
    };

    assert!(!questions.is_empty());

    for q in &questions {
        assert!(!q.task.is_empty());
        assert!(!q.question.is_empty());
        let root = fixture_dir(&q.fixture);
        assert!(root.is_dir(), "fixture '{}' must exist", q.fixture);

        let ai_opts = AiExplainOptions {
            ai: false,
            provider: None,
            model: None,
            dry_run: true,
            no_cache: true,
        };

        let res = execute_explain_with_ai(&root, Some(&q.target), &ai_opts);
        assert!(res.is_ok(), "explain failed for benchmark {}", q.id);

        let Ok(report) = res else {
            continue;
        };

        assert!(
            report.dry_run.is_some(),
            "dry_run context pack must be generated"
        );
        let Some(pack) = report.dry_run else {
            continue;
        };

        // 1. Verify expected modules and files are captured in user payload
        for expected_mod in &q.expected_grounding_facts.modules {
            assert!(
                pack.user_payload.contains(expected_mod),
                "Benchmark '{}': user payload missing expected module '{}'",
                q.id,
                expected_mod
            );
        }
        for expected_file in &q.expected_grounding_facts.files {
            assert!(
                pack.user_payload.contains(expected_file),
                "Benchmark '{}': user payload missing expected file '{}'",
                q.id,
                expected_file
            );
        }

        // 2. Verify expected exports are present if snippet included
        for expected_exp in &q.expected_grounding_facts.exports {
            assert!(
                pack.user_payload.contains(expected_exp),
                "Benchmark '{}': user payload missing expected export '{}'",
                q.id,
                expected_exp
            );
        }

        // 3. Verify expected findings or finding rules
        for expected_finding in &q.expected_grounding_facts.findings {
            assert!(
                pack.user_payload.contains(expected_finding),
                "Benchmark '{}': user payload missing finding '{}'",
                q.id,
                expected_finding
            );
        }
        for expected_rule in &q.expected_grounding_facts.finding_rules {
            assert!(
                pack.user_payload.contains(expected_rule),
                "Benchmark '{}': user payload missing finding rule '{}'",
                q.id,
                expected_rule
            );
        }

        // 4. Verify circular cycles if applicable
        if let Some(ref cycle) = q.expected_grounding_facts.circular_cycle {
            assert!(
                pack.user_payload.contains(cycle),
                "Benchmark '{}': user payload missing cycle path '{}'",
                q.id,
                cycle
            );
        }
    }
}

#[test]
fn grounding_guard_enforces_confirmed_downgrade_and_hallucination_rejection() {
    let det = ReviewReport {
        risk: RiskReport {
            level: RiskLevel::High,
            score: 75,
            reasons: vec!["Critical security finding present".to_string()],
        },
        impact: ChangeImpactReport {
            changed_files: Vec::new(),
            directly_affected_modules: vec!["src/auth".to_string()],
            transitively_affected_modules: Vec::new(),
            changed_files_without_tests: Vec::new(),
        },
        findings: vec![
            Finding {
                id: "SEC-LEAK-001".to_string(),
                category: "security".to_string(),
                severity: Severity::Critical,
                confidence: forge_core::Confidence::Certain,
                title: "Hardcoded secret in auth module".to_string(),
                description: "desc".to_string(),
                evidence: "sk-****REDACTED****".to_string(),
                location: None,
                recommendation: "Rotate token immediately".to_string(),
                rule_id: "security/hardcoded-secret".to_string(),
            },
            Finding {
                id: "ARCH-CYCLE-001".to_string(),
                category: "architecture".to_string(),
                severity: Severity::High,
                confidence: forge_core::Confidence::High,
                title: "Circular dependency".to_string(),
                description: "desc".to_string(),
                evidence: "a <-> b".to_string(),
                location: None,
                recommendation: "Refactor interface".to_string(),
                rule_id: "arch/circular".to_string(),
            },
        ],
    };

    let mut known_files = BTreeSet::new();
    known_files.insert("src/auth.ts".to_string());
    known_files.insert("src/config.ts".to_string());

    let candidate_items = vec![
        // Valid Confirmed item
        AiReviewItem {
            classification: AiClassification::Confirmed,
            title: "Confirmed API secret leak".to_string(),
            rationale: "Token was committed in plaintext".to_string(),
            file: Some("src/auth.ts".to_string()),
            line: Some(42),
            related_finding_id: Some("SEC-LEAK-001".to_string()),
        },
        // Unlinked Confirmed item (hallucinated finding ID) -> must downgrade to Probable
        AiReviewItem {
            classification: AiClassification::Confirmed,
            title: "Unlinked SQL injection".to_string(),
            rationale: "Query looks suspicious but has no deterministic rule finding".to_string(),
            file: Some("src/auth.ts".to_string()),
            line: Some(50),
            related_finding_id: Some("SEC-SQLI-FAKE".to_string()),
        },
        // Confirmed with None finding ID -> must downgrade to Probable
        AiReviewItem {
            classification: AiClassification::Confirmed,
            title: "Unspecified confirmed item".to_string(),
            rationale: "No finding ID supplied".to_string(),
            file: Some("src/config.ts".to_string()),
            line: Some(1),
            related_finding_id: None,
        },
        // Hallucinated file -> must be discarded
        AiReviewItem {
            classification: AiClassification::Suggestion,
            title: "Hallucinated file comment".to_string(),
            rationale: "File does not exist in repo".to_string(),
            file: Some("src/nonexistent/secret_handler.ts".to_string()),
            line: Some(99),
            related_finding_id: None,
        },
    ];

    let guarded = apply_deterministic_guard(&det, &known_files, candidate_items);

    // 4 candidate items -> 1 discarded hallucination = 3 guarded items
    assert_eq!(guarded.len(), 3);

    // Item 0: Stays Confirmed
    assert_eq!(guarded[0].classification, AiClassification::Confirmed);
    assert_eq!(
        guarded[0].related_finding_id.as_deref(),
        Some("SEC-LEAK-001")
    );

    // Item 1: Downgraded to Probable
    assert_eq!(guarded[1].classification, AiClassification::Probable);
    assert_eq!(guarded[1].title, "Unlinked SQL injection");

    // Item 2: Downgraded to Probable
    assert_eq!(guarded[2].classification, AiClassification::Probable);
    assert_eq!(guarded[2].title, "Unspecified confirmed item");
}

#[test]
fn scan_and_report_outputs_remain_strictly_deterministic_with_ai_layer_present() {
    let root = fixture_dir("next-app");

    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let scan_res1 = execute_scan(&scan_opts);
    let scan_res2 = execute_scan(&scan_opts);

    assert!(scan_res1.is_ok());
    assert!(scan_res2.is_ok());

    if let (Ok((repo1, code1)), Ok((repo2, code2))) = (scan_res1, scan_res2) {
        assert_eq!(code1, code2);
        assert_eq!(repo1.findings.len(), repo2.findings.len());
        assert_eq!(repo1.modules.len(), repo2.modules.len());

        let json1 = serde_json::to_string(&repo1).unwrap_or_default();
        let json2 = serde_json::to_string(&repo2).unwrap_or_default();
        assert_eq!(json1, json2, "Repeated scans must be byte-identical");
    }

    let report_opts = ReportOptions {
        hotspots_top_n: 5,
        include_git: false,
    };

    let rep1 = build_report_bundle(&root, &report_opts);
    let rep2 = build_report_bundle(&root, &report_opts);

    assert!(rep1.is_ok());
    assert!(rep2.is_ok());

    if let (Ok(b1), Ok(b2)) = (rep1, rep2) {
        let j1 = serde_json::to_string(&b1).unwrap_or_default();
        let j2 = serde_json::to_string(&b2).unwrap_or_default();
        assert_eq!(j1, j2, "Repeated report bundles must be byte-identical");
    }
}

#[test]
fn opt_in_live_eval_executes_only_when_flag_is_explicitly_set() {
    if std::env::var("EFORGE_AI_EVAL_LIVE").is_err() {
        // Skip silently in standard offline CI runs (PRD §79 invariant)
        return;
    }

    // When EFORGE_AI_EVAL_LIVE=1 is set, run live evaluation
    let root = fixture_dir("next-app");
    let opts = AiReviewOptions {
        fast: false,
        base: None,
        head: None,
        fail_on: None,
        provider: None,
        model: None,
        dry_run: false,
        no_cache: false,
    };

    let res = execute_ai_review(&root, None, &opts);
    assert!(res.is_ok(), "Live AI review evaluation failed");
}
