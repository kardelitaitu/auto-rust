use anyhow::Result;
use log::info;

use super::cli::RunArgs;
use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;

fn system_prompt() -> String {
    crate::bacon_core::read_role_prompt("observer")
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
            let mut result = PipelineCtx::new(format!("Implement spec: {} ({})", meta.title, name));
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

    let config = args.nvidia_config();
    let prompt = args
        .prompt
        .as_deref()
        .unwrap_or("Scan codebase for improvements");

    // Quick health check before the first LLM call
    if !llm.health_check().await {
        anyhow::bail!("LLM health check failed — check that Ollama/NVIDIA service is running");
    }

    info!("NVIDIA Observer calling API with model: {}", config.model);
    let response = nvidia_api::chat(&config, &system_prompt(), prompt).await?;

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Observer confidence: {}", conf.as_str());
    }

    Ok(PipelineCtx::new(response)
        .with_dry_run(ctx.dry_run)
        .with_confidence(confidence))
}
