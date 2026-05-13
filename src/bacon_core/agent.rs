//! PipelineAgent trait — canonical pipeline orchestration for all bacon agents.
//!
//! Defines a shared [`PipelineAgent`] trait with stage methods that each agent
//! implements, plus a default [`run()`] that drives the full pipeline with
//! resume, scope-reduction loop, confirmation gates, and crash recovery.
//!
//! This eliminates the duplicated `Pipeline::run()` between `bacon_agent_pi`
//! and `bacon_agent_nvidia`.

use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};

use super::{
    check_stale_in_progress, confirm, log_agent_config, should_run, PipelineConfig, PipelineCtx,
    Stage,
};

/// A pipeline agent that can run all stages of the bacon pipeline.
///
/// Implementors provide stage execution methods that delegate to their
/// local module implementations. The default [`run()`] method handles
/// orchestration, resume, scope-reduction fallback, and user gates.
#[async_trait]
pub trait PipelineAgent: Send + Sync {
    /// Human-readable agent name for logging.
    fn name(&self) -> &str;

    /// Whether the pipeline is in sandbox (no-write) mode.
    fn dry_run(&self) -> bool;

    /// Whether to skip interactive confirmation gates.
    fn auto(&self) -> bool;

    /// Whether fast-path is enabled (skip strategist + auditor).
    fn fast(&self) -> bool;

    /// Optional stage to resume from.
    fn resume_stage(&self) -> Option<Stage>;

    /// Reference to the agent routing configuration.
    fn pipeline_cfg(&self) -> &PipelineConfig;

    // ------------------------------------------------------------------
    // Stage execution methods — each agent delegates to its own modules.
    // ------------------------------------------------------------------

    /// Run the Observer stage. Returns the updated pipeline context.
    async fn run_observer(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Strategist stage. Returns the updated pipeline context
    /// with an optional spec_path pointing to the generated spec package.
    async fn run_strategist(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Coder stage. Returns the updated pipeline context with
    /// optional scope-reduction signals.
    async fn run_coder(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Auditor stage. Returns the updated pipeline context.
    async fn run_auditor(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run scope reduction (default: delegate to strategist).
    ///
    /// Called when the Coder signals that scope reduction is needed.
    /// Some agents may override this to post-process external strategist
    /// output into a spec package.
    async fn run_reduce_scope(&self, ctx: &PipelineCtx) -> Result<PipelineCtx> {
        info!("=== Scope reduction: calling Strategist ===");
        self.run_strategist(ctx).await
    }

    // ------------------------------------------------------------------
    // Default pipeline orchestration
    // ------------------------------------------------------------------

    /// Run the full pipeline with all stages, resume support, scope-reduction
    /// fallback loop, confirmation gates, and crash recovery.
    async fn run(&self) -> Result<()> {
        let resume_stage = self.resume_stage();

        if let Some(stage) = &resume_stage {
            info!("Resuming from stage: {:?}", stage);
        }

        if self.dry_run() {
            info!("DRY RUN — no files will be modified");
        }

        // Crash recovery: check for stale in-progress specs
        check_stale_in_progress()?;

        // Fast path: skip strategist + auditor if --fast
        let fast_path = self.fast();

        let base_ctx = PipelineCtx::new(String::new()).with_dry_run(self.dry_run());

        // Observer
        let mut ctx = if should_run(&resume_stage, Stage::Observer) {
            let agent = self.pipeline_cfg().agent_for(&Stage::Observer);
            info!("=== Stage 1: Observer (agent: {}) ===", agent);
            log_agent_config(agent);
            self.run_observer(&base_ctx).await?
        } else {
            base_ctx
        };
        ctx.dry_run = self.dry_run();

        // Strategist (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Strategist) {
            let agent = self.pipeline_cfg().agent_for(&Stage::Strategist);
            info!("=== Stage 2: Strategist (agent: {}) ===", agent);
            log_agent_config(agent);
            ctx = self.run_strategist(&ctx).await?;

            // User confirmation gate
            if !self.auto() && !confirm("Implement this plan? [Y/n]: ", true)? {
                info!("User declined — aborting pipeline");
                return Ok(());
            }
        }

        // Coder with Coder→Strategist scope reduction fallback loop
        const MAX_SCOPE_REDUCTIONS: u32 = 3;
        if should_run(&resume_stage, Stage::Coder) {
            let mut coder_attempts = 0u32;
            loop {
                let agent = self.pipeline_cfg().agent_for(&Stage::Coder);
                // Pass the current scope_reduction_count into the coder context
                ctx.scope_reduction_count = coder_attempts;
                info!(
                    "=== Stage 3: Coder (agent: {}), pass {}-A ===",
                    agent,
                    coder_attempts + 1
                );
                log_agent_config(agent);

                ctx = self.run_coder(&ctx).await?;

                // Check if Coder signalled scope reduction needed
                if ctx.scope_reduction_needed {
                    coder_attempts += 1;
                    if coder_attempts >= MAX_SCOPE_REDUCTIONS {
                        warn!(
                            "Max scope reductions ({}) exhausted — aborting pipeline",
                            MAX_SCOPE_REDUCTIONS
                        );
                        // Hard-fail: mark needs-human-approval with accumulated errors
                        if let Some(ref spec_path) = ctx.spec_path {
                            let error_report = ctx.coder_errors.join("\n---\n");
                            let report = format!(
                                "Scope reduction exhausted after {} attempts.\n\nAccumulated errors:\n{}",
                                MAX_SCOPE_REDUCTIONS,
                                error_report
                            );
                            let validation_path = spec_path.join("validation.md");
                            let _ = std::fs::write(
                                &validation_path,
                                format!("# Coder Failure Report\n\n{}", report),
                            );
                            // Update spec.yaml status
                            let meta_path = spec_path.join("spec.yaml");
                            if let Ok(content) = std::fs::read_to_string(&meta_path) {
                                let updated = content.replace("approved", "needs-human-approval");
                                let _ = std::fs::write(&meta_path, updated);
                            }
                        }
                        break;
                    }

                    // Reset reduction flag for next iteration
                    ctx.scope_reduction_needed = false;

                    // Call Strategist for scope reduction
                    let strat_agent = self.pipeline_cfg().agent_for(&Stage::Strategist);
                    info!(
                        "=== Scope reduction pass {}/{}: calling Strategist (agent: {}) ===",
                        coder_attempts, MAX_SCOPE_REDUCTIONS, strat_agent
                    );
                    log_agent_config(strat_agent);
                    ctx = self.run_reduce_scope(&ctx).await?;

                    // Continue the loop to retry Coder with reduced scope
                    continue;
                }

                // No scope reduction needed — Coder succeeded
                break;
            }

            // User confirmation gate (only if Coder succeeded)
            if !self.auto()
                && !ctx.scope_reduction_needed
                && !confirm("Apply this diff? [y/N]: ", false)?
            {
                info!("User declined diff — aborting pipeline");
                return Ok(());
            }
        }

        // Auditor (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Auditor) {
            let agent = self.pipeline_cfg().agent_for(&Stage::Auditor);
            info!("=== Stage 4: Auditor (agent: {}) ===", agent);
            log_agent_config(agent);
            self.run_auditor(&ctx).await?;
        }

        info!("Pipeline complete");
        Ok(())
    }
}
