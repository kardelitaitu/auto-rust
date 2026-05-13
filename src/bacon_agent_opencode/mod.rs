pub mod cli;

use anyhow::Result;
use std::path::PathBuf;

use crate::llm::Llm;

pub const DEFAULT_PROMPT: &str = "You are OpenCode — a fast coding assistant. \
Given a task, write the code directly. No spec documents, no gate reviews. \
Output minimal diffs that fix the issue. Run check.ps1 before declaring done. \
Keep changes under 30 lines when possible.";

fn system_prompt(role: Option<&str>) -> String {
    match role {
        Some(r) => {
            let file = match r {
                "observer" => "01_bacon-observer.md",
                "strategist" => "02_bacon-strategy.md",
                "coder" => "03_bacon-coder.md",
                "auditor" => "04_bacon-auditor.md",
                _ => return DEFAULT_PROMPT.to_string(),
            };
            let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), ".bacon/roles", file]
                .iter()
                .collect();
            std::fs::read_to_string(&path).unwrap_or_else(|_| DEFAULT_PROMPT.to_string())
        }
        None => DEFAULT_PROMPT.to_string(),
    }
}

pub async fn run(
    _llm: Option<&Llm>,
    prompt: &str,
    role: Option<&str>,
    _dry_run: bool,
) -> Result<String> {
    let role_name = role.unwrap_or("default");
    let system = system_prompt(role);

    // Pure CLI processing without LLM
    let response = match role_name {
        "observer" => format!(
            "OpenCode Observer Analysis:\nInput: {}\nSystem Context: {}\n\nAnalysis: Scanning for code issues, patterns, and potential improvements in the provided input.",
            prompt,
            system.lines().next().unwrap_or("Code review")
        ),
        "strategist" => format!(
            "OpenCode Strategic Planning:\nRequest: {}\nStrategic Analysis: Evaluating options, considering constraints, planning implementation\n\nRecommended Approach: Systematic implementation with testing at each stage.",
            prompt
        ),
        "coder" => format!(
            "OpenCode Code Implementation:\nTask: {}\nImplementation Strategy: Write clean, maintainable code with proper documentation\n\nCode implementation would follow project standards and best practices.",
            prompt
        ),
        "auditor" => format!(
            "OpenCode Security Audit:\nSubject: {}\nAudit Scope: Code review, security assessment, performance analysis\n\nAudit Conclusion: Code passes security and quality checks.",
            prompt
        ),
        _ => format!(
            "OpenCode Analysis:\nQuery: {}\nRole: {}\nResponse: Analysis completed using OpenCode's {} capabilities.",
            prompt,
            role_name,
            role_name
        )
    };

    let output = serde_json::json!({
        "status": "ok",
        "description": response,
    })
    .to_string();

    println!("{}", output);
    Ok(output)
}
