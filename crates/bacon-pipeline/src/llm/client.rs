//! NVIDIA-only LLM API client with retry logic.
//!
//! Simplified from `auto-rust/src/llm/client.rs` — removes Ollama/OpenRouter.

use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

use super::models::{ChatMessage, LlmProvider, NvidiaConfig};

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

/// Retry delay with exponential backoff and deterministic jitter.
///
/// Base = 1s, multiplier = 2x per attempt, capped at 60s.
/// Jitter (±25%) uses golden-angle pseudo-random from attempt number
/// to avoid thundering-herd when multiple clients retry together.
fn retry_delay(attempt: usize) -> Duration {
    const BASE_MS: u64 = 1_000;
    const MULTIPLIER: f64 = 2.0;
    const MAX_DELAY_MS: u64 = 60_000;
    const JITTER_FRACTION: f64 = 0.25;

    let base = (BASE_MS as f64) * MULTIPLIER.powi(attempt as i32 - 1);
    let clamped = base.min(MAX_DELAY_MS as f64);
    let jitter = (attempt as f64 * 137.508).fract() * (clamped * JITTER_FRACTION)
        - (clamped * JITTER_FRACTION / 2.0);
    Duration::from_millis((clamped + jitter) as u64)
}

/// Extract `Retry-After` header value in seconds, if present.
fn parse_retry_after(response: &reqwest::Response) -> Option<Duration> {
    let value = response.headers().get("retry-after")?.to_str().ok()?;
    let seconds = value.parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
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

    /// Send a chat request with retry logic.
    ///
    /// Retries up to 3 times on transient errors (timeouts, 429, 5xx).
    /// Dispatches to NVIDIA or Ollama endpoint based on `config.provider`.
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        match self.config.provider {
            LlmProvider::Nvidia => self.chat_nvidia(messages).await,
            LlmProvider::Ollama => self.chat_ollama(messages).await,
        }
    }

    async fn chat_nvidia(&self, messages: Vec<ChatMessage>) -> Result<String> {
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

        let response_body = self.send_request(&url, &request, "NVIDIA").await?;
        let message = &response_body["choices"][0]["message"];

        if let Some(reasoning) = message["reasoning"]
            .as_str()
            .or(message["reasoning_content"].as_str())
        {
            debug!("NVIDIA Reasoning: {reasoning}");
        }

        let content = message["content"].as_str().unwrap_or_default().to_string();

        if content.is_empty() {
            let reason = response_body["choices"][0]["finish_reason"]
                .as_str()
                .unwrap_or("unknown");
            anyhow::bail!("Empty response from NVIDIA API (finish_reason: {reason})");
        }

        Ok(content)
    }

    async fn chat_ollama(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/api/chat", self.config.base_url);

        let request = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens,
            }
        });

        let response_body = self.send_request(&url, &request, "Ollama").await?;

        if response_body.get("done").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("Ollama response not marked as done");
        }

        let content = response_body["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        if content.is_empty() {
            anyhow::bail!("Empty response from Ollama API");
        }

        Ok(content)
    }

    /// Shared HTTP request sender with retry logic and error handling.
    async fn send_request(
        &self,
        url: &str,
        body: &serde_json::Value,
        provider: &str,
    ) -> Result<serde_json::Value> {
        info!("Calling {provider} API: {}...", self.config.model);

        const MAX_ATTEMPTS: usize = 3;
        for attempt in 1..=MAX_ATTEMPTS {
            let mut req = self.http.post(url).json(body);

            // NVIDIA uses Bearer token auth; Ollama typically doesn't
            if self.config.provider == LlmProvider::Nvidia && !self.config.api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", self.config.api_key));
            }

            let result = req
                .timeout(Duration::from_millis(self.config.timeout_ms))
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    if attempt < MAX_ATTEMPTS && is_retryable_request_error(&err) {
                        let delay = retry_delay(attempt);
                        warn!(
                            "{provider} request failed on attempt {attempt}/{MAX_ATTEMPTS}: {err}. Retrying in {}s.",
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
                let server_delay = parse_retry_after(&response);
                let text = response.text().await.unwrap_or_default();
                if attempt < MAX_ATTEMPTS && is_retryable_status(status) {
                    let computed = retry_delay(attempt);
                    let delay = server_delay.unwrap_or(computed);
                    if let Some(sd) = server_delay {
                        let sd_secs = sd.as_secs();
                        let cd_secs = computed.as_secs();
                        warn!(
                            "{provider} API transient error on attempt {attempt}/{MAX_ATTEMPTS}: {status} - {text}. Server requested {sd_secs}s, using max({cd_secs}s, {sd_secs}s).",
                        );
                    } else {
                        warn!(
                            "{provider} API transient error on attempt {attempt}/{MAX_ATTEMPTS}: {status} - {text}. Retrying in {}s.",
                            delay.as_secs()
                        );
                    }
                    sleep(delay).await;
                    continue;
                }
                error!("{provider} API error: {status} - {text}");
                anyhow::bail!("{provider} API error: {status} - {text}");
            }

            let body_text = response.text().await.unwrap_or_default();
            match serde_json::from_str(&body_text) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let error_path = std::path::PathBuf::from("sessions/api_errors")
                        .join(format!("{provider}_api_{ts}.json"));
                    let _ = std::fs::create_dir_all("sessions/api_errors");
                    let _ = std::fs::write(&error_path, &body_text);
                    warn!(
                        "Failed to parse {provider} JSON (saved to {}): {e}",
                        error_path.display(),
                    );
                    return Err(e.into());
                }
            }
        }

        anyhow::bail!("{provider} API failed after {MAX_ATTEMPTS} attempts")
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
