//! `eforge push`, `eforge pull`, and `eforge sync` CLI command implementations.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge_core::cache::EFORGE_DIR;
use forge_core::scoring::calculate_health_score;
use forge_core::sync::{
    compute_sync_content_hash, DeltaSummary, SyncManifest, SyncPullResponse, SyncPushPayload,
    SyncStatus,
};
use forge_core::ForgeError;

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::exit_code::{parse_severity_threshold, ExitCode};

pub const BASELINE_FILE: &str = "baseline.json";

/// Returns the path to the local cached baseline snapshot (`<repo_root>/.eforge/baseline.json`).
pub fn baseline_path(repo_root: &Path) -> PathBuf {
    repo_root.join(EFORGE_DIR).join(BASELINE_FILE)
}

/// Saves remote pull response into the local baseline cache.
pub fn save_cached_baseline(
    repo_root: &Path,
    response: &SyncPullResponse,
) -> Result<(), ForgeError> {
    let path = baseline_path(repo_root);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    let json_text =
        serde_json::to_string_pretty(response).map_err(|e| ForgeError::Serialization {
            message: format!("failed to serialize baseline response: {e}"),
        })?;
    fs::write(&path, json_text).map_err(|source| ForgeError::Io { path, source })?;
    Ok(())
}

/// Loads cached baseline response from `<repo_root>/.eforge/baseline.json`, if it exists.
pub fn load_cached_baseline(repo_root: &Path) -> Result<Option<SyncPullResponse>, ForgeError> {
    let path = baseline_path(repo_root);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|source| ForgeError::Io {
        path: path.clone(),
        source,
    })?;
    let resp: SyncPullResponse =
        serde_json::from_str(&text).map_err(|e| ForgeError::Serialization {
            message: format!(
                "failed to deserialize baseline JSON from {}: {e}",
                path.display()
            ),
        })?;
    Ok(Some(resp))
}

/// Resolves effective target server URL and active API authentication token.
pub fn resolve_sync_server_and_token(
    server: Option<&str>,
    token: Option<&str>,
) -> Result<(String, String), ForgeError> {
    let config = forge_core::sync::load_credentials().unwrap_or_default();
    let target_server = server
        .map(|s| s.trim_end_matches('/').to_string())
        .or_else(|| config.default_server.clone())
        .unwrap_or_else(|| "https://forge.elcode.id".to_string());

    let active_token = token
        .map(|t| t.to_string())
        .or_else(|| forge_core::sync::resolve_token(Some(&target_server)))
        .or_else(|| forge_core::sync::resolve_token(None));

    let resolved_token = match active_token {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return Err(ForgeError::Sync {
                message: format!(
                    "authentication required for '{target_server}'. Run 'eforge login' or set EFORGE_API_TOKEN"
                ),
            });
        }
    };

    Ok((target_server, resolved_token))
}

/// Infers repository ID from config project_name or root directory name.
pub fn resolve_repo_id(repo_root: &Path, config_name: Option<&str>) -> String {
    if let Some(name) = config_name {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    if let Ok(canonical) = repo_root.canonicalize() {
        if let Some(file_name) = canonical.file_name() {
            return file_name.to_string_lossy().to_string();
        }
    }
    "repo".to_string()
}

/// Resolves active Git branch or falls back to "main".
pub fn resolve_branch(repo_root: &Path, explicit_branch: Option<&str>) -> String {
    if let Some(b) = explicit_branch {
        if !b.trim().is_empty() {
            return b.trim().to_string();
        }
    }
    if repo_root.join(".git").exists() {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo_root)
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() && text != "HEAD" {
                    return text;
                }
            }
        }
    }
    "main".to_string()
}

/// Fetches remote baseline from Forge Server and caches locally.
pub fn run_sync_pull(
    repo_root: &Path,
    server: Option<&str>,
    token: Option<&str>,
    branch: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let (target_server, resolved_token) = match resolve_sync_server_and_token(server, token) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::ConfigurationError;
        }
    };

    let forge_cfg = forge_core::config::load_config(repo_root).unwrap_or_default();
    let repo_id = resolve_repo_id(repo_root, forge_cfg.project_name.as_deref());
    let target_branch = resolve_branch(repo_root, branch);

    let url = format!(
        "{}/api/v1/sync/pull?repo_id={}&branch={}",
        target_server.trim_end_matches('/'),
        urlencode(&repo_id),
        urlencode(&target_branch)
    );

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();

    let pull_resp: SyncPullResponse = match agent
        .get(&url)
        .set("Authorization", &format!("Bearer {resolved_token}"))
        .call()
    {
        Ok(resp) => match resp.into_json() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Error parsing pull response from server: {e}");
                return ExitCode::ExecutionError;
            }
        },
        Err(ureq::Error::Status(404, _)) => {
            let not_found_resp = SyncPullResponse {
                repo_id: repo_id.clone(),
                branch: target_branch.clone(),
                baseline_commit_hash: None,
                baseline_health_score: None,
                baseline_findings: Vec::new(),
                baseline_content_hash: None,
                sync_status: SyncStatus::NotFound,
            };
            if let Err(e) = save_cached_baseline(repo_root, &not_found_resp) {
                eprintln!("Warning: Could not save baseline cache: {e}");
            }
            if format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&not_found_resp).unwrap_or_default()
                );
            } else {
                println!(
                    "Notice: No remote baseline snapshot found for {repo_id}@{target_branch} (status: not_found)."
                );
            }
            return ExitCode::Success;
        }
        Err(err) => {
            eprintln!("Error connecting to Forge Server ({target_server}): {err}");
            return ExitCode::ExecutionError;
        }
    };

    if let Err(e) = save_cached_baseline(repo_root, &pull_resp) {
        eprintln!("Warning: Could not save baseline cache: {e}");
    }

    if format == OutputFormat::Json {
        println!(
            "{}",
            serde_json::to_string_pretty(&pull_resp).unwrap_or_default()
        );
        return ExitCode::Success;
    }

    println!("✓ Successfully retrieved remote baseline");
    println!("  Repository:       {repo_id}");
    println!("  Branch:           {target_branch}");
    println!("  Sync Status:      {:?}", pull_resp.sync_status);
    if let Some(ref sha) = pull_resp.baseline_commit_hash {
        println!("  Baseline Commit:  {sha}");
    }
    if let Some(ref health) = pull_resp.baseline_health_score {
        println!("  Baseline Health:  {}/100", health.overall);
    }
    println!("  Baseline Findings: {}", pull_resp.baseline_findings.len());
    println!("  Cached to:        .eforge/baseline.json");

    ExitCode::Success
}

/// Uploads current local repository analysis snapshot to Forge Server.
pub fn run_sync_push(
    repo_root: &Path,
    server: Option<&str>,
    token: Option<&str>,
    branch: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let (target_server, resolved_token) = match resolve_sync_server_and_token(server, token) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::ConfigurationError;
        }
    };

    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo_ir, _) = match execute_scan(&scan_opts) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error executing repository scan for push: {e}");
            return ExitCode::ExecutionError;
        }
    };

    let target_branch = resolve_branch(repo_root, branch);
    let head_rev = forge_git::log::read_head_revision(repo_root).unwrap_or(None);
    let commit_hash = head_rev
        .as_ref()
        .map(|r| r.head_sha.clone())
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
    let timestamp_secs = head_rev
        .as_ref()
        .map(|r| r.commit_unix_ts as u64)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });

    let health_score = repo_ir
        .health
        .clone()
        .unwrap_or_else(|| calculate_health_score(&repo_ir));
    let content_hash = compute_sync_content_hash(&repo_ir.files, &repo_ir.findings);

    let manifest = SyncManifest {
        repo_id: repo_ir.name.clone(),
        repo_name: repo_ir.name.clone(),
        branch: target_branch.clone(),
        commit_hash: commit_hash.clone(),
        timestamp_secs,
        content_hash: content_hash.clone(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let payload = SyncPushPayload {
        manifest,
        findings_count: repo_ir.findings.len(),
        health_score: health_score.clone(),
        findings: repo_ir.findings.clone(),
        modules: repo_ir.modules.clone(),
        metadata: BTreeMap::from([
            ("pushed_by".to_string(), "eforge-cli".to_string()),
            ("os".to_string(), std::env::consts::OS.to_string()),
        ]),
    };

    let url = format!("{}/api/v1/sync/push", target_server.trim_end_matches('/'));
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let resp_res = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {resolved_token}"))
        .send_json(&payload);

    match resp_res {
        Ok(_) => {
            let baseline = SyncPullResponse {
                repo_id: repo_ir.name.clone(),
                branch: target_branch.clone(),
                baseline_commit_hash: Some(commit_hash.clone()),
                baseline_health_score: Some(health_score.clone()),
                baseline_findings: repo_ir.findings.clone(),
                baseline_content_hash: Some(content_hash.clone()),
                sync_status: SyncStatus::UpToDate,
            };
            let _ = save_cached_baseline(repo_root, &baseline);

            if format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "success",
                        "repo_id": repo_ir.name,
                        "branch": target_branch,
                        "commit_hash": commit_hash,
                        "content_hash": content_hash,
                        "health_score": health_score.overall,
                        "findings_count": repo_ir.findings.len(),
                    }))
                    .unwrap_or_default()
                );
                return ExitCode::Success;
            }

            println!("✓ Pushed snapshot successfully to {target_server}");
            println!("  Repository:     {}", repo_ir.name);
            println!("  Branch:         {target_branch}");
            println!("  Commit:         {commit_hash}");
            println!("  Content Hash:   {content_hash}");
            println!("  Health Score:   {}/100", health_score.overall);
            println!("  Findings Count: {}", repo_ir.findings.len());
            println!("  Baseline Cache: Updated (.eforge/baseline.json)");

            ExitCode::Success
        }
        Err(err) => {
            eprintln!("Error pushing snapshot to Forge Server ({target_server}): {err}");
            ExitCode::ExecutionError
        }
    }
}

/// Synchronizes repository state with remote Forge Server in CI or local pipeline.
pub fn run_sync_pipeline(
    repo_root: &Path,
    server: Option<&str>,
    token: Option<&str>,
    branch: Option<&str>,
    fail_on: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let (target_server, resolved_token) = match resolve_sync_server_and_token(server, token) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::ConfigurationError;
        }
    };

    let target_branch = resolve_branch(repo_root, branch);
    let forge_cfg = forge_core::config::load_config(repo_root).unwrap_or_default();
    let repo_id = resolve_repo_id(repo_root, forge_cfg.project_name.as_deref());

    // 1. Try pulling remote baseline (or fallback to cached baseline)
    let url = format!(
        "{}/api/v1/sync/pull?repo_id={}&branch={}",
        target_server.trim_end_matches('/'),
        urlencode(&repo_id),
        urlencode(&target_branch)
    );

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();

    let baseline_opt: Option<SyncPullResponse> = match agent
        .get(&url)
        .set("Authorization", &format!("Bearer {resolved_token}"))
        .call()
    {
        Ok(resp) => resp.into_json().ok(),
        Err(_) => load_cached_baseline(repo_root).unwrap_or(None),
    };

    // 2. Scan current repository state
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo_ir, _) = match execute_scan(&scan_opts) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error scanning repository for sync: {e}");
            return ExitCode::ExecutionError;
        }
    };

    let head_rev = forge_git::log::read_head_revision(repo_root).unwrap_or(None);
    let commit_hash = head_rev
        .as_ref()
        .map(|r| r.head_sha.clone())
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
    let timestamp_secs = head_rev
        .as_ref()
        .map(|r| r.commit_unix_ts as u64)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });

    let health_score = repo_ir
        .health
        .clone()
        .unwrap_or_else(|| calculate_health_score(&repo_ir));
    let content_hash = compute_sync_content_hash(&repo_ir.files, &repo_ir.findings);

    // 3. Compute DeltaSummary against baseline
    let empty_findings = Vec::new();
    let baseline_findings = baseline_opt
        .as_ref()
        .map(|b| &b.baseline_findings)
        .unwrap_or(&empty_findings);
    let baseline_health = baseline_opt
        .as_ref()
        .and_then(|b| b.baseline_health_score.as_ref());

    let delta = DeltaSummary::compute(
        &repo_ir.findings,
        baseline_findings,
        &health_score,
        baseline_health,
    );

    // 4. Push new snapshot
    let manifest = SyncManifest {
        repo_id: repo_id.clone(),
        repo_name: repo_ir.name.clone(),
        branch: target_branch.clone(),
        commit_hash: commit_hash.clone(),
        timestamp_secs,
        content_hash: content_hash.clone(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let payload = SyncPushPayload {
        manifest,
        findings_count: repo_ir.findings.len(),
        health_score: health_score.clone(),
        findings: repo_ir.findings.clone(),
        modules: repo_ir.modules.clone(),
        metadata: BTreeMap::from([
            ("pushed_by".to_string(), "eforge-sync".to_string()),
            ("ci".to_string(), "true".to_string()),
        ]),
    };

    let push_url = format!("{}/api/v1/sync/push", target_server.trim_end_matches('/'));
    let push_res = agent
        .post(&push_url)
        .set("Authorization", &format!("Bearer {resolved_token}"))
        .send_json(&payload);

    if let Err(err) = push_res {
        eprintln!("Warning: Failed to push snapshot to Forge Server ({target_server}): {err}");
    } else {
        let baseline = SyncPullResponse {
            repo_id: repo_id.clone(),
            branch: target_branch.clone(),
            baseline_commit_hash: Some(commit_hash.clone()),
            baseline_health_score: Some(health_score.clone()),
            baseline_findings: repo_ir.findings.clone(),
            baseline_content_hash: Some(content_hash.clone()),
            sync_status: SyncStatus::UpToDate,
        };
        let _ = save_cached_baseline(repo_root, &baseline);
    }

    if format == OutputFormat::Json {
        let out_json = serde_json::json!({
            "repo_id": repo_id,
            "branch": target_branch,
            "commit_hash": commit_hash,
            "health_score": health_score.overall,
            "delta": delta,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out_json).unwrap_or_default()
        );
    } else {
        println!("ELCODE Forge Remote Sync Pipeline:");
        println!("  Repository:       {repo_id}@{target_branch}");
        println!("  Commit:           {commit_hash}");
        println!(
            "  Health Score:     {}/100 (delta: {:+})",
            health_score.overall, delta.health_score_delta
        );
        println!(
            "  Delta Findings:   +{} added, -{} resolved, {} unchanged",
            delta.added_findings, delta.resolved_findings, delta.unchanged_findings
        );
        println!(
            "  Clean Status:     {}",
            if delta.is_clean {
                "PASS ✓"
            } else {
                "DEGRADED"
            }
        );
    }

    // 5. Check fail_on gate threshold
    if let Some(threshold_str) = fail_on {
        if let Some(threshold_sev) = parse_severity_threshold(threshold_str) {
            let newly_added: Vec<_> = repo_ir
                .findings
                .iter()
                .filter(|f| !baseline_findings.iter().any(|b| b.id == f.id))
                .cloned()
                .collect();
            let eval_exit = ExitCode::evaluate_findings(&newly_added, Some(threshold_sev));
            if eval_exit != ExitCode::Success {
                eprintln!(
                    "Policy violation: Newly introduced findings meet or exceed fail-on threshold {:?}",
                    threshold_sev
                );
                return eval_exit;
            }
        }
    }

    ExitCode::Success
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}
