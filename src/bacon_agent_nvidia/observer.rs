use anyhow::Result;
use log::info;

use super::cli::RunArgs;
use super::nvidia_api;
use super::types::PipelineCtx;

fn system_prompt() -> String {
    crate::bacon_core::read_role_prompt("observer")
}

pub async fn run(_llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let config = args.nvidia_config();
    let prompt = args
        .prompt
        .as_deref()
        .unwrap_or("Scan codebase for improvements");

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
