use anyhow::Result;
use log::info;

use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let spec_path = match &ctx.spec_path {
        Some(p) => p.clone(),
        None => anyhow::bail!("No spec path provided to Auditor"),
    };

    let meta = spec_io::read_spec_meta(&spec_path)?;
    let spec_name = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let system_prompt = include_str!("../../.bacon/roles/04_bacon-auditor.md");

    let user_prompt = format!(
        "Audit this implemented spec: {} ({})\n\n\
         Title: {}\nStatus: {}\n\n\
         Does the implementation match the spec? \
         Are all acceptance criteria met? \
         Any missed edge cases?\n\n\
         Respond with PASS or FAIL as the first word, then explain your reasoning.\n\
         PASS → the spec should be marked done and moved to _done/.\n\
         FAIL → mark needs-human-approval and explain what's missing.",
        meta.title, spec_name, meta.title, meta.status
    );

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    info!("Calling Auditor LLM...");
    let response = llm
        .chat(messages)
        .await
        .map_err(|e| anyhow::anyhow!("Auditor LLM call failed: {}", e))?;

    println!("=== Auditor Output ===");
    println!("{}", response);
    println!("======================");

    let decision = response.trim().to_lowercase();
    if decision.starts_with("pass") {
        info!("Auditor PASS");
        if ctx.dry_run {
            info!("DRY RUN: would move spec to _done/");
        } else {
            promote_to_done(&spec_path)?;
        }
    } else {
        info!("Auditor FAIL — marking needs-human-approval");
        if ctx.dry_run {
            info!("DRY RUN: would mark needs-human-approval");
        } else {
            mark_needs_approval(&spec_path, &response)?;
        }
    }

    Ok(PipelineCtx::new(response))
}

fn promote_to_done(path: &std::path::Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "done".to_string();
    spec_io::write_spec_meta(path, &meta)?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    info!("Moving {} to _done/", name);
    spec_io::move_to_done(path)?;
    Ok(())
}

fn mark_needs_approval(path: &std::path::Path, report: &str) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "needs-human-approval".to_string();
    spec_io::write_spec_meta(path, &meta)?;

    let validation_path = path.join("validation.md");
    let existing = std::fs::read_to_string(&validation_path).unwrap_or_default();
    std::fs::write(
        &validation_path,
        format!("# Audit Report\n\n{}\n\n{}", report, existing),
    )?;

    info!("Marked spec as needs-human-approval");
    Ok(())
}
