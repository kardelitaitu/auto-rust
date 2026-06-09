use crate::core::PipelineConfig;
use crate::llm::models::{LlmProvider, NvidiaConfig};
use std::ffi::OsString;
use std::sync::{LazyLock, Mutex};

#[allow(dead_code)]
static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[allow(dead_code)]
struct EnvGuard {
    vars: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    #[allow(dead_code)]
    fn new(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let mut collected = Vec::new();
        for &(name, ref value) in vars {
            let original = std::env::var_os(name);
            match value {
                Some(v) => {
                    std::env::set_var(name, v);
                }
                None => {
                    std::env::remove_var(name);
                }
            }
            collected.push((name, original));
        }
        Self { vars: collected }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for &(name, ref original) in &self.vars {
            match original {
                Some(val) => std::env::set_var(name, val),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Shared pipeline run arguments.
///
/// Re-exported from `core::cli_types` to provide a single
/// canonical CLI interface for all bacon agents.
pub use crate::core::cli_types::RunArgs;

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_f64(name: &str) -> Option<f64> {
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
        api_key: default_api_key,
        base_url: default_base_url,
        model: default_model,
        temperature: default_temperature,
        top_p: default_top_p,
        max_tokens: default_max_tokens,
        timeout_ms: _default_timeout_ms,
        provider: _,
    } = defaults;

    let api_key_val = env_string("NVIDIA_API_KEY")
        .or(agent_cfg.api_key)
        .unwrap_or(default_api_key);
    // Debug: print the values
    // println!("DEBUG: env NVIDIA_API_KEY = {:?}", env_string("NVIDIA_API_KEY"));
    // println!("DEBUG: agent_cfg.api_key = {:?}", agent_cfg.api_key);
    // println!("DEBUG: chosen api_key = {}", api_key_val);

    NvidiaConfig {
        provider: match agent_cfg.provider.as_deref() {
            Some("ollama") => LlmProvider::Ollama,
            _ => LlmProvider::Nvidia,
        },
        api_key: api_key_val,
        base_url: env_string("NVIDIA_BASE_URL")
            .or(agent_cfg.base_url)
            .unwrap_or(default_base_url),
        model: env_string("NVIDIA_MODEL")
            .or(agent_cfg.model)
            .unwrap_or(default_model),
        temperature: env_f64("NVIDIA_TEMPERATURE")
            .or(agent_cfg.temperature)
            .unwrap_or(default_temperature),
        top_p: env_f64("NVIDIA_TOP_P")
            .or(agent_cfg.top_p)
            .unwrap_or(default_top_p),
        max_tokens: env_u32("NVIDIA_MAX_TOKENS")
            .or_else(|| agent_cfg.max_tokens.map(|v| v as u32))
            .unwrap_or(default_max_tokens),
        timeout_ms: env_u32("NVIDIA_TIMEOUT_MS")
            .map(|v| v as u64)
            .or(agent_cfg.timeout_ms)
            .unwrap_or(_default_timeout_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cli_types::RunArgs;
    use tempfile::tempdir;

    #[test]
    fn test_nvidia_config_from_args_falls_back_to_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = tempdir().unwrap();
        let bacon_toml_path = temp_dir.path().join("bacon.toml");
        std::fs::write(&bacon_toml_path, "").unwrap();
        let _guard = EnvGuard::new(&[
            ("BACON_CONFIG", Some(bacon_toml_path.into_os_string())),
            ("NVIDIA_API_KEY", Some(OsString::from("env-test-key"))),
        ]);
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "env-test-key");
    }

    #[test]
    fn test_nvidia_config_from_args_placeholder_when_no_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = tempdir().unwrap();
        let bacon_toml_path = temp_dir.path().join("bacon.toml");
        std::fs::write(&bacon_toml_path, "").unwrap();
        let _guard = EnvGuard::new(&[
            ("BACON_CONFIG", Some(bacon_toml_path.into_os_string())),
            ("NVIDIA_API_KEY", Some(OsString::from(""))),
        ]);
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.api_key, "");
    }

    #[test]
    fn test_nvidia_config_from_args_uses_default_when_env_blank() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = tempdir().unwrap();
        let bacon_toml_path = temp_dir.path().join("bacon.toml");
        std::fs::write(&bacon_toml_path, "").unwrap();
        let _guard = EnvGuard::new(&[
            ("BACON_CONFIG", Some(bacon_toml_path.into_os_string())),
            ("NVIDIA_MODEL", Some(OsString::from(""))),
        ]);
        let args = RunArgs::default();
        let config = nvidia_config_from_args(&args);
        assert_eq!(config.model, "meta/llama-3.3-70b-instruct");
    }
}
