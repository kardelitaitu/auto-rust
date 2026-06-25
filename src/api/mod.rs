// last audited 26-06-26 by Buffy

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

    #[test]
    fn test_circuit_state_equality() {
        assert_eq!(CircuitState::Closed, CircuitState::Closed);
        assert_eq!(CircuitState::Open, CircuitState::Open);
        assert_eq!(CircuitState::HalfOpen, CircuitState::HalfOpen);
        assert_ne!(CircuitState::Closed, CircuitState::Open);
        assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
        assert_ne!(CircuitState::HalfOpen, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_state_debug() {
        assert_eq!(format!("{:?}", CircuitState::Closed), "Closed");
        assert_eq!(format!("{:?}", CircuitState::Open), "Open");
        assert_eq!(format!("{:?}", CircuitState::HalfOpen), "HalfOpen");
    }

    #[test]
    fn test_circuit_state_clone() {
        let state = CircuitState::Open;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_circuit_state_copy() {
        let state1 = CircuitState::HalfOpen;
        let state2 = state1;
        assert_eq!(state1, state2);
    }
}
