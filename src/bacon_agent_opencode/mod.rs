pub mod cli;

use anyhow::Result;
use log::info;
use std::path::PathBuf;

use crate::llm::{ChatMessage, Llm};

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

pub async fn run(llm: &Llm, prompt: &str, role: Option<&str>) -> Result<String> {
    let system = system_prompt(role);
    let messages = vec![ChatMessage::system(&system), ChatMessage::user(prompt)];
    info!("Calling OpenCode...");
    let response = llm.chat(messages).await?;
    println!("{}", response);
    Ok(response)
}
