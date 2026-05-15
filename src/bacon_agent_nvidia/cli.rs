use super::nvidia_api::NvidiaConfig;

/// Shared pipeline run arguments.
///
/// Re-exported from `bacon_core::cli_types` to provide a single
/// canonical CLI interface for all bacon agents.
pub use crate::bacon_core::cli_types::RunArgs;

/// Build an NvidiaConfig from pipeline run args + env fallbacks.
///
/// NVIDIA API settings come from environment variables, not CLI flags.
/// The shared `RunArgs` type carries only core pipeline fields.
pub fn nvidia_config_from_args(_args: &RunArgs) -> NvidiaConfig {
    NvidiaConfig {
        api_key: std::env::var("NVIDIA_API_KEY")
            .unwrap_or_else(|_| "nvapi-placeholder-key".to_string()),
        base_url: std::env::var("NVIDIA_BASE_URL")
            .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string()),
        model: std::env::var("NVIDIA_MODEL")
            .unwrap_or_else(|_| "minimaxai/minimax-m2.7".to_string()),
        temperature: std::env::var("NVIDIA_TEMPERATURE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0),
        top_p: std::env::var("NVIDIA_TOP_P")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.95),
        max_tokens: std::env::var("NVIDIA_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8192),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bacon_core::cli_types::RunArgs;

    #[test]
    fn test_nvidia_config_from_args_falls_back_to_env() {
        std::env::set_var("NVIDIA_API_KEY", "env-test-key");
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "env-test-key");
        std::env::remove_var("NVIDIA_API_KEY");
    }

    #[test]
    fn test_nvidia_config_from_args_placeholder_when_no_env() {
        std::env::remove_var("NVIDIA_API_KEY");
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "nvapi-placeholder-key");
    }
}
