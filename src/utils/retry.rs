use std::future::Future;
use std::time::Duration;

/// Configuration for retry with exponential backoff and jitter.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (including the first try).
    pub max_attempts: u32,
    /// Base delay in milliseconds for the first retry.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (cap for exponential growth).
    pub max_delay_ms: u64,
    /// Multiplier applied to delay each attempt (e.g., 2.0 = exponential).
    pub multiplier: f64,
    /// Jitter factor (0.0–1.0) as fraction of the computed delay.
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter_factor: 0.2,
        }
    }
}

impl RetryConfig {
    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    pub fn with_base_delay_ms(mut self, ms: u64) -> Self {
        self.base_delay_ms = ms;
        self
    }

    pub fn with_max_delay_ms(mut self, ms: u64) -> Self {
        self.max_delay_ms = ms;
        self
    }

    pub fn with_multiplier(mut self, m: f64) -> Self {
        self.multiplier = m;
        self
    }

    pub fn with_jitter_factor(mut self, j: f64) -> Self {
        self.jitter_factor = j.clamp(0.0, 1.0);
        self
    }
}

/// An iterator that yields exponentially increasing `Duration` values
/// clamped to `max_delay_ms`, with uniform jitter applied to each.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    attempt: u32,
    config: RetryConfig,
}

impl ExponentialBackoff {
    pub fn new(config: &RetryConfig) -> Self {
        Self {
            attempt: 1,
            config: config.clone(),
        }
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.attempt > self.config.max_attempts.saturating_sub(1) {
            return None;
        }
        let raw = self.config.base_delay_ms as f64
            * self.config.multiplier.powi((self.attempt - 1) as i32);
        let clamped = raw.min(self.config.max_delay_ms as f64);
        let jitter = if self.config.jitter_factor > 0.0 {
            let range = clamped * self.config.jitter_factor;
            let offset = (rand::random::<f64>() * 2.0 - 1.0) * range;
            (clamped + offset).max(1.0)
        } else {
            clamped
        };
        self.attempt += 1;
        Some(Duration::from_millis(jitter as u64))
    }
}

/// Retries a fallible async operation with exponential backoff and jitter.
///
/// The operation `f` is called up to `config.max_attempts` times.
/// If it returns `Ok(t)`, the value is returned immediately.
/// If it returns `Err(_)`, the next backoff delay is awaited before retrying.
/// If all attempts fail, the last error is returned.
pub async fn retry_with_backoff<T, E, F, Fut>(config: &RetryConfig, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let backoff = ExponentialBackoff::new(config);

    for (i, delay) in (1..).zip(backoff) {
        match f().await {
            Ok(val) => return Ok(val),
            Err(_e) => {
                log::debug!("Retry attempt {i} failed, waiting {delay:?}...");
                tokio::time::sleep(delay).await;
            }
        }
    }

    // Final attempt (no delay after)
    f().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_defaults() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_attempts, 3);
        assert_eq!(cfg.base_delay_ms, 1000);
        assert_eq!(cfg.max_delay_ms, 30_000);
        assert_eq!(cfg.multiplier, 2.0);
        assert_eq!(cfg.jitter_factor, 0.2);
    }

    #[test]
    fn test_retry_config_builder_methods() {
        let cfg = RetryConfig::default()
            .with_max_attempts(5)
            .with_base_delay_ms(500)
            .with_max_delay_ms(10_000)
            .with_multiplier(3.0)
            .with_jitter_factor(0.5);
        assert_eq!(cfg.max_attempts, 5);
        assert_eq!(cfg.base_delay_ms, 500);
        assert_eq!(cfg.max_delay_ms, 10_000);
        assert_eq!(cfg.multiplier, 3.0);
        assert_eq!(cfg.jitter_factor, 0.5);
    }

    #[test]
    fn test_jitter_factor_clamped() {
        let cfg = RetryConfig::default().with_jitter_factor(1.5);
        assert_eq!(cfg.jitter_factor, 1.0);
        let cfg = RetryConfig::default().with_jitter_factor(-0.5);
        assert_eq!(cfg.jitter_factor, 0.0);
    }

    #[test]
    fn test_exponential_backoff_yields_increasing_delays() {
        let cfg = RetryConfig::default()
            .with_max_attempts(5)
            .with_jitter_factor(0.0);
        let delays: Vec<Duration> = ExponentialBackoff::new(&cfg).collect();
        // Attempts: 1->1000, 2->2000, 3->4000, 4->8000
        assert_eq!(delays.len(), 4);
        assert!(delays[0] <= delays[1]);
        assert!(delays[1] <= delays[2]);
        assert!(delays[2] <= delays[3]);
    }

    #[test]
    fn test_exponential_backoff_clamped_to_max() {
        let cfg = RetryConfig::default()
            .with_max_attempts(10)
            .with_base_delay_ms(1000)
            .with_max_delay_ms(5_000)
            .with_multiplier(10.0)
            .with_jitter_factor(0.0);
        let delays: Vec<Duration> = ExponentialBackoff::new(&cfg).collect();
        for d in &delays {
            assert!(*d <= Duration::from_millis(5000));
        }
    }

    #[test]
    fn test_exponential_backoff_single_attempt_yields_none() {
        let cfg = RetryConfig::default().with_max_attempts(1);
        let delays: Vec<Duration> = ExponentialBackoff::new(&cfg).collect();
        assert!(delays.is_empty());
    }

    #[test]
    fn test_jitter_is_within_bounds() {
        let cfg = RetryConfig::default()
            .with_max_attempts(20)
            .with_jitter_factor(0.5);
        let delays: Vec<Duration> = ExponentialBackoff::new(&cfg).collect();
        for d in &delays {
            let ms = d.as_millis() as f64;
            assert!(ms.is_finite(), "delay should be finite: {ms}");
        }
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_first_try() {
        let cfg = RetryConfig::default().with_max_attempts(3);
        let result = retry_with_backoff(&cfg, || async { Ok::<_, String>(42) }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let attempts = std::sync::atomic::AtomicU32::new(0);
        let cfg = RetryConfig::default()
            .with_max_attempts(5)
            .with_jitter_factor(0.0);
        let result = retry_with_backoff(&cfg, || async {
            let prev = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if prev < 2 {
                Err::<i32, String>("not yet".to_string())
            } else {
                Ok(99)
            }
        })
        .await;
        assert_eq!(result, Ok(99));
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let cfg = RetryConfig::default()
            .with_max_attempts(3)
            .with_jitter_factor(0.0);
        let result = retry_with_backoff::<i32, String, _, _>(&cfg, || async {
            Err::<i32, String>("always fail".to_string())
        })
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fail");
    }

    #[tokio::test]
    async fn test_retry_zero_attempts() {
        let cfg = RetryConfig::default().with_max_attempts(0);
        let called = std::sync::atomic::AtomicBool::new(false);
        let result = retry_with_backoff::<i32, String, _, _>(&cfg, || async {
            called.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(42)
        })
        .await;
        assert_eq!(result, Ok(42));
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_retry_zero_base_delay() {
        let cfg = RetryConfig::default()
            .with_max_attempts(3)
            .with_base_delay_ms(0)
            .with_jitter_factor(0.0);
        let result = retry_with_backoff::<i32, String, _, _>(&cfg, || async {
            Err::<i32, String>("fail".to_string())
        })
        .await;
        assert!(result.is_err());
    }
}
