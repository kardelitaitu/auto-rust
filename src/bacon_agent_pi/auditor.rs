use anyhow::Result;
use log::info;

use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

fn role_prompt() -> String {
    crate::bacon_core::read_role_prompt("auditor")
}

pub async fn run(llm: &Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let spec_path = match &ctx.spec_path {
        Some(p) => p.clone(),
        None if ctx.dry_run => {
            info!(
                "DRY RUN: no spec path available; would run Auditor after Coder implements a spec"
            );
            return Ok(PipelineCtx::new(ctx.description.clone()).with_dry_run(true));
        }
        None => anyhow::bail!("No spec path provided to Auditor"),
    };

    let meta = spec_io::read_spec_meta(&spec_path)?;
    let spec_name = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Read spec content files for full context
    let plan = std::fs::read_to_string(spec_path.join("plan.md")).unwrap_or_default();
    let baseline = std::fs::read_to_string(spec_path.join("baseline.md")).unwrap_or_default();
    let validation_spec =
        std::fs::read_to_string(spec_path.join("validation.md")).unwrap_or_default();
    let impl_notes =
        std::fs::read_to_string(spec_path.join("implementation-notes.md")).unwrap_or_default();

    // Capture the git diff if available
    let diff = std::process::Command::new("git")
        .args(["diff", "--", "src/"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                if out.len() > 10000 {
                    Some(format!("{}...\n[truncated at 10000 chars]", &out[..10000]))
                } else if !out.is_empty() {
                    Some(out)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            "(no diff available — check implementation-notes.md for patch details)".to_string()
        });

    let system_prompt = role_prompt();

    let user_prompt = format!(
        "Audit this implemented spec: {} ({})\n\n\
         Title: {}\nStatus: {}\n\n\
         ## Plan (from plan.md)\n{}\n\n\
         ## Baseline\n{}\n\n\
         ## Validation Criteria\n{}\n\n\
         ## Implementation Notes\n{}\n\n\
         ## Git Diff (working tree)\n```diff\n{}\n```\n\n\
         Does the implementation match the spec? \
         Are all acceptance criteria met? \
         Any missed edge cases, scope violations, or regressions?\n\n\
         Respond with PASS or FAIL as the first word, then explain your reasoning.\n\
         PASS → the spec should be marked done and moved to _done/.\n\
         FAIL → mark needs-human-approval and explain what's missing.",
        meta.title,
        spec_name,
        meta.title,
        meta.status,
        plan,
        baseline,
        validation_spec,
        impl_notes,
        diff
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

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("Auditor confidence: {}", conf.as_str());
    }

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

    Ok(PipelineCtx::new(response).with_confidence(confidence))
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
