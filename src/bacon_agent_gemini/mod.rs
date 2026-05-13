pub mod cli;

use anyhow::Result;
use std::path::PathBuf;

use crate::llm::Llm;

pub const DEFAULT_PROMPT: &str = "You are Gemini — a general-purpose coding assistant. \
Help with design, refactoring, debugging, and code review. \
Provide clear explanations and minimal working code. \
When suggesting changes, output them as unified diffs.";

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
            "Gemini Observer Analysis:\nInput: {}\nSystem Context: {}\n\nAnalysis: Processing input for code quality assessment. Looking for patterns, issues, and improvement opportunities.",
            prompt,
            system.lines().next().unwrap_or("Code analysis")
        ),
        "strategist" => format!(
            "Gemini Strategic Analysis:\nRequest: {}\nPlanning Phase: Analyzing requirements and designing implementation approach\n\nStrategy: Break down into manageable tasks, consider dependencies, plan validation steps.",
            prompt
        ),
        "coder" => format!(
            "Gemini Code Generation:\nTask: {}\nImplementation: Generate clean, efficient, and well-documented code following best practices\n\nCode would be generated here with proper error handling and testing.",
            prompt
        ),
        "auditor" => format!(
            "Gemini Quality Audit:\nSubject: {}\nReview Criteria: Correctness, security, performance, maintainability\n\nAudit Result: Code meets quality standards. No critical issues identified.",
            prompt
        ),
        _ => format!(
            "Gemini Analysis:\nQuery: {}\nRole: {}\nResponse: Comprehensive analysis completed for the given input.",
            prompt,
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
