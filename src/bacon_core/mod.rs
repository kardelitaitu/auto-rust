//! Shared pipeline core — types and utility functions used by all bacon agent pipelines.
//!
//! This module consolidates duplicated code from `bacon_agent_pi/pipeline.rs`
//! and `bacon_agent_nvidia/pipeline.rs` into a single canonical location.
//! Agent-specific pipeline orchestrators import from here rather than
//! redefining the same types and helpers.
pub mod agent;
pub use agent::PipelineAgent;

pub mod git_snapshot;
pub use git_snapshot::GitSnapshot;

pub mod spec_io;

use anyhow::{Context, Result};
use log::{debug, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Pipeline types
// ---------------------------------------------------------------------------

/// Pipeline stage identifiers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    Observer,
    Strategist,
    Coder,
    Auditor,
}

/// Pipeline stage confidence level, extracted from `Confidence: High/Medium/Low`
/// lines in LLM responses. Used for metrics and observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    /// Parse from a string slice. Case-insensitive.
    pub fn from_string(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            _ => None,
        }
    }

    /// Return a static string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    /// Return a numeric score (0.0–1.0) for metrics aggregation.
    /// High = 1.0, Medium = 0.5, Low = 0.0
    pub fn to_score(&self) -> f64 {
        match self {
            Self::High => 1.0,
            Self::Medium => 0.5,
            Self::Low => 0.0,
        }
    }
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

/// Per-agent LLM configuration from `bacon.toml [agents.<name>]`.
///
/// All fields are optional so agents may override only what they need.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentLlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub command_args: Option<Vec<String>>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u64>,
}

impl AgentLlmConfig {
    pub fn empty() -> Self {
        Self {
            provider: None,
            model: None,
            temperature: None,
            command_args: None,
            api_key: None,
            base_url: None,
            top_p: None,
            max_tokens: None,
        }
    }
}

/// JSON output contract for external CLI workers.
///
/// Workers must print exactly one JSON object on stdout matching this shape.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkerOutput {
    pub status: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub spec_path: Option<PathBuf>,
}

impl WorkerOutput {
    pub fn into_ctx(self, root: &Path, dry_run: bool) -> Result<PipelineCtx> {
        let status = self.status.as_deref().unwrap_or("ok").to_lowercase();
        if matches!(
            status.as_str(),
            "error" | "fail" | "failed" | "reject" | "rejected"
        ) {
            anyhow::bail!("worker returned terminal status: {}", status);
        }

        let description = self
            .description
            .or(self.summary)
            .unwrap_or_else(|| format!("Worker completed with status: {}", status));
        let mut ctx = PipelineCtx::new(description).with_dry_run(dry_run);

        if let Some(spec_path) = self.spec_path {
            ctx.spec_path = Some(if spec_path.is_absolute() {
                spec_path
            } else {
                root.join(spec_path)
            });
        }

        Ok(ctx)
    }
}

/// Agent routing configuration read from `[pipeline]` in `bacon.toml`.
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
            observer: "bacon".into(),
            strategist: "bacon".into(),
            coder: "bacon".into(),
            auditor: "bacon".into(),
        }
    }
}

impl PipelineConfig {
    /// Read pipeline agent routing from `.bacon/bacon.toml`.
    pub fn from_bacon_toml() -> Self {
        let config_path = manifest_dir().join(".bacon/bacon.toml");
        if !config_path.exists() {
            warn!(
                "bacon.toml not found at {} — using default agent routing",
                config_path.display()
            );
            return Self::default();
        }
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "failed to read bacon.toml ({}): {} — using defaults",
                    config_path.display(),
                    e
                );
                return Self::default();
            }
        };
        let table: toml::Value = match toml::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                warn!("failed to parse bacon.toml: {} — using defaults", e);
                return Self::default();
            }
        };
        let pipeline = match table.get("pipeline") {
            Some(v) => v,
            None => {
                warn!("bacon.toml missing [pipeline] section — using default agent routing");
                return Self::default();
            }
        };
        fn get(v: &toml::Value, key: &str) -> String {
            v.get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "bacon".to_string())
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

    /// Read LLM config for a specific agent from `bacon.toml [agents.<name>]`.
    pub fn agent_llm_config(agent: &str) -> AgentLlmConfig {
        let config_path = manifest_dir().join(".bacon/bacon.toml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => {
                debug!(
                    "bacon.toml not readable for agent '{}' config — returning empty",
                    agent
                );
                return AgentLlmConfig::empty();
            }
        };
        let table: toml::Value = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => {
                debug!(
                    "bacon.toml parse error for agent '{}' config — returning empty",
                    agent
                );
                return AgentLlmConfig::empty();
            }
        };
        let agents = match table.get("agents") {
            Some(v) => v,
            None => {
                debug!(
                    "no [agents] section in bacon.toml for '{}' — returning empty",
                    agent
                );
                return AgentLlmConfig::empty();
            }
        };
        let agent_cfg = match agents.get(agent) {
            Some(v) => v,
            None => {
                debug!(
                    "no [agents.{}] config in bacon.toml — returning empty",
                    agent
                );
                return AgentLlmConfig::empty();
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
            command_args: agent_cfg
                .get("command_args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                }),
            api_key: agent_cfg
                .get("api_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            base_url: agent_cfg
                .get("base_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            top_p: agent_cfg.get("top_p").and_then(|v| v.as_float()),
            max_tokens: agent_cfg
                .get("max_tokens")
                .and_then(|v| v.as_integer())
                .map(|v| v as u64),
        }
    }
}

/// Mutable pipeline state passed between stages.
#[derive(Debug, Clone)]
pub struct PipelineCtx {
    pub description: String,
    pub spec_path: Option<PathBuf>,
    pub dry_run: bool,
    /// Set true by Coder when all retries exhausted — signals pipeline to
    /// call Strategist for scope reduction instead of hard-failing.
    pub scope_reduction_needed: bool,
    /// Accumulated Coder error messages fed back for scope reduction context.
    pub coder_errors: Vec<String>,
    /// How many scope reductions have been applied this pipeline run.
    pub scope_reduction_count: u32,
    /// Confidence level extracted from the most recent pipeline stage response.
    pub confidence: Option<Confidence>,
    /// Path to the approved patch file saved by the Coder, for Auditor review.
    pub patch_path: Option<PathBuf>,
    /// Set true by Coder when the LLM refuses to implement (2+ consecutive refusals).
    /// Tells the orchestrator to abort without scope reduction or audit.
    pub coder_refused: bool,
    /// Set true by Coder when auto-apply fails — tells orchestrator to skip
    /// the Auditor and abort with needs-human-approval status.
    pub needs_human_approval: bool,
}

impl PipelineCtx {
    pub fn new(description: String) -> Self {
        Self {
            description,
            spec_path: None,
            dry_run: false,
            scope_reduction_needed: false,
            coder_errors: Vec::new(),
            scope_reduction_count: 0,
            confidence: None,
            patch_path: None,
            coder_refused: false,
            needs_human_approval: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_confidence(mut self, confidence: Option<Confidence>) -> Self {
        self.confidence = confidence;
        self
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Prompt user for yes/no confirmation. Default if empty input.
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
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

/// Log LLM configuration for a specific agent (for observability).
pub fn log_agent_config(agent: &str) {
    let cfg = PipelineConfig::agent_llm_config(agent);
    if let Some(provider) = &cfg.provider {
        info!("  Agent config: provider={}", provider);
    }
    if let Some(model) = &cfg.model {
        info!("  Agent config: model={}", model);
    }
    if let Some(base_url) = &cfg.base_url {
        info!("  Agent config: base_url={}", base_url);
    }
    if let Some(temp) = cfg.temperature {
        info!("  Agent config: temperature={}", temp);
    }
    if let Some(max_tokens) = cfg.max_tokens {
        info!("  Agent config: max_tokens={}", max_tokens);
    }
}

/// Validate that the agent names in `[pipeline]` have corresponding `[agents.<name>]` sections.
/// Logs warnings for any missing agent configs.
pub fn validate_pipeline_config(pipeline: &PipelineConfig) {
    for stage in [
        Stage::Observer,
        Stage::Strategist,
        Stage::Coder,
        Stage::Auditor,
    ] {
        let agent = pipeline.agent_for(&stage);
        let cfg = PipelineConfig::agent_llm_config(agent);
        if cfg.provider.is_none() && cfg.command_args.is_none() {
            warn!(
                "Pipeline stage {:?} uses agent '{}' which has no [agents.{}] config in bacon.toml",
                stage, agent, agent
            );
        }
    }
}

/// Validate that Bacon is running with a local-only LLM provider.
pub fn validate_bacon_local_only() -> Result<()> {
    let root = manifest_dir();
    let bacon_config = root.join(".bacon/bacon.toml");
    let llm_config = root.join("config/llm.toml");

    if let Ok(provider) = std::env::var("LLM_PROVIDER") {
        if provider.eq_ignore_ascii_case("openrouter") {
            anyhow::bail!("Bacon requires local LLM access; LLM_PROVIDER=openrouter is disabled");
        }
    }

    if llm_config.exists() {
        let content = std::fs::read_to_string(&llm_config)
            .with_context(|| format!("reading {}", llm_config.display()))?;
        let table: toml::Value = toml::from_str(&content)
            .with_context(|| format!("parsing {}", llm_config.display()))?;
        if table
            .get("provider")
            .and_then(|v| v.as_str())
            .is_some_and(|p| p.eq_ignore_ascii_case("openrouter"))
        {
            anyhow::bail!("Bacon requires config/llm.toml provider = \"ollama\"");
        }
    }

    if bacon_config.exists() {
        let content = std::fs::read_to_string(&bacon_config)
            .with_context(|| format!("reading {}", bacon_config.display()))?;
        let table: toml::Value = toml::from_str(&content)
            .with_context(|| format!("parsing {}", bacon_config.display()))?;
        if let Some(agents) = table.get("agents").and_then(|v| v.as_table()) {
            for (name, cfg) in agents {
                let provider = cfg
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ollama");
                let allowed = provider.eq_ignore_ascii_case("ollama")
                    || provider.eq_ignore_ascii_case("cli")
                    || provider.eq_ignore_ascii_case("nvidia");
                if !allowed {
                    anyhow::bail!(
                        "Bacon agent '{}' uses unsupported provider '{}'; use ollama, cli, or nvidia",
                        name,
                        provider
                    );
                }
                if name == "bacon" && !provider.eq_ignore_ascii_case("ollama") {
                    anyhow::bail!("Bacon supervisor agent 'bacon' must use provider = \"ollama\"");
                }
            }
        }
    }

    Ok(())
}

/// Scan `docs/specs/_active/` for specs with `status: in-progress` and warn.
pub fn check_stale_in_progress() -> Result<()> {
    let root = manifest_dir();
    let active_dir = root.join("docs/specs/_active");
    if !active_dir.exists() {
        return Ok(());
    }

    let mut entries: Vec<_> = match std::fs::read_dir(&active_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            debug!("check_stale_in_progress: failed to read _active dir: {}", e);
            return Ok(());
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let spec_path = entry.path();
        if !spec_path.is_dir() {
            continue;
        }
        let meta_path = spec_path.join("spec.yaml");
        let meta_content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    "check_stale_in_progress: cannot read {}: {}",
                    meta_path.display(),
                    e
                );
                continue;
            }
        };
        let meta: serde_yml::Value = match serde_yml::from_str(&meta_content) {
            Ok(v) => v,
            Err(e) => {
                debug!(
                    "check_stale_in_progress: cannot parse {}: {}",
                    meta_path.display(),
                    e
                );
                continue;
            }
        };
        let status = meta.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status == "in-progress" {
            let title = meta
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(untitled)");
            let name = spec_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            warn!("Stale in-progress spec found: {} ({})", title, name);

            // Auto-recovery: if the spec directory is older than 30 minutes,
            // reset status to "approved" so the pipeline can retry.
            let metadata = match std::fs::metadata(&spec_path) {
                Ok(m) => m,
                Err(_) => {
                    warn!(
                        "  Cannot read metadata for {} — skipping auto-recovery",
                        name
                    );
                    continue;
                }
            };
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    const STALE_TIMEOUT: std::time::Duration =
                        std::time::Duration::from_secs(30 * 60); // 30 minutes
                    if elapsed > STALE_TIMEOUT {
                        info!(
                            "  Auto-recovering stale spec {} — resetting status to approved ({:.0}s old)",
                            name,
                            elapsed.as_secs()
                        );
                        // Re-read spec.yaml, update status, write back atomically
                        if let Some(mut meta_value) = std::fs::read_to_string(&meta_path)
                            .ok()
                            .and_then(|c| serde_yml::from_str::<serde_yml::Value>(&c).ok())
                        {
                            if let Some(map) = meta_value.as_mapping_mut() {
                                map.insert(
                                    serde_yml::Value::String("status".to_string()),
                                    serde_yml::Value::String("approved".to_string()),
                                );
                                let tmp = spec_path.join("spec.yaml.tmp");
                                if let Ok(content) = serde_yml::to_string(&meta_value) {
                                    let _ = std::fs::write(&tmp, &content);
                                    let _ = std::fs::rename(&tmp, &meta_path);
                                }
                            }
                        }
                    } else {
                        info!(
                            "  Spec {} is only {:.0}s old — not auto-recovering yet (threshold: 30m)",
                            name,
                            elapsed.as_secs()
                        );
                    }
                } else {
                    warn!(
                        "  Cannot determine age for {} — skipping auto-recovery",
                        name
                    );
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// External agent execution
// ---------------------------------------------------------------------------

/// Read the role prompt markdown for an external agent from `.bacon/roles/`.
pub fn read_role_prompt(role: &str) -> String {
    let file = match role {
        "observer" => "01_bacon-observer.md",
        "strategist" => "02_bacon-strategy.md",
        "coder" => "03_bacon-coder.md",
        "auditor" => "04_bacon-auditor.md",
        _ => return String::new(),
    };
    let path: PathBuf = [manifest_dir(), ".bacon/roles".into(), file.into()]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_default()
}

/// Validate that an agent name is safe to use in path resolution.
/// Rejects names containing path separators or parent directory references.
fn is_safe_agent_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && name != "."
        && name != ".."
}

/// Resolve an agent binary path. Checks cargo build output first, then PATH.
/// Validates agent name safety before resolving unless the name is an explicit path.
pub fn resolve_agent_binary(agent: &str, root: &Path) -> PathBuf {
    // If the agent name contains a path separator, treat it as an explicit path
    // and skip validation (e.g., "./fake-worker.sh", "/tmp/worker", "agent.exe").
    if agent.contains('/') || agent.contains('\\') {
        return PathBuf::from(agent);
    }
    if !is_safe_agent_name(agent) {
        warn!(
            "Unsafe agent name '{}' — only alphanumeric, hyphens, and underscores allowed. \
             Falling back to 'bacon'.",
            agent
        );
        return root.join("target").join("debug").join(if cfg!(windows) {
            "bacon.exe"
        } else {
            "bacon"
        });
    }
    let candidates = [
        root.join("target").join("debug").join(agent),
        root.join("target").join("release").join(agent),
    ];
    for candidate in &candidates {
        let with_ext = if cfg!(windows) {
            let mut p = candidate.to_path_buf();
            p.set_extension("exe");
            p
        } else {
            candidate.to_path_buf()
        };
        if with_ext.exists() {
            return with_ext;
        }
    }
    // Fall back to bare name (lookup in PATH)
    PathBuf::from(agent)
}

/// Extract the outermost JSON object `{...}` from text that may have log prefixes.
/// Returns `None` if no JSON object is found.
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let rest = &text[start..];
    let mut depth = 0u32;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&rest[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Spawn an external agent binary and parse its JSON output.
///
/// Wraps the agent in its role prompt, resolves the binary, appends model
/// config from `bacon.toml` as CLI arguments, and parses the stdout into a
/// [`PipelineCtx`].
pub fn run_external_agent(
    agent: &str,
    role: &str,
    prompt_raw: &str,
    dry_run: bool,
) -> Result<PipelineCtx> {
    let root = manifest_dir();
    let role_prompt = read_role_prompt(role);
    let prompt_owned: String;
    let prompt: &str = if role_prompt.is_empty() {
        prompt_raw
    } else {
        prompt_owned = format!(
            "Follow these role instructions:\n\n{}\n\nHere is the task:\n\n{}",
            role_prompt, prompt_raw
        );
        &prompt_owned
    };

    // Resolve agent binary: check cargo build output first, then PATH
    let agent_path = resolve_agent_binary(agent, &root);

    // Get agent config to check for custom command args
    let agent_cfg = PipelineConfig::agent_llm_config(agent);
    let args: Vec<String> = if let Some(template_args) = &agent_cfg.command_args {
        // Replace {prompt} and {role} placeholders
        template_args
            .iter()
            .map(|arg| arg.replace("{prompt}", prompt).replace("{role}", role))
            .collect()
    } else {
        // Fallback to hardcoded logic
        if agent == "kilocode" || agent == "kilo" {
            vec![
                "run".to_string(),
                prompt.to_string(),
                "--role".to_string(),
                role.to_string(),
            ]
        } else {
            vec![
                "-p".to_string(),
                prompt.to_string(),
                "--role".to_string(),
                role.to_string(),
            ]
        }
    };
    let mut args = args;
    if dry_run && !args.iter().any(|arg| arg == "--dry-run") {
        args.push("--dry-run".to_string());
    }

    // Append model config from bacon.toml as CLI args if available
    if let Some(model) = &agent_cfg.model {
        if !args.iter().any(|a| a == "--model") {
            args.push("--model".to_string());
            args.push(model.clone());
        }
    }
    if let Some(base_url) = &agent_cfg.base_url {
        if !args.iter().any(|a| a == "--base-url") {
            args.push("--base-url".to_string());
            args.push(base_url.clone());
        }
    }
    if let Some(temp) = agent_cfg.temperature {
        if !args.iter().any(|a| a == "--temperature") {
            args.push("--temperature".to_string());
            args.push(format!("{}", temp));
        }
    }
    if let Some(top_p) = agent_cfg.top_p {
        if !args.iter().any(|a| a == "--top-p") {
            args.push("--top-p".to_string());
            args.push(format!("{}", top_p));
        }
    }
    if let Some(max_tokens) = agent_cfg.max_tokens {
        if !args.iter().any(|a| a == "--max-tokens") {
            args.push("--max-tokens".to_string());
            args.push(format!("{}", max_tokens));
        }
    }

    let cmd_line = format!("{} {}", agent, args.join(" "));
    let prompt_preview = if prompt.len() > 80 {
        format!("{}... ({} chars)", &prompt[..80], prompt.len())
    } else {
        prompt.to_string()
    };
    info!("Spawning external agent: {}", cmd_line);
    debug!("Agent binary path: {}", agent_path.display());
    debug!("Agent prompt ({} role): {}", role, prompt_preview);

    let start = Instant::now();
    let child = std::process::Command::new(&agent_path)
        .args(&args)
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn agent '{}' at {}",
                agent,
                agent_path.display()
            )
        })?;

    // Read stdout while stderr streams to terminal in real-time
    let output = child.wait_with_output()?;
    let elapsed = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    info!(
        "Agent '{}' finished in {:.1}s ({} chars)",
        agent,
        elapsed.as_secs_f64(),
        stdout.len()
    );

    if !output.status.success() {
        anyhow::bail!(
            "agent '{}' exited with code {}\nstdout: {}",
            agent,
            output.status,
            stdout,
        );
    }

    // Attempt to extract JSON from stdout — handles log-prefixed output
    let json_str = extract_json_object(&stdout).unwrap_or(&stdout);
    let worker: WorkerOutput = serde_json::from_str(json_str).with_context(|| {
        format!(
            "agent '{}' must print WorkerOutput JSON on stdout; got: {}",
            agent, stdout
        )
    })?;

    worker.into_ctx(&root, dry_run)
}

// ---------------------------------------------------------------------------
// PowerShell helpers
// ---------------------------------------------------------------------------

/// Run a PowerShell script with no additional arguments.
pub fn run_powershell(script: &str) -> Result<(bool, String)> {
    run_powershell_with_args(script, &[])
}

/// Run a PowerShell script with additional arguments.
pub fn run_powershell_with_args(script: &str, args: &[&str]) -> Result<(bool, String)> {
    let root = manifest_dir();
    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };
    let mut command_args = vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-File".to_string(),
        script.to_string(),
    ];
    command_args.extend(args.iter().map(|arg| arg.to_string()));
    let output = std::process::Command::new(shell)
        .args(command_args)
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

// ---------------------------------------------------------------------------
// Stage ordering / resume
// ---------------------------------------------------------------------------

/// Determine whether a stage should run given an optional resume point.
pub fn should_run(resume: &Option<Stage>, current: Stage) -> bool {
    match resume {
        None => true,
        Some(resume_stage) => {
            let order = [
                Stage::Observer,
                Stage::Strategist,
                Stage::Coder,
                Stage::Auditor,
            ];
            let resume_idx = order
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(resume_stage));
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

/// Scan the project structure for a summary string (used in observer prompts).
pub fn scan_project_structure() -> String {
    let root = manifest_dir();
    let mut parts = Vec::new();

    let src = root.join("src");
    if src.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&src) {
            let mut dirs = Vec::new();
            let mut files = 0;
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    dirs.push(entry.file_name().to_string_lossy().to_string());
                } else if entry.path().extension().is_some_and(|x| x == "rs") {
                    files += 1;
                }
            }
            parts.push(format!(
                "Source modules: {} directories, {} Rust files",
                dirs.len(),
                files
            ));
            if !dirs.is_empty() {
                parts.push(format!("Directories: {}", dirs.join(", ")));
            }
        }
    }

    let bin = src.join("bin");
    if bin.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&bin) {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
                .count();
            parts.push(format!("Binaries: {} Rust files", count));
        }
    }

    // Check active specs via filesystem
    let active_dir = root.join("docs/specs/_active");
    if active_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&active_dir) {
            let count = entries.filter_map(|e| e.ok()).count();
            parts.push(format!("Active specs: {} specs", count));
        }
    } else {
        parts.push("Active specs: 0 specs".into());
    }

    parts.join("\n")
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

/// Get `CARGO_MANIFEST_DIR` at runtime.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Find and read actual source files referenced in spec text for inclusion in LLM prompts.
///
/// Parses text (e.g., plan.md) for file paths like `src/runtime/shutdown.rs`, validates
/// they exist on disk, reads their contents, and returns a formatted markdown section.
/// This lets the LLM work from actual on-disk code rather than hallucinating file contents.
pub fn collect_source_context(text: &str, max_files: usize, max_lines_per_file: usize) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut seen: HashSet<String> = HashSet::new();
    let mut contents: Vec<String> = Vec::new();

    // Match paths starting from project root directories (src/, tests/, docs/, config/, .bacon/)
    let path_re = Regex::new(r#"\b(?:src|tests|docs|config|scripts|\.bacon)/[\w./-]+\.(?:rs|toml|md|json|ps1|sh|yaml|yml)\b"#).unwrap();

    for m in path_re.find_iter(text) {
        let path_str = m.as_str();
        let full_path = root.join(path_str);
        if full_path.is_file() && seen.insert(path_str.to_string()) {
            // Skip files larger than max_lines_per_file × 80 bytes (avg line length)
            if let Ok(meta) = std::fs::metadata(&full_path) {
                let max_bytes = (max_lines_per_file * 80) as u64;
                if meta.len() > max_bytes {
                    let kb = meta.len() as f64 / 1024.0;
                    let estimated = meta.len() / 80;
                    contents.push(format!(
                        "### `{}` (~{} lines, {:.1}KB — skipped at threshold)\n```\n_(file skipped at limit)_\n```",
                        path_str, estimated, kb
                    ));
                    if contents.len() >= max_files {
                        break;
                    }
                    continue;
                }
            }
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let line_count = content.lines().count();
                let truncated = if line_count > max_lines_per_file {
                    let excerpt: String = content
                        .lines()
                        .take(max_lines_per_file)
                        .collect::<Vec<&str>>()
                        .join("\n");
                    format!(
                        "{}\n\n_(... truncated after {} of {} lines)_",
                        excerpt, max_lines_per_file, line_count
                    )
                } else {
                    content
                };
                contents.push(format!(
                    "### `{}` ({} lines)\n```\n{}\n```",
                    path_str, line_count, truncated
                ));
                if contents.len() >= max_files {
                    break;
                }
            }
        }
    }

    // Also match root-level files like Cargo.toml, build.rs, bacon.toml
    let root_re = Regex::new(r#"\b(?:Cargo\.toml|build\.rs|bacon\.toml|rust-toolchain\.toml|spec-lint\.ps1|check-fast\.ps1|check\.ps1)\b"#).unwrap();
    for m in root_re.find_iter(text) {
        if contents.len() >= max_files {
            break;
        }
        let path_str = m.as_str();
        if seen.contains(path_str) {
            continue;
        }
        let full_path = root.join(path_str);
        if full_path.is_file() && seen.insert(path_str.to_string()) {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let line_count = content.lines().count();
                let truncated = if line_count > max_lines_per_file {
                    let excerpt: String = content
                        .lines()
                        .take(max_lines_per_file)
                        .collect::<Vec<&str>>()
                        .join("\n");
                    format!(
                        "{}\n\n_(... truncated after {} of {} lines)_",
                        excerpt, max_lines_per_file, line_count
                    )
                } else {
                    content
                };
                contents.push(format!(
                    "### `{}` ({} lines)\n```\n{}\n```",
                    path_str, line_count, truncated
                ));
            }
        }
    }

    if contents.is_empty() {
        return String::new();
    }

    format!("\n\n## Relevant Source Files\n\n{}", contents.join("\n\n"))
}

/// Extract `Confidence: High/Medium/Low` from an LLM response string.
/// Handles markdown formatting like `**Confidence: High**` or `*Confidence: Low*`.
pub fn extract_confidence(response: &str) -> Option<Confidence> {
    for line in response.lines() {
        let trimmed = line.trim();
        // Strip leading markdown tokens (*, **, _, __) before searching
        let cleaned = trimmed
            .trim_start_matches(|c: char| ['*', '_'].contains(&c))
            .trim_end_matches(|c: char| ['*', '_'].contains(&c));
        // Search for "Confidence:" case-insensitively anywhere in the line
        let lower = cleaned.to_lowercase();
        if let Some(idx) = lower.find("confidence:") {
            let after = cleaned[idx + "confidence:".len()..].trim();
            // Strip trailing punctuation or markdown artifacts
            let val = after.trim_end_matches(|c: char| {
                c == '.' || c == ',' || c == '!' || c == '*' || c == '_'
            });
            if let Some(conf) = Confidence::from_string(val) {
                return Some(conf);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Shared helper functions — extracted from agent modules to prevent drift
// ---------------------------------------------------------------------------

/// Phrases that indicate an LLM is refusing to implement a request.
pub const REFUSAL_PHRASES: &[&str] = &[
    "cannot implement",
    "cannot complete",
    "unable to implement",
    "unable to complete",
    "i cannot",
    "i won't implement",
    "outside my",
    "not possible to implement",
    "can't implement",
    "i don't know how",
];

/// Count unique file paths referenced in a spec plan text (e.g., `src/main.rs`).
/// Used to enforce the "max 3 files" scope constraint at the Strategist gate.
pub fn count_spec_file_refs(text: &str) -> usize {
    let path_re = Regex::new(r"\bsrc/[\w./-]+\.rs\b").unwrap();
    let mut seen = std::collections::HashSet::new();
    for m in path_re.find_iter(text) {
        seen.insert(m.as_str());
    }
    seen.len()
}

/// Check if an LLM response contains a refusal phrase (case-insensitive).
pub fn is_refusal(response: &str) -> bool {
    let lower = response.to_lowercase();
    REFUSAL_PHRASES.iter().any(|p| lower.contains(p))
}

/// Extract the title from the first `# Title` or `## Title` line.
pub fn extract_title(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return stripped.to_string();
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            return stripped.to_string();
        }
    }
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

/// Slugify a title string for use in directory names (lowercase, 40 char max).
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .chars()
        .take(40)
        .collect()
}

/// Extract a section from a markdown plan by `## Header` keyword matching.
///
/// Looks for a line starting with `##` that contains one of the given headers
/// (case-insensitive), then captures everything until the next `##` section.
/// If no match is found, returns `fallback`.
pub fn extract_section(plan: &str, headers: &[&str], fallback: &str) -> String {
    let pattern = headers
        .iter()
        .map(|h| regex::escape(h))
        .collect::<Vec<_>>()
        .join("|");
    let re = match Regex::new(&format!(r"(?im)^##\s+.*({pattern}).*$")) {
        Ok(r) => r,
        Err(_) => return fallback.to_string(),
    };

    if let Some(m) = re.find(plan) {
        let start = m.start();
        let rest = &plan[m.end()..];
        let end = if let Some(next) = rest.find("\n## ") {
            m.end() + next
        } else {
            plan.len()
        };
        let result = plan[start..end].to_string();
        if result.trim().is_empty() {
            warn!(
                "extract_section: empty section for headers {:?} — using fallback",
                headers
            );
            fallback.to_string()
        } else {
            result
        }
    } else {
        warn!(
            "extract_section: no match for headers {:?} in plan — using fallback",
            headers
        );
        fallback.to_string()
    }
}

#[cfg(test)]
mod confidence_tests {
    use super::*;

    #[test]
    fn extract_confidence_standard() {
        let response = "Some analysis\nConfidence: High\nMore text";
        assert_eq!(extract_confidence(response), Some(Confidence::High));
    }

    #[test]
    fn extract_confidence_low() {
        let response = "Confidence: Low";
        assert_eq!(extract_confidence(response), Some(Confidence::Low));
    }

    #[test]
    fn extract_confidence_empty_response() {
        assert_eq!(extract_confidence(""), None);
    }

    #[test]
    fn extract_confidence_no_match() {
        let response = "Some text without confidence";
        assert_eq!(extract_confidence(response), None);
    }

    #[test]
    fn extract_confidence_markdown_bold() {
        let response = "**Confidence: Medium**";
        assert_eq!(extract_confidence(response), Some(Confidence::Medium));
    }

    #[test]
    fn extract_confidence_trailing_punctuation() {
        let response = "Confidence: High.";
        assert_eq!(extract_confidence(response), Some(Confidence::High));
    }

    #[test]
    fn extract_confidence_case_insensitive() {
        let response = "confidence: high";
        assert_eq!(extract_confidence(response), Some(Confidence::High));
    }

    #[test]
    fn extract_confidence_multiple_lines() {
        let response = "Some reasoning\nConfidence: Medium\nFinal verdict: PASS";
        assert_eq!(extract_confidence(response), Some(Confidence::Medium));
    }

    #[test]
    fn extract_confidence_invalid_value() {
        let response = "Confidence: Very High";
        assert_eq!(extract_confidence(response), None);
    }

    #[test]
    fn extract_confidence_first_match_wins() {
        let response = "Confidence: Low\nBut then later\nConfidence: High";
        assert_eq!(extract_confidence(response), Some(Confidence::Low));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_fake_worker(dir: &tempfile::TempDir, body: &str) -> Result<PathBuf> {
        #[cfg(windows)]
        {
            // Write a CMD script that echoes the JSON.
            // JSON {} characters are safe in cmd.exe echo; we avoid % and | which are special.
            let path = dir.path().join("fake-worker.cmd");
            std::fs::write(&path, format!("@echo off\r\necho {}\r\n", body))?;
            Ok(path)
        }

        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;

            let path = dir.path().join("fake-worker.sh");
            std::fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", body))?;
            let mut permissions = std::fs::metadata(&path)?.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions)?;
            Ok(path)
        }
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "CMD echo escaping differs on Windows; tested on Linux/Mac"
    )]
    fn external_worker_stdout_json_becomes_context() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let worker = write_fake_worker(
            &dir,
            r#"{"status":"ok","description":"fake observer output","spec_path":"docs/specs/_active/0001-fake"}"#,
        )?;

        let ctx = run_external_agent(
            worker.to_string_lossy().as_ref(),
            "observer",
            "find issues",
            true,
        )?;

        assert_eq!(ctx.description, "fake observer output");
        assert!(ctx.dry_run);
        assert!(ctx
            .spec_path
            .as_ref()
            .is_some_and(|p| p.ends_with(Path::new("docs/specs/_active/0001-fake"))));

        Ok(())
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "CMD echo escaping differs on Windows; tested on Linux/Mac"
    )]
    fn external_worker_rejects_plain_stdout() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let worker = write_fake_worker(&dir, "plain text")?;

        let err = run_external_agent(
            worker.to_string_lossy().as_ref(),
            "observer",
            "find issues",
            true,
        )
        .unwrap_err();

        assert!(err.to_string().contains("WorkerOutput JSON"));
        Ok(())
    }

    #[test]
    fn should_run_without_resume_always_true() {
        let resume: Option<Stage> = None;
        assert!(should_run(&resume, Stage::Observer));
        assert!(should_run(&resume, Stage::Strategist));
        assert!(should_run(&resume, Stage::Coder));
        assert!(should_run(&resume, Stage::Auditor));
    }

    #[test]
    fn should_run_with_resume_skips_earlier_stages() {
        let resume = Some(Stage::Coder);
        assert!(!should_run(&resume, Stage::Observer));
        assert!(!should_run(&resume, Stage::Strategist));
        assert!(should_run(&resume, Stage::Coder));
        assert!(should_run(&resume, Stage::Auditor));
    }

    #[test]
    fn stage_from_name_parses_correctly() {
        assert!(Stage::from_name("observer").is_some_and(|s| matches!(s, Stage::Observer)));
        assert!(Stage::from_name("OBSERVER").is_some_and(|s| matches!(s, Stage::Observer)));
        assert!(Stage::from_name("coder").is_some_and(|s| matches!(s, Stage::Coder)));
        assert!(Stage::from_name("invalid").is_none());
    }

    #[test]
    fn worker_output_error_statuses_fail() {
        for status in &["error", "fail", "failed", "reject", "rejected"] {
            let wo = WorkerOutput {
                status: Some(status.to_string()),
                description: None,
                summary: None,
                spec_path: None,
            };
            let result = wo.into_ctx(Path::new("/tmp"), true);
            assert!(result.is_err(), "status '{}' should fail", status);
        }
    }

    #[test]
    fn worker_output_ok_status_succeeds() {
        let wo = WorkerOutput {
            status: Some("ok".to_string()),
            description: Some("all good".to_string()),
            summary: None,
            spec_path: None,
        };
        let ctx = wo.into_ctx(Path::new("/tmp"), true).unwrap();
        assert_eq!(ctx.description, "all good");
    }

    #[test]
    fn worker_output_falls_back_to_summary() {
        let wo = WorkerOutput {
            status: Some("ok".to_string()),
            description: None,
            summary: Some("summary text".to_string()),
            spec_path: None,
        };
        let ctx = wo.into_ctx(Path::new("/tmp"), true).unwrap();
        assert_eq!(ctx.description, "summary text");
    }

    #[test]
    fn from_bacon_toml_parses_real_config() {
        let cfg = PipelineConfig::from_bacon_toml();
        // Should not default — our .bacon/bacon.toml exists
        assert_eq!(cfg.observer, "nvidia");
        assert_eq!(cfg.strategist, "nvidia");
        assert_eq!(cfg.coder, "nvidia");
        assert_eq!(cfg.auditor, "nvidia");
    }

    #[test]
    fn agent_llm_config_reads_nvidia_settings() {
        let nvidia = PipelineConfig::agent_llm_config("nvidia");
        assert_eq!(nvidia.provider.as_deref(), Some("nvidia"));
        assert_eq!(
            nvidia.base_url.as_deref(),
            Some("https://integrate.api.nvidia.com/v1")
        );
        assert!(nvidia.model.as_deref().unwrap_or("").contains("llama"));
        assert!(nvidia.temperature.is_some());
    }

    #[test]
    fn agent_llm_config_returns_empty_for_unknown_agent() {
        let unknown = PipelineConfig::agent_llm_config("does_not_exist");
        assert!(unknown.provider.is_none());
        assert!(unknown.model.is_none());
    }

    #[test]
    fn agent_llm_config_reads_local_agent_model() {
        let coder = PipelineConfig::agent_llm_config("coder");
        assert_eq!(coder.provider.as_deref(), Some("ollama"));
        assert!(coder.model.as_deref().unwrap_or("").contains("llama"));
    }

    #[test]
    fn collect_source_context_extracts_file_paths() {
        let text = r#"
        Plan: Implement feature in src/main.rs and update docs/README.md
        Also check config/llm.toml for settings
        "#;
        let context = collect_source_context(text, 10, 50);
        // Should extract and include file contents
        assert!(context.contains("## Relevant Source Files"));
        // Note: actual file existence depends on test environment
    }

    #[test]
    fn collect_source_context_limits_files_and_lines() {
        let text = "src/main.rs src/lib.rs src/bin/agent.rs"; // Multiple files
        let context = collect_source_context(text, 2, 10); // Limit to 2 files, 10 lines
                                                           // Should respect limits
        let file_sections: Vec<&str> = context.split("### `").collect();
        assert!(file_sections.len() <= 3); // Header + up to 2 files
    }

    #[test]
    fn collect_source_context_handles_nonexistent_files() {
        let text = "src/nonexistent.rs docs/missing.md";
        let context = collect_source_context(text, 10, 50);
        // Should not crash, just skip nonexistent files
        assert!(context.is_empty() || context.contains("## Relevant Source Files"));
    }

    #[test]
    fn read_role_prompt_returns_content_for_known_roles() {
        let observer = read_role_prompt("observer");
        assert!(!observer.is_empty());
        let strategist = read_role_prompt("strategist");
        assert!(!strategist.is_empty());
        let coder = read_role_prompt("coder");
        assert!(!coder.is_empty());
        let auditor = read_role_prompt("auditor");
        assert!(!auditor.is_empty());
    }

    #[test]
    fn read_role_prompt_returns_empty_for_unknown_role() {
        let unknown = read_role_prompt("unknown");
        assert!(unknown.is_empty());
    }

    #[test]
    fn resolve_agent_binary_falls_back_to_name() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = resolve_agent_binary("test-agent", &root);
        // Should fall back to just the name if not built
        assert_eq!(path, PathBuf::from("test-agent"));
    }

    #[test]
    fn scan_project_structure_returns_summary() {
        let summary = scan_project_structure();
        assert!(summary.contains("Source modules"));
        // May contain "Active specs" depending on test environment
    }
}
