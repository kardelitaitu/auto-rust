mod cli;
mod config;
mod browser;
mod orchestrator;
mod session;

#[path = "../task/mod.rs"]
mod task;

mod utils;
mod logger;

use anyhow::Result;
use log::{info, LevelFilter};

fn main() {
    // Set current directory to project root
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(target) = exe_path.parent() {
            let target_str = target.to_string_lossy();
            // In dev mode: target/debug/rust-orchestrator.exe -> go to project root
            // In release mode: (some path)/rust-orchestrator.exe -> stay where we are
            if target_str.contains("target\\debug") || target_str.contains("target/release") {
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
        eprintln!("Error: {}", e);
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
    // Initialize file logger
    let logger = logger::FileLogger::new("log")?;
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Info);

    info!("Rust Orchestrator - Starting up...");

    // Parse CLI
    let args = cli::parse_args();

    // Load configuration
    let config = config::load_config()?;

    // Discover and connect to browsers
    let sessions = browser::discover_browsers(&config).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        return Ok(());
    }

    // Parse tasks into groups (separated by "then")
    let groups = cli::parse_task_groups(&args.tasks)?;
    let task_groups_display = cli::format_task_groups(&groups);
    info!("Processing {}", task_groups_display);

    // Create orchestrator and run tasks
    let mut orchestrator = orchestrator::Orchestrator::new(config);

    for (i, group) in groups.iter().enumerate() {
        info!("Executing group {}/{}", i + 1, groups.len());
        orchestrator.execute_group(group, &sessions).await?;
        info!("Group {} complete", i + 1);
    }

    info!("All tasks completed successfully");
    Ok(())
}
