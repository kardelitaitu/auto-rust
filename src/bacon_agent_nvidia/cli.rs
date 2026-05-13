use clap::Parser;

use super::nvidia_api::NvidiaConfig;

#[derive(Parser, Debug)]
#[command(
    name = "bacon_agent_nvidia",
    about = "NVIDIA AI agent for Bacon pipeline",
    version
)]
pub struct RunArgs {
    /// User prompt describing the task
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Pipeline stage to run
    #[arg(long)]
    pub role: Option<String>,

    /// Resume from a specific stage
    #[arg(long)]
    pub stage: Option<String>,

    /// Skip strategist + auditor (fast path)
    #[arg(long)]
    pub fast: bool,

    /// Skip confirmation gates
    #[arg(long)]
    pub auto: bool,

    /// Automatically apply approved patches to the working tree
    #[arg(long)]
    pub auto_apply: bool,

    /// Sandbox mode (no writes)
    #[arg(long)]
    pub dry_run: bool,

    /// Resume a specific spec by number
    #[arg(long)]
    pub spec: Option<u32>,

    /// API key for NVIDIA
    #[arg(long)]
    pub api_key: Option<String>,

    /// Base URL for NVIDIA API
    #[arg(long, default_value = "https://integrate.api.nvidia.com/v1")]
    pub base_url: String,

    /// Model to use
    #[arg(long, default_value = "minimaxai/minimax-m2.7")]
    pub model: String,

    /// Temperature for generation
    #[arg(long, default_value = "1.0")]
    pub temperature: f32,

    /// Top-p for generation
    #[arg(long, default_value = "0.95")]
    pub top_p: f32,

    /// Max tokens to generate
    #[arg(long, default_value = "8192")]
    pub max_tokens: u32,

    /// Log level (debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

impl RunArgs {
    pub fn nvidia_config(&self) -> NvidiaConfig {
        NvidiaConfig {
            api_key: self
                .api_key
                .clone()
                .or_else(|| std::env::var("NVIDIA_API_KEY").ok())
                .unwrap_or_else(|| "nvapi-placeholder-key".to_string()),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
        }
    }
}
