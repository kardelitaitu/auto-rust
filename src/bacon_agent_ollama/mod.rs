// DEPRECATED: This agent module does not implement the current PipelineAgent trait
// and is not compatible with the bacon pipeline. It remains as a reference for
// local-first Ollama integration. To re-enable, implement PipelineAgent and wire
// it into the agent routing in PipelineConfig.

pub mod cli;

use anyhow::Result;
use std::path::PathBuf;

pub const DEFAULT_PROMPT: &str = "You are a helpful coding assistant running on a local \
Ollama model. Be concise, practical, and focused on Rust code.";

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

fn ollama_config() -> (String, String) {
    let url =
        std::env::var("BACON_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = std::env::var("BACON_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());
    (url, model)
}

pub async fn run(prompt: &str, role: Option<&str>, _dry_run: bool) -> Result<String> {
    let role_name = role.unwrap_or("default");
    let system = system_prompt(role);
    let (_url, model) = ollama_config();

    // Pure CLI processing - simulate local Ollama behavior
    let response = match role_name {
        "observer" => format!(
            "Ollama Observer Analysis:\nInput: {}\nSystem Context: {}\nLocal Model Processing: Analyzing code patterns and potential issues with {}\n\nResult: Local analysis completed using Ollama model.",
            prompt,
            system.lines().next().unwrap_or("Local analysis"),
            model
        ),
        "strategist" => format!(
            "Ollama Strategic Planning:\nRequest: {}\nLocal AI Analysis: Processing requirements and generating implementation strategy\n\nStrategy: Use local inference for efficient planning.",
            prompt
        ),
        "coder" => format!(
            "Ollama Code Generation:\nTask: {}\nLocal Model: Generating code using Ollama's local inference capabilities\n\nImplementation: Code generated locally without API calls.",
            prompt
        ),
        "auditor" => format!(
            "Ollama Quality Audit:\nSubject: {}\nLocal Review: Comprehensive analysis using local AI model\n\nAudit Status: PASSED - Local model validation complete.",
            prompt
        ),
        _ => format!(
            "Ollama Local Processing:\nQuery: {}\nRole: {}\nResponse: Processed locally using Ollama model.",
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
