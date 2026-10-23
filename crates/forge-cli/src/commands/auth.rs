//! `eforge login`, `eforge logout`, and `eforge auth status` CLI commands.

use forge_core::sync::{load_credentials, save_credentials, CredentialsConfig, ServerCredential};

use crate::cli::OutputFormat;
use crate::exit_code::ExitCode;

/// Authenticates with a remote ELCODE Forge Server.
pub fn run_auth_login(
    server: &str,
    token: Option<&str>,
    username: Option<&str>,
    no_validate: bool,
) -> ExitCode {
    let normalized_server = server.trim_end_matches('/');

    let active_token = match token {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => match std::env::var("EFORGE_API_TOKEN") {
            Ok(env_tok) if !env_tok.trim().is_empty() => env_tok.trim().to_string(),
            _ => {
                eprintln!(
                    "Error: API token required. Provide --token <token> or set EFORGE_API_TOKEN environment variable."
                );
                return ExitCode::ConfigurationError;
            }
        },
    };

    if !no_validate {
        let health_url = format!("{normalized_server}/api/v1/health");
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        match agent
            .get(&health_url)
            .set("Authorization", &format!("Bearer {active_token}"))
            .call()
        {
            Ok(_) => {}
            Err(ureq::Error::Status(401, _)) => {
                eprintln!(
                    "Authentication rejected: Server at '{normalized_server}' returned 401 Unauthorized for the provided token."
                );
                return ExitCode::ExecutionError;
            }
            Err(err) => {
                eprintln!(
                    "Notice: Could not validate health endpoint at '{health_url}' ({err}). Saving credentials anyway."
                );
            }
        }
    }

    let mut config = match load_credentials() {
        Ok(c) => c,
        Err(_) => CredentialsConfig::new(),
    };

    config.set_credential(ServerCredential {
        server_url: normalized_server.to_string(),
        token: active_token,
        username: username.map(|s| s.trim().to_string()),
        expires_at: None,
    });

    if let Err(err) = save_credentials(&config) {
        eprintln!("Error saving credentials: {err}");
        return ExitCode::ExecutionError;
    }

    println!("✓ Successfully authenticated with {normalized_server}");
    if let Some(user) = username {
        println!("  User handle: {user}");
    }
    println!("  Credentials stored securely in ~/.eforge/credentials.yaml");

    ExitCode::Success
}

/// Logs out and removes stored credentials for a remote server.
pub fn run_auth_logout(server: Option<&str>) -> ExitCode {
    let mut config = load_credentials().unwrap_or_default();

    let target = match server {
        Some(s) => Some(s.trim_end_matches('/').to_string()),
        None => config.default_server.clone(),
    };

    match target {
        Some(ref s) => {
            if config.remove_credential(s).is_some() {
                if let Err(err) = save_credentials(&config) {
                    eprintln!("Error updating credentials: {err}");
                    return ExitCode::ExecutionError;
                }
                println!("✓ Successfully logged out from {s}");
                ExitCode::Success
            } else {
                eprintln!("No stored credentials found for {s}");
                ExitCode::ExecutionError
            }
        }
        None => {
            eprintln!("Error: No server specified and no default credentials found.");
            ExitCode::ExecutionError
        }
    }
}

/// Displays active authentication and token status.
pub fn run_auth_status(server: Option<&str>, format: OutputFormat) -> ExitCode {
    let env_token = std::env::var("EFORGE_API_TOKEN").ok();
    let config = load_credentials().unwrap_or_default();

    let target_server = match server {
        Some(s) => s.trim_end_matches('/').to_string(),
        None => config
            .default_server
            .clone()
            .unwrap_or_else(|| "https://forge.elcode.id".to_string()),
    };

    let active_token = forge_core::sync::resolve_token(Some(&target_server));
    let token_source = if env_token.is_some() {
        "EFORGE_API_TOKEN (environment variable)"
    } else if config.get_credential(&target_server).is_some() {
        "~/.eforge/credentials.yaml"
    } else {
        "none"
    };

    let masked = active_token.as_deref().map(mask_token);

    if format == OutputFormat::Json {
        let json_val = serde_json::json!({
            "target_server": target_server,
            "authenticated": masked.is_some(),
            "token_source": token_source,
            "masked_token": masked,
            "configured_servers": config.servers.keys().collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json_val).unwrap_or_default()
        );
        return ExitCode::Success;
    }

    println!("ELCODE Forge Authentication Status:");
    println!("  Target Server:    {target_server}");
    println!("  Credential Store: {token_source}");
    if let Some(ref m) = masked {
        println!("  Active Token:     {m}");
        println!("  Status:           Authenticated ✓");
    } else {
        println!("  Status:           Not authenticated (run 'eforge login')");
    }
    if !config.servers.is_empty() {
        println!("  Configured Servers:");
        for s in config.servers.keys() {
            let is_default = config.default_server.as_deref() == Some(s.as_str());
            println!("    - {s}{}", if is_default { " (default)" } else { "" });
        }
    }

    ExitCode::Success
}

fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    }
}
