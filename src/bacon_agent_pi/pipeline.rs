use anyhow::Result;
use log::info;
use serde::Deserialize;

use crate::llm::Llm;

use super::cli::RunArgs;
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

        // Observer
        let mut ctx = if should_run(&resume_stage, Stage::Observer) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Observer);
            info!("=== Stage 1: Observer (agent: {}) ===", agent);
            super::observer::run(&self.llm, &self.args).await?
        } else {
            PipelineCtx::new(String::new())
        };

        // Strategist
        if should_run(&resume_stage, Stage::Strategist) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Strategist);
            info!("=== Stage 2: Strategist (agent: {}) ===", agent);
            ctx = super::strategist::run(&self.llm, &self.args, &ctx).await?;
        }

        // Coder
        if should_run(&resume_stage, Stage::Coder) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Coder);
            info!("=== Stage 3: Coder (agent: {}) ===", agent);
            ctx = super::coder::run(&self.llm, &self.args, &ctx).await?;
        }

        // Auditor
        if should_run(&resume_stage, Stage::Auditor) {
            let agent = self.pipeline_cfg.agent_for(&Stage::Auditor);
            info!("=== Stage 4: Auditor (agent: {}) ===", agent);
            super::auditor::run(&self.llm, &self.args, &ctx).await?;
        }

        info!("Pipeline complete");
        Ok(())
    }
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
