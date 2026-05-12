use anyhow::Result;
use log::{info, warn};
use std::path::Path;

use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::pipeline;
use super::spec_io;
use super::types::PipelineCtx;

const MAX_RETRIES: u32 = 3;

pub async fn run(llm: &Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let spec_path = match &ctx.spec_path {
        Some(p) => p.clone(),
        None => anyhow::bail!("No spec path provided to Coder"),
    };

    let plan = std::fs::read_to_string(spec_path.join("plan.md"))?;
    let spec_name = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Mark in-progress
    if !ctx.dry_run {
        mark_in_progress(&spec_path)?;
    }

    let system_prompt = include_str!("../../.bacon/roles/03_bacon-coder.md");

    let mut attempt = 1u32;
    let mut last_error = String::new();

    loop {
        let user_prompt = if attempt == 1 {
            format!(
                "Implement the following spec ({}):\n\n\
                 Spec path: {}\n\n\
                 Plan:\n{}\n\n\
                 Read the spec files, implement the changes, then run check.ps1 to verify.",
                spec_name,
                spec_path.display(),
                plan
            )
        } else {
            format!(
                "The previous implementation attempt failed. Fix these errors:\n\n\
                 {}\n\n\
                 Spec path: {}\n\n\
                 Plan:\n{}",
                last_error,
                spec_path.display(),
                plan
            )
        };

        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(&user_prompt),
        ];

        info!("Calling Coder LLM (attempt {}/{})...", attempt, MAX_RETRIES);
        let response = llm
            .chat(messages)
            .await
            .map_err(|e| anyhow::anyhow!("Coder LLM call failed: {}", e))?;

        println!("=== Coder Output (attempt {}) ===", attempt);
        println!("{}", response);
        println!("================================");

        if ctx.dry_run {
            info!("DRY RUN: would run check.ps1 and mark implemented");
            break;
        }

        // Run check.ps1
        info!(
            "Running check.ps1 to verify implementation (attempt {})...",
            attempt
        );
        let (passed, output) = pipeline::run_powershell("check.ps1")?;

        if passed {
            info!("check.ps1 passed on attempt {}", attempt);
            mark_implemented(&spec_path)?;
            break;
        }

        warn!("check.ps1 failed on attempt {}", attempt);

        if attempt >= MAX_RETRIES {
            warn!("Max retries exhausted — marking needs-human-approval");
            mark_needs_human_approval(&spec_path, &output)?;
            break;
        }

        last_error = output;
        attempt += 1;
    }

    let mut output = PipelineCtx::new(ctx.description.clone());
    output.spec_path = Some(spec_path);
    output.dry_run = ctx.dry_run;
    Ok(output)
}

fn mark_in_progress(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "in-progress".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status set to: in-progress");
    Ok(())
}

fn mark_implemented(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "implemented".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status set to: implemented");
    Ok(())
}

fn mark_needs_human_approval(path: &Path, report: &str) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "needs-human-approval".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    std::fs::write(
        path.join("validation.md"),
        format!("# Coder Failure Report\n\n{}", report),
    )?;
    info!("Spec status set to: needs-human-approval (retries exhausted)");
    Ok(())
}
