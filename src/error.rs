#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("network error talking to mvt-rs: {0}")]
    Http(#[from] reqwest::Error),

    #[error("mvt-rs rejected the request (HTTP {status}): {message}")]
    Api {
        status: u16,
        message: String,
        kind: String,
    },

    #[error("mvt-rs response body was not valid JSON: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Debug, serde::Deserialize)]
struct MvtRsErrorBody {
    status: u16,
    error: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Parses mvt-rs's documented JSON error-body shape
/// (`{"status":<u16>,"error":"<msg>","type":"<kind>"}`, from
/// `mvt-rs/src/error.rs`'s `impl Writer for AppError`). Falls back to the raw
/// response body text (or a placeholder for an empty body) if the body isn't
/// that shape, since not every non-2xx response mvt-server-mcp might see is
/// guaranteed to come from mvt-rs's own error renderer.
pub fn parse_error_body(status: reqwest::StatusCode, body: &[u8]) -> ClientError {
    if let Ok(parsed) = serde_json::from_slice::<MvtRsErrorBody>(body) {
        return ClientError::Api {
            status: parsed.status,
            message: parsed.error,
            kind: parsed.kind,
        };
    }
    let text = String::from_utf8_lossy(body).trim().to_string();
    let message = if text.is_empty() {
        "(empty response body)".to_string()
    } else {
        text
    };
    ClientError::Api {
        status: status.as_u16(),
        message,
        kind: "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mvt_rs_error_json_body() {
        let body = br#"{"status":404,"error":"Category abc not found","type":"NotFound"}"#;
        let status = reqwest::StatusCode::NOT_FOUND;
        let result = parse_error_body(status, body);

        match result {
            ClientError::Api {
                status,
                message,
                kind,
            } => {
                assert_eq!(status, 404);
                assert_eq!(message, "Category abc not found");
                assert_eq!(kind, "NotFound");
            }
            _ => panic!("Expected Api variant"),
        }
    }

    #[test]
    fn falls_back_to_raw_text_when_body_is_not_json() {
        let body = b"Internal Server Error";
        let status = reqwest::StatusCode::BAD_GATEWAY;
        let result = parse_error_body(status, body);

        match result {
            ClientError::Api {
                status,
                message,
                kind,
            } => {
                assert_eq!(status, 502);
                assert_eq!(message, "Internal Server Error");
                assert_eq!(kind, "Unknown");
            }
            _ => panic!("Expected Api variant"),
        }
    }

    #[test]
    fn falls_back_when_body_is_empty() {
        let body = b"";
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let result = parse_error_body(status, body);

        match result {
            ClientError::Api {
                status,
                message,
                kind,
            } => {
                assert_eq!(status, 500);
                assert_eq!(message, "(empty response body)");
                assert_eq!(kind, "Unknown");
            }
            _ => panic!("Expected Api variant"),
        }
    }
}
