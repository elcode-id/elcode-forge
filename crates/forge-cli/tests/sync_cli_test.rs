use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use forge_cli::commands::auth::{run_auth_login, run_auth_logout, run_auth_status};
use forge_cli::commands::sync::{
    baseline_path, load_cached_baseline, run_sync_pipeline, run_sync_pull, run_sync_push,
};
use forge_cli::exit_code::ExitCode;
use forge_cli::OutputFormat;
use forge_core::sync::{SyncPullResponse, SyncStatus};

fn spawn_sync_mock_server<F>(handler: F, running: Arc<AtomicBool>) -> String
where
    F: Fn(tiny_http::Request) + Send + 'static,
{
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(_) => return "http://127.0.0.1:0".to_string(),
    };
    let server = match tiny_http::Server::from_listener(listener, None) {
        Ok(s) => s,
        Err(_) => return "http://127.0.0.1:0".to_string(),
    };
    let port = match server.server_addr().to_ip() {
        Some(a) => a.port(),
        None => 0,
    };
    let base_url = format!("http://127.0.0.1:{}", port);

    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            match server.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(Some(req)) => handler(req),
                Ok(None) => {}
                Err(_) => break,
            }
        }
    });

    base_url
}

fn create_test_repo(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eforge_cli_test_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(dir.join("src"));

    let config = "version: 1\nproject_name: test-sync-repo\nlanguages: [\"rust\"]\n";
    let _ = fs::write(dir.join("eforge.config.yaml"), config);
    let _ = fs::write(
        dir.join("src/lib.rs"),
        "pub fn hello() -> &'static str { \"hello\" }\n",
    );

    dir
}

#[test]
fn test_auth_login_logout_and_status() {
    let creds_file = std::env::temp_dir().join(format!("creds_{}.yaml", std::process::id()));
    std::env::set_var("EFORGE_CREDENTIALS_PATH", &creds_file);

    let running = Arc::new(AtomicBool::new(true));
    let server_url = spawn_sync_mock_server(
        |req| {
            let auth = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str());

            if auth == Some("Bearer valid_token_12345") {
                let resp =
                    tiny_http::Response::from_string("{\"status\":\"ok\"}").with_status_code(200);
                let _ = req.respond(resp);
            } else {
                let resp = tiny_http::Response::from_string("Unauthorized").with_status_code(401);
                let _ = req.respond(resp);
            }
        },
        Arc::clone(&running),
    );

    // 1. Successful login
    let exit_login = run_auth_login(
        &server_url,
        Some("valid_token_12345"),
        Some("tester"),
        false,
    );
    assert_eq!(exit_login, ExitCode::Success);

    // 2. Status inspection
    let exit_status = run_auth_status(Some(&server_url), OutputFormat::Json);
    assert_eq!(exit_status, ExitCode::Success);

    // 3. Status inspection terminal
    let exit_status_term = run_auth_status(Some(&server_url), OutputFormat::Terminal);
    assert_eq!(exit_status_term, ExitCode::Success);

    // 4. Logout
    let exit_logout = run_auth_logout(Some(&server_url));
    assert_eq!(exit_logout, ExitCode::Success);

    running.store(false, Ordering::SeqCst);
    let _ = fs::remove_file(&creds_file);
    std::env::remove_var("EFORGE_CREDENTIALS_PATH");
}

#[test]
fn test_sync_push_and_pull_lifecycle() {
    let repo_dir = create_test_repo("push_pull");
    let creds_file = repo_dir.join("creds.yaml");
    std::env::set_var("EFORGE_CREDENTIALS_PATH", &creds_file);

    let running = Arc::new(AtomicBool::new(true));
    let server_url = spawn_sync_mock_server(
        |mut req| {
            let url = req.url().to_string();
            if url.starts_with("/api/v1/sync/push") {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let resp =
                    tiny_http::Response::from_string("{\"status\":\"ok\"}").with_status_code(200);
                let _ = req.respond(resp);
            } else if url.starts_with("/api/v1/sync/pull") {
                let mock_pull = SyncPullResponse {
                    repo_id: "test-sync-repo".to_string(),
                    branch: "main".to_string(),
                    baseline_commit_hash: Some("mock_commit_111".to_string()),
                    baseline_health_score: None,
                    baseline_findings: Vec::new(),
                    baseline_content_hash: Some("mock_content_hash_222".to_string()),
                    sync_status: SyncStatus::UpToDate,
                };
                let body = serde_json::to_string(&mock_pull).unwrap_or_default();
                let mut resp = tiny_http::Response::from_string(body).with_status_code(200);
                if let Ok(h) =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                {
                    resp.add_header(h);
                }
                let _ = req.respond(resp);
            } else {
                let _ = req
                    .respond(tiny_http::Response::from_string("Not Found").with_status_code(404));
            }
        },
        Arc::clone(&running),
    );

    // Test Push
    let push_code = run_sync_push(
        &repo_dir,
        Some(&server_url),
        Some("test_token"),
        Some("main"),
        OutputFormat::Terminal,
    );
    assert_eq!(push_code, ExitCode::Success);

    // Baseline cache must exist after push
    assert!(baseline_path(&repo_dir).exists());
    let cached = load_cached_baseline(&repo_dir);
    assert!(cached.is_ok());
    let cached_opt = cached.unwrap_or(None);
    assert!(cached_opt.is_some());
    if let Some(base) = cached_opt {
        assert_eq!(base.repo_id, "test-sync-repo");
    }

    // Test Pull
    let pull_code = run_sync_pull(
        &repo_dir,
        Some(&server_url),
        Some("test_token"),
        Some("main"),
        OutputFormat::Json,
    );
    assert_eq!(pull_code, ExitCode::Success);

    running.store(false, Ordering::SeqCst);
    let _ = fs::remove_dir_all(&repo_dir);
    std::env::remove_var("EFORGE_CREDENTIALS_PATH");
}

#[test]
fn test_sync_pipeline_end_to_end() {
    let repo_dir = create_test_repo("pipeline");
    let creds_file = repo_dir.join("creds.yaml");
    std::env::set_var("EFORGE_CREDENTIALS_PATH", &creds_file);

    let running = Arc::new(AtomicBool::new(true));
    let server_url = spawn_sync_mock_server(
        |mut req| {
            let url = req.url().to_string();
            if url.starts_with("/api/v1/sync/pull") {
                let mock_pull = SyncPullResponse {
                    repo_id: "test-sync-repo".to_string(),
                    branch: "main".to_string(),
                    baseline_commit_hash: Some("sha_base".to_string()),
                    baseline_health_score: None,
                    baseline_findings: Vec::new(),
                    baseline_content_hash: Some("hash_base".to_string()),
                    sync_status: SyncStatus::UpToDate,
                };
                let body = serde_json::to_string(&mock_pull).unwrap_or_default();
                let resp = tiny_http::Response::from_string(body).with_status_code(200);
                let _ = req.respond(resp);
            } else if url.starts_with("/api/v1/sync/push") {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let resp =
                    tiny_http::Response::from_string("{\"status\":\"ok\"}").with_status_code(200);
                let _ = req.respond(resp);
            } else {
                let _ = req
                    .respond(tiny_http::Response::from_string("Not Found").with_status_code(404));
            }
        },
        Arc::clone(&running),
    );

    let sync_code = run_sync_pipeline(
        &repo_dir,
        Some(&server_url),
        Some("tok_abc"),
        Some("main"),
        Some("critical"),
        OutputFormat::Terminal,
    );
    assert_eq!(sync_code, ExitCode::Success);

    running.store(false, Ordering::SeqCst);
    let _ = fs::remove_dir_all(&repo_dir);
    std::env::remove_var("EFORGE_CREDENTIALS_PATH");
}
