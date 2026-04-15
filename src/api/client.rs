use anyhow::{Result, bail};
use reqwest::Client;
use std::time::Duration;
use serde::de::DeserializeOwned;

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        
        Self {
            client,
            base_url,
        }
    }

    #[allow(dead_code)]
pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {}: {}", status, text);
        }
        
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn get_with_key<T: DeserializeOwned>(&self, path: &str, api_key: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client
            .get(&url)
            .header("X-API-Key", api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {}: {}", status, text);
        }
        
        let result = response.json().await?;
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[allow(dead_code)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    half_open_timeout_ms: u64,
    failures: u32,
    successes: u32,
    state: CircuitState,
}

impl CircuitBreaker {
    #[allow(dead_code)]
    pub fn new(failure_threshold: u32, success_threshold: u32, half_open_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            half_open_timeout_ms,
            failures: 0,
            successes: 0,
            state: CircuitState::Closed,
        }
    }

    #[allow(dead_code)]
    pub fn is_closed(&self) -> bool {
        self.state == CircuitState::Closed
    }

    #[allow(dead_code)]
    pub fn record_success(&mut self) {
        self.successes += 1;
        
        if self.state == CircuitState::HalfOpen && self.successes >= self.success_threshold {
            self.state = CircuitState::Closed;
            self.failures = 0;
            self.successes = 0;
        }
    }

    #[allow(dead_code)]
    pub fn record_failure(&mut self) {
        self.failures += 1;
        
        if self.state == CircuitState::HalfOpen || self.failures >= self.failure_threshold {
            self.state = CircuitState::Open;
            self.failures = 0;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 3, 30000)
    }
}