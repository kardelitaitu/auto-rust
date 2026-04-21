use anyhow::Result;
use log::{info, warn, LevelFilter};
use rust_orchestrator::{browser, cli, config, health_logger, logger, metrics, orchestrator};
use std::sync::Arc;
use tokio::sync::broadcast;

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
    rt.block_on(async { run_async().await })
}

async fn run_async() -> Result<()> {
    let logger = logger::FileLogger::new("log")?;
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Info);

    info!("Rust Orchestrator - Starting up...");

    // Create shutdown channel for graceful termination
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);
    let shutdown_tx = Arc::new(shutdown_tx);

    // Set up signal handlers for graceful shutdown
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        info!("\nReceived shutdown signal (Ctrl+C)");
        let _ = shutdown_tx_clone.send(());
    });

    let args = cli::parse_args();
    let config = config::load_config()?;
    config::validate_config(&config)?;

    let sessions = browser::discover_browsers(&config).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        // Wait for shutdown signal in idle mode
        wait_for_shutdown(shutdown_tx.subscribe()).await;
        info!("Shutting down gracefully...");
        return Ok(());
    }

    let groups = cli::parse_task_groups(&args.tasks);

    // Validate tasks and log warnings for unknown task names
    let _validation_results = cli::validate_task_groups(&groups);

    let task_groups_display = cli::format_task_groups(&groups);
    info!("Processing {task_groups_display}");

    let mut orchestrator = orchestrator::Orchestrator::new(config);
    let metrics = Arc::new(metrics::MetricsCollector::new(1000));

    // Start periodic health logger
    let health_config = health_logger::HealthLoggerConfig::default();
    let health_logger = health_logger::HealthLogger::new(health_config, metrics.clone());
    let _health_handle = health_logger.start();

    // Execute task groups with shutdown awareness
    let mut shutdown_rx = shutdown_tx.subscribe();
    let mut group_index = 0;

    for (i, group) in groups.iter().enumerate() {
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, stopping before group {}", i + 1);
            break;
        }

        info!("Executing group {}/{}", i + 1, groups.len());
        group_index = i + 1;

        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested, stopping during group {}", i + 1);
                break;
            }
            result = orchestrator.execute_group(group, &sessions, metrics.clone()) => {
                match result {
                    Ok(()) => {
                        info!("Group {} complete", i + 1);
                    }
                    Err(e) => {
                        warn!("Group {} failed: {}", i + 1, e);
                    }
                }
            }
        }
    }

    // Stop health logger and wait for it to finish
    health_logger.stop();
    let _ = _health_handle.await;

    if let Err(e) = metrics.export_summary() {
        warn!("Failed to export run summary: {e}");
    }

    // Wait for shutdown signal if tasks completed
    if shutdown_rx.try_recv().is_ok() {
        info!("Shutdown requested during execution");
    }

    info!(
        "Tasks done (completed {}/{} groups) - browser kept open",
        group_index,
        groups.len()
    );
    Ok(())
}

/// Wait for shutdown signal
async fn wait_for_shutdown(mut shutdown_rx: broadcast::Receiver<()>) {
    let _ = shutdown_rx.recv().await;
}
