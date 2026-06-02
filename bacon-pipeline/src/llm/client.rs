//! NVIDIA-only LLM API client with retry logic.
//!
//! Simplified from `auto-rust/src/llm/client.rs` — removes Ollama/OpenRouter.

use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

use super::models::{ChatMessage, NvidiaConfig};

/// Check if an HTTP status should trigger a retry.
fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

/// Check if a request error should trigger a retry.
fn is_retryable_request_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

/// Retry delay based on attempt number.
fn retry_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_secs(10),
        2 => Duration::from_secs(30),
        _ => Duration::from_secs(60),
    }
}

/// NVIDIA-only LLM client.
pub struct LlmClient {
    config: NvidiaConfig,
    http: Client,
}

impl LlmClient {
    /// Create a new client from an NVIDIA config.
    pub fn new(config: NvidiaConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(600))
            .http2_adaptive_window(true)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        Self { config, http }
    }

    /// Send a chat request to the NVIDIA API with retry logic.
    ///
    /// Retries up to 3 times on transient errors (timeouts, 429, 5xx).
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let request = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "max_tokens": self.config.max_tokens,
            "stream": false,
            "chat_template_kwargs": {
                "thinking": true,
                "reasoning_effort": "high"
            }
        });

        info!("Calling NVIDIA API: {}...", self.config.model);

        const MAX_ATTEMPTS: usize = 3;
        for attempt in 1..=MAX_ATTEMPTS {
            let result = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .json(&request)
                .timeout(Duration::from_millis(self.config.timeout_ms))
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    if attempt < MAX_ATTEMPTS && is_retryable_request_error(&err) {
                        let delay = retry_delay(attempt);
                        warn!(
                            "NVIDIA request failed on attempt {}/{}: {}. Retrying in {}s.",
                            attempt,
                            MAX_ATTEMPTS,
                            err,
                            delay.as_secs()
                        );
                        sleep(delay).await;
                        continue;
                    }
                    return Err(err.into());
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                if attempt < MAX_ATTEMPTS && is_retryable_status(status) {
                    let delay = retry_delay(attempt);
                    warn!(
                        "NVIDIA API transient error on attempt {}/{}: {} - {}. Retrying in {}s.",
                        attempt,
                        MAX_ATTEMPTS,
                        status,
                        text,
                        delay.as_secs()
                    );
                    sleep(delay).await;
                    continue;
                }
                error!("NVIDIA API error: {status} - {text}");
                anyhow::bail!("NVIDIA API error: {status} - {text}");
            }

            let body_text = response.text().await.unwrap_or_default();
            let body: serde_json::Value = match serde_json::from_str(&body_text) {
                Ok(v) => v,
                Err(e) => {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let error_path = std::path::PathBuf::from("sessions/api_errors")
                        .join(format!("nvidia_api_{ts}.json"));
                    let _ = std::fs::create_dir_all("sessions/api_errors");
                    let _ = std::fs::write(&error_path, &body_text);
                    warn!(
                        "Failed to parse NVIDIA JSON (saved to {}): {}",
                        error_path.display(),
                        e
                    );
                    return Err(e.into());
                }
            };

            let message = &body["choices"][0]["message"];

            if let Some(reasoning) = message["reasoning"]
                .as_str()
                .or(message["reasoning_content"].as_str())
            {
                debug!("NVIDIA Reasoning: {reasoning}");
            }

            let content = message["content"].as_str().unwrap_or_default().to_string();

            if content.is_empty() {
                let reason = body["choices"][0]["finish_reason"]
                    .as_str()
                    .unwrap_or("unknown");
                anyhow::bail!("Empty response from NVIDIA API (finish_reason: {reason})");
            }

            return Ok(content);
        }

        anyhow::bail!("NVIDIA API failed after {MAX_ATTEMPTS} attempts")
    }

    /// Quick connectivity check.
    pub async fn health_check(&self) -> bool {
        self.health_check_result().await.unwrap_or(false)
    }

    /// Quick connectivity check returning a Result.
    pub async fn health_check_result(&self) -> Result<bool> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let request = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 1,
        });

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_status() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn test_llm_client_new() {
        let config = NvidiaConfig::default();
        let client = LlmClient::new(config);
        let _ = client; // just verify it doesn't panic
    }
}
