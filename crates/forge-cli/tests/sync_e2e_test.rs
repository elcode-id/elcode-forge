//! Comprehensive end-to-end integration test validating the CLI synchronization workflow
//! against an actual running `forge-server` instance.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_cli::commands::auth::{run_auth_login, run_auth_logout, run_auth_status};
use forge_cli::commands::sync::{
    baseline_path, load_cached_baseline, run_sync_pipeline, run_sync_pull, run_sync_push,
};
use forge_cli::exit_code::ExitCode;
use forge_cli::OutputFormat;
use forge_core::sync::SyncStatus;
use forge_server::{create_app, hash_api_key, AppState, Db, ServerConfig};

fn create_e2e_test_repo(label: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let dir =
        std::env::temp_dir().join(format!("eforge_e2e_repo_{}_{}", label, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src"))?;

    let config = "version: 1\nproject_name: e2e-sync-repo\nlanguages: [\"rust\"]\n";
    fs::write(dir.join("eforge.config.yaml"), config)?;
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn core_logic() -> i32 { 42 }\n",
    )?;

    Ok(dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cli_and_server_sync_e2e_lifecycle() -> Result<(), Box<dyn Error>> {
    // 1. Initialize in-memory forge-server database and configuration
    let db = Db::connect_in_memory().await?;
    let admin_key = "e2e-admin-secret-key-67890";
    let key_hash = hash_api_key(admin_key);
    let now = chrono::Utc::now().timestamp();
    db.create_api_key("admin_key_1", "admin", &key_hash, now, None)
        .await?;

    let config = ServerConfig {
        admin_api_key: Some(admin_key.to_string()),
        jwt_secret: "super-secret-jwt-signing-key-for-e2e-testing-32chars".to_string(),
        ..Default::default()
    };

    let state = AppState::new(db, config);
    let app = create_app(state);

    // 2. Bind ephemeral TCP port and start background Axum server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let server_url = format!("http://127.0.0.1:{}", local_addr.port());

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // Allow the server task a brief instant to start listening
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 3. Configure temporary credentials store path
    let creds_file =
        std::env::temp_dir().join(format!("eforge_e2e_creds_{}.yaml", std::process::id()));
    std::env::set_var("EFORGE_CREDENTIALS_PATH", &creds_file);

    // 4. Run blocking CLI operations on dedicated worker thread
    let server_url_clone = server_url.clone();
    let creds_file_clone = creds_file.clone();
    tokio::task::spawn_blocking(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        // Test eforge login against running forge-server
        let login_exit = run_auth_login(&server_url_clone, Some(admin_key), Some("admin"), false);
        assert_eq!(login_exit, ExitCode::Success);

        // Test eforge auth status
        let status_exit = run_auth_status(Some(&server_url_clone), OutputFormat::Json);
        assert_eq!(status_exit, ExitCode::Success);

        // Create temporary target repository and execute eforge push
        let repo_dir = create_e2e_test_repo("sync_run")?;
        let push_exit = run_sync_push(
            &repo_dir,
            Some(&server_url_clone),
            None,
            Some("main"),
            OutputFormat::Json,
        );
        assert_eq!(push_exit, ExitCode::Success);

        // Verify baseline cache was populated
        let b_path = baseline_path(&repo_dir);
        assert!(b_path.exists());
        let baseline_opt = load_cached_baseline(&repo_dir).map_err(|e| e.to_string())?;
        assert!(baseline_opt.is_some());
        if let Some(b) = baseline_opt {
            assert_eq!(b.sync_status, SyncStatus::UpToDate);
            assert_eq!(b.branch, "main");
            assert!(b.baseline_commit_hash.is_some());
        }

        // Test eforge pull
        let pull_exit = run_sync_pull(
            &repo_dir,
            Some(&server_url_clone),
            None,
            Some("main"),
            OutputFormat::Json,
        );
        assert_eq!(pull_exit, ExitCode::Success);

        // Modify repository and execute full eforge sync pipeline
        fs::write(
            repo_dir.join("src/lib.rs"),
            "pub fn core_logic() -> i32 { 100 }\npub fn additional() -> &'static str { \"ok\" }\n",
        )?;

        let sync_exit = run_sync_pipeline(
            &repo_dir,
            Some(&server_url_clone),
            None,
            Some("main"),
            None,
            OutputFormat::Json,
        );
        assert_eq!(sync_exit, ExitCode::Success);

        // Test eforge logout
        let logout_exit = run_auth_logout(Some(&server_url_clone));
        assert_eq!(logout_exit, ExitCode::Success);

        let _ = fs::remove_dir_all(&repo_dir);
        let _ = fs::remove_file(&creds_file_clone);

        Ok(())
    })
    .await?
    .map_err(|e| -> Box<dyn Error> { e })?;

    Ok(())
}
