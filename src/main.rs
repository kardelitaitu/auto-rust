use anyhow::Result;
use auto::runtime::execution::{execute_task_groups_with_shutdown, RuntimeGroupRunner};
use auto::session::cleanup::cleanup_managed_tabs;
use auto::{browser, cli, config, health_logger, logger, metrics, orchestrator};
use log::{info, warn, LevelFilter};
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

    let browser_filters = cli::parse_browser_filters(args.browsers.as_deref());
    let groups = cli::parse_task_groups(&args.tasks);

    if !groups.is_empty() {
        cli::validate_task_groups_strict(&groups)?;
    }

    let sessions = browser::discover_browsers_with_filters(&config, &browser_filters).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        // Wait for shutdown signal in idle mode
        wait_for_shutdown(shutdown_tx.subscribe()).await;
        info!("Shutdown signal received. Script stopped, browser left unchanged.");
        return Ok(());
    }

    let mut orchestrator = orchestrator::Orchestrator::new(config);
    let metrics = Arc::new(metrics::MetricsCollector::new(1000));

    // Start periodic health logger
    let health_config = health_logger::HealthLoggerConfig::default();
    let health_logger = health_logger::HealthLogger::new(health_config, metrics.clone());
    let _health_handle = health_logger.start();

    // Execute task groups with shutdown awareness
    let mut shutdown_rx = shutdown_tx.subscribe();
    let mut group_runner = RuntimeGroupRunner {
        orchestrator: &mut orchestrator,
        sessions: &sessions,
        metrics: metrics.clone(),
        total_groups: groups.len(),
    };
    let group_outcome =
        execute_task_groups_with_shutdown(&groups, &mut shutdown_rx, &mut group_runner).await;

    // Stop health logger and wait for it to finish
    health_logger.stop();
    let _ = _health_handle.await;

    let healthy_sessions = sessions
        .iter()
        .filter(|session| session.is_healthy())
        .count();
    if !sessions.is_empty() && healthy_sessions * 100 < sessions.len() * 80 {
        warn!(
            "Session health degraded: {}/{} healthy sessions remaining",
            healthy_sessions,
            sessions.len()
        );
    }

    // Calculate fan-out metrics for Phase 4
    let planned_groups = groups.len();
    let completed_groups = group_outcome.completed_groups;
    let planned_executions = groups.iter().map(|g| g.len()).sum::<usize>() * sessions.len();
    let actual_executions = metrics.get_stats().total_tasks;

    if let Err(e) = metrics.export_summary_to(
        "run-summary.json",
        sessions.len(),
        healthy_sessions,
        planned_groups,
        completed_groups,
        planned_executions,
        actual_executions,
    ) {
        warn!("Failed to export run summary: {e}");
    }

    if group_outcome.shutdown_requested {
        info!(
            "Tasks stopped by shutdown signal (completed {}/{}) - script stopped, browser left unchanged",
            group_outcome.completed_groups,
            groups.len()
        );
        return Ok(());
    }

    cleanup_managed_tabs(&sessions).await;

    info!(
        "Tasks done (completed {}/{} groups) - browser kept open",
        group_outcome.completed_groups,
        groups.len()
    );
    Ok(())
}

/// Wait for shutdown signal
async fn wait_for_shutdown(mut shutdown_rx: broadcast::Receiver<()>) {
    let _ = shutdown_rx.recv().await;
}
