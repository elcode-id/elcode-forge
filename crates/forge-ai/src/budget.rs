use forge_core::config::AiConfig;
use forge_core::error::ForgeError;

use crate::context::ContextPack;
use crate::usage::UsageSummary;

/// Verifies that candidate context does not exceed configured maximum context boundaries.
pub fn preflight_check(pack: &ContextPack, cfg: &AiConfig) -> Result<(), ForgeError> {
    if pack.estimated_tokens > cfg.max_context_tokens {
        return Err(ForgeError::Ai {
            message: format!(
                "estimated context tokens ({}) exceeds configured maximum limit of max_context_tokens ({})",
                pack.estimated_tokens, cfg.max_context_tokens
            ),
        });
    }
    Ok(())
}

/// Computes estimated total cost in USD for the given token metrics using configured unit prices.
///
/// Returns `None` if no unit pricing is specified in configuration.
#[must_use]
pub fn compute_cost_usd(input_tokens: u32, output_tokens: u32, cfg: &AiConfig) -> Option<f64> {
    if cfg.price_input_per_1k.is_none() && cfg.price_output_per_1k.is_none() {
        return None;
    }

    let input_price = cfg.price_input_per_1k.unwrap_or(0.0);
    let output_price = cfg.price_output_per_1k.unwrap_or(0.0);

    let input_cost = (input_tokens as f64 / 1000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1000.0) * output_price;

    Some(input_cost + output_cost)
}

/// Checks current daily spending against `cfg.budget_daily_usd`.
///
/// Returns [`ForgeError::AiBudgetExceeded`] if today's recorded spending meets or exceeds the cap.
pub fn enforce_daily_budget(summary: &UsageSummary, cfg: &AiConfig) -> Result<(), ForgeError> {
    if let Some(limit_usd) = cfg.budget_daily_usd {
        if let Some(spent_usd) = summary.today_cost_usd {
            if spent_usd >= limit_usd {
                return Err(ForgeError::AiBudgetExceeded {
                    spent_usd,
                    limit_usd,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_check_enforces_max_context_cap() {
        let cfg = AiConfig {
            max_context_tokens: 500,
            ..Default::default()
        };

        let pack_ok = ContextPack {
            system_prompt: "Sys".to_string(),
            user_payload: "User".to_string(),
            estimated_tokens: 350,
            included: vec!["sec".to_string()],
            dropped: Vec::new(),
            injection_signals: Vec::new(),
        };
        assert!(preflight_check(&pack_ok, &cfg).is_ok());

        let pack_over = ContextPack {
            system_prompt: "Sys".to_string(),
            user_payload: "User".to_string(),
            estimated_tokens: 800,
            included: vec!["sec".to_string()],
            dropped: Vec::new(),
            injection_signals: Vec::new(),
        };
        assert!(preflight_check(&pack_over, &cfg).is_err());
    }

    #[test]
    fn compute_cost_usd_evaluates_pricing_correctly() {
        let cfg_none = AiConfig::default();
        assert_eq!(compute_cost_usd(1000, 500, &cfg_none), None);

        let cfg_priced = AiConfig {
            price_input_per_1k: Some(0.002),
            price_output_per_1k: Some(0.006),
            ..Default::default()
        };

        let cost = compute_cost_usd(2000, 1000, &cfg_priced);
        assert!(cost.is_some());
        if let Some(val) = cost {
            // (2 * 0.002) + (1 * 0.006) = 0.004 + 0.006 = 0.010
            assert!((val - 0.010).abs() < 0.0001);
        }
    }

    #[test]
    fn enforce_daily_budget_halts_when_limit_exceeded() {
        let cfg = AiConfig {
            budget_daily_usd: Some(5.0),
            ..Default::default()
        };

        let summary_ok = UsageSummary {
            today: "2026-10-19".to_string(),
            today_cost_usd: Some(3.5),
            ..Default::default()
        };
        assert!(enforce_daily_budget(&summary_ok, &cfg).is_ok());

        let summary_exceeded = UsageSummary {
            today: "2026-10-19".to_string(),
            today_cost_usd: Some(5.1),
            ..Default::default()
        };
        let res = enforce_daily_budget(&summary_exceeded, &cfg);
        assert!(matches!(res, Err(ForgeError::AiBudgetExceeded { .. })));
    }
}
