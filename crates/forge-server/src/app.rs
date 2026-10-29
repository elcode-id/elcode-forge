//! Axum application router and server runner.

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::auth::hash_api_key;
use crate::config::ServerConfig;
use crate::db::Db;
use crate::error::ServerError;
use crate::routes::{
    auth::token_handler,
    health::health_handler,
    history::project_history_handler,
    orgs::{
        create_org_handler, get_org_summary_handler, get_org_vulnerabilities_handler,
        link_project_handler, list_orgs_handler,
    },
    projects::{list_projects_handler, project_branches_handler, project_overview_handler},
    sync::{pull_handler, push_handler},
    webhooks::{create_webhook_handler, delete_webhook_handler, list_webhooks_handler},
};
use crate::state::AppState;

/// Constructs the primary Axum router.
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/auth/token", post(token_handler))
        .route("/api/v1/sync/push", post(push_handler))
        .route("/api/v1/sync/pull", get(pull_handler))
        .route("/api/v1/projects", get(list_projects_handler))
        .route(
            "/api/v1/projects/:repo_id/overview",
            get(project_overview_handler),
        )
        .route(
            "/api/v1/projects/:repo_id/branches",
            get(project_branches_handler),
        )
        .route(
            "/api/v1/projects/:repo_id/history",
            get(project_history_handler),
        )
        .route(
            "/api/v1/orgs",
            post(create_org_handler).get(list_orgs_handler),
        )
        .route("/api/v1/orgs/:id/projects", post(link_project_handler))
        .route("/api/v1/orgs/:id/summary", get(get_org_summary_handler))
        .route(
            "/api/v1/orgs/:id/vulnerabilities",
            get(get_org_vulnerabilities_handler),
        )
        .route(
            "/api/v1/webhooks",
            post(create_webhook_handler).get(list_webhooks_handler),
        )
        .route(
            "/api/v1/webhooks/:id",
            axum::routing::delete(delete_webhook_handler),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Graceful shutdown listener on Ctrl+C.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received termination signal; starting graceful shutdown...");
}

/// Connects database, runs migrations, and starts the HTTP server.
pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    info!(
        "Connecting to database backend at '{}'...",
        config.database_url
    );
    let db = Db::connect(&config.database_url).await?;
    info!("Running database schema migrations...");
    db.migrate().await?;

    if let Some(admin_key) = &config.admin_api_key {
        let key_hash = hash_api_key(admin_key);
        let now = chrono::Utc::now().timestamp();
        let _ = db
            .create_api_key("admin_boot_key", "admin", &key_hash, now, None)
            .await;
        info!("Bootstrapped administrator API key successfully.");
    }

    let bind_addr = config.socket_addr();
    let state = AppState::new(db, config);
    let app = create_app(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to bind on {bind_addr}: {e}")))?;

    info!("ELCODE Forge Server listening on http://{bind_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| ServerError::Internal(format!("Server execution failed: {e}")))?;

    info!("ELCODE Forge Server stopped cleanly.");
    Ok(())
}
