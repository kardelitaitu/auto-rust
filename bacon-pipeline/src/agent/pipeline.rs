use anyhow::Result;
use log::{info, warn};

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

    pub async fn run(&self) -> Result<()> {
        if self.args.parallel && self.auto {
            self.run_parallel().await
        } else {
            PipelineAgent::run(self).await
        }
    }

    /// Parallel mode: run Observer+Strategist once, then process all approved
    /// specs concurrently through Coder+Auditor.
    async fn run_parallel(&self) -> Result<()> {
        let base_ctx = PipelineCtx::new(String::new(), None, None, None).with_dry_run(self.dry_run);

        // Run Observer
        info!("=== Parallel mode: running Observer ===");
        let ctx = self.run_observer(&base_ctx).await?;

        // Run Strategist to produce the initial spec
        info!("=== Parallel mode: running Strategist ===");
        let ctx = self.run_strategist(&ctx).await?;

        if ctx.spec_path.is_none() {
            info!("No spec produced; nothing to parallelize");
            return Ok(());
        }

        // Collect all approved specs (including the one just created)
        let specs = crate::core::spec_io::list_approved_specs()?;
        if specs.is_empty() {
            info!("No approved specs found for parallel execution");
            return Ok(());
        }

        info!(
            "Parallel execution: processing {} approved specs",
            specs.len()
        );

        let handles: Vec<_> = specs
            .into_iter()
            .map(|(spec_path, meta)| {
                let pid = meta.id.clone();
                tokio::spawn(async move {
                    let result = run_single_spec(spec_path, &meta).await;
                    (pid, result)
                })
            })
            .collect();

        let mut passed = 0;
        let mut failed = 0;
        for handle in handles {
            match handle.await {
                Ok((_pid, Ok(()))) => {
                    passed += 1;
                }
                Ok((_pid, Err(e))) => {
                    failed += 1;
                    warn!("Spec failed: {e:#}");
                }
                Err(e) => {
                    failed += 1;
                    warn!("Spec task panicked: {e}");
                }
            }
        }

        info!("Parallel execution complete: {passed} passed, {failed} failed/panicked");

        if failed > 0 {
            anyhow::bail!("{failed} spec(s) failed during parallel execution");
        }
        Ok(())
    }
}

/// Run Coder + Auditor for a single approved spec, using direct module calls.
async fn run_single_spec(
    spec_path: std::path::PathBuf,
    meta: &crate::core::spec_io::SpecMeta,
) -> Result<()> {
    let pipeline_cfg = PipelineConfig::from_bacon_toml();
    let dry_run = std::env::var("BACON_DRY_RUN").is_ok_and(|v| v == "true");
    let args = RunArgs {
        prompt: Some(format!("Implement spec: {}", meta.title)),
        spec: None,
        stage: None,
        fast: false,
        dry_run,
        auto: true,
        auto_apply: false,
        parallel: false,
        max_attempts: None,
        ci: false,
    };

    // Create a PipelineCtx for this spec
    let ctx = PipelineCtx::new(format!("Implement spec: {}", meta.title), None, None, None)
        .with_dry_run(dry_run);
    let mut ctx = ctx;
    ctx.spec_path = Some(spec_path);

    // Coder
    let agent = pipeline_cfg.agent_for(&Stage::Coder);
    let llm = {
        let config = crate::llm::llm_config_for_agent(agent);
        Llm::from_config(config)
    };
    ctx = crate::agent::coder::run(&llm, &args, &ctx).await?;

    if ctx.coder_refused || ctx.needs_human_approval {
        anyhow::bail!("Coder failed for spec '{}'", meta.title);
    }

    // Auditor
    let agent = pipeline_cfg.agent_for(&Stage::Auditor);
    let llm = {
        let config = crate::llm::llm_config_for_agent(agent);
        Llm::from_config(config)
    };
    crate::agent::auditor::run(&llm, &args, &ctx).await?;

    // Move spec to _done/
    use crate::core::spec_io::move_to_done;
    if let Some(ref path) = ctx.spec_path {
        if let Err(e) = move_to_done(path) {
            warn!("Failed to move spec to _done: {e}");
        }
    }

    info!("Spec '{}' completed successfully", meta.title);
    Ok(())
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

    fn parallel(&self) -> bool {
        self.args.parallel
    }

    fn ci(&self) -> bool {
        self.args.ci
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cli_types::RunArgs;
    use std::sync::Once;

    fn setup() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let root =
                std::env::temp_dir().join(format!("bacon-pipeline-tests-{}", std::process::id()));
            let _ = std::fs::create_dir_all(root.join(".bacon/sessions"));
            crate::config::init(crate::config::ProjectConfig::with_defaults(root));
        });
    }

    /// Base RunArgs for tests: auto mode, no prompt override.
    fn base_args() -> RunArgs {
        RunArgs {
            prompt: None,
            spec: None,
            stage: None,
            fast: false,
            dry_run: false,
            auto: true,
            auto_apply: false,
            parallel: false,
            max_attempts: None,
            ci: false,
        }
    }

    // ------------------------------------------------------------------
    // Pipeline::new() — validation
    // ------------------------------------------------------------------

    #[test]
    fn pipeline_new_succeeds_with_clean_env() {
        setup();
        std::env::remove_var("LLM_PROVIDER");
        let result = Pipeline::new(base_args(), false, true);
        assert!(
            result.is_ok(),
            "expected Ok, got error: {:?}",
            result.as_ref().err()
        );
    }

    #[test]
    fn pipeline_new_fails_with_openrouter_env() {
        setup();
        std::env::set_var("LLM_PROVIDER", "openrouter");
        let result = Pipeline::new(base_args(), false, true);
        std::env::remove_var("LLM_PROVIDER");
        let err = match &result {
            Err(e) => format!("{e}"),
            Ok(_) => panic!("expected Err with LLM_PROVIDER=openrouter"),
        };
        assert!(
            err.contains("openrouter") || err.contains("OpenRouter"),
            "error should mention OpenRouter: {err}"
        );
    }

    // ------------------------------------------------------------------
    // Pipeline construction — delegates and config wiring
    // ------------------------------------------------------------------
    // Orchestration behavior (stage sequencing, resume, fast-path,
    // confidence, coder outcomes) is tested via MockAgent in
    // core::agent::tests.  Pipeline::run() integration with real LLM
    // is covered by tests/bacon_pipeline_integration.rs.
    // ------------------------------------------------------------------
}
