use super::nvidia_api::NvidiaConfig;
use crate::bacon_core::PipelineConfig;

/// Shared pipeline run arguments.
///
/// Re-exported from `bacon_core::cli_types` to provide a single
/// canonical CLI interface for all bacon agents.
pub use crate::bacon_core::cli_types::RunArgs;

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_f32(name: &str) -> Option<f32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
}

fn env_u32(name: &str) -> Option<u32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
}

/// Build an `NvidiaConfig` from pipeline run args + env and bacon.toml fallbacks.
///
/// Env vars win first, then `.bacon/bacon.toml [agents.nvidia]`, then the
/// built-in llama defaults. The shared `RunArgs` type carries only core
/// pipeline fields.
#[must_use]
pub fn nvidia_config_from_args(_args: &RunArgs) -> NvidiaConfig {
    let defaults = NvidiaConfig::default();
    let agent_cfg = PipelineConfig::agent_llm_config("nvidia");

    let NvidiaConfig {
        api_key,
        base_url: default_base_url,
        model: default_model,
        temperature: default_temperature,
        top_p: default_top_p,
        max_tokens: default_max_tokens,
    } = defaults;

    NvidiaConfig {
        api_key,
        base_url: env_string("NVIDIA_BASE_URL")
            .or(agent_cfg.base_url)
            .unwrap_or(default_base_url),
        model: env_string("NVIDIA_MODEL")
            .or(agent_cfg.model)
            .unwrap_or(default_model),
        temperature: env_f32("NVIDIA_TEMPERATURE")
            .or(agent_cfg.temperature.map(|v| v as f32))
            .unwrap_or(default_temperature),
        top_p: env_f32("NVIDIA_TOP_P")
            .or(agent_cfg.top_p.map(|v| v as f32))
            .unwrap_or(default_top_p),
        max_tokens: env_u32("NVIDIA_MAX_TOKENS")
            .or(agent_cfg.max_tokens.map(|v| v as u32))
            .unwrap_or(default_max_tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bacon_agent_nvidia::nvidia_api::DEFAULT_NVIDIA_MODEL;
    use crate::bacon_core::cli_types::RunArgs;

    #[test]
    fn test_nvidia_config_from_args_falls_back_to_env() {
        let _guard = crate::bacon_agent_nvidia::nvidia_api::test_env_guard();
        std::env::set_var("NVIDIA_API_KEY", "env-test-key");
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "env-test-key");
        std::env::remove_var("NVIDIA_API_KEY");
    }

    #[test]
    fn test_nvidia_config_from_args_placeholder_when_no_env() {
        let _guard = crate::bacon_agent_nvidia::nvidia_api::test_env_guard();
        std::env::set_var("NVIDIA_API_KEY", "");
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "nvapi-placeholder-key");
        std::env::remove_var("NVIDIA_API_KEY");
    }

    #[test]
    fn test_nvidia_config_from_args_uses_bacon_toml_model_when_env_blank() {
        let _guard = crate::bacon_agent_nvidia::nvidia_api::test_env_guard();
        std::env::set_var("NVIDIA_MODEL", "");
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.model, DEFAULT_NVIDIA_MODEL);
        std::env::remove_var("NVIDIA_MODEL");
    }
}
