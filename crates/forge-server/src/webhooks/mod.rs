//! Outbound webhook notification dispatcher supporting Slack, Discord, and generic HTTP endpoints.

use std::sync::Arc;
use std::time::Duration;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::{error, info, warn};

use forge_core::ir::{Finding, HealthScore, Severity};
use forge_core::sync::DeltaSummary;

use crate::db::models::WebhookRow;
use crate::db::Db;

type HmacSha256 = Hmac<Sha256>;

/// Distinct events that trigger outbound webhook notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    /// Overall health score fell below configured threshold or degraded significantly.
    HealthScoreDrop,
    /// One or more Critical severity security vulnerabilities or secrets discovered.
    CriticalVulnerability,
    /// Strong or restrictive copyleft license violation detected.
    LicenseViolation,
    /// An analysis snapshot was successfully pushed.
    SnapshotPushed,
}

impl WebhookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HealthScoreDrop => "health_score_drop",
            Self::CriticalVulnerability => "critical_vulnerability",
            Self::LicenseViolation => "license_violation",
            Self::SnapshotPushed => "snapshot_pushed",
        }
    }
}

/// Standardized outbound webhook payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: WebhookEvent,
    pub repo_id: String,
    pub branch: String,
    pub commit_hash: String,
    pub health_score: HealthScore,
    pub delta: Option<DeltaSummary>,
    pub critical_findings: Vec<Finding>,
    pub timestamp_secs: i64,
    pub message: String,
}

/// Computes an HMAC-SHA256 hexadecimal signature of `body` using `secret`.
pub fn generate_hmac_signature(secret: &str, body: &[u8]) -> String {
    if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
        mac.update(body);
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    } else {
        String::new()
    }
}

/// Verifies whether `signature_hex` matches the HMAC-SHA256 signature of `body` using `secret`.
pub fn verify_hmac_signature(secret: &str, body: &[u8], signature_hex: &str) -> bool {
    let clean_hex = signature_hex
        .strip_prefix("sha256=")
        .unwrap_or(signature_hex)
        .trim();

    if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
        mac.update(body);
        let expected = mac.finalize().into_bytes();
        if let Ok(given) = hex::decode(clean_hex) {
            return expected.as_slice() == given.as_slice();
        }
    }
    false
}

/// Formats payload for Slack incoming webhooks.
pub fn format_slack_payload(payload: &WebhookPayload) -> serde_json::Value {
    let title = format!(
        "[{}] {} ({}) — Event: {}",
        payload.event.as_str().to_uppercase(),
        payload.repo_id,
        payload.branch,
        payload.event.as_str()
    );

    serde_json::json!({
        "text": payload.message,
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": title,
                    "emoji": true
                }
            },
            {
                "type": "section",
                "fields": [
                    { "type": "mrkdwn", "text": format!("*Commit:*\n`{}`", &payload.commit_hash[..payload.commit_hash.len().min(8)]) },
                    { "type": "mrkdwn", "text": format!("*Health Score:*\n`{}/100`", payload.health_score.overall) },
                    { "type": "mrkdwn", "text": format!("*Critical Findings:*\n`{}`", payload.critical_findings.len()) },
                    { "type": "mrkdwn", "text": format!("*Status:*\n`{}`", payload.message) }
                ]
            }
        ]
    })
}

/// Formats payload for Discord webhook embeds.
pub fn format_discord_payload(payload: &WebhookPayload) -> serde_json::Value {
    let color = match payload.event {
        WebhookEvent::CriticalVulnerability => 0xE02424, // Red
        WebhookEvent::HealthScoreDrop => 0xE3A008,       // Yellow/Orange
        WebhookEvent::LicenseViolation => 0x9061F9,      // Purple
        WebhookEvent::SnapshotPushed => 0x0E9F6E,        // Green
    };

    serde_json::json!({
        "content": format!("**[ELCODE Forge Alert]** {}", payload.message),
        "embeds": [
            {
                "title": format!("{} ({})", payload.repo_id, payload.branch),
                "description": payload.message,
                "color": color,
                "fields": [
                    { "name": "Event", "value": payload.event.as_str(), "inline": true },
                    { "name": "Health Score", "value": format!("{}/100", payload.health_score.overall), "inline": true },
                    { "name": "Commit", "value": &payload.commit_hash[..payload.commit_hash.len().min(8)], "inline": true },
                    { "name": "Critical Issues", "value": payload.critical_findings.len().to_string(), "inline": true }
                ],
                "footer": {
                    "text": "ELCODE Forge Continuous Code Intelligence"
                }
            }
        ]
    })
}

/// Evaluates if a webhook subscription triggers for a given payload.
pub fn should_trigger(webhook: &WebhookRow, payload: &WebhookPayload) -> bool {
    if webhook.is_active != 1 {
        return false;
    }

    if let Some(ref r) = webhook.repo_id {
        if r != &payload.repo_id {
            return false;
        }
    }

    let events: Vec<String> = serde_json::from_str(&webhook.events_json).unwrap_or_default();
    let matches_event = events.is_empty()
        || events
            .iter()
            .any(|e| e == "*" || e == payload.event.as_str());

    if !matches_event {
        return false;
    }

    if payload.event == WebhookEvent::HealthScoreDrop {
        if let Some(threshold) = webhook.min_health_threshold {
            if payload.health_score.overall as i64 >= threshold {
                return false;
            }
        }
    }

    true
}

/// Dispatches notifications for an event to all matching registered webhooks.
pub async fn dispatch_webhooks(db: Arc<Db>, payload: WebhookPayload) {
    let webhooks = match db.list_webhooks(Some(&payload.repo_id)).await {
        Ok(list) => list,
        Err(e) => {
            warn!("Failed to query webhooks for {}: {e}", payload.repo_id);
            return;
        }
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build();

    let Ok(client) = client else {
        warn!("Failed to construct reqwest client for webhook dispatch");
        return;
    };

    for webhook in webhooks {
        if !should_trigger(&webhook, &payload) {
            continue;
        }

        let client = client.clone();
        let payload = payload.clone();
        let webhook_id = webhook.id.clone();
        let url = webhook.url.clone();
        let secret = webhook.secret.clone();

        tokio::spawn(async move {
            info!(
                "Dispatching webhook [{webhook_id}] to '{url}' for event '{:?}'",
                payload.event
            );

            let is_slack = url.contains("hooks.slack.com");
            let is_discord = url.contains("discord.com/api/webhooks")
                || url.contains("discordapp.com/api/webhooks");

            let body_bytes = if is_slack {
                match serde_json::to_vec(&format_slack_payload(&payload)) {
                    Ok(b) => b,
                    Err(_) => return,
                }
            } else if is_discord {
                match serde_json::to_vec(&format_discord_payload(&payload)) {
                    Ok(b) => b,
                    Err(_) => return,
                }
            } else {
                match serde_json::to_vec(&payload) {
                    Ok(b) => b,
                    Err(_) => return,
                }
            };

            let mut request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header(
                    "User-Agent",
                    concat!("ELCODE-Forge-Webhook/", env!("CARGO_PKG_VERSION")),
                )
                .header("X-Forge-Event", payload.event.as_str());

            if let Some(sec) = secret {
                let sig = generate_hmac_signature(&sec, &body_bytes);
                request = request.header("X-Forge-Signature", format!("sha256={sig}"));
            }

            match request.body(body_bytes).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!(
                            "Successfully delivered webhook [{webhook_id}] to '{url}' (status: {})",
                            resp.status()
                        );
                    } else {
                        warn!(
                            "Webhook delivery [{webhook_id}] to '{url}' failed with status: {}",
                            resp.status()
                        );
                    }
                }
                Err(err) => {
                    error!("Webhook delivery [{webhook_id}] to '{url}' encountered error: {err}");
                }
            }
        });
    }
}

/// Context parameters for evaluating outbound webhook events.
#[derive(Debug, Clone)]
pub struct EventEvaluationContext<'a> {
    pub repo_id: &'a str,
    pub branch: &'a str,
    pub commit_hash: &'a str,
    pub health_score: &'a HealthScore,
    pub previous_health: Option<&'a HealthScore>,
    pub findings: &'a [Finding],
    pub delta: Option<&'a DeltaSummary>,
}

/// Helper function to inspect findings and health delta and emit relevant webhook events.
pub fn evaluate_and_dispatch_events(db: Arc<Db>, ctx: EventEvaluationContext<'_>) {
    let now = chrono::Utc::now().timestamp();
    let critical_findings: Vec<_> = ctx
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .cloned()
        .collect();

    // 1. Critical vulnerability event
    if !critical_findings.is_empty() {
        let payload = WebhookPayload {
            event: WebhookEvent::CriticalVulnerability,
            repo_id: ctx.repo_id.to_string(),
            branch: ctx.branch.to_string(),
            commit_hash: ctx.commit_hash.to_string(),
            health_score: ctx.health_score.clone(),
            delta: ctx.delta.cloned(),
            critical_findings: critical_findings.clone(),
            timestamp_secs: now,
            message: format!(
                "Discovered {} critical severity security finding(s) in {} ({})",
                critical_findings.len(),
                ctx.repo_id,
                ctx.branch
            ),
        };
        let db_clone = Arc::clone(&db);
        tokio::spawn(async move {
            dispatch_webhooks(db_clone, payload).await;
        });
    }

    // 2. Health score drop event
    let health_dropped = if let Some(prev) = ctx.previous_health {
        ctx.health_score.overall < prev.overall && (prev.overall - ctx.health_score.overall) >= 5
    } else {
        ctx.health_score.overall < 80
    };

    if health_dropped {
        let prev_overall = ctx.previous_health.map(|h| h.overall).unwrap_or(100);
        let payload = WebhookPayload {
            event: WebhookEvent::HealthScoreDrop,
            repo_id: ctx.repo_id.to_string(),
            branch: ctx.branch.to_string(),
            commit_hash: ctx.commit_hash.to_string(),
            health_score: ctx.health_score.clone(),
            delta: ctx.delta.cloned(),
            critical_findings: critical_findings.clone(),
            timestamp_secs: now,
            message: format!(
                "Codebase health dropped from {}/100 to {}/100 in {} ({})",
                prev_overall, ctx.health_score.overall, ctx.repo_id, ctx.branch
            ),
        };
        let db_clone = Arc::clone(&db);
        tokio::spawn(async move {
            dispatch_webhooks(db_clone, payload).await;
        });
    }

    // 3. License copyleft violation event
    let license_violations: Vec<_> = ctx
        .findings
        .iter()
        .filter(|f| {
            f.category == "license"
                || f.rule_id.contains("license")
                || f.rule_id.contains("copyleft")
        })
        .cloned()
        .collect();

    if !license_violations.is_empty() {
        let payload = WebhookPayload {
            event: WebhookEvent::LicenseViolation,
            repo_id: ctx.repo_id.to_string(),
            branch: ctx.branch.to_string(),
            commit_hash: ctx.commit_hash.to_string(),
            health_score: ctx.health_score.clone(),
            delta: ctx.delta.cloned(),
            critical_findings: Vec::new(),
            timestamp_secs: now,
            message: format!(
                "Detected {} license compliance issue(s) in {} ({})",
                license_violations.len(),
                ctx.repo_id,
                ctx.branch
            ),
        };
        let db_clone = Arc::clone(&db);
        tokio::spawn(async move {
            dispatch_webhooks(db_clone, payload).await;
        });
    }

    // 4. Always emit generic snapshot push event
    let payload = WebhookPayload {
        event: WebhookEvent::SnapshotPushed,
        repo_id: ctx.repo_id.to_string(),
        branch: ctx.branch.to_string(),
        commit_hash: ctx.commit_hash.to_string(),
        health_score: ctx.health_score.clone(),
        delta: ctx.delta.cloned(),
        critical_findings,
        timestamp_secs: now,
        message: format!(
            "Analysis snapshot successfully ingested for {} ({}) at commit {}",
            ctx.repo_id,
            ctx.branch,
            &ctx.commit_hash[..ctx.commit_hash.len().min(8)]
        ),
    };
    let db_clone = Arc::clone(&db);
    tokio::spawn(async move {
        dispatch_webhooks(db_clone, payload).await;
    });
}
