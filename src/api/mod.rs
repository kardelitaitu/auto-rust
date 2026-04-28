pub mod client;
pub use client::{ApiClient, CircuitBreaker, CircuitState, RetryPolicy};

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test to verify all re-exports are accessible.
    #[test]
    fn test_api_re_exports_exist() {
        // These just need to compile - verifies module structure
        let _: Option<ApiClient> = None;
        let _: Option<CircuitBreaker> = None;
        let _: Option<CircuitState> = None;
        let _: Option<RetryPolicy> = None;
    }

    #[test]
    fn test_circuit_state_variants() {
        // Verify CircuitState enum variants are accessible
        let _closed = CircuitState::Closed;
        let _open = CircuitState::Open;
        let _half_open = CircuitState::HalfOpen;
    }
}
