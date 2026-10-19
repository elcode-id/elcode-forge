//! Loopback HTTP server (`eforge serve`) for local dashboard live intelligence.

pub mod routes;

use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;

use forge_core::ForgeError;
use tiny_http::{Header, Response, Server, StatusCode};

pub use routes::{route_request, ApiResponse, ServeState};

/// Configuration options for `eforge serve`.
#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub repo_root: PathBuf,
    pub port: u16,
    pub allow_origin: Option<String>,
    pub static_dir: Option<PathBuf>,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            repo_root: PathBuf::from("."),
            port: 7878,
            allow_origin: None,
            static_dir: None,
        }
    }
}

/// Runs the local-only `eforge serve` HTTP server.
pub fn run_serve(opts: &ServeOptions) -> Result<(), ForgeError> {
    let addr_str = format!("127.0.0.1:{}", opts.port);
    let addr: SocketAddr = addr_str.parse().map_err(|err| ForgeError::Config {
        message: format!("Invalid socket address '{addr_str}': {err}"),
    })?;

    let server = Server::http(addr).map_err(|err| ForgeError::Io {
        path: opts.repo_root.clone(),
        source: std::io::Error::new(std::io::ErrorKind::AddrInUse, err.to_string()),
    })?;

    let state = ServeState::new(
        opts.repo_root.clone(),
        opts.allow_origin.clone(),
        opts.static_dir.clone(),
    )?;

    println!("ELCODE Forge Intelligence API listening on http://{addr_str}");
    if let Some(origin) = &opts.allow_origin {
        println!("CORS allowed origin: {origin}");
    }
    if let Some(static_dir) = &opts.static_dir {
        println!("Serving static files from: {}", static_dir.display());
    }

    for request in server.incoming_requests() {
        let method = request.method().as_str();
        let url = request.url();

        let api_resp = route_request(&state, method, url);

        let mut headers = Vec::with_capacity(api_resp.headers.len());
        for (name, val) in api_resp.headers {
            if let Ok(hdr) = Header::from_bytes(name.as_bytes(), val.as_bytes()) {
                headers.push(hdr);
            }
        }

        let body_len = api_resp.body.len();
        let response = Response::new(
            StatusCode(api_resp.status),
            headers,
            Cursor::new(api_resp.body),
            Some(body_len),
            None,
        );

        let _ = request.respond(response);
    }

    Ok(())
}
