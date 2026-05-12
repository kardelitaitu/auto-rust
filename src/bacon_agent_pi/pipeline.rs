use anyhow::{Context, Result};
use log::{info, warn};
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;

use crate::llm::Llm;

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

#[derive(Debug)]
pub enum Stage {
    Observer,
    Strategist,
    Coder,
    Auditor,
}

impl Stage {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "observer" => Some(Self::Observer),
            "strategist" => Some(Self::Strategist),
            "coder" => Some(Self::Coder),
            "auditor" => Some(Self::Auditor),
            _ => None,
        }
    }
}

/// Per-agent LLM configuration from bacon.toml [agents.<name>]
#[derive(Debug, Clone, Deserialize)]
pub struct AgentLlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipelineConfig {
    pub observer: String,
    pub strategist: String,
    pub coder: String,
    pub auditor: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            observer: "pi".into(),
            strategist: "pi".into(),
            coder: "pi".into(),
            auditor: "pi".into(),
        }
    }
}

impl PipelineConfig {
    pub fn from_bacon_toml() -> Self {
        let config_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".bacon/bacon.toml");
        if !config_path.exists() {
            return Self::default();
        }
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let table: toml::Value = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        let pipeline = match table.get("pipeline") {
            Some(v) => v,
            None => return Self::default(),
        };
        fn get(v: &toml::Value, key: &str) -> String {
            v.get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "pi".to_string())
        }
        Self {
            observer: get(pipeline, "observer"),
            strategist: get(pipeline, "strategist"),
            coder: get(pipeline, "coder"),
            auditor: get(pipeline, "auditor"),
        }
    }

    pub fn agent_for(&self, stage: &Stage) -> &str {
        match stage {
            Stage::Observer => &self.observer,
            Stage::Strategist => &self.strategist,
            Stage::Coder => &self.coder,
            Stage::Auditor => &self.auditor,
        }
    }

    /// Read LLM config for a specific agent from bacon.toml [agents.<name>]
    pub fn agent_llm_config(agent: &str) -> AgentLlmConfig {
        let config_path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".bacon/bacon.toml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => {
                return AgentLlmConfig {
                    provider: None,
                    model: None,
                    temperature: None,
                }
            }
        };
        let table: toml::Value = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => {
                return AgentLlmConfig {
                    provider: None,
                    model: None,
                    temperature: None,
                }
            }
        };
        let agents = match table.get("agents") {
            Some(v) => v,
            None => {
                return AgentLlmConfig {
                    provider: None,
                    model: None,
                    temperature: None,
                }
            }
        };
        let agent_cfg = match agents.get(agent) {
            Some(v) => v,
            None => {
                return AgentLlmConfig {
                    provider: None,
                    model: None,
                    temperature: None,
                }
            }
        };
        AgentLlmConfig {
            provider: agent_cfg
                .get("provider")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            model: agent_cfg
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            temperature: agent_cfg.get("temperature").and_then(|v| v.as_float()),
        }
    }
}

pub struct Pipeline {
    pub args: RunArgs,
    pub dry_run: bool,
    pub auto: bool,
    pub llm: Llm,
    pub pipeline_cfg: PipelineConfig,
}

impl Pipeline {
    pub fn new(args: RunArgs, dry_run: bool, auto: bool) -> Result<Self> {
        let llm = Llm::new().map_err(|e| anyhow::anyhow!("failed to initialize LLM: {}", e))?;
        let pipeline_cfg = PipelineConfig::from_bacon_toml();
        Ok(Self {
            args,
            dry_run,
            auto,
            llm,
            pipeline_cfg,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let resume_stage = self.args.stage.as_deref().and_then(Stage::from_name);

        if let Some(stage) = &resume_stage {
            info!("Resuming from stage: {:?}", stage);
        }

        if self.dry_run {
            info!("DRY RUN — no files will be modified");
        }

        // Crash recovery: check for stale in-progress specs
        check_stale_in_progress()?;

        // Fast path: skip strategist + auditor if --fast
        let fast_path = self.args.fast;

        let base_ctx = PipelineCtx::new(String::new()).with_dry_run(self.dry_run);

        // Observer
        let mut ctx = if should_run(&resume_stage, Stage::Observer) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Observer);
            info!("=== Stage 1: Observer (agent: {}) ===", agent);
            log_agent_config(agent);
            if agent == "pi" {
                super::observer::run(&self.llm, &self.args, &base_ctx).await?
            } else {
                let prompt = self
                    .args
                    .prompt
                    .as_deref()
                    .unwrap_or("scan for improvements");
                run_external_agent(agent, "observer", prompt)?;
                PipelineCtx::new(format!("Delegated to {}", agent)).with_dry_run(self.dry_run)
            }
        } else {
            base_ctx
        };
        ctx.dry_run = self.dry_run;

        // Strategist (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Strategist) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Strategist);
            info!("=== Stage 2: Strategist (agent: {}) ===", agent);
            log_agent_config(agent);
            ctx = if agent == "pi" {
                super::strategist::run(&self.llm, &self.args, &ctx).await?
            } else {
                run_external_agent(agent, "strategist", &ctx.description)?;
                PipelineCtx::new(format!("Delegated to {}", agent)).with_dry_run(self.dry_run)
            };

            // User confirmation gate
            if !self.auto && !confirm("Implement this plan? [Y/n]: ", true)? {
                info!("User declined — aborting pipeline");
                return Ok(());
            }
        }

        // Coder
        if should_run(&resume_stage, Stage::Coder) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Coder);
            info!("=== Stage 3: Coder (agent: {}) ===", agent);
            log_agent_config(agent);
            ctx = if agent == "pi" {
                super::coder::run(&self.llm, &self.args, &ctx).await?
            } else {
                run_external_agent(agent, "coder", &ctx.description)?;
                PipelineCtx::new(format!("Delegated to {}", agent)).with_dry_run(self.dry_run)
            };

            // User confirmation gate
            if !self.auto && !confirm("Apply this diff? [y/N]: ", false)? {
                info!("User declined diff — aborting pipeline");
                return Ok(());
            }
        }

        // Auditor (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Auditor) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Auditor);
            info!("=== Stage 4: Auditor (agent: {}) ===", agent);
            log_agent_config(agent);
            if agent == "pi" {
                super::auditor::run(&self.llm, &self.args, &ctx).await?;
            } else {
                run_external_agent(agent, "auditor", &ctx.description)?;
            }
        }

        info!("Pipeline complete");
        Ok(())
    }
}

/// Prompt user for yes/no confirmation. Default if empty input.
fn confirm(prompt: &str, default: bool) -> Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    print!("{} ", prompt.replace("[Y/n]", hint));
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Ok(default);
    }
    Ok(input == "y" || input == "yes")
}

fn log_agent_config(agent: &str) {
    let cfg = PipelineConfig::agent_llm_config(agent);
    if let Some(provider) = &cfg.provider {
        info!("  Agent config: provider={}", provider);
    }
    if let Some(model) = &cfg.model {
        info!("  Agent config: model={}", model);
    }
}

fn check_stale_in_progress() -> Result<()> {
    let active = spec_io::list_active_specs()?;
    for spec_path in &active {
        let meta = match spec_io::read_spec_meta(spec_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.status == "in-progress" {
            let name = spec_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            warn!("Stale in-progress spec found: {} ({})", meta.title, name);
            warn!("Manual review recommended before continuing");
        }
    }
    Ok(())
}

pub fn run_external_agent(agent: &str, role: &str, prompt: &str) -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: &[&str] = if agent == "kilocode" || agent == "kilo" {
        &["run", prompt, "--role", role]
    } else {
        &["-p", prompt, "--role", role]
    };

    let status = std::process::Command::new(agent)
        .args(args)
        .current_dir(&root)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute agent '{}'", agent))?;

    if !status.success() {
        anyhow::bail!("agent '{}' exited with code {}", agent, status);
    }
    Ok(())
}

pub fn run_powershell(script: &str) -> Result<(bool, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };
    let output = std::process::Command::new(shell)
        .args(["-NoProfile", "-File", script])
        .current_dir(&root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };
    Ok((output.status.success(), combined))
}

fn should_run(resume: &Option<Stage>, current: Stage) -> bool {
    match resume {
        None => true,
        Some(_) => {
            let order = [
                Stage::Observer,
                Stage::Strategist,
                Stage::Coder,
                Stage::Auditor,
            ];
            let resume_idx = order
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(&current));
            let current_idx = order
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(&current));
            match (resume_idx, current_idx) {
                (Some(r), Some(c)) => c >= r,
                _ => true,
            }
        }
    }
}
