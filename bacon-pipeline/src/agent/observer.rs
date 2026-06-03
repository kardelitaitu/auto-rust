use anyhow::Result;
use log::info;

use super::spec_io;
use super::types::PipelineCtx;
use crate::core::cli_types::RunArgs;

fn system_prompt() -> String {
    crate::core::read_role_prompt("observer")
}

pub async fn run(llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    // If a user prompt was provided, skip spec scan and go straight to LLM
    if args.prompt.is_none() {
        // Check for an approved spec — fast-path if one exists
        if let Some((spec_path, meta)) = spec_io::find_approved_spec()? {
            let name = spec_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("(unknown)")
                .to_string();
            info!(
                "Found approved spec — fast-tracking: {} ({})",
                meta.title, name
            );
            let mut result = PipelineCtx::new(
                format!("Implement spec: {} ({})", meta.title, name),
                ctx.fs.clone(),
                ctx.runner.clone(),
                ctx.llm.clone(),
            );
            result.spec_path = Some(spec_path);
            result.dry_run = ctx.dry_run;
            return Ok(result);
        }
        info!("No approved specs pending; scanning for new improvements");
    }

    // Duplicate check: scan _done/ and _abandoned/ for matching specs
    if let Some(ref prompt) = args.prompt {
        if let Ok(duplicates) = spec_io::find_specs_matching(prompt) {
            if !duplicates.is_empty() {
                let paths: Vec<String> = duplicates
                    .iter()
                    .map(|(p, t)| format!("{} ({})", p.display(), t))
                    .collect();
                info!(
                    "Found {} existing specs matching prompt — skipping LLM scan:\n{}",
                    duplicates.len(),
                    paths.join("\n")
                );
            }
        }
    }

    let prompt = args.prompt.as_deref().unwrap_or(
        "Scan codebase for one grounded, low-risk maintenance improvement. \
         Only propose work proven by the included source excerpts. \
         Do not propose performance tuning, dependency changes, or rewrites. \
         If no concrete issue is visible, respond exactly: No clear improvement found",
    );

    info!("NVIDIA Observer calling API...");
    let context = crate::core::gather_project_context();
    let enriched_prompt = format!("{context}\n\n## Task\n\n{prompt}");
    let messages = vec![
        crate::llm::ChatMessage::system(system_prompt()),
        crate::llm::ChatMessage::user(enriched_prompt),
    ];
    let response = llm.chat(messages).await?;

    // Extract and log confidence
    let confidence = crate::core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Observer confidence: {}", conf.as_str());
    }

    Ok(PipelineCtx::new(
        response,
        ctx.fs.clone(),
        ctx.runner.clone(),
        ctx.llm.clone(),
    )
    .with_dry_run(ctx.dry_run)
    .with_confidence(confidence))
}
