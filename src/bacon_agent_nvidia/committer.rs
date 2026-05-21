use anyhow::Result;
use log::{info, warn};
use std::process::Command;

use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;

pub async fn run(_llm: &crate::llm::Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    info!("=== Stage 5: Committer (agent: nvidia) ===");

    // We only want to commit if we have a valid spec that was just implemented and audited.
    let spec_path = match &ctx.spec_path {
        Some(p) => p,
        None => {
            info!("No spec path in context, skipping Committer stage.");
            return Ok(ctx.clone());
        }
    };

    let meta = match spec_io::read_spec_meta(spec_path) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to read spec meta, cannot generate commit message: {}", e);
            return Ok(ctx.clone());
        }
    };

    // Ensure the spec is actually "done" before we commit
    if meta.status != "done" {
        info!("Spec status is not 'done' (current: {}), skipping commit.", meta.status);
        return Ok(ctx.clone());
    }

    if ctx.dry_run {
        info!("DRY RUN: would run check.ps1 and commit changes for: {}", meta.title);
        return Ok(ctx.clone());
    }

    info!("Running final full suite validation (check.ps1)...");
    
    // We run the full check.ps1 suite. If it fails, we abort the commit but don't fail the pipeline.
    let (passed, output) = crate::bacon_core::run_powershell_with_args("check.ps1", &[])?;
    
    if !passed {
        warn!("check.ps1 failed! Changes will NOT be committed. Manual review required.");
        warn!("check.ps1 output:\n{}", output);
        // We return the context unchanged; the changes remain in the working tree.
        return Ok(ctx.clone());
    }

    info!("check.ps1 passed. Preparing to commit...");

    // Check if there are actually changes to commit
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    
    let status_str = String::from_utf8_lossy(&status.stdout);
    if status_str.trim().is_empty() {
        info!("No changes detected in working tree, nothing to commit.");
        return Ok(ctx.clone());
    }

    // Stage all changes (we assume the autonomous pipeline is the only thing running)
    let add_result = Command::new("git")
        .args(["add", "."])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if !add_result.success() {
        warn!("git add . failed, skipping commit.");
        return Ok(ctx.clone());
    }

    // Determine commit prefix based on the area or content
    let prefix = if meta.title.to_lowercase().contains("test") || meta.title.to_lowercase().contains("coverage") {
        "test"
    } else if meta.title.to_lowercase().contains("fix") {
        "fix"
    } else if meta.title.to_lowercase().contains("doc") || meta.title.to_lowercase().contains("readme") {
        "docs"
    } else {
        "feat"
    };

    let spec_num = spec_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.split('-').next())
        .unwrap_or("unknown");

    let commit_msg = format!("{}: {} ({})", prefix, meta.title, spec_num);

    info!("Executing: git commit -m \"{}\"", commit_msg);
    let commit_result = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if commit_result.success() {
        info!("Successfully committed changes!");
    } else {
        warn!("git commit failed.");
    }

    Ok(ctx.clone())
}