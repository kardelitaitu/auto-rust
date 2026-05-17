use anyhow::Result;
use log::{info, warn};

use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;

fn role_prompt() -> String {
    crate::bacon_core::read_role_prompt("auditor")
}

pub async fn run(_llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let config = crate::bacon_agent_nvidia::cli::nvidia_config_from_args(args);

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
    let plan = spec_io::read_spec_file(&spec_path, "plan.md");
    let validation_spec = spec_io::read_spec_file(&spec_path, "validation.md");

    // Read the approved patch file if available; fall back to git diff
    let diff = if let Some(patch_path) = &ctx.patch_path {
        match std::fs::read_to_string(patch_path) {
            Ok(content) => {
                info!("Reading approved patch from: {}", patch_path.display());
                if content.len() > 10000 {
                    format!("{}...\n[truncated at 10000 chars]", &content[..10000])
                } else {
                    content
                }
            }
            Err(e) => {
                warn!(
                    "Could not read patch file at {}: {}",
                    patch_path.display(),
                    e
                );
                "(no patch file available)".to_string()
            }
        }
    } else {
        warn!("No patch_path in context — falling back to working tree diff (may be empty)");
        std::process::Command::new("git")
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
            .unwrap_or_else(|| "(no diff available — no patch file present)".to_string())
    };

    let system_prompt = role_prompt();
    let user_prompt = format!(
        "Audit this implemented spec: {} ({})\n\n\
	         Title: {}\nStatus: {}\n\n\
	         ## Plan (from plan.md)\n{}\n\n\
	         ## Validation Criteria\n{}\n\n\
	         ## Git Diff (working tree)\n```diff\n{}\n```\n\n\
	         Does the implementation match the spec? \
	         Are all acceptance criteria met? \
	         Any missed edge cases, scope violations, or regressions?\n\n\
	         Respond with PASS or FAIL as the first word, then explain your reasoning.\n\
	         PASS → the spec should be marked done and moved to _done/.\n\
	         FAIL → mark needs-human-approval and explain what's missing.",
        meta.title, spec_name, meta.title, meta.status, plan, validation_spec, diff
    );

    info!("NVIDIA Auditor calling API with model: {}", config.model);
    let response = nvidia_api::chat(&config, &system_prompt, &user_prompt).await?;

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Auditor confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Auditor Output ===");
    println!("{}", response);
    println!("=============================");

    let decision_first = response.split_whitespace().next().unwrap_or("");
    if decision_first.eq_ignore_ascii_case("PASS") {
        info!("NVIDIA Auditor PASS");
        if ctx.dry_run {
            info!("DRY RUN: would move spec to _done/");
        } else {
            promote_to_done(&spec_path)?;
        }
    } else {
        info!("NVIDIA Auditor FAIL — marking needs-human-approval");
        if ctx.dry_run {
            info!("DRY RUN: would mark needs-human-approval");
        } else {
            mark_needs_approval(&spec_path, &response)?;
        }
    }

    Ok(PipelineCtx::new(response).with_confidence(confidence))
}

fn promote_to_done(path: &std::path::Path) -> Result<()> {
    // Gate: run spec-lint before archiving — catches structural errors
    let spec_path_arg = path.to_string_lossy().to_string();
    let (passed, output) = crate::bacon_core::run_powershell_with_args(
        "spec-lint.ps1",
        &["-Directory", spec_path_arg.as_str()],
    )?;
    if !passed {
        anyhow::bail!(
            "spec-lint failed for {} — not moving to _done/:\n{}",
            path.display(),
            output
        );
    }

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
    let existing = std::fs::read_to_string(&validation_path).unwrap_or_else(|e| {
        warn!("Failed to read existing validation.md: {}", e);
        String::new()
    });
    std::fs::write(
        &validation_path,
        format!("# Audit Report\n\n{}\n\n{}", report, existing),
    )?;

    info!("Marked spec as needs-human-approval");
    Ok(())
}
