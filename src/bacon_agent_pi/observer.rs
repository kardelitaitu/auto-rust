use anyhow::Result;
use log::info;

use crate::bacon_core::{read_role_prompt, scan_project_structure};
use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, args: &RunArgs, base: &PipelineCtx) -> Result<PipelineCtx> {
    // If user provided a prompt, always use it — no randomness
    if args.prompt.is_some() {
        return scan_for_improvements(llm, args, base).await;
    }

    // No user prompt: 70% implement existing specs, 30% create new ones
    let approved: Vec<_> = spec_io::list_active_specs()?
        .into_iter()
        .filter(|p| {
            spec_io::read_spec_meta(p)
                .ok()
                .is_some_and(|m| m.status == "approved")
        })
        .collect();

    if !approved.is_empty() && rand::random::<f64>() < 0.7 {
        let idx = rand::random::<usize>() % approved.len();
        let spec_path = &approved[idx];
        let meta = spec_io::read_spec_meta(spec_path)?;
        let name = spec_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("(unknown)")
            .to_string();
        info!(
            "70% roll: implementing existing spec — {} ({})",
            meta.title, name
        );
        let mut ctx = PipelineCtx::new(format!("Implement spec: {} ({})", meta.title, name));
        ctx.spec_path = Some(spec_path.clone());
        ctx.dry_run = base.dry_run;
        return Ok(ctx);
    }

    if approved.is_empty() {
        info!("No approved specs pending; scanning for new improvements");
    } else {
        info!("30% roll: scanning for a new improvement instead of implementing an existing spec");
    }

    scan_for_improvements(llm, args, base).await
}

async fn scan_for_improvements(
    llm: &Llm,
    args: &RunArgs,
    base: &PipelineCtx,
) -> Result<PipelineCtx> {
    let system_prompt = read_role_prompt("observer");
    let codebase = scan_project_structure();

    let user_prompt = if let Some(prompt) = &args.prompt {
        format!(
            "The user's request: {}\n\n\
             Project structure:\n{}\n\n\
             Find a small, actionable improvement. Keep scope small: \
             max 30 lines changed, 3 files, no new dependencies.",
            prompt, codebase
        )
    } else {
        format!(
            "Project structure:\n{}\n\n\
             Scan for a small improvement worth automating. \
             Keep scope small: max 30 lines, 3 files, no new dependencies.",
            codebase
        )
    };

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    info!("Calling Observer LLM...");
    let response = llm
        .chat(messages)
        .await
        .map_err(|e| anyhow::anyhow!("Observer LLM call failed: {}", e))?;

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("Observer confidence: {}", conf.as_str());
    }

    println!("=== Observer Output ===");
    println!("{}", response);
    println!("=======================");

    let mut ctx = PipelineCtx::new(response);
    ctx.dry_run = base.dry_run;
    ctx.confidence = confidence;
    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_project_structure_returns_plain_text() {
        let output = scan_project_structure();
        assert!(!output.is_empty(), "should not be empty");
        assert!(
            output.contains("Source modules:"),
            "should list source modules, got: {}",
            output
        );
        assert!(
            output.contains("Binaries:"),
            "should list binaries, got: {}",
            output
        );
        // Verify it's NOT JSON — plain text doesn't start with { or [
        assert!(
            !output.trim_start().starts_with('{'),
            "should not be JSON object: {}",
            output
        );
        assert!(
            !output.trim_start().starts_with('['),
            "should not be JSON array: {}",
            output
        );
    }

    #[test]
    fn scan_project_structure_returns_summary() {
        let output = scan_project_structure();
        assert!(!output.is_empty(), "should not be empty");
        assert!(
            output.contains("Source modules:"),
            "should list source modules, got: {}",
            output
        );
        // Verify it's NOT JSON — plain text doesn't start with { or [
        assert!(
            !output.trim_start().starts_with('{'),
            "should not be JSON object: {}",
            output
        );
        assert!(
            !output.trim_start().starts_with('['),
            "should not be JSON array: {}",
            output
        );
    }

    #[test]
    fn scan_project_structure_contains_src_and_bin() {
        let output = scan_project_structure();
        // Should reference source modules and binaries sections
        assert!(
            output.contains("Source modules:"),
            "should have Source modules section, got: {}",
            output
        );
        assert!(
            output.contains("Binaries:"),
            "should have Binaries section, got: {}",
            output
        );
    }

    #[test]
    fn pipeline_ctx_new_passes_text_through_as_is() {
        let text = "Remove unused function foo from bar.rs — no callers";
        let ctx = PipelineCtx::new(text.to_string());
        assert_eq!(ctx.description, text, "text should be identical");
    }

    #[test]
    fn pipeline_ctx_new_does_not_parse_json_looking_text() {
        // Even text that looks like JSON should pass through unmodified
        let json_looking = r#"{"status": "ok", "description": "refactor error handling"}"#;
        let ctx = PipelineCtx::new(json_looking.to_string());
        assert_eq!(
            ctx.description, json_looking,
            "JSON-looking text should not be parsed"
        );
    }

    #[test]
    fn pipeline_ctx_new_defaults() {
        let ctx = PipelineCtx::new("test".to_string());
        assert!(!ctx.dry_run, "dry_run should default to false");
        assert!(ctx.spec_path.is_none(), "spec_path should default to None");
    }
}
