//! `forge-server` binary entrypoint.

use std::process;
use tracing::error;
use tracing_subscriber::EnvFilter;

use forge_server::{run_server, ServerConfig};

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("forge_server=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    let config = match ServerConfig::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("Configuration error: {err}");
            process::exit(1);
        }
    };

    if let Err(err) = run_server(config).await {
        error!("Fatal server error: {err}");
        process::exit(1);
    }
}
