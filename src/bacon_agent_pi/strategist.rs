use anyhow::Result;
use log::{info, warn};
use std::path::PathBuf;

use crate::llm::{ChatMessage, Llm};

use super::cli::Args;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, _args: &Args, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let system_prompt = include_str!("../../.bacon/roles/02_bacon-strategy.md");

    let user_prompt = format!(
        "Review this finding and design an implementation plan:\n\n\
         {}\n\n\
         If the approach is sound and risks acceptable, output a plan with:\n\
         1. A clear title (one line)\n\
         2. Step-by-step implementation steps\n\
         3. Current state description (baseline)\n\
         4. Any API changes needed\n\
         5. How to validate\n\
         6. Design decisions and risks\n\n\
         If risks are too high, start your response with 'REJECTED:' and explain why.",
        ctx.description
    );

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    info!("Calling Strategist LLM...");
    let response = llm
        .chat(messages)
        .await
        .map_err(|e| anyhow::anyhow!("Strategist LLM call failed: {}", e))?;

    // Check for rejection
    if response.trim().starts_with("REJECTED:") {
        warn!("Strategist rejected the proposal: {}", response);
        return Err(anyhow::anyhow!("Strategist rejected: {}", response));
    }

    println!("=== Strategist Output ===");
    println!("{}", response);
    println!("=========================");

    // Write spec package
    let spec_path = write_spec_package(&response)?;
    info!("Spec package created at: {}", spec_path.display());

    let mut output = PipelineCtx::new(ctx.description.clone());
    output.spec_path = Some(spec_path);
    Ok(output)
}

fn write_spec_package(plan: &str) -> Result<PathBuf> {
    let number = spec_io::next_spec_number()?;
    let title = extract_title(plan);
    let dir_name = format!("{:04}-{}", number, slugify(&title));
    let active = spec_io::active_dir();
    let spec_dir = active.join(&dir_name);
    std::fs::create_dir_all(&spec_dir)?;

    // spec.yaml
    let meta = spec_io::SpecMeta {
        id: format!("{:04}-{}", number, slugify(&title)),
        title: title.clone(),
        status: "approved".to_string(),
        owner: "pipeline".to_string(),
        implementer: "pipeline".to_string(),
        priority: "P2".to_string(),
    };
    spec_io::write_spec_meta(&spec_dir, &meta)?;

    // plan.md
    std::fs::write(spec_dir.join("plan.md"), plan)?;

    // baseline.md
    let baseline = extract_section(plan, "baseline", "Current state description.");
    std::fs::write(spec_dir.join("baseline.md"), baseline)?;

    // internal-api-outline.md
    let api = extract_section(plan, "api", "No API changes.");
    std::fs::write(spec_dir.join("internal-api-outline.md"), api)?;

    // validation.md
    let validation = extract_section(plan, "validat", "Run check.ps1.");
    std::fs::write(spec_dir.join("validation.md"), validation)?;

    // notes.md
    let notes = extract_section(plan, "risk|decision|design", "See plan.md.");
    std::fs::write(spec_dir.join("notes.md"), notes)?;

    // ci-commands.md
    std::fs::write(spec_dir.join("ci-commands.md"), "check.ps1\n")?;

    // quality-rules.md
    std::fs::write(
        spec_dir.join("quality-rules.md"),
        "Follow project conventions. All checks must pass.\n",
    )?;

    // validation-checklist.md
    std::fs::write(
        spec_dir.join("validation-checklist.md"),
        "- [ ] check.ps1 passes\n- [ ] All acceptance criteria met\n",
    )?;

    // implementation-notes.md
    std::fs::write(
        spec_dir.join("implementation-notes.md"),
        "# Implementation Notes\n\n(TBD by Coder)\n",
    )?;

    // README.md
    std::fs::write(
        spec_dir.join("README.md"),
        format!(
            "# {}\n\nStatus: `approved`\n\nOwner: `pipeline`\nImplementer: `pipeline`\n",
            title
        ),
    )?;

    Ok(spec_dir)
}

fn extract_title(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return stripped.to_string();
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            return stripped.to_string();
        }
    }
    // Fallback: use first non-empty line
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .chars()
        .take(40)
        .collect()
}

fn extract_section(plan: &str, keywords: &str, fallback: &str) -> String {
    let lines = plan.lines();
    let mut result = String::new();
    let mut in_section = false;

    for line in lines {
        let lower = line.to_lowercase();
        if lower.starts_with("## ") || lower.starts_with("##") && lower.contains(keywords) {
            in_section = true;
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if lower.starts_with("## ") && in_section {
            break;
        }
        if in_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    if result.is_empty() {
        return fallback.to_string();
    }
    result
}
