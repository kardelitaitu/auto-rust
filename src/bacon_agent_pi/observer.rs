use anyhow::Result;
use log::info;

use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, args: &RunArgs, base: &PipelineCtx) -> Result<PipelineCtx> {
    // First check _active/ for pending specs
    let active = spec_io::list_active_specs()?;

    for spec_path in &active {
        let meta = spec_io::read_spec_meta(spec_path)?;
        if meta.status == "approved" {
            let name = spec_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            info!("Found pending spec: {} ({})", meta.title, name);
            let mut ctx = PipelineCtx::new(format!("Implement spec: {} ({})", meta.title, name));
            ctx.spec_path = Some(spec_path.clone());
            ctx.dry_run = base.dry_run;
            return Ok(ctx);
        }
    }

    // No pending specs — ask LLM to find a small improvement
    info!("No pending specs found, scanning codebase for improvements");

    let system_prompt = include_str!("../../.bacon/roles/01_bacon-observer.md");
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

    println!("=== Observer Output ===");
    println!("{}", response);
    println!("=======================");

    let mut ctx = PipelineCtx::new(response);
    ctx.dry_run = base.dry_run;
    Ok(ctx)
}

fn scan_project_structure() -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut parts = Vec::new();

    let src = root.join("src");
    if src.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&src) {
            let dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir() || e.path().extension().is_some_and(|x| x == "rs"))
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if e.path().is_dir() {
                        format!("src/{}/", name)
                    } else {
                        format!("src/{}", name)
                    }
                })
                .collect();
            if !dirs.is_empty() {
                parts.push("Source modules:".to_string());
                parts.extend(dirs.iter().map(|d| format!("  {}", d)));
            }
        }
    }

    let bin = src.join("bin");
    if bin.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&bin) {
            let bins: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
                .map(|e| format!("  bin/{}", e.file_name().to_string_lossy()))
                .collect();
            if !bins.is_empty() {
                parts.push("Binaries:".to_string());
                parts.extend(bins);
            }
        }
    }

    if let Ok(specs) = spec_io::list_active_specs() {
        if !specs.is_empty() {
            let names: Vec<_> = specs
                .iter()
                .filter_map(|p| {
                    let name = p.file_name()?.to_string_lossy().to_string();
                    let meta = spec_io::read_spec_meta(p).ok()?;
                    Some(format!("  [{}] {} ({})", meta.status, meta.title, name))
                })
                .collect();
            if !names.is_empty() {
                parts.push("Active specs:".to_string());
                parts.extend(names);
            }
        }
    }

    parts.join("\n")
}
