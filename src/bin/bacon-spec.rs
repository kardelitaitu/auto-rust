#![deny(warnings)]

//! bacon-spec — Observer + Strategist stages.
//!
//! Scans the codebase (Observer) and optionally generates a spec package (Strategist).
//! Outputs the spec path as JSON `WorkerOutput` on stdout for composability.
//!
//! Usage:
//!   bacon-spec [--prompt "scan for X"] [--dry-run] [--auto] [--fast] [--max-attempts N] [--spec N]

// last audited 26-06-26 by Buffy

use anyhow::Result;
use auto::bacon_agent_nvidia::{observer, strategist};
use auto::bacon_core::cli_types::RunArgs;
use auto::bacon_core::{validate_bacon_local_only, PipelineCtx};
use clap::Parser;
use log::info;

#[derive(Parser, Debug)]
#[command(
    name = "bacon-spec",
    about = "Generate a spec package (Observer + Strategist)"
)]
struct Cli {
    #[arg(short = 'p', long, help = "Task description for the Observer")]
    prompt: Option<String>,

    #[arg(long, help = "Sandbox mode — no files modified")]
    dry_run: bool,

    #[arg(short = 'y', long, help = "Skip confirmation gates")]
    auto: bool,

    #[arg(long, help = "Skip Strategist — observer only")]
    fast: bool,

    #[arg(long, help = "Override max retry attempts per stage")]
    max_attempts: Option<u32>,

    #[arg(long, help = "Target a specific spec number")]
    spec: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize bacon-pipeline configuration
    bacon_pipeline::config::init(bacon_pipeline::ProjectConfig::with_defaults(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ));

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    validate_bacon_local_only()?;

    let run_args = RunArgs {
        prompt: cli.prompt,
        stage: None,
        spec: cli.spec,
        fast: cli.fast,
        dry_run: cli.dry_run,
        auto: cli.auto,
        auto_apply: false,
        parallel: false,
        max_attempts: cli.max_attempts,
        ci: false,
    };

    let llm = bacon_pipeline::llm::Llm::from_env()?;
    let base_ctx = PipelineCtx::new(String::new(), None, None, None).with_dry_run(cli.dry_run);

    // Observer
    info!("=== Observer stage ===");
    let ctx = observer::run(&llm, &run_args, &base_ctx).await?;
    info!("Observer confidence: {:?}", ctx.confidence);

    // Strategist (unless --fast or confidence is Low in non-auto mode)
    let ctx = if cli.fast {
        info!("--fast: skipping Strategist");
        ctx
    } else {
        info!("=== Strategist stage ===");
        strategist::run(&llm, &run_args, &ctx).await?
    };

    // Output as WorkerOutput JSON
    let status = if ctx.spec_path.is_some() {
        "ok"
    } else {
        "no-spec"
    };
    let output = serde_json::json!({
        "status": status,
        "description": ctx.description,
        "spec_path": ctx.spec_path.as_ref().map(|p| p.display().to_string()),
    });
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
