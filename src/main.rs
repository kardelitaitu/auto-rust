//! # Rust Orchestrator

mod cli;
mod config;
mod browser;
mod orchestrator;
mod session;
mod result;

#[path = "../task/mod.rs"]
mod task;

mod utils;
mod logger;
mod validation;
mod api;
mod metrics;
mod tests;

use anyhow::Result;
use log::{info, warn, LevelFilter};

fn main() {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(target) = exe_path.parent() {
            let target_str = target.to_string_lossy();
            if target_str.contains("target\\debug") || target_str.contains("target\\release") {
                if let Some(root) = target.parent() {
                    if let Some(project_root) = root.parent() {
                        let _ = std::env::set_current_dir(project_root);
                    }
                }
            } else {
                let _ = std::env::set_current_dir(target);
            }
        }
    }

    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        run_async().await
    })
}

async fn run_async() -> Result<()> {
    let logger = logger::FileLogger::new("log")?;
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Info);

    info!("Rust Orchestrator - Starting up...");

    let args = cli::parse_args();
    let config = config::load_config()?;
    config::validate_config(&config)?;

    let sessions = browser::discover_browsers(&config).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        return Ok(());
    }

    let groups = cli::parse_task_groups(&args.tasks);
    let task_groups_display = cli::format_task_groups(&groups);
    info!("Processing {task_groups_display}");

    let mut orchestrator = orchestrator::Orchestrator::new(config);
    let metrics = metrics::MetricsCollector::new(1000);

    for (i, group) in groups.iter().enumerate() {
        info!("Executing group {}/{}", i + 1, groups.len());
        orchestrator.execute_group(group, &sessions).await?;
        info!("Group {} complete", i + 1);
    }

    if let Err(e) = metrics.export_summary() {
        warn!("Failed to export run summary: {e}");
    }

    // Don't close browser - keep it open for user
    info!("Tasks done - browser kept open");
    Ok(())
}