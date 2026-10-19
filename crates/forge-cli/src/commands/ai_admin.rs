//! `eforge ai status` and `eforge ai usage` administrative commands.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::Instant;

use forge_ai::{build_provider, mask_secret, summarize_usage, AiMessage, AiRequest, UsageSummary};
use forge_core::config::{load_config, AiProviderKind};
use forge_core::error::ForgeError;
use serde::{Deserialize, Serialize};

use crate::ai::orchestrate::current_utc_date;
use crate::cli::OutputFormat;

/// Diagnostic status report for the configured AI provider and credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiStatusReport {
    /// Whether AI capabilities are explicitly enabled in configuration.
    pub enabled: bool,
    /// Configured or default provider kind.
    pub provider: String,
    /// Configured or default model name.
    pub model: String,
    /// Base URL if customized (e.g. for local inference endpoints).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Target environment variable name storing the API credential.
    pub api_key_env: String,
    /// Credential status: masked key (`present (sk-...1234)`), `missing`, or `not required`.
    pub credential_status: String,
    /// Whether the local response cache is enabled.
    pub cache_enabled: bool,
    /// Configured daily budget in USD, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_daily_usd: Option<f64>,
    /// Measured round-trip ping latency in milliseconds, if `--ping` was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_latency_ms: Option<u64>,
    /// Ping status description (e.g. "reachable", "skipped (ai.enabled is false)", or error details).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<String>,
}

/// Aggregated usage metrics and budget status for `eforge ai usage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiUsageReport {
    /// Token consumption and call count breakdown for today and all-time.
    pub summary: UsageSummary,
    /// Remaining USD balance for today based on `budget_daily_usd`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_daily_budget_usd: Option<f64>,
    /// Number of cached JSON response files currently in `.eforge/ai-cache/`.
    pub cache_entries_count: usize,
}

/// Inspects AI configuration and credentials status, optionally performing a `--ping` check.
pub fn run_ai_status(repo_root: &Path, ping: bool) -> Result<AiStatusReport, ForgeError> {
    let forge_cfg = load_config(repo_root).unwrap_or_default();
    let ai_cfg = forge_cfg.ai;

    let provider_name = match ai_cfg.provider {
        Some(AiProviderKind::OpenAiCompatible) => "openai-compatible".to_string(),
        Some(AiProviderKind::Anthropic) => "anthropic".to_string(),
        Some(AiProviderKind::Gemini) => "gemini".to_string(),
        None => "openai-compatible".to_string(),
    };

    let model = ai_cfg
        .model
        .clone()
        .unwrap_or_else(|| match ai_cfg.provider {
            Some(AiProviderKind::Anthropic) => "claude-3-5-sonnet-20241022".to_string(),
            Some(AiProviderKind::Gemini) => "gemini-1.5-pro".to_string(),
            _ => "gpt-4o-mini".to_string(),
        });

    let default_env_var = match ai_cfg.provider {
        Some(AiProviderKind::Anthropic) => "ANTHROPIC_API_KEY",
        Some(AiProviderKind::Gemini) => "GEMINI_API_KEY",
        _ => "OPENAI_API_KEY",
    };
    let api_key_env = ai_cfg
        .api_key_env
        .clone()
        .unwrap_or_else(|| default_env_var.to_string());

    let credential_status = match std::env::var(&api_key_env) {
        Ok(raw_val) => {
            let masked = mask_secret(&raw_val);
            format!("present ({masked})")
        }
        Err(_) => {
            // Local endpoints (e.g. localhost, 127.0.0.1) may not require keys
            if let Some(ref base) = ai_cfg.base_url {
                if base.contains("localhost") || base.contains("127.0.0.1") {
                    "not required (local base_url)".to_string()
                } else {
                    "missing".to_string()
                }
            } else {
                "missing".to_string()
            }
        }
    };

    let mut ping_latency_ms = None;
    let mut ping_status = None;

    if ping {
        if !ai_cfg.enabled {
            ping_status = Some("skipped (ai.enabled is false)".to_string());
        } else {
            let start = Instant::now();
            match build_provider(&ai_cfg, &|k| std::env::var(k).ok()) {
                Ok(provider) => {
                    let req = AiRequest::new("You are a ping test.", vec![AiMessage::user("ping")])
                        .with_max_tokens(5);
                    match provider.generate(&req) {
                        Ok(_) => {
                            ping_latency_ms = Some(start.elapsed().as_millis() as u64);
                            ping_status = Some("reachable".to_string());
                        }
                        Err(err) => {
                            ping_status = Some(format!("error: {err}"));
                        }
                    }
                }
                Err(err) => {
                    ping_status = Some(format!("config error: {err}"));
                }
            }
        }
    }

    Ok(AiStatusReport {
        enabled: ai_cfg.enabled,
        provider: provider_name,
        model,
        base_url: ai_cfg.base_url,
        api_key_env,
        credential_status,
        cache_enabled: ai_cfg.cache_enabled,
        budget_daily_usd: ai_cfg.budget_daily_usd,
        ping_latency_ms,
        ping_status,
    })
}

/// Summarizes AI token usage, cache statistics, and remaining budget.
pub fn run_ai_usage(repo_root: &Path) -> Result<AiUsageReport, ForgeError> {
    let forge_cfg = load_config(repo_root).unwrap_or_default();
    let ai_cfg = forge_cfg.ai;

    let today = current_utc_date();
    let summary = summarize_usage(repo_root, &today)?;

    let remaining_daily_budget_usd = match (ai_cfg.budget_daily_usd, summary.today_cost_usd) {
        (Some(cap), Some(spent)) => Some((cap - spent).max(0.0)),
        (Some(cap), None) => Some(cap),
        _ => None,
    };

    // Count cache files in `.eforge/ai-cache/`
    let cache_dir = repo_root.join(".eforge").join("ai-cache");
    let mut cache_entries_count = 0;
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".json") {
                        cache_entries_count += 1;
                    }
                }
            }
        }
    }

    Ok(AiUsageReport {
        summary,
        remaining_daily_budget_usd,
        cache_entries_count,
    })
}

/// Renders [`AiStatusReport`] in Terminal or JSON format.
pub fn render_ai_status_report(
    report: &AiStatusReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut out = String::new();
            let _ = writeln!(out, "# ELCODE Forge — AI Status\n");
            let _ = writeln!(
                out,
                "- **Status:** {}",
                if report.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            let _ = writeln!(out, "- **Provider:** `{}`", report.provider);
            let _ = writeln!(out, "- **Model:** `{}`", report.model);
            if let Some(ref base) = report.base_url {
                let _ = writeln!(out, "- **Base URL:** `{base}`");
            }
            let _ = writeln!(out, "- **API Key Env:** `{}`", report.api_key_env);
            let _ = writeln!(out, "- **Credential:** {}", report.credential_status);
            let _ = writeln!(
                out,
                "- **Response Cache:** {}",
                if report.cache_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            if let Some(budget) = report.budget_daily_usd {
                let _ = writeln!(out, "- **Daily Budget:** ${budget:.2}");
            }
            if let Some(status) = &report.ping_status {
                if let Some(ms) = report.ping_latency_ms {
                    let _ = writeln!(out, "- **Ping:** {status} ({ms} ms)");
                } else {
                    let _ = writeln!(out, "- **Ping:** {status}");
                }
            }
            Ok(out)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            let _ = writeln!(out, "ELCODE Forge — AI Status");
            let _ = writeln!(
                out,
                "Status:          {}",
                if report.enabled {
                    "Enabled"
                } else {
                    "Disabled (ai.enabled: false)"
                }
            );
            let _ = writeln!(out, "Provider:        {}", report.provider);
            let _ = writeln!(out, "Model:           {}", report.model);
            if let Some(ref base) = report.base_url {
                let _ = writeln!(out, "Base URL:        {base}");
            }
            let _ = writeln!(out, "API Key Env:     {}", report.api_key_env);
            let _ = writeln!(out, "Credential:      {}", report.credential_status);
            let _ = writeln!(
                out,
                "Response Cache:  {}",
                if report.cache_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            if let Some(budget) = report.budget_daily_usd {
                let _ = writeln!(out, "Daily Budget:    ${budget:.2}");
            }
            if let Some(status) = &report.ping_status {
                if let Some(ms) = report.ping_latency_ms {
                    let _ = writeln!(out, "Ping Latency:    {ms} ms ({status})");
                } else {
                    let _ = writeln!(out, "Ping:            {status}");
                }
            }
            Ok(out)
        }
    }
}

/// Renders [`AiUsageReport`] in Terminal or JSON format.
pub fn render_ai_usage_report(
    report: &AiUsageReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut out = String::new();
            let _ = writeln!(
                out,
                "# ELCODE Forge — AI Usage Summary ({})\n",
                report.summary.today
            );
            let _ = writeln!(out, "## Today's Activity");
            let _ = writeln!(out, "- **Invocations:** {}", report.summary.today_calls);
            let _ = writeln!(
                out,
                "- **Tokens:** {} input / {} output (total: {})",
                report.summary.today_input_tokens,
                report.summary.today_output_tokens,
                report.summary.today_input_tokens + report.summary.today_output_tokens
            );
            if let Some(spent) = report.summary.today_cost_usd {
                let _ = writeln!(out, "- **Estimated Spend:** ${spent:.4}");
            }
            if let Some(rem) = report.remaining_daily_budget_usd {
                let _ = writeln!(out, "- **Remaining Daily Budget:** ${rem:.4}");
            }

            let _ = writeln!(out, "\n## All-Time Activity");
            let _ = writeln!(
                out,
                "- **Total Invocations:** {}",
                report.summary.total_calls
            );
            let _ = writeln!(
                out,
                "- **Total Tokens:** {} input / {} output (total: {})",
                report.summary.total_input_tokens,
                report.summary.total_output_tokens,
                report.summary.total_input_tokens + report.summary.total_output_tokens
            );
            if let Some(total_spend) = report.summary.total_cost_usd {
                let _ = writeln!(out, "- **Total Spend:** ${total_spend:.4}");
            }
            let _ = writeln!(
                out,
                "- **Cached Response Files:** {}",
                report.cache_entries_count
            );

            Ok(out)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            let _ = writeln!(
                out,
                "ELCODE Forge — AI Usage Summary ({})",
                report.summary.today
            );
            let _ = writeln!(
                out,
                "Today's Invocations:     {}",
                report.summary.today_calls
            );
            let _ = writeln!(
                out,
                "Today's Tokens:          {} in / {} out",
                report.summary.today_input_tokens, report.summary.today_output_tokens
            );
            if let Some(spent) = report.summary.today_cost_usd {
                let _ = writeln!(out, "Today's Spend:           ${spent:.4}");
            }
            if let Some(rem) = report.remaining_daily_budget_usd {
                let _ = writeln!(out, "Remaining Daily Budget:  ${rem:.4}");
            }

            let _ = writeln!(
                out,
                "\nAll-Time Invocations:    {}",
                report.summary.total_calls
            );
            let _ = writeln!(
                out,
                "All-Time Tokens:         {} in / {} out",
                report.summary.total_input_tokens, report.summary.total_output_tokens
            );
            if let Some(total_spend) = report.summary.total_cost_usd {
                let _ = writeln!(out, "All-Time Spend:          ${total_spend:.4}");
            }
            let _ = writeln!(
                out,
                "Cached Responses:        {} file(s)",
                report.cache_entries_count
            );

            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_next_app() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn run_ai_status_default_disabled_without_ping_makes_zero_network_calls() {
        let root = fixture_next_app();
        let res = run_ai_status(&root, false);
        assert!(res.is_ok());
        if let Ok(rep) = res {
            assert!(!rep.enabled);
            assert_eq!(rep.provider, "openai-compatible");
            assert!(rep.ping_status.is_none());
            assert!(rep.ping_latency_ms.is_none());

            let rendered_res = render_ai_status_report(&rep, OutputFormat::Terminal);
            assert!(rendered_res.is_ok());
            if let Ok(rendered) = rendered_res {
                assert!(rendered.contains("ELCODE Forge — AI Status"));
                assert!(rendered.contains("Disabled (ai.enabled: false)"));
            }
        }
    }

    #[test]
    fn run_ai_status_with_ping_on_disabled_ai_reports_skipped() {
        let root = fixture_next_app();
        let res = run_ai_status(&root, true);
        assert!(res.is_ok());
        if let Ok(rep) = res {
            assert!(!rep.enabled);
            assert_eq!(
                rep.ping_status.as_deref(),
                Some("skipped (ai.enabled is false)")
            );
            assert!(rep.ping_latency_ms.is_none());
        }
    }

    #[test]
    fn run_ai_usage_on_fixture_returns_valid_summary_and_reports() {
        let temp_dir = std::env::temp_dir().join("eforge_ai_usage_admin_test");
        let _ = fs::create_dir_all(&temp_dir);

        let res = run_ai_usage(&temp_dir);
        assert!(res.is_ok());
        if let Ok(rep) = res {
            assert_eq!(rep.summary.today_calls, 0);
            assert_eq!(rep.cache_entries_count, 0);

            let term_res = render_ai_usage_report(&rep, OutputFormat::Terminal);
            assert!(term_res.is_ok());
            if let Ok(term) = term_res {
                assert!(term.contains("ELCODE Forge — AI Usage Summary"));
                assert!(term.contains("Today's Invocations:     0"));
            }

            let json_res = render_ai_usage_report(&rep, OutputFormat::Json);
            assert!(json_res.is_ok());
            if let Ok(json_str) = json_res {
                assert!(json_str.contains("\"today_calls\": 0"));
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
