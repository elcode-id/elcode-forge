use std::io::Read;
use std::time::Duration;

use forge_core::error::ForgeError;

/// Maximum allowable response payload size (4 MiB) to avoid memory exhaustion from hostile endpoints.
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Constructs a configured `ureq::Agent` with custom timeout settings.
pub fn create_agent(timeout_secs: u64) -> ureq::Agent {
    let timeout = Duration::from_secs(timeout_secs.max(1));
    ureq::AgentBuilder::new()
        .timeout_connect(timeout)
        .timeout_read(timeout)
        .timeout_write(timeout)
        .build()
}

/// Reads an HTTP response body up to [`MAX_RESPONSE_BYTES`], converting I/O errors to [`ForgeError::Ai`].
pub fn read_bounded_body(reader: &mut dyn Read) -> Result<String, ForgeError> {
    let mut buffer = Vec::new();
    let mut take = reader.take(MAX_RESPONSE_BYTES as u64 + 1);
    take.read_to_end(&mut buffer).map_err(|e| ForgeError::Ai {
        message: format!("failed to read response stream: {e}"),
    })?;

    if buffer.len() > MAX_RESPONSE_BYTES {
        return Err(ForgeError::Ai {
            message: format!(
                "AI provider response exceeded maximum allowed size of {MAX_RESPONSE_BYTES} bytes"
            ),
        });
    }

    String::from_utf8(buffer).map_err(|e| ForgeError::Ai {
        message: format!("invalid UTF-8 in AI provider response: {e}"),
    })
}

/// Maps a `ureq::Error` into a safe [`ForgeError::Ai`], redacting authorization headers.
pub fn map_ureq_error(provider_name: &str, err: ureq::Error) -> ForgeError {
    match err {
        ureq::Error::Status(code, resp) => {
            let mut reader = resp.into_reader();
            let body_snippet = read_bounded_body(&mut reader)
                .unwrap_or_else(|_| "<failed to read error body>".to_string());
            let truncated = if body_snippet.len() > 500 {
                format!("{}...", &body_snippet[..500])
            } else {
                body_snippet
            };
            ForgeError::Ai {
                message: format!("{provider_name} returned HTTP {code}: {truncated}"),
            }
        }
        ureq::Error::Transport(transport) => ForgeError::Ai {
            message: format!("{provider_name} transport/connection error: {transport}"),
        },
    }
}
