//! NVIDIA AI agent for Bacon pipeline
//! 
//! This agent calls the NVIDIA AI API endpoint:
//! - Base URL: https://integrate.api.nvidia.com/v1
//! - API Key: nvapi-* format
//! - Models: minimaxai/minimax-m2.7 and others

use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use auto_rust::bacon_core::read_role_prompt;

fn load_env_file() {
    let env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "nvidia",
    about = "NVIDIA AI agent for Bacon pipeline",
    version
)]
struct Args {
    /// User prompt describing the task
    #[arg(short, long)]
    prompt: String,

    /// Pipeline stage to run
    #[arg(long)]
    role: String,

    /// Sandbox mode (no writes)
    #[arg(long)]
    dry_run: bool,

    /// API key for NVIDIA
    #[arg(long)]
    api_key: Option<String>,

    /// Base URL for NVIDIA API
    #[arg(long, default_value = "https://integrate.api.nvidia.com/v1")]
    base_url: String,

    /// Model to use
    #[arg(long, default_value = "minimaxai/minimax-m2.7")]
    model: String,

    /// Temperature for generation
    #[arg(long, default_value = "1.0")]
    temperature: f32,

    /// Top-p for generation
    #[arg(long, default_value = "0.95")]
    top_p: f32,

    /// Max tokens to generate
    #[arg(long, default_value = "8192")]
    max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerOutput {
    status: Option<String>,
    description: Option<String>,
    summary: Option<String>,
    spec_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    top_p: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

impl Default for WorkerOutput {
    fn default() -> Self {
        Self {
            status: Some("ok".to_string()),
            description: None,
            summary: None,
            spec_path: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    load_env_file();

    let args = Args::parse();

    // Get API key from args or environment
    const PLACEHOLDER_KEY: &str = "nvapi-placeholder-key";

    let api_key = if let Some(key) = args.api_key {
        key
    } else {
        std::env::var("NVIDIA_API_KEY").unwrap_or_else(|_| {
            eprintln!("Warning: No NVIDIA API key provided. Using placeholder.");
            PLACEHOLDER_KEY.to_string()
        })
    };

    // Fail fast if no real API key is available in live mode
    if !args.dry_run && api_key == PLACEHOLDER_KEY {
        anyhow::bail!(
            "NVIDIA_API_KEY is not set. Set the NVIDIA_API_KEY environment variable or pass --api-key. "
        );
    }

    // Dry-run mode: produce mock output without calling API
    if args.dry_run {
        let response = match args.role.as_str() {
            "observer" => format!(
                "## NVIDIA Observer Analysis\n\nScanning codebase with NVIDIA AI for patterns, issues, and improvement opportunities.\n\n**Prompt**: {}\n**Model**: {}\n\nDry-run: Analysis would normally process compiler warnings and errors.",
                args.prompt, args.model
            ),
            "strategist" => format!(
                "## NVIDIA Strategic Planning\n\n1. Analyze requirements from: {}\n2. Design solution approach\n3. Plan implementation steps\n4. Define validation criteria\n\n**Model**: {}",
                args.prompt, args.model
            ),
            "coder" => format!(
                "## NVIDIA Code Generation\n\nGenerating minimal, safe code changes for: {}\n\n**Model**: {}\n\nDry-run: Changes would be implemented following project standards.",
                args.prompt, args.model
            ),
            "auditor" => format!(
                "## NVIDIA Quality Audit\n\n**PASS** - Code quality and security assessment for: {}\n\n**Model**: {}\n\nReview: Correctness, security, performance, and maintainability checks passed.",
                args.prompt, args.model
            ),
            _ => format!("NVIDIA agent processing role: {} with prompt: {}", args.role, args.prompt),
        };

        let output = WorkerOutput {
            description: Some(response),
            ..WorkerOutput::default()
        };
        let json = serde_json::to_string_pretty(&output)?;
        println!("{}", json);
        return Ok(());
    }

    // Create role-specific system prompt from .bacon/roles/ files
    let system_prompt = read_role_prompt(&args.role);

    // Prepare messages for NVIDIA API
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
            ..Default::default()
        },
        ChatMessage {
            role: "user".to_string(),
            content: Some(args.prompt.clone()),
            ..Default::default()
        },
    ];

    // Call NVIDIA API
    let response = call_nvidia_api(
        &api_key,
        &args.base_url,
        &args.model,
        messages,
        args.temperature,
        args.top_p,
        args.max_tokens,
    )
    .await
    .context("Failed to call NVIDIA API")?

    // Create output
    let output = WorkerOutput {
        description: Some(response),
        ..WorkerOutput::default()
    };

    // Output JSON as required by Bacon contract
    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(())
}

async fn call_nvidia_api(
    api_key: &str,
    base_url: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: f32,
    top_p: f32,
    max_tokens: u32,
) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/chat/completions", base_url);

    eprintln!("[nvidia] POST {} ({} messages, {} max_tokens)", url, messages.len(), max_tokens);

    // Create request
    let request = ChatRequest {
        model: model.to_string(),
        messages,
        temperature,
        top_p,
        max_tokens,
        stream: false,
    };

    // Send request
    eprintln!("[nvidia] Waiting for response...");
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&request)
        .send()
        .await
        .context("Failed to send request to NVIDIA API")?;
    eprintln!("[nvidia] HTTP {}", response.status());

    // Check status
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("NVIDIA API error: {} - {}", status, text);
    }

    // Read streaming response
    let text = response.text().await.context("Failed to read response text")?;
    let mut content = String::new();
    for line in text.lines() {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                break;
            }
            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(choice) = chunk.get("choices").and_then(|c| c.as_array()).and_then(|c| c.first()) {
                    if let Some(delta) = choice.get("delta") {
                        if let Some(c) = delta.get("content").and_then(|c| c.as_str()) {
                            content.push_str(c);
                        }
                        if let Some(r) = delta.get("reasoning_content").and_then(|r| r.as_str()) {
                            // Optionally include reasoning, but for now skip
                        }
                    }
                    if choice.get("finish_reason").is_some() {
                        break;
                    }
                }
            }
        }
    }

    if content.is_empty() {
        let reason = chat_response
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
            .unwrap_or("unknown");
        anyhow::bail!("Empty response from NVIDIA API (finish_reason: {})", reason);
    }

    eprintln!("[nvidia] Response received ({} chars)", content.len());
    Ok(content)
}