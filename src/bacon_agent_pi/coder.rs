use anyhow::Result;
use log::info;
use std::path::Path;

use crate::llm::{ChatMessage, Llm};

use super::cli::Args;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, _args: &Args, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let spec_path = match &ctx.spec_path {
        Some(p) => p.clone(),
        None => anyhow::bail!("No spec path provided to Coder"),
    };

    let plan = std::fs::read_to_string(spec_path.join("plan.md"))?;
    let spec_name = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

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

    // Mark spec as implemented
    mark_implemented(&spec_path)?;

    let mut output = PipelineCtx::new(ctx.description.clone());
    output.spec_path = Some(spec_path);
    Ok(output)
}

fn mark_implemented(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "implemented".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status set to: implemented");
    Ok(())
}
