use anyhow::Result;

use crate::bacon_core::{
    run_external_agent, validate_bacon_local_only, PipelineAgent, PipelineConfig, PipelineCtx,
    Stage,
};
use crate::llm::Llm;

use super::cli::RunArgs;

pub struct Pipeline {
    pub args: RunArgs,
    pub dry_run: bool,
    pub auto: bool,
    pub llm: Llm,
    pub pipeline_cfg: PipelineConfig,
}

impl Pipeline {
    pub fn new(args: RunArgs, dry_run: bool, auto: bool) -> Result<Self> {
        validate_bacon_local_only()?;
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

    /// Thin wrapper that delegates to the PipelineAgent trait default.
    pub async fn run(&self) -> Result<()> {
        PipelineAgent::run(self).await
    }
}

// ---------------------------------------------------------------------------
// PipelineAgent trait implementation — stage methods
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl PipelineAgent for Pipeline {
    fn name(&self) -> &str {
        "nvidia"
    }

    fn dry_run(&self) -> bool {
        self.dry_run
    }

    fn auto(&self) -> bool {
        self.auto
    }

    fn fast(&self) -> bool {
        self.args.fast
    }

    fn resume_stage(&self) -> Option<Stage> {
        self.args.stage.as_deref().and_then(Stage::from_name)
    }

    fn pipeline_cfg(&self) -> &PipelineConfig {
        &self.pipeline_cfg
    }

    async fn run_observer(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        let agent = self.pipeline_cfg.agent_for(&Stage::Observer);
        if agent == "bacon" {
            super::observer::run(&self.llm, &self.args, ctx).await
        } else {
            let prompt = self
                .args
                .prompt
                .as_deref()
                .unwrap_or("scan for improvements");
            run_external_agent(agent, "observer", prompt, self.dry_run)
        }
    }

    async fn run_strategist(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        let agent = self.pipeline_cfg.agent_for(&Stage::Strategist);
        if agent == "bacon" {
            super::strategist::run(&self.llm, &self.args, ctx).await
        } else {
            let mut next = run_external_agent(agent, "strategist", &ctx.description, self.dry_run)?;
            if next.spec_path.is_none() {
                next.spec_path = ctx.spec_path.clone();
            }
            Ok(next)
        }
    }

    async fn run_coder(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        let agent = self.pipeline_cfg.agent_for(&Stage::Coder);
        if agent == "bacon" {
            super::coder::run(&self.llm, &self.args, ctx).await
        } else {
            let mut next = run_external_agent(agent, "coder", &ctx.description, self.dry_run)?;
            if next.spec_path.is_none() {
                next.spec_path = ctx.spec_path.clone();
            }
            Ok(next)
        }
    }

    async fn run_auditor(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        let agent = self.pipeline_cfg.agent_for(&Stage::Auditor);
        if agent == "bacon" {
            super::auditor::run(&self.llm, &self.args, ctx).await
        } else {
            let ctx = run_external_agent(agent, "auditor", &ctx.description, self.dry_run)?;
            Ok(ctx)
        }
    }
}
