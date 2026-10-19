//! HTTP routing logic for `eforge serve`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::path_guard::resolve_safe_path;
use forge_core::ForgeError;
use serde_json::json;

use crate::commands::explain::execute_explain;
use crate::commands::report::{build_report_bundle, ReportBundle, ReportOptions};
use crate::commands::schema::render_report_schema;

/// Thread-safe in-memory state for `eforge serve`.
#[derive(Debug, Clone)]
pub struct ServeState {
    pub repo_root: PathBuf,
    pub bundle: ReportBundle,
    pub bundle_json: String,
    pub bundle_etag: String,
    pub schema_json: String,
    pub allow_origin: Option<String>,
    pub static_dir: Option<PathBuf>,
}

impl ServeState {
    /// Initializes [`ServeState`] by building the initial [`ReportBundle`].
    pub fn new(
        repo_root: PathBuf,
        allow_origin: Option<String>,
        static_dir: Option<PathBuf>,
    ) -> Result<Self, ForgeError> {
        let opts = ReportOptions::default();
        let bundle = build_report_bundle(&repo_root, &opts)?;
        let bundle_json =
            serde_json::to_string_pretty(&bundle).map_err(|err| ForgeError::Serialization {
                message: format!("Failed to serialize ReportBundle: {err}"),
            })?;
        let hash = forge_core::cache::ForgeCache::hash_bytes(bundle_json.as_bytes());
        let bundle_etag = format!("\"{hash}\"");
        let schema_json = render_report_schema()?;

        Ok(Self {
            repo_root,
            bundle,
            bundle_json,
            bundle_etag,
            schema_json,
            allow_origin,
            static_dir,
        })
    }
}

/// Generic HTTP response abstraction for testability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl ApiResponse {
    pub fn json(status: u16, body_str: String, allow_origin: Option<&str>) -> Self {
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Cache-Control".to_string(), "no-store".to_string()),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ];
        if let Some(origin) = allow_origin {
            headers.push((
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            ));
            headers.push((
                "Access-Control-Allow-Methods".to_string(),
                "GET, OPTIONS".to_string(),
            ));
            headers.push((
                "Access-Control-Allow-Headers".to_string(),
                "Content-Type".to_string(),
            ));
        }
        Self {
            status,
            headers,
            body: body_str.into_bytes(),
        }
    }

    pub fn text(status: u16, body_str: String, allow_origin: Option<&str>) -> Self {
        let mut headers = vec![
            (
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            ),
            ("Cache-Control".to_string(), "no-store".to_string()),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ];
        if let Some(origin) = allow_origin {
            headers.push((
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            ));
        }
        Self {
            status,
            headers,
            body: body_str.into_bytes(),
        }
    }
}

fn parse_query_params(raw_query: &str) -> BTreeMap<String, String> {
    let mut params = BTreeMap::new();
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut split = pair.splitn(2, '=');
        let key = split.next().unwrap_or("").to_string();
        let val = split.next().unwrap_or("").to_string();
        if !key.is_empty() {
            params.insert(key, val);
        }
    }
    params
}

fn serve_static_file(static_root: &Path, subpath: &str, allow_origin: Option<&str>) -> ApiResponse {
    let candidate = subpath.trim_start_matches('/');
    let target = if candidate.is_empty() {
        "index.html"
    } else {
        candidate
    };

    match resolve_safe_path(static_root, Path::new(target)) {
        Ok(safe_path) => {
            if safe_path.is_file() {
                match fs::read(&safe_path) {
                    Ok(bytes) => {
                        let content_type = match safe_path.extension().and_then(|ext| ext.to_str())
                        {
                            Some("html") => "text/html; charset=utf-8",
                            Some("js") | Some("mjs") => "application/javascript",
                            Some("css") => "text/css",
                            Some("json") => "application/json",
                            Some("svg") => "image/svg+xml",
                            Some("png") => "image/png",
                            Some("wasm") => "application/wasm",
                            _ => "application/octet-stream",
                        };
                        let mut headers = vec![
                            ("Content-Type".to_string(), content_type.to_string()),
                            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
                        ];
                        if let Some(origin) = allow_origin {
                            headers.push((
                                "Access-Control-Allow-Origin".to_string(),
                                origin.to_string(),
                            ));
                        }
                        ApiResponse {
                            status: 200,
                            headers,
                            body: bytes,
                        }
                    }
                    Err(err) => ApiResponse::text(
                        500,
                        format!("Internal error reading file: {err}"),
                        allow_origin,
                    ),
                }
            } else {
                ApiResponse::text(404, "File not found".to_string(), allow_origin)
            }
        }
        Err(_) => ApiResponse::text(403, "Access denied".to_string(), allow_origin),
    }
}

/// Pure, unit-testable HTTP request router for `eforge serve`.
pub fn route_request(state: &ServeState, method: &str, url: &str) -> ApiResponse {
    let allow_origin = state.allow_origin.as_deref();

    // Handle preflight OPTIONS
    if method == "OPTIONS" {
        return ApiResponse::text(204, String::new(), allow_origin);
    }

    if method != "GET" {
        return ApiResponse::text(405, "Method Not Allowed".to_string(), allow_origin);
    }

    let mut split = url.splitn(2, '?');
    let path = split.next().unwrap_or("/");
    let query_str = split.next().unwrap_or("");
    let query = parse_query_params(query_str);

    match path {
        "/api/v1/health" => {
            let res = json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION")
            });
            ApiResponse::json(200, res.to_string(), allow_origin)
        }
        "/api/v1/bundle" => {
            let mut resp = ApiResponse::json(200, state.bundle_json.clone(), allow_origin);
            resp.headers
                .push(("ETag".to_string(), state.bundle_etag.clone()));
            resp
        }
        "/api/v1/schema" => ApiResponse::json(200, state.schema_json.clone(), allow_origin),
        "/api/v1/findings" => {
            let sev_filter = query.get("severity").map(|s| s.to_lowercase());
            let rule_filter = query.get("rule_prefix").map(|r| r.as_str());

            let filtered: Vec<_> = state
                .bundle
                .repository
                .findings
                .iter()
                .filter(|f| {
                    if let Some(sev) = &sev_filter {
                        let f_sev = format!("{:?}", f.severity).to_lowercase();
                        if &f_sev != sev {
                            return false;
                        }
                    }
                    if let Some(prefix) = rule_filter {
                        if !f.rule_id.starts_with(prefix) {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            match serde_json::to_string_pretty(&filtered) {
                Ok(body) => ApiResponse::json(200, body, allow_origin),
                Err(err) => ApiResponse::json(
                    500,
                    json!({ "error": format!("{err}") }).to_string(),
                    allow_origin,
                ),
            }
        }
        "/api/v1/explain" => {
            let target = query.get("target").map(|s| s.as_str());
            match execute_explain(&state.repo_root, target) {
                Ok(report) => match serde_json::to_string_pretty(&report) {
                    Ok(body) => ApiResponse::json(200, body, allow_origin),
                    Err(err) => ApiResponse::json(
                        500,
                        json!({ "error": format!("{err}") }).to_string(),
                        allow_origin,
                    ),
                },
                Err(err) => ApiResponse::json(
                    500,
                    json!({ "error": format!("{err}") }).to_string(),
                    allow_origin,
                ),
            }
        }
        "/api/v1/history" => {
            let limit = query
                .get("limit")
                .and_then(|l| l.parse::<usize>().ok())
                .unwrap_or(0);
            match crate::commands::history::load_history(&state.repo_root, limit) {
                Ok(snapshots) => match serde_json::to_string_pretty(&snapshots) {
                    Ok(body) => ApiResponse::json(200, body, allow_origin),
                    Err(err) => ApiResponse::json(
                        500,
                        json!({ "error": format!("{err}") }).to_string(),
                        allow_origin,
                    ),
                },
                Err(err) => ApiResponse::json(
                    500,
                    json!({ "error": format!("{err}") }).to_string(),
                    allow_origin,
                ),
            }
        }
        _ => {
            if let Some(static_root) = &state.static_dir {
                serve_static_file(static_root, path, allow_origin)
            } else {
                ApiResponse::text(404, "Not Found".to_string(), allow_origin)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> Result<ServeState, ForgeError> {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        ServeState::new(fixture, Some("http://localhost:5173".to_string()), None)
    }

    #[test]
    fn test_route_health() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "GET", "/api/v1/health");
        assert_eq!(resp.status, 200);
        let val: serde_json::Value =
            serde_json::from_slice(&resp.body).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })?;
        assert_eq!(val["status"], "ok");
        assert_eq!(val["version"], env!("CARGO_PKG_VERSION"));
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "http://localhost:5173"));
        Ok(())
    }

    #[test]
    fn test_route_bundle_and_etag() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "GET", "/api/v1/bundle");
        assert_eq!(resp.status, 200);
        let has_etag = resp
            .headers
            .iter()
            .any(|(k, v)| k == "ETag" && !v.is_empty());
        assert!(has_etag);
        let val: serde_json::Value =
            serde_json::from_slice(&resp.body).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })?;
        assert_eq!(val["schema_version"], 1);
        Ok(())
    }

    #[test]
    fn test_route_findings_filter() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "GET", "/api/v1/findings?severity=high");
        assert_eq!(resp.status, 200);
        let findings: Vec<serde_json::Value> =
            serde_json::from_slice(&resp.body).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })?;
        for f in findings {
            assert_eq!(f["severity"], "high");
        }
        Ok(())
    }

    #[test]
    fn test_route_explain() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "GET", "/api/v1/explain?target=src/components");
        assert_eq!(resp.status, 200);
        let val: serde_json::Value =
            serde_json::from_slice(&resp.body).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })?;
        assert_eq!(val["target"], "src/components");
        Ok(())
    }

    #[test]
    fn test_rejects_non_get() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "POST", "/api/v1/health");
        assert_eq!(resp.status, 405);
        Ok(())
    }

    #[test]
    fn test_allows_options_preflight() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "OPTIONS", "/api/v1/bundle");
        assert_eq!(resp.status, 204);
        assert!(resp
            .headers
            .iter()
            .any(|(k, v)| k == "Access-Control-Allow-Origin" && v == "http://localhost:5173"));
        Ok(())
    }

    #[test]
    fn test_route_history() -> Result<(), ForgeError> {
        let state = test_state()?;
        let resp = route_request(&state, "GET", "/api/v1/history?limit=10");
        assert_eq!(resp.status, 200);
        let val: serde_json::Value =
            serde_json::from_slice(&resp.body).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })?;
        assert!(val.is_array());
        Ok(())
    }
}
