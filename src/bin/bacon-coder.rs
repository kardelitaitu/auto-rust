//! bacon-coder — Coder stage only.
//!
//! Reads an existing spec package and implements it using SEARCH/REPLACE blocks.
//! Requires --spec-path pointing to an active spec directory.
//! Outputs the patch path as JSON `WorkerOutput` on stdout.
//!
//! Usage:
//!   bacon-coder --spec-path <PATH> [--dry-run] [--auto] [--max-attempts N] [--auto-apply]

use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

use auto::bacon_agent_nvidia::coder;
use auto::bacon_core::cli_types::RunArgs;
use auto::bacon_core::{validate_bacon_local_only, PipelineCtx};

#[derive(Parser, Debug)]
#[command(
    name = "bacon-coder",
    about = "Implement a spec (Coder stage)",
    long_about = "Reads an approved spec package and applies SEARCH/REPLACE changes. \
                  Requires --spec-path pointing to a spec directory under docs/specs/_active/."
)]
struct Cli {
    #[arg(
        long = "spec-path",
        required = true,
        help = "Path to the spec directory (e.g. docs/specs/_active/0001-add-foo)"
    )]
    spec_path: PathBuf,

    #[arg(long, help = "Sandbox mode — no files modified")]
    dry_run: bool,

    #[arg(short = 'y', long, help = "Skip confirmation gates")]
    auto: bool,

    #[arg(long, help = "Override max SEARCH/REPLACE retry attempts (default: 4)")]
    max_attempts: Option<u32>,

    #[arg(long, help = "Auto-apply verified patches after gating")]
    auto_apply: bool,

    #[arg(long, help = "Emit GitHub Actions-compatible annotations")]
    ci: bool,
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

    // Resolve spec path relative to manifest dir if not absolute
    let spec_path = if cli.spec_path.is_absolute() {
        cli.spec_path.clone()
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.join(&cli.spec_path)
    };
    if !spec_path.join("spec.yaml").exists() {
        anyhow::bail!("spec.yaml not found at {}", spec_path.display());
    }

    let run_args = RunArgs {
        prompt: None,
        stage: None,
        spec: None,
        fast: false,
        dry_run: cli.dry_run,
        auto: cli.auto,
        auto_apply: cli.auto_apply,
        parallel: false,
        max_attempts: cli.max_attempts,
        ci: cli.ci,
    };

    let llm = bacon_pipeline::llm::Llm::from_env()?;
    let mut ctx = PipelineCtx::new(String::new(), None, None, None).with_dry_run(cli.dry_run);
    ctx.spec_path = Some(spec_path);

    // Coder
    info!("=== Coder stage ===");
    let ctx = coder::run(&llm, &run_args, &ctx).await?;

    // Determine result status
    let (status, description) = if ctx.coder_refused {
        ("coder-refused", "Coder refused to implement".to_string())
    } else if ctx.needs_human_approval {
        (
            "needs-human-approval",
            "Patch needs human approval".to_string(),
        )
    } else if ctx.scope_reduction_needed {
        ("retries-exhausted", "Coder retries exhausted".to_string())
    } else if let Some(ref patch_path) = ctx.patch_path {
        ("ok", format!("Patch saved to {}", patch_path.display()))
    } else if ctx.dry_run {
        ("dry-run", ctx.description.clone())
    } else {
        ("unknown", "Coder completed with no patch".to_string())
    };

    // Output as WorkerOutput JSON
    let output = serde_json::json!({
        "status": status,
        "description": description,
        "spec_path": ctx.spec_path.as_ref().map(|p| p.display().to_string()),
        "patch_path": ctx.patch_path.as_ref().map(|p| p.display().to_string()),
    });
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}
