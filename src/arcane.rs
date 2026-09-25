use std::time::Duration;

use reqwest::Method;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Semaphore;

use crate::config::ArcaneConfig;

#[cfg(test)]
#[path = "arcane_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct ArcaneClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    permits: Arc<Semaphore>,
}

pub const MAX_UPSTREAM_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_CONCURRENT_UPSTREAM_REQUESTS: usize = 16;

#[derive(Debug, Error)]
pub enum ArcaneError {
    #[error("configuration error: {0}")]
    Config(&'static str),
    #[error("failed to build Arcane HTTP client")]
    Build(#[source] reqwest::Error),
    #[error("Arcane API transport failed")]
    Transport(#[source] reqwest::Error),
    #[error("Arcane API response exceeds {limit} bytes")]
    ResponseTooLarge { limit: usize },
    #[error("Arcane API error {status}: {message}")]
    Http {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("Arcane API returned invalid JSON")]
    Decode(#[source] serde_json::Error),
    #[error("Arcane request concurrency limiter is closed")]
    ConcurrencyClosed,
}

impl ArcaneError {
    pub fn status(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transport(error) if error.is_timeout() || error.is_connect())
            || self.status().is_some_and(|status| status.is_server_error())
    }
}

impl ArcaneClient {
    pub fn new(cfg: &ArcaneConfig) -> Result<Self, ArcaneError> {
        if cfg.api_url.trim().is_empty() {
            return Err(ArcaneError::Config("RARCANE_API_URL is not set"));
        }
        if cfg.api_key.trim().is_empty() {
            return Err(ArcaneError::Config("RARCANE_API_KEY is not set"));
        }
        let http = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(ArcaneError::Build)?;
        Ok(Self {
            http,
            base_url: normalize_base_url(&cfg.api_url),
            api_key: cfg.api_key.clone(),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_UPSTREAM_REQUESTS)),
        })
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        query: Option<&Value>,
        body: Option<&Value>,
        timeout: Option<Duration>,
    ) -> Result<Value, ArcaneError> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|_| ArcaneError::ConcurrencyClosed)?;
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .http
            .request(method, url)
            .header("X-API-Key", &self.api_key);
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        if let Some(query) = query.and_then(Value::as_object) {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(body);
        }

        let mut response = request.send().await.map_err(ArcaneError::Transport)?;
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_UPSTREAM_RESPONSE_BYTES as u64)
        {
            return Err(ArcaneError::ResponseTooLarge {
                limit: MAX_UPSTREAM_RESPONSE_BYTES,
            });
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(ArcaneError::Transport)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_UPSTREAM_RESPONSE_BYTES {
                return Err(ArcaneError::ResponseTooLarge {
                    limit: MAX_UPSTREAM_RESPONSE_BYTES,
                });
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            let message = serde_json::from_slice::<Value>(&bytes)
                .ok()
                .and_then(|value| {
                    value
                        .get("message")
                        .and_then(Value::as_str)
                        .or_else(|| value.get("error").and_then(Value::as_str))
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
            return Err(ArcaneError::Http {
                status,
                message: redact(&message, &self.api_key),
            });
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(ArcaneError::Decode)
    }
}

fn normalize_base_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with("/api") {
        trimmed.to_owned()
    } else {
        format!("{trimmed}/api")
    }
}

pub fn encode_path_segment(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn redact(message: &str, api_key: &str) -> String {
    let redacted = message.replace("X-API-Key", "[redacted-header]");
    if api_key.is_empty() {
        redacted
    } else {
        redacted.replace(api_key, "[redacted-secret]")
    }
}
