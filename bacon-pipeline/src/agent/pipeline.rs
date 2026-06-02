use anyhow::Result;

use crate::core::{
    run_external_agent, validate_bacon_local_only, validate_pipeline_config, PipelineAgent,
    PipelineConfig, PipelineCtx, Stage,
};
use crate::llm::Llm;

use crate::core::cli_types::RunArgs;

pub struct Pipeline {
    pub args: RunArgs,
    pub dry_run: bool,
    pub auto: bool,
    pub pipeline_cfg: PipelineConfig,
}

impl Pipeline {
    pub fn new(args: RunArgs, dry_run: bool, auto: bool) -> Result<Self> {
        validate_bacon_local_only()?;
        let pipeline_cfg = PipelineConfig::from_bacon_toml();
        validate_pipeline_config(&pipeline_cfg);
        Ok(Self {
            args,
            dry_run,
            auto,
            pipeline_cfg,
        })
    }

    fn llm_for_agent(agent_name: &str) -> Result<Llm> {
        let config = crate::llm::llm_config_for_agent(agent_name);
        Ok(Llm::from_config(config))
    }

    /// Thin wrapper that delegates to the `PipelineAgent` trait default.
    pub async fn run(&self) -> Result<()> {
        PipelineAgent::run(self).await
    }
}

// ---------------------------------------------------------------------------
// PipelineAgent trait implementation — stage methods
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl PipelineAgent for Pipeline {
    fn name(&self) -> &'static str {
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
        let agent_cfg = PipelineConfig::agent_llm_config(agent);

        if agent == "bacon"
            || agent == "nvidia"
            || agent_cfg.provider.as_deref() == Some("nvidia")
            || agent_cfg.provider.as_deref() == Some("ollama")
        {
            let llm = Self::llm_for_agent(agent)?;
            super::observer::run(&llm, &self.args, ctx).await
        } else {
            let prompt = self
                .args
                .prompt
                .as_deref()
                .unwrap_or("scan for improvements");
            let context = crate::core::gather_project_context();
            let enriched_prompt = format!("{context}\n\n## Task\n\n{prompt}");
            run_external_agent(agent, "observer", &enriched_prompt, self.dry_run)
        }
    }

    async fn run_strategist(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        let agent = self.pipeline_cfg.agent_for(&Stage::Strategist);
        let agent_cfg = PipelineConfig::agent_llm_config(agent);

        if agent == "bacon"
            || agent == "nvidia"
            || agent_cfg.provider.as_deref() == Some("nvidia")
            || agent_cfg.provider.as_deref() == Some("ollama")
        {
            let llm = Self::llm_for_agent(agent)?;
            super::strategist::run(&llm, &self.args, ctx).await
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
        let agent_cfg = PipelineConfig::agent_llm_config(agent);

        if agent == "bacon"
            || agent == "nvidia"
            || agent_cfg.provider.as_deref() == Some("nvidia")
            || agent_cfg.provider.as_deref() == Some("ollama")
        {
            let llm = Self::llm_for_agent(agent)?;
            super::coder::run(&llm, &self.args, ctx).await
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
        let agent_cfg = PipelineConfig::agent_llm_config(agent);

        if agent == "bacon"
            || agent == "nvidia"
            || agent_cfg.provider.as_deref() == Some("nvidia")
            || agent_cfg.provider.as_deref() == Some("ollama")
        {
            let llm = Self::llm_for_agent(agent)?;
            super::auditor::run(&llm, &self.args, ctx).await
        } else {
            let ctx = run_external_agent(agent, "auditor", &ctx.description, self.dry_run)?;
            Ok(ctx)
        }
    }
}
