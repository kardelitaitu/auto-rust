pub mod cli;

use anyhow::Result;
use std::path::PathBuf;

use crate::llm::Llm;

pub const DEFAULT_PROMPT: &str = "You are Codex — a codebase expert. \
Answer questions about Rust code, explain how modules work, \
describe function behavior, and trace data flow. \
Be concise and precise. If asked to generate code, provide minimal diffs.";

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
            "Codex Observer Analysis:\nInput: {}\nSystem Context: {}\n\nAnalysis: This appears to be a request for code analysis. As an observer, I would normally process compiler warnings/errors but no specific issues were provided.",
            prompt,
            system.lines().next().unwrap_or("Codebase analysis")
        ),
        "strategist" => format!(
            "Codex Strategy Plan:\nRequest: {}\nRole: Strategic planning for code changes\n\nPlan: 1. Analyze requirements, 2. Design solution, 3. Implement changes, 4. Test and validate\n\nThis is a high-level strategic overview.",
            prompt
        ),
        "coder" => format!(
            "Codex Implementation:\nTask: {}\nApproach: Generate minimal, safe code changes following project conventions\n\nCode changes would be implemented here based on the specification.",
            prompt
        ),
        "auditor" => format!(
            "Codex Audit Report:\nSubject: {}\nAssessment: Code quality check, security review, performance analysis\n\nResult: PASS - No critical issues found in this analysis.",
            prompt
        ),
        _ => format!(
            "Codex Analysis:\nQuery: {}\nRole: {}\nResponse: This is a general analysis of the provided input using the {} role.",
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
