use anyhow::Result;
use log::{info, warn};

use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;

fn role_prompt() -> String {
    crate::bacon_core::read_role_prompt("auditor")
}

pub async fn run(llm: &crate::llm::Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let system_prompt = role_prompt();

    // Resolve spec context: prefer files on disk, fall back to ctx.description
    let (_spec_path, meta, spec_name, plan, validation_spec) = match &ctx.spec_path {
        Some(p) => {
            let meta = spec_io::read_spec_meta(p)?;
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let plan = spec_io::read_spec_file(p, "plan.md");
            let validation_spec = spec_io::read_spec_file(p, "validation.md");
            (Some(p.clone()), meta, name, plan, validation_spec)
        }
        None => {
            warn!("No spec path — using ctx.description as plan for audit");
            let meta = spec_io::SpecMeta {
                id: "adhoc".to_string(),
                title: "Ad-hoc task".to_string(),
                status: "implemented".to_string(),
                owner: "pipeline".to_string(),
                implementer: "auto".to_string(),
                priority: "medium".to_string(),
            };
            let plan = ctx.description.clone();
            let validation_spec = "See plan for validation criteria.".to_string();
            (None, meta, "adhoc".to_string(), plan, validation_spec)
        }
    };

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

    info!("NVIDIA Auditor calling API...");
    let messages = vec![
        crate::llm::ChatMessage::system(system_prompt),
        crate::llm::ChatMessage::user(user_prompt),
    ];
    let response = llm.chat(messages).await?;

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Auditor confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Auditor Output ===");
    println!("{}", response);
    println!("=============================");

    let mut output = PipelineCtx::new(response).with_confidence(confidence).with_dry_run(ctx.dry_run);
    output.spec_path = ctx.spec_path.clone();
    output.patch_path = ctx.patch_path.clone();

    let decision_first = output.description.split_whitespace().next().unwrap_or("");
    if let Some(ref spec_path) = output.spec_path {
        if decision_first.eq_ignore_ascii_case("PASS") {
            info!("NVIDIA Auditor PASS");
            if output.dry_run {
                info!("DRY RUN: would move spec to _done/");
            } else {
                let archived_path = promote_to_done(spec_path)?;
                output.spec_path = Some(archived_path);
            }
        } else {
            info!("NVIDIA Auditor FAIL — marking needs-human-approval");
            if !output.dry_run {
                write_audit_report(spec_path, &output.description)?;
            }
            output.set_needs_approval();
        }
    } else {
        // No spec path on disk — just log the audit result
        if decision_first.eq_ignore_ascii_case("PASS") {
            info!("NVIDIA Auditor PASS (adhoc — no spec to archive)");
        } else {
            warn!("NVIDIA Auditor found issues (adhoc — no spec to file)");
        }
    }

    Ok(output)
}

fn promote_to_done(path: &std::path::Path) -> Result<std::path::PathBuf> {
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

    // Rewrite docs array in spec.yaml to point to _done/ instead of _active/
    let yaml_path = path.join("spec.yaml");
    match std::fs::read_to_string(&yaml_path) {
        Ok(content) => {
            let updated_content = content
                .replace("docs/specs/_active/", "docs/specs/_done/")
                .replace("docs\\specs\\_active\\", "docs\\specs\\_done\\");
            if let Err(e) = std::fs::write(&yaml_path, updated_content) {
                warn!("Failed to write updated spec.yaml paths: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed to read spec.yaml for path rewrite: {}", e);
        }
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    info!("Moving {} to _done/", name);
    let dest = spec_io::move_to_done(path)?;
    Ok(dest)
}

fn write_audit_report(path: &std::path::Path, report: &str) -> Result<()> {
    let validation_path = path.join("validation.md");
    let existing = std::fs::read_to_string(&validation_path).unwrap_or_else(|e| {
        warn!("Failed to read existing validation.md: {}", e);
        String::new()
    });
    std::fs::write(
        &validation_path,
        format!("# Audit Report\n\n{}\n\n{}", report, existing),
    )?;

    info!("Wrote audit report to validation.md");
    Ok(())
}
