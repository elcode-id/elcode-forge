use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use forge_core::error::ForgeError;
use serde::{Deserialize, Serialize};

/// Represents a single AI invocation logged into `.eforge/ai-usage.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Date of execution in YYYY-MM-DD format.
    pub date: String,
    /// Identifier of the AI provider (e.g. "openai", "anthropic", "gemini", "mock").
    pub provider: String,
    /// Model name used for completion.
    pub model: String,
    /// CLI subcommand or internal task name (e.g. "review", "explain", "fix").
    pub command: String,
    /// Number of prompt/input tokens.
    pub input_tokens: u32,
    /// Number of generated completion/output tokens.
    pub output_tokens: u32,
    /// Estimated cost in USD if unit pricing was configured.
    pub cost_usd: Option<f64>,
    /// Whether this response was served directly from the response cache.
    pub cached: bool,
}

/// Aggregated usage metrics for daily budget enforcement and analytics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    /// The reference date for today's queries (YYYY-MM-DD).
    pub today: String,
    /// Total calls executed today.
    pub today_calls: usize,
    /// Total input tokens consumed today.
    pub today_input_tokens: u64,
    /// Total output tokens generated today.
    pub today_output_tokens: u64,
    /// Total spending in USD today (if pricing was configured for records).
    pub today_cost_usd: Option<f64>,
    /// All-time calls across all records in the ledger.
    pub total_calls: usize,
    /// All-time input tokens across all records.
    pub total_input_tokens: u64,
    /// All-time output tokens across all records.
    pub total_output_tokens: u64,
    /// All-time spending in USD across all records.
    pub total_cost_usd: Option<f64>,
}

/// Appends a new usage record to `<repo_root>/.eforge/ai-usage.jsonl`.
pub fn append_usage(repo_root: &Path, rec: &UsageRecord) -> Result<(), ForgeError> {
    let dir = repo_root.join(".eforge");
    create_dir_all(&dir).map_err(|source| ForgeError::Io {
        path: dir.clone(),
        source,
    })?;

    let path = dir.join("ai-usage.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| ForgeError::Io {
            path: path.clone(),
            source,
        })?;

    let line = serde_json::to_string(rec).map_err(|e| ForgeError::Serialization {
        message: format!("failed to serialize usage record: {e}"),
    })?;

    writeln!(file, "{line}").map_err(|source| ForgeError::Io { path, source })?;

    Ok(())
}

/// Reads the usage ledger at `<repo_root>/.eforge/ai-usage.jsonl` and summarizes usage for `today`.
///
/// Malformed or corrupted lines are safely skipped without failing the entire summary.
pub fn summarize_usage(repo_root: &Path, today: &str) -> Result<UsageSummary, ForgeError> {
    let path = repo_root.join(".eforge").join("ai-usage.jsonl");
    let mut summary = UsageSummary {
        today: today.to_string(),
        ..Default::default()
    };

    if !path.exists() {
        return Ok(summary);
    }

    let file = File::open(&path).map_err(|source| ForgeError::Io {
        path: path.clone(),
        source,
    })?;

    let reader = BufReader::new(file);
    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record: UsageRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let is_today = record.date == today;

        if is_today {
            summary.today_calls += 1;
            summary.today_input_tokens += record.input_tokens as u64;
            summary.today_output_tokens += record.output_tokens as u64;
            if let Some(cost) = record.cost_usd {
                let current = summary.today_cost_usd.unwrap_or(0.0);
                summary.today_cost_usd = Some(current + cost);
            }
        }

        summary.total_calls += 1;
        summary.total_input_tokens += record.input_tokens as u64;
        summary.total_output_tokens += record.output_tokens as u64;
        if let Some(cost) = record.cost_usd {
            let current = summary.total_cost_usd.unwrap_or(0.0);
            summary.total_cost_usd = Some(current + cost);
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_empty_or_nonexistent_returns_defaults() {
        let temp_dir = std::env::temp_dir().join(format!(
            "eforge_usage_empty_{}",
            blake3::hash(b"empty_test").to_hex()
        ));
        let res = summarize_usage(&temp_dir, "2026-10-19");
        assert!(res.is_ok());
        if let Ok(summary) = res {
            assert_eq!(summary.today, "2026-10-19");
            assert_eq!(summary.today_calls, 0);
            assert_eq!(summary.total_calls, 0);
            assert_eq!(summary.today_cost_usd, None);
        }
    }

    #[test]
    fn append_and_summarize_usage_tracks_metrics_and_skips_corrupted_lines() {
        let temp_dir = std::env::temp_dir().join(format!(
            "eforge_usage_append_{}",
            blake3::hash(b"append_test").to_hex()
        ));
        let today = "2026-10-19";
        let yesterday = "2026-10-18";

        let rec1 = UsageRecord {
            date: yesterday.to_string(),
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            command: "review".to_string(),
            input_tokens: 1000,
            output_tokens: 200,
            cost_usd: Some(0.005),
            cached: false,
        };

        let rec2 = UsageRecord {
            date: today.to_string(),
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            command: "explain".to_string(),
            input_tokens: 500,
            output_tokens: 100,
            cost_usd: Some(0.0025),
            cached: false,
        };

        let rec3 = UsageRecord {
            date: today.to_string(),
            provider: "mock".to_string(),
            model: "mock-v1".to_string(),
            command: "review".to_string(),
            input_tokens: 300,
            output_tokens: 50,
            cost_usd: None,
            cached: true,
        };

        assert!(append_usage(&temp_dir, &rec1).is_ok());
        assert!(append_usage(&temp_dir, &rec2).is_ok());

        // Incur a corrupted line directly into the file
        let path = temp_dir.join(".eforge").join("ai-usage.jsonl");
        if let Ok(mut f) = OpenOptions::new().append(true).open(&path) {
            let _ = writeln!(f, "{{ invalid_json: true ... }}");
            let _ = writeln!(f);
        }

        assert!(append_usage(&temp_dir, &rec3).is_ok());

        let res = summarize_usage(&temp_dir, today);
        assert!(res.is_ok());
        if let Ok(summary) = res {
            assert_eq!(summary.today, today);
            // Today has rec2 + rec3
            assert_eq!(summary.today_calls, 2);
            assert_eq!(summary.today_input_tokens, 800);
            assert_eq!(summary.today_output_tokens, 150);
            assert!(summary.today_cost_usd.is_some());
            if let Some(cost) = summary.today_cost_usd {
                assert!((cost - 0.0025).abs() < 0.0001);
            }

            // Total has rec1 + rec2 + rec3 (corrupted line skipped)
            assert_eq!(summary.total_calls, 3);
            assert_eq!(summary.total_input_tokens, 1800);
            assert_eq!(summary.total_output_tokens, 350);
            assert!(summary.total_cost_usd.is_some());
            if let Some(cost) = summary.total_cost_usd {
                assert!((cost - 0.0075).abs() < 0.0001);
            }
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
