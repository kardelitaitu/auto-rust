use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::StatusCode;
use std::time::Duration;
use tokio::time::sleep;

use super::LlmClient;
use crate::llm::models::{
    ChatChoice, ChatMessage, ChatResponse, LlmProvider, OpenRouterResponse, Role,
};

pub(crate) fn is_retryable_nvidia_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn is_retryable_nvidia_request_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

/// Retry delay with exponential backoff and deterministic jitter.
///
/// Base = 1s, multiplier = 2x per attempt, capped at 60s.
/// Jitter (±25%) uses golden-angle pseudo-random from attempt number
/// to avoid thundering-herd when multiple clients retry together.
#[cfg(not(test))]
fn nvidia_retry_delay(attempt: usize) -> Duration {
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

#[cfg(test)]
fn nvidia_retry_delay(_attempt: usize) -> Duration {
    Duration::from_millis(1)
}

/// Extract `Retry-After` header value in seconds, if present.
fn parse_retry_after(response: &reqwest::Response) -> Option<Duration> {
    let value = response.headers().get("retry-after")?.to_str().ok()?;
    let seconds = value.parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
}

/// Cleans output by removing any inline reasoning/thinking blocks wrapped in `<think>` tags.
/// Also handles partial/cutoff thinking blocks gracefully.
#[must_use]
pub fn strip_thinking_tags(text: &str) -> String {
    let mut cleaned = text.to_string();
    while let Some(start_idx) = cleaned.find("<think>") {
        if let Some(end_idx) = cleaned.find("</think>") {
            cleaned.replace_range(start_idx..end_idx + 8, "");
        } else {
            cleaned.replace_range(start_idx.., "");
            break;
        }
    }
    cleaned.trim().to_string()
}

/// Brief computed delay before an OpenRouter fallback attempt.
/// Uses the same exponential backoff as NVIDIA (attempt 1 ~1s) to space out
/// fallback requests and avoid consecutive rate-limit rejections.
fn openrouter_fallback_delay() -> Duration {
    #[cfg(not(test))]
    {
        nvidia_retry_delay(1)
    }
    #[cfg(test)]
    {
        Duration::from_millis(1)
    }
}

impl LlmClient {
    #[allow(clippy::cast_precision_loss)]
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        match self.config.provider {
            LlmProvider::Ollama => self.ollama_chat(messages).await,
            LlmProvider::OpenRouter => self.openrouter_chat(messages).await,
            LlmProvider::Nvidia => self.nvidia_chat(messages).await,
        }
    }

    pub async fn chat_with_fallback(&self, messages: Vec<ChatMessage>) -> Result<String> {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        match self.config.provider {
            LlmProvider::Ollama => match self.ollama_chat(messages.clone()).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    warn!("Ollama failed: {e}, trying fallback...");
                    if let Some(ref fallback) = self.fallback_config {
                        if fallback.provider == LlmProvider::OpenRouter {
                            let fallback_client = LlmClient::new(fallback.clone());
                            return fallback_client.openrouter_chat(messages).await;
                        }
                    }
                    Err(e)
                }
            },
            LlmProvider::OpenRouter => self.openrouter_chat(messages).await,
            LlmProvider::Nvidia => self.nvidia_chat(messages).await,
        }
    }

    async fn nvidia_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.nvidia.base_url);

        let mut request = serde_json::json!({
            "model": self.config.nvidia.model,
            "messages": messages,
            "temperature": self.config.nvidia.temperature,
            "top_p": self.config.nvidia.top_p,
            "max_tokens": self.config.nvidia.max_tokens,
            "stream": false,
            "chat_template_kwargs": {
                "thinking": true,
                "reasoning_effort": "high"
            }
        });
        if let Some(val) = self.config.nvidia.presence_penalty {
            request["presence_penalty"] = serde_json::json!(val);
        }
        if let Some(val) = self.config.nvidia.frequency_penalty {
            request["frequency_penalty"] = serde_json::json!(val);
        }

        info!(
            "Calling NVIDIA API (High Thinking): {}...",
            self.config.nvidia.model
        );

        const MAX_ATTEMPTS: usize = 3;
        for attempt in 1..=MAX_ATTEMPTS {
            let result = self
                .http
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.nvidia.api_key),
                )
                .json(&request)
                .timeout(Duration::from_millis(self.config.nvidia.timeout_ms))
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    if attempt < MAX_ATTEMPTS && is_retryable_nvidia_request_error(&err) {
                        let delay = nvidia_retry_delay(attempt);
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
                let server_delay = parse_retry_after(&response);
                let text = response.text().await.unwrap_or_default();
                if attempt < MAX_ATTEMPTS && is_retryable_nvidia_status(status) {
                    let computed = nvidia_retry_delay(attempt);
                    let delay = server_delay.unwrap_or(computed);
                    if let Some(sd) = server_delay {
                        warn!(
                            "NVIDIA API transient error on attempt {}/{}: {} - {}. Server requested {}s, using max({}s, {}s).",
                            attempt, MAX_ATTEMPTS, status, text,
                            sd.as_secs(), computed.as_secs(), sd.as_secs()
                        );
                    } else {
                        warn!(
                            "NVIDIA API transient error on attempt {}/{}: {} - {}. Retrying in {}s.",
                            attempt, MAX_ATTEMPTS, status, text, delay.as_secs()
                        );
                    }
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
                    // Persist raw body for debugging
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

            // Log reasoning if present
            if let Some(reasoning) = message["reasoning"]
                .as_str()
                .or(message["reasoning_content"].as_str())
            {
                debug!("NVIDIA Reasoning: {reasoning}");
            }

            let content = message["content"].as_str().unwrap_or_default().to_string();
            let content = strip_thinking_tags(&content);

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

    async fn ollama_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/api/chat", self.config.ollama.base_url);

        let mut options = serde_json::json!({
            "temperature": self.config.ollama.temperature,
            "num_predict": self.config.ollama.max_tokens,
        });
        if let Some(val) = self.config.ollama.presence_penalty {
            options["presence_penalty"] = serde_json::json!(val);
        }
        if let Some(val) = self.config.ollama.frequency_penalty {
            options["frequency_penalty"] = serde_json::json!(val);
        }

        // Merge system instructions into the first user message for better compatibility
        // with local models (like older Gemma) which can loop/fail on standard system role formats.
        // Skip merging when `no_system_merge` is set (e.g. for Gemma 4+ with proper chat templates).
        let processed_messages = if self.config.ollama.no_system_merge {
            // Send messages as-is — system role preserved for models with proper templates
            messages
        } else {
            let mut merged = Vec::new();
            let mut system_contents = Vec::new();

            for msg in &messages {
                match msg.role {
                    Role::System => {
                        system_contents.push(msg.content.clone());
                    }
                    Role::User => {
                        let mut user_content = msg.content.clone();
                        if !system_contents.is_empty() {
                            let system_merged = system_contents.join("\n\n");
                            user_content = format!(
                                "System Instructions:\n{}\n\nUser Request:\n{}",
                                system_merged, user_content
                            );
                            system_contents.clear();
                        }
                        merged.push(ChatMessage::user(user_content));
                    }
                    Role::Assistant => {
                        merged.push(msg.clone());
                    }
                }
            }

            if !system_contents.is_empty() {
                let system_merged = system_contents.join("\n\n");
                merged.push(ChatMessage::user(system_merged));
            }
            merged
        };

        let request = serde_json::json!({
            "model": self.config.ollama.model,
            "messages": processed_messages,
            "options": options,
            "stream": false,
        });

        info!("Calling Ollama: {}...", self.config.ollama.model);

        let response = self
            .http
            .post(&url)
            .json(&request)
            .timeout(Duration::from_millis(self.config.ollama.timeout_ms))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Ollama error: {status} - {text}");
            anyhow::bail!("Ollama error: {status} - {text}");
        }

        // Read raw body first for debug logging (to capture all fields including reasoning/thinking)
        let body_text = response.text().await?;
        info!("Raw Ollama response body: {body_text}");

        let chat_response: ChatResponse = serde_json::from_str(&body_text)?;

        if let Some(err) = chat_response.error {
            anyhow::bail!("Ollama error: {err}");
        }

        let message = chat_response.message.unwrap_or_else(|| ChatMessage {
            role: Role::Assistant,
            content: String::new(),
            reasoning_content: None,
        });

        if let Some(ref reasoning) = message.reasoning_content {
            info!("[thinking] Ollama reasoning trace: {}", reasoning);
        }

        let content = strip_thinking_tags(&message.content);

        // Verify that the response contains actual content rather than only tokenizer junk.
        // Uses a regex to strip <unused\d+> tokens (common when Ollama applies wrong chat template).
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| {
            regex::Regex::new(r"<unused\d*>|<unused\d+>|<unuse$|<unus$|<unu$|<un$|<u$|<$")
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to compile unused-token regex: {e}. Falling back to simple pattern."
                    );
                    match regex::Regex::new(r"<unused\d+>") {
                        Ok(r) => r,
                        Err(_) => unreachable!("simple fallback regex must compile"),
                    }
                })
        });
        let cleaned = re.replace_all(&content, "").to_string();
        if cleaned.trim().is_empty() {
            anyhow::bail!("Ollama returned an empty response or only tokenizer junk");
        }

        Ok(content)
    }

    async fn openrouter_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.openrouter.base_url);

        // Build list of models to try: primary + fallbacks
        let mut models_to_try = vec![self.config.openrouter.model.clone()];
        models_to_try.extend(self.config.openrouter.fallback_models.clone());

        let mut last_error = None;

        for (idx, model) in models_to_try.iter().enumerate() {
            let is_fallback = idx > 0;
            let attempt = idx + 1;

            if is_fallback {
                info!(
                    "OpenRouter fallback attempt {}/{} using model: {}",
                    attempt,
                    models_to_try.len(),
                    model
                );
            } else {
                info!("Calling OpenRouter: {model}");
            }

            let mut request = serde_json::json!({
                "model": model,
                "messages": &messages,
                "temperature": self.config.openrouter.temperature,
            });
            if let Some(val) = self.config.openrouter.presence_penalty {
                request["presence_penalty"] = serde_json::json!(val);
            }
            if let Some(val) = self.config.openrouter.frequency_penalty {
                request["frequency_penalty"] = serde_json::json!(val);
            }

            let result = self
                .http
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.openrouter.api_key),
                )
                .header("Content-Type", "application/json")
                .json(&request)
                .timeout(Duration::from_millis(self.config.openrouter.timeout_ms))
                .send()
                .await;

            match result {
                Ok(response) => {
                    let body_result = response.text().await;

                    match body_result {
                        Ok(body_text) => {
                            // Try to parse as OpenRouter response
                            match serde_json::from_str::<OpenRouterResponse>(&body_text) {
                                Ok(openrouter_response) => {
                                    // Check for API-level errors
                                    if let Some(err) = openrouter_response.error {
                                        warn!(
                                            "OpenRouter API error on attempt {} with model {}: {}",
                                            attempt, model, err.message
                                        );
                                        last_error = Some(anyhow::anyhow!(
                                            "OpenRouter API error: {}",
                                            err.message
                                        ));
                                        let fd = openrouter_fallback_delay();
                                        sleep(fd).await;
                                        continue; // Try next fallback
                                    }

                                    // Extract content from successful response
                                    let (content, reasoning) = openrouter_response
                                        .choices
                                        .and_then(|choices| choices.into_iter().next())
                                        .map(|choice| match choice {
                                            ChatChoice::WithMessage { message } => {
                                                (message.content, message.reasoning_content)
                                            }
                                            ChatChoice::WithContent { content } => (content, None),
                                        })
                                        .unwrap_or_else(|| (String::new(), None));

                                    if let Some(ref reasoning) = reasoning {
                                        info!(
                                            "[thinking] OpenRouter reasoning trace: {}",
                                            reasoning
                                        );
                                    }

                                    let content = strip_thinking_tags(&content);

                                    if content.is_empty() {
                                        warn!(
                                            "OpenRouter empty response on attempt {attempt} with model {model}"
                                        );
                                        last_error = Some(anyhow::anyhow!(
                                            "Empty response from model: {model}"
                                        ));
                                        continue; // Try next fallback
                                    }
                                    if is_fallback {
                                        info!("OpenRouter fallback model {model} succeeded");
                                    }
                                    return Ok(content);
                                }
                                Err(parse_err) => {
                                    warn!("OpenRouter JSON parse error on attempt {attempt} with model {model}: {parse_err}");
                                    last_error = Some(anyhow::anyhow!(
                                        "JSON parse error: {parse_err} - Body: {body_text}"
                                    ));
                                    continue; // Try next fallback
                                }
                            }
                        }
                        Err(body_err) => {
                            warn!(
                                "OpenRouter body read error on attempt {attempt} with model {model}: {body_err}"
                            );
                            last_error =
                                Some(anyhow::anyhow!("Failed to read response body: {body_err}"));
                            continue; // Try next fallback
                        }
                    }
                }
                Err(req_err) => {
                    let is_timeout = req_err.is_timeout();
                    if is_timeout {
                        warn!(
                            "OpenRouter timeout on attempt {} with model {} (timeout_ms: {})",
                            attempt, model, self.config.openrouter.timeout_ms
                        );
                    } else {
                        warn!(
                            "OpenRouter request error on attempt {attempt} with model {model}: {req_err}"
                        );
                    }
                    last_error = Some(anyhow::anyhow!(
                        "Request failed for model {model}: {req_err}"
                    ));
                    let fd = openrouter_fallback_delay();
                    sleep(fd).await;
                    continue; // Try next fallback
                }
            }
        }

        // All models exhausted
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "All OpenRouter models failed (primary + {} fallbacks)",
                self.config.openrouter.fallback_models.len()
            )
        }))
    }

    pub async fn health_check(&self) -> bool {
        self.health_check_result().await.unwrap_or(false)
    }

    pub async fn health_check_result(&self) -> Result<bool> {
        match self.config.provider {
            LlmProvider::Ollama => self.ollama_health().await,
            LlmProvider::OpenRouter => self.openrouter_health().await,
            LlmProvider::Nvidia => self.nvidia_health().await,
        }
    }

    async fn nvidia_health(&self) -> Result<bool> {
        let url = format!("{}/chat/completions", self.config.nvidia.base_url);

        // Simple check: send a request with 1 token max to verify connectivity
        let request = serde_json::json!({
            "model": self.config.nvidia.model,
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 1,
        });

        let response = self
            .http
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.nvidia.api_key),
            )
            .json(&request)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("NVIDIA health check failed ({status}): {text}");
            return Ok(false);
        }

        Ok(true)
    }

    async fn ollama_health(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.config.ollama.base_url);

        let response = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn openrouter_health(&self) -> Result<bool> {
        let url = "https://openrouter.ai/api/v1/models";

        let response = self
            .http
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.openrouter.api_key),
            )
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}
