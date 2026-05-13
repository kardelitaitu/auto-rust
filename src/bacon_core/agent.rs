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
    check_stale_in_progress, confirm, log_agent_config, should_run, Confidence, PipelineConfig,
    PipelineCtx, Stage,
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

    /// Check confidence after a stage completes. If Low, warn and prompt in non-auto mode.
    fn check_confidence(stage: &str, ctx: &PipelineCtx, auto: bool) -> Result<bool> {
        if let Some(Confidence::Low) = ctx.confidence {
            warn!(
                "Low confidence from {} stage — response may be unreliable",
                stage
            );
            if !auto {
                println!(
                    "\n⚠ Low confidence from {}. Review the output above.",
                    stage
                );
                if !confirm("Continue pipeline? [y/N]: ", false)? {
                    info!("User aborted due to low confidence from {}", stage);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

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
        if !Self::check_confidence("Observer", &ctx, self.auto())? {
            return Ok(());
        }

        // Strategist (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Strategist) {
            let agent = self.pipeline_cfg().agent_for(&Stage::Strategist);
            info!("=== Stage 2: Strategist (agent: {}) ===", agent);
            log_agent_config(agent);
            ctx = self.run_strategist(&ctx).await?;

            if !Self::check_confidence("Strategist", &ctx, self.auto())? {
                return Ok(());
            }

            // User confirmation gate
            if !self.auto() && !confirm("Implement this plan? [Y/n]: ", true)? {
                info!("User declined — aborting pipeline");
                return Ok(());
            }
        }

        // Coder — single pass with internal retry loop (MAX_ATTEMPTS = 4)
        // No outer scope-reduction loop: if the Coder fails after all retries,
        // the spec goes directly to needs-human-approval instead of calling
        // the Strategist again. This caps worst-case LLM calls at 6
        // (1 Observer + 1 Strategist + 4 Coder) instead of 21.
        if should_run(&resume_stage, Stage::Coder) {
            let agent = self.pipeline_cfg().agent_for(&Stage::Coder);
            info!("=== Stage 3: Coder (agent: {}) ===", agent);
            log_agent_config(agent);

            ctx = self.run_coder(&ctx).await?;

            // Check if Coder aborted due to 2 consecutive refusals
            if ctx.coder_refused {
                warn!(
                    "Coder refused to implement after consecutive refusals — \
                     pipeline aborted, spec marked needs-human-approval"
                );
                return Ok(());
            }

            // Auto-apply failed — skip Auditor, abort pipeline
            if ctx.needs_human_approval {
                warn!(
                    "Auto-apply gate rejected the patch — \
                     pipeline aborted, spec waiting for human approval"
                );
                return Ok(());
            }

            // Scope reduction needed: inner Coder retries exhausted.
            // Write failure report and mark needs-human-approval directly —
            // do NOT call Strategist for a reduced-scope plan.
            if ctx.scope_reduction_needed {
                warn!(
                    "Coder retries exhausted — marking needs-human-approval with accumulated errors"
                );
                if let Some(ref spec_path) = ctx.spec_path {
                    let error_report = ctx.coder_errors.join("\n---\n");
                    let report = format!(
                        "Coder retries exhausted.\n\nAccumulated errors:\n{}",
                        error_report
                    );
                    let validation_path = spec_path.join("validation.md");
                    if let Err(e) = std::fs::write(
                        &validation_path,
                        format!("# Coder Failure Report\n\n{}", report),
                    ) {
                        warn!("Failed to write failure report: {}", e);
                    }
                    let meta_path = spec_path.join("spec.yaml");
                    match std::fs::read_to_string(&meta_path) {
                        Ok(content) => {
                            let updated = content
                                .replace("in-progress", "needs-human-approval")
                                .replace("approved", "needs-human-approval");
                            if let Err(e) = std::fs::write(&meta_path, updated) {
                                warn!("Failed to update spec status: {}", e);
                            }
                        }
                        Err(e) => warn!("Failed to read spec.yaml for status update: {}", e),
                    }
                }
                return Ok(());
            }

            if !Self::check_confidence("Coder", &ctx, self.auto())? {
                return Ok(());
            }

            // User confirmation gate (only if Coder succeeded)
            if !self.auto() && !ctx.scope_reduction_needed {
                // Print the diff so the user can see it before approving
                if let Some(ref patch_path) = ctx.patch_path {
                    info!("Approved patch saved to: {}", patch_path.display());
                    if let Ok(diff) = std::fs::read_to_string(patch_path) {
                        println!("=== Proposed Changes ===");
                        // Print first 80 lines of diff
                        for line in diff.lines().take(80) {
                            println!("{}", line);
                        }
                        if diff.lines().count() > 80 {
                            println!("... (truncated at 80 lines)");
                        }
                        println!("========================\n");
                    }
                }
                if !confirm("Apply this diff? [y/N]: ", false)? {
                    info!("User declined diff — aborting pipeline");
                    return Ok(());
                }
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
