//! Simple NVIDIA AI agent for Bacon pipeline
//!
//! This agent calls the NVIDIA AI API endpoint using reqwest directly.

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

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
#[command(name = "nvidia", about = "NVIDIA AI agent for Bacon pipeline", version)]
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
    #[arg(long, default_value = "meta/llama-3.3-70b-instruct")]
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

#[tokio::main]
async fn main() -> Result<()> {
    load_env_file();

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let args = Args::parse();

    // Get API key from args or environment
    let api_key = if let Some(key) = args.api_key {
        key
    } else {
        std::env::var("NVIDIA_API_KEY").unwrap_or_else(|_| {
            eprintln!("Warning: No NVIDIA API key provided. Using placeholder.");
            "nvapi-placeholder-key".to_string()
        })
    };

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

    info!(
        "Starting NVIDIA {} role with model: {}",
        args.role, args.model
    );
    debug!(
        "Base URL: {}, temperature: {}, max_tokens: {}",
        args.base_url, args.temperature, args.max_tokens
    );
    debug!("Prompt length: {} chars", args.prompt.len());

    // Generic system prompt — role instructions come from the pipeline via user prompt
    let system_prompt = "You are an AI assistant in the Bacon autonomous coding pipeline. Follow the role instructions in the user's message carefully. Do NOT use tools or function calls. Output only plain text.";

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

    eprintln!("[nvidia] Calling API (model: {})...", args.model);
    let start = Instant::now();

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
    .context("Failed to call NVIDIA API")?;

    let elapsed = start.elapsed();
    info!("NVIDIA API responded in {:.1}s", elapsed.as_secs_f64());

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
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let url = format!("{}/chat/completions", base_url);

    eprintln!(
        "[nvidia] POST {} ({} messages, {} max_tokens)",
        url,
        messages.len(),
        max_tokens
    );

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
        .header("Accept", "application/json")
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

    // Parse response
    let chat_response: ChatResponse = response
        .json()
        .await
        .context("Failed to parse NVIDIA API response")?;

    // Extract content — fall back to reasoning field if content is null
    let content = chat_response
        .choices
        .first()
        .and_then(|choice| {
            choice
                .message
                .content
                .as_deref()
                .or(choice.message.reasoning.as_deref())
                .or(choice.message.reasoning_content.as_deref())
        })
        .unwrap_or_default()
        .to_string();

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
