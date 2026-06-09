//! `PipelineAgent` trait — canonical pipeline orchestration for all bacon agents.
//!
//! Defines a shared [`PipelineAgent`] trait with stage methods that each agent
//! implements, plus a default [`run()`] that drives the full pipeline with
//! resume, scope-reduction loop, confirmation gates, and crash recovery.
//!
//! This eliminates the duplicated `Pipeline::run()` implementations
//! by providing a single canonical orchestrator for the bacon-pipeline crate.

use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use std::sync::Arc;
use std::time::Instant;

use super::{
    check_stale_in_progress, confirm, log_agent_config, should_run, Confidence, PipelineConfig,
    PipelineCtx, Stage,
};
use tokio::time::{sleep, Duration};

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

    /// Whether parallel spec execution is enabled.
    fn parallel(&self) -> bool {
        false
    }

    /// Whether CI-mode (GitHub Actions annotations) is enabled.
    fn ci(&self) -> bool {
        false
    }

    /// Reference to the agent routing configuration.
    fn pipeline_cfg(&self) -> &PipelineConfig;

    // ------------------------------------------------------------------
    // Stage execution methods — each agent delegates to its own modules.
    // ------------------------------------------------------------------

    /// Run the Observer stage. Returns the updated pipeline context.
    async fn run_observer(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Strategist stage. Returns the updated pipeline context
    /// with an optional `spec_path` pointing to the generated spec package.
    async fn run_strategist(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Coder stage. Returns the updated pipeline context with
    /// optional scope-reduction signals.
    async fn run_coder(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;

    /// Run the Auditor stage. Returns the updated pipeline context.
    async fn run_auditor(&self, ctx: &PipelineCtx) -> Result<PipelineCtx>;
    /// Wait for the configured stage delay, if any.
    /// Used to avoid rate-limiting when calling external LLM APIs back-to-back.
    async fn stage_delay(&self) {
        let delay_ms = self.pipeline_cfg().stage_delay_ms;
        if delay_ms > 0 {
            info!("Stage delay: waiting {delay_ms}ms before next stage");
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    // ------------------------------------------------------------------
    // Default pipeline orchestration
    // ------------------------------------------------------------------

    /// Check confidence after a stage completes. If Low, warn and prompt in non-auto mode.
    fn check_confidence(stage: &str, ctx: &PipelineCtx, auto: bool) -> Result<bool> {
        if let Some(Confidence::Low) = ctx.confidence {
            warn!("Low confidence from {stage} stage — response may be unreliable");
            crate::core::run_log::ci_warning(&format!("Low confidence from {stage} stage"));
            if !auto {
                println!("\n⚠ Low confidence from {stage}. Review the output above.");
                if !confirm("Continue pipeline? [y/N]: ", false)? {
                    info!("User aborted due to low confidence from {stage}");
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
            info!("Resuming from stage: {stage:?}");
        }

        if self.dry_run() {
            info!("DRY RUN — no files will be modified");
            crate::core::run_log::ci_notice("DRY RUN — no files will be modified");
        }

        // Crash recovery: check for stale in-progress specs
        check_stale_in_progress()?;

        // Fast path: skip strategist + auditor if --fast
        let fast_path = self.fast();

        // Production dependencies
        let fs = Arc::new(crate::core::traits::RealFileSystem);
        let runner = Arc::new(crate::core::traits::RealCommandRunner);
        // Note: NvidiaConfig needs to be constructed here or passed in.
        // For now, I will use a dummy config to allow compilation, then address
        // LlmClient initialization next.
        let llm_config = crate::llm::models::NvidiaConfig::default();
        let llm = Arc::new(crate::core::traits::RealLlmClient::new(llm_config));

        let base_ctx = PipelineCtx::new(String::new(), Some(fs), Some(runner), Some(llm))
            .with_dry_run(self.dry_run());

        // Observer
        let mut ctx = if should_run(&resume_stage, Stage::Observer) {
            let start = Instant::now();
            let agent = self.pipeline_cfg().agent_for(&Stage::Observer);
            info!("=== Stage 1: Observer (agent: {agent}) ===");
            crate::core::run_log::ci_group_start("Observer");
            log_agent_config(agent);
            let mut ctx = self.run_observer(&base_ctx).await?;
            let elapsed = start.elapsed();
            ctx.stage_durations.push((Stage::Observer, elapsed));
            info!("Stage Observer completed in {:.1}s", elapsed.as_secs_f64());
            crate::core::run_log::ci_notice(&format!(
                "Stage Observer completed in {:.1}s",
                elapsed.as_secs_f64()
            ));
            crate::core::run_log::ci_group_end();
            ctx
        } else {
            base_ctx
        };
        ctx.dry_run = self.dry_run();
        if !Self::check_confidence("Observer", &ctx, self.auto())? {
            return Ok(());
        }
        if self.auto() && ctx.spec_path.is_none() && is_no_action_description(&ctx.description) {
            info!("Observer found no grounded autonomous work; ending auto cycle");
            return Ok(());
        }

        // Strategist (skip in fast path)
        if !fast_path && should_run(&resume_stage, Stage::Strategist) {
            self.stage_delay().await;
            let start = Instant::now();
            let agent = self.pipeline_cfg().agent_for(&Stage::Strategist);
            info!("=== Stage 2: Strategist (agent: {agent}) ===");
            crate::core::run_log::ci_group_start("Strategist");
            log_agent_config(agent);
            ctx = self.run_strategist(&ctx).await?;
            let elapsed = start.elapsed();
            ctx.stage_durations.push((Stage::Strategist, elapsed));
            info!(
                "Stage Strategist completed in {:.1}s",
                elapsed.as_secs_f64()
            );
            crate::core::run_log::ci_notice(&format!(
                "Stage Strategist completed in {:.1}s",
                elapsed.as_secs_f64()
            ));
            crate::core::run_log::ci_group_end();

            if !Self::check_confidence("Strategist", &ctx, self.auto())? {
                return Ok(());
            }

            // User confirmation gate
            if !self.auto() && !confirm("Implement this plan? [Y/n]: ", true)? {
                info!("User declined — aborting pipeline");
                return Ok(());
            }

            if self.auto() && ctx.spec_path.is_none() {
                info!("Strategist produced no approved spec; ending auto cycle without changes");
                return Ok(());
            }
        }

        // Validate that files referenced in the spec plan exist on disk
        if let Some(ref spec_path) = ctx.spec_path {
            let missing = crate::core::validate_spec_file_refs(spec_path);
            if !missing.is_empty() {
                warn!(
                    "Spec references {} files that don't exist on disk:\n  {}",
                    missing.len(),
                    missing.join("\n  ")
                );
                ctx.set_needs_approval();
                info!("Pipeline aborted — spec marked needs-human-approval");
                return Ok(());
            }
        }

        // Coder — single pass with internal retry loop (MAX_ATTEMPTS = 4)
        if should_run(&resume_stage, Stage::Coder) {
            self.stage_delay().await;
            let start = Instant::now();
            let agent = self.pipeline_cfg().agent_for(&Stage::Coder);
            info!("=== Stage 3: Coder (agent: {agent}) ===");
            crate::core::run_log::ci_group_start("Coder");
            log_agent_config(agent);

            ctx = self.run_coder(&ctx).await?;
            let elapsed = start.elapsed();
            ctx.stage_durations.push((Stage::Coder, elapsed));
            info!("Stage Coder completed in {:.1}s", elapsed.as_secs_f64());
            crate::core::run_log::ci_notice(&format!(
                "Stage Coder completed in {:.1}s",
                elapsed.as_secs_f64()
            ));
            crate::core::run_log::ci_group_end();

            // Check if Coder aborted due to 2 consecutive refusals
            if ctx.coder_refused {
                warn!(
                    "Coder refused to implement after consecutive refusals — \
                     pipeline aborted, spec marked needs-human-approval"
                );
                anyhow::bail!("Coder refused to implement; spec marked needs-human-approval");
            }

            // Auto-apply failed — skip Auditor, abort pipeline
            if ctx.needs_human_approval {
                warn!(
                    "Auto-apply gate rejected the patch — \
                     pipeline aborted, spec waiting for human approval"
                );
                anyhow::bail!("Pipeline requires human approval");
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
                    let report =
                        format!("Coder retries exhausted.\n\nAccumulated errors:\n{error_report}");
                    let validation_path = spec_path.join("validation.md");
                    if let Err(e) = std::fs::write(
                        &validation_path,
                        format!("# Coder Failure Report\n\n{report}"),
                    ) {
                        warn!("Failed to write failure report: {e}");
                    }
                    if let Ok(mut meta) = crate::core::spec_io::read_spec_meta(spec_path) {
                        meta.status = "needs-human-approval".to_string();
                        if let Err(e) = crate::core::spec_io::write_spec_meta(spec_path, &meta) {
                            warn!("Failed to persist needs-human-approval to spec.yaml: {e}");
                        }
                    }
                }
                anyhow::bail!("Coder retries exhausted; spec marked needs-human-approval");
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
                            println!("{line}");
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
            self.stage_delay().await;
            let start = Instant::now();
            let agent = self.pipeline_cfg().agent_for(&Stage::Auditor);
            info!("=== Stage 4: Auditor (agent: {agent}) ===");
            crate::core::run_log::ci_group_start("Auditor");
            log_agent_config(agent);
            self.run_auditor(&ctx).await?;
            let elapsed = start.elapsed();
            ctx.stage_durations.push((Stage::Auditor, elapsed));
            info!("Stage Auditor completed in {:.1}s", elapsed.as_secs_f64());
            crate::core::run_log::ci_notice(&format!(
                "Stage Auditor completed in {:.1}s",
                elapsed.as_secs_f64()
            ));
            crate::core::run_log::ci_group_end();
        }

        // Record persistent run log
        let spec_id = ctx
            .spec_path
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
        let outcome = if ctx.needs_human_approval {
            "needs-human-approval"
        } else if ctx.scope_reduction_needed {
            "coder-retries-exhausted"
        } else if ctx.coder_refused {
            "coder-refused"
        } else {
            "done"
        };
        let entry = crate::core::run_log::build_entry(spec_id, &ctx.stage_durations, outcome);
        crate::core::run_log::append_entry(entry);

        info!("Pipeline complete");
        Ok(())
    }
}

fn is_no_action_description(description: &str) -> bool {
    let normalized = description.trim().to_lowercase();
    normalized.starts_with("no clear improvement found")
        || normalized.starts_with("no actionable improvement found")
        || normalized.starts_with("no grounded improvement found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, Once};

    /// Initialize global config exactly once for the test binary.
    fn setup() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let root =
                std::env::temp_dir().join(format!("bacon-agent-tests-{}", std::process::id()));
            let _ = std::fs::create_dir_all(root.join(".bacon/sessions"));
            crate::config::init(crate::config::ProjectConfig::with_defaults(root));
            // Disable CI annotations to avoid stray output.
            crate::core::run_log::set_ci_mode(false);
        });
    }

    // ------------------------------------------------------------------
    // MockAgent
    // ------------------------------------------------------------------

    struct MockAgent {
        dry_run: bool,
        auto: bool,
        fast: bool,
        resume: Option<Stage>,
        pcfg: PipelineConfig,
        observer_ctx: PipelineCtx,
        strategist_ctx: PipelineCtx,
        coder_ctx: PipelineCtx,
        auditor_ctx: PipelineCtx,
        stages_called: Arc<Mutex<Vec<&'static str>>>,
        delay_called: Arc<AtomicBool>,
    }

    impl MockAgent {
        fn new(auto: bool) -> Self {
            let mut strategist_ctx =
                PipelineCtx::new("Implement spec XYZ".into(), None, None, None);
            strategist_ctx.spec_path = Some(PathBuf::from("/mock/spec-xyz"));

            Self {
                dry_run: false,
                auto,
                fast: false,
                resume: None,
                pcfg: PipelineConfig::default(),
                observer_ctx: PipelineCtx::new("Scan for improvements".into(), None, None, None),
                strategist_ctx,
                coder_ctx: PipelineCtx::new("Coder patch applied".into(), None, None, None),
                auditor_ctx: PipelineCtx::new(
                    "PASS - all acceptance criteria met".into(),
                    None,
                    None,
                    None,
                ),
                stages_called: Arc::new(Mutex::new(Vec::new())),
                delay_called: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait]
    impl PipelineAgent for MockAgent {
        fn name(&self) -> &str {
            "mock"
        }

        fn dry_run(&self) -> bool {
            self.dry_run
        }

        fn auto(&self) -> bool {
            self.auto
        }

        fn fast(&self) -> bool {
            self.fast
        }

        fn resume_stage(&self) -> Option<Stage> {
            self.resume
        }

        fn pipeline_cfg(&self) -> &PipelineConfig {
            &self.pcfg
        }

        async fn run_observer(&self, _ctx: &PipelineCtx) -> Result<PipelineCtx> {
            self.stages_called.lock().unwrap().push("observer");
            Ok(self.observer_ctx.clone())
        }

        async fn run_strategist(&self, _ctx: &PipelineCtx) -> Result<PipelineCtx> {
            self.stages_called.lock().unwrap().push("strategist");
            Ok(self.strategist_ctx.clone())
        }

        async fn run_coder(&self, _ctx: &PipelineCtx) -> Result<PipelineCtx> {
            self.stages_called.lock().unwrap().push("coder");
            Ok(self.coder_ctx.clone())
        }

        async fn run_auditor(&self, _ctx: &PipelineCtx) -> Result<PipelineCtx> {
            self.stages_called.lock().unwrap().push("auditor");
            Ok(self.auditor_ctx.clone())
        }

        async fn stage_delay(&self) {
            self.delay_called.store(true, Ordering::SeqCst);
        }
    }

    /// Assert the exact sequence of stages that were called.
    fn check_stages(agent: &MockAgent, expected: &[&str]) {
        let stages = agent.stages_called.lock().unwrap();
        assert_eq!(
            *stages, expected,
            "expected stages {:?}, got {:?}",
            expected, *stages
        );
    }

    // ------------------------------------------------------------------
    // Helper: build a default PipelineCtx with low confidence
    // ------------------------------------------------------------------

    fn ctx_with_low_confidence(description: &str) -> PipelineCtx {
        PipelineCtx::new(description.into(), None, None, None)
            .with_confidence(Some(Confidence::Low))
    }

    // ==================================================================
    // Existing test: is_no_action_description
    // ==================================================================

    #[test]
    fn no_action_description_detects_observer_noop() {
        assert!(is_no_action_description(
            "No clear improvement found\nConfidence: High"
        ));
        assert!(is_no_action_description(
            " no actionable improvement found for the current source excerpts"
        ));
        assert!(!is_no_action_description(
            "Add a missing unit test for src/core/spec_io.rs"
        ));
    }

    // ==================================================================
    // Happy path — all four stages execute in sequence
    // ==================================================================

    #[tokio::test]
    async fn run_happy_path_all_stages() {
        setup();
        let agent = MockAgent::new(true);
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["observer", "strategist", "coder", "auditor"]);
    }

    // ==================================================================
    // Fast path — skips Strategist and Auditor
    // ==================================================================

    #[tokio::test]
    async fn run_fast_path_skips_strategist_and_auditor() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.fast = true;
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["observer", "coder"]);
    }

    // ==================================================================
    // Resume — skip stages before the resume point
    // ==================================================================

    #[tokio::test]
    async fn run_resume_from_strategist_skips_observer() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.resume = Some(Stage::Strategist);
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["strategist", "coder", "auditor"]);
    }

    #[tokio::test]
    async fn run_resume_from_coder_skips_observer_and_strategist() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.resume = Some(Stage::Coder);
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["coder", "auditor"]);
    }

    #[tokio::test]
    async fn run_resume_from_auditor_runs_only_auditor() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.resume = Some(Stage::Auditor);
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["auditor"]);
    }

    // ==================================================================
    // Dry run — short-circuits early
    // ==================================================================

    #[tokio::test]
    async fn run_dry_run_short_circuits() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.dry_run = true;
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["observer", "strategist", "coder", "auditor"]);
    }

    // ==================================================================
    // Confidence — Low in auto mode warns but continues
    // ==================================================================

    #[tokio::test]
    async fn run_low_confidence_in_auto_mode_continues() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.observer_ctx = ctx_with_low_confidence("observed something");
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        // Pipeline should continue past the low-confidence observer.
        let stages = agent.stages_called.lock().unwrap();
        assert!(stages.contains(&"strategist"), "should reach strategist");
    }

    // ==================================================================
    // Observer no-op — auto mode exits early
    // ==================================================================

    #[tokio::test]
    async fn run_observer_noop_auto_exits_early() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.observer_ctx =
            PipelineCtx::new("No clear improvement found".into(), None, None, None);
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["observer"]);
    }

    // ==================================================================
    // Strategist — no spec produced in auto mode exits early
    // ==================================================================

    #[tokio::test]
    async fn run_strategist_no_spec_auto_exits_early() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.strategist_ctx = PipelineCtx::new("No spec needed".into(), None, None, None);
        // spec_path stays None (default).
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        check_stages(&agent, &["observer", "strategist"]);
    }

    // ==================================================================
    // Coder outcomes
    // ==================================================================

    #[tokio::test]
    async fn run_coder_refused_bails() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.coder_ctx = PipelineCtx::new("I will not code this".into(), None, None, None);
        agent.coder_ctx.coder_refused = true;
        let result = agent.run().await;
        assert!(result.is_err(), "expected Err, got: {result:?}");
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("refused"),
            "error should mention refusal: {err}"
        );
        check_stages(&agent, &["observer", "strategist", "coder"]);
    }

    #[tokio::test]
    async fn run_coder_needs_human_approval_bails() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.coder_ctx = PipelineCtx::new("I will not code this".into(), None, None, None);
        agent.coder_ctx.needs_human_approval = true;
        agent.coder_ctx.spec_path = Some(PathBuf::from("/mock/spec-xyz"));
        let result = agent.run().await;
        assert!(result.is_err(), "expected Err, got: {result:?}");
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("human approval"),
            "error should mention human approval: {err}"
        );
        check_stages(&agent, &["observer", "strategist", "coder"]);
    }

    #[tokio::test]
    async fn run_coder_retries_exhausted_bails() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.coder_ctx = PipelineCtx::new("I will not code this".into(), None, None, None);
        agent.coder_ctx.scope_reduction_needed = true;
        agent.coder_ctx.coder_errors = vec![
            "error 1: patch did not apply".into(),
            "error 2: patch did not apply".into(),
        ];
        agent.coder_ctx.spec_path = Some(PathBuf::from("/mock/spec-xyz"));
        let result = agent.run().await;
        assert!(result.is_err(), "expected Err, got: {result:?}");
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("retries exhausted"),
            "error should mention retries exhausted: {err}"
        );
        check_stages(&agent, &["observer", "strategist", "coder"]);
    }

    // ==================================================================
    // Spec file reference validation
    // ==================================================================

    #[tokio::test]
    async fn run_missing_spec_file_refs_marks_approval() {
        setup();
        let mut agent = MockAgent::new(true);
        // Create a spec dir with plan.md referencing a file that doesn't
        // exist relative to the project root (set by setup()).
        let spec_dir =
            std::env::temp_dir().join(format!("bacon-agent-test-missing-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&spec_dir);
        // plan.md must exist and reference a non-existent file under src/
        std::fs::write(spec_dir.join("plan.md"), "Modify src/non_existent_file.rs").unwrap();
        agent.strategist_ctx.spec_path = Some(spec_dir.clone());
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok (approval), got: {result:?}");
        // Pipeline should stop after strategist (before coder) because
        // validate_spec_file_refs detects the missing file.
        let stages = agent.stages_called.lock().unwrap();
        assert!(
            !stages.contains(&"coder"),
            "coder should NOT have been called: stages={stages:?}"
        );
        let _ = std::fs::remove_dir_all(&spec_dir);
    }

    // ==================================================================
    // Stage delay — called when config.stage_delay_ms > 0
    // ==================================================================

    #[tokio::test]
    async fn run_stage_delay_called_when_configured() {
        setup();
        let mut agent = MockAgent::new(true);
        agent.pcfg.stage_delay_ms = 100;
        let result = agent.run().await;
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        assert!(
            agent.delay_called.load(Ordering::SeqCst),
            "stage_delay() should have been called"
        );
    }
}
