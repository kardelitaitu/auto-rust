//! bacon-review — Auditor stage only.
//!
//! Reads a spec package and the implemented patch, then audits PASS/FAIL.
//! Requires --spec-path pointing to an active spec directory.
//! On PASS, promotes the spec to _done/. On FAIL, marks needs-human-approval.
//!
//! Usage:
//!   bacon-review --spec-path <PATH> [--dry-run] [--auto]

use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

use auto::bacon_agent_nvidia::auditor;
use auto::bacon_core::cli_types::RunArgs;
use auto::bacon_core::spec_io;
use auto::bacon_core::{validate_bacon_local_only, PipelineCtx};

#[derive(Parser, Debug)]
#[command(
    name = "bacon-review",
    about = "Audit an implemented spec (Auditor stage)",
    long_about = "Reads a spec package and its patch, runs the LLM Auditor to \
                  PASS/FAIL the implementation, and updates the spec status."
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
}

#[tokio::main]
async fn main() -> Result<()> {
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
        auto_apply: false,
        parallel: false,
        max_attempts: None,
    };

    let llm = auto::llm::Llm::new()?;
    let mut ctx = PipelineCtx::new(String::new()).with_dry_run(cli.dry_run);
    ctx.spec_path = Some(spec_path.clone());

    // Try to find a patch: check for approved patches dir, then git diff
    // Prefer the most recent approved patch for this spec
    let patch = find_spec_patch(&spec_path);
    if let Some(ref patch_path) = patch {
        info!("Using patch: {}", patch_path.display());
        ctx.patch_path = Some(patch_path.clone());
    } else {
        info!("No approved patch found — Auditor will use git diff");
    }

    // Auditor
    info!("=== Auditor stage ===");
    let _ctx = auditor::run(&llm, &run_args, &ctx).await?;

    // Read the updated spec meta to check status
    let meta = spec_io::read_spec_meta(&spec_path)?;
    let status = if meta.status == "implemented" || meta.status == "done" {
        "pass"
    } else if meta.status == "needs-human-approval" {
        "fail"
    } else {
        &meta.status
    };

    let output = serde_json::json!({
        "status": status,
        "description": format!("Auditor verdict: {}", status),
        "spec_path": spec_path.as_os_str(),
    });
    println!("{}", serde_json::to_string(&output)?);

    Ok(())
}

/// Find the most recent approved patch for a spec.
/// Looks in `.bacon/sessions/approved_patches/` for patches matching the spec dir name.
fn find_spec_patch(spec_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let spec_name = spec_path.file_name()?.to_string_lossy().to_string();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let patches_dir = root
        .join(".bacon")
        .join("sessions")
        .join("approved_patches");

    if !patches_dir.is_dir() {
        return None;
    }

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&patches_dir)
        .ok()?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(&spec_name))
        })
        .map(|e| e.path())
        .collect();

    // Most recent first
    candidates.sort_by_key(|p| std::fs::metadata(p).ok().and_then(|m| m.modified().ok()));
    candidates.reverse();
    candidates.into_iter().next()
}
