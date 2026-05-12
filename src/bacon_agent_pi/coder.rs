use anyhow::Result;
use log::{info, warn};
use std::path::Path;

use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::pipeline;
use super::spec_io;
use super::types::PipelineCtx;

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

    let user_prompt = format!(
        "Implement the following spec ({}):\n\n\
         Spec path: {}\n\n\
         Plan:\n{}\n\n\
         Read the spec files, implement the changes, then run check.ps1 to verify.\n\
         If check.ps1 fails, explain what went wrong.",
        spec_name,
        spec_path.display(),
        plan
    );

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    info!("Calling Coder LLM...");
    let response = llm
        .chat(messages)
        .await
        .map_err(|e| anyhow::anyhow!("Coder LLM call failed: {}", e))?;

    println!("=== Coder Output ===");
    println!("{}", response);
    println!("====================");

    // Run check.ps1 to verify changes
    if ctx.dry_run {
        info!("DRY RUN: would run check.ps1 and mark implemented");
    } else {
        info!("Running check.ps1 to verify implementation...");
        let passed = pipeline::run_powershell("check.ps1")?;
        if passed {
            info!("check.ps1 passed");
            mark_implemented(&spec_path)?;
        } else {
            warn!("check.ps1 failed — implementation may need fixes");
            mark_implemented(&spec_path)?;
        }
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
