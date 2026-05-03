use anyhow::Result;
use auto::runtime::execution::{execute_task_groups_with_shutdown, RuntimeGroupRunner};
use auto::session::cleanup::cleanup_managed_tabs;
use auto::{browser, cli, config, health_logger, logger, metrics, orchestrator};
use log::{info, warn, LevelFilter};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Detect and set the appropriate working directory based on executable location.
/// When running from target/debug or target/release, changes to project root.
/// Otherwise, changes to the executable's directory.
pub fn setup_working_directory() -> Result<(), std::io::Error> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(target) = exe_path.parent() {
            let target_str = target.to_string_lossy();
            if target_str.contains("target\\debug") || target_str.contains("target\\release") {
                if let Some(root) = target.parent() {
                    if let Some(project_root) = root.parent() {
                        std::env::set_current_dir(project_root)?;
                    }
                }
            } else {
                std::env::set_current_dir(target)?;
            }
        }
    }
    Ok(())
}

/// Calculate whether session health is degraded based on healthy vs total sessions.
/// Returns true if less than 80% of sessions are healthy.
pub fn is_session_health_degraded(healthy: usize, total: usize) -> bool {
    if total == 0 {
        return false;
    }
    healthy * 100 < total * 80
}

/// Format a health warning message for degraded sessions.
pub fn format_health_warning(healthy: usize, total: usize) -> String {
    format!(
        "Session health degraded: {}/{} healthy sessions remaining",
        healthy, total
    )
}

fn main() {
    if let Err(e) = setup_working_directory() {
        eprintln!("Warning: Failed to set working directory: {e}");
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

/// Run in dry-run mode: show what would be executed without actually running.
///
/// Validates all tasks and prints execution plan without connecting to browsers.
async fn run_dry_run(groups: &[Vec<cli::TaskDefinition>], config: &config::Config) -> Result<()> {
    use auto::task::registry::TaskRegistry;

    println!("=== DRY RUN MODE ===");
    println!("No tasks will be executed. Showing execution plan only.\n");

    // Build registry with external tasks if discovery is enabled
    let mut registry = TaskRegistry::with_built_in_tasks();
    let external_loaded = registry.load_external_tasks(&config.task_discovery);

    if external_loaded > 0 {
        println!("External tasks loaded: {}", external_loaded);
    }

    // Show registry diagnostics
    let diag = registry.diagnostics();
    println!("\nRegistry state:");
    println!("  Total tasks: {}", diag.total_tasks);
    println!("  Built-in tasks: {}", diag.built_in_tasks);
    println!("  External tasks: {}", diag.external_tasks);

    // Validate and show each task group
    println!("\nExecution plan:");
    if groups.is_empty() {
        println!("  No task groups specified.");
        return Ok(());
    }

    for (group_idx, group) in groups.iter().enumerate() {
        println!("\n  Group {} ({} tasks):", group_idx + 1, group.len());

        for (task_idx, task_def) in group.iter().enumerate() {
            let normalized_name = auto::task::normalize_task_name(&task_def.name);

            match registry.lookup(normalized_name) {
                Ok(descriptor) => {
                    let source_info = match &descriptor.source {
                        auto::task::registry::TaskSource::BuiltInRust => "BuiltInRust".to_string(),
                        auto::task::registry::TaskSource::ConfiguredPath(path) => {
                            format!("ConfiguredPath({})", path.display())
                        }
                        auto::task::registry::TaskSource::Unknown => "Unknown".to_string(),
                    };

                    println!(
                        "    {}. {} [{}] (policy: {})",
                        task_idx + 1,
                        task_def.name,
                        source_info,
                        descriptor.policy_name
                    );

                    // Show payload if present
                    if !task_def.payload.is_empty() {
                        let payload_str = task_def
                            .payload
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        println!("       payload: {}", payload_str);
                    }
                }
                Err(e) => {
                    println!("    {}. {} [UNKNOWN - {}]", task_idx + 1, task_def.name, e);
                }
            }
        }
    }

    // Summary
    let total_tasks: usize = groups.iter().map(|g| g.len()).sum();
    let total_groups = groups.len();

    println!("\n=== SUMMARY ===");
    println!("Task groups: {}", total_groups);
    println!("Total task executions: {}", total_tasks);
    println!("\nDry run complete. No tasks were executed.");

    Ok(())
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

    // Handle --list-tasks flag
    if args.list_tasks {
        use auto::task::registry::format_task_list;
        print!("{}", format_task_list());
        return Ok(());
    }

    let config = config::load_config()?;
    config::validate_config(&config)?;

    let browser_filters = cli::parse_browser_filters(args.browsers.as_deref());
    let groups = cli::parse_task_groups(&args.tasks);

    if !groups.is_empty() {
        cli::validate_task_groups_strict(&groups)?;
    }

    // Handle --dry-run flag
    if args.dry_run {
        return run_dry_run(&groups, &config).await;
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
    let total_sessions = sessions.len();
    if !sessions.is_empty() && is_session_health_degraded(healthy_sessions, total_sessions) {
        warn!(
            "{}",
            format_health_warning(healthy_sessions, total_sessions)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_health_degraded_empty_sessions() {
        // Empty sessions should not be considered degraded
        assert!(!is_session_health_degraded(0, 0));
    }

    #[test]
    fn test_session_health_degraded_all_healthy() {
        // All healthy sessions - not degraded
        assert!(!is_session_health_degraded(5, 5));
        assert!(!is_session_health_degraded(1, 1));
        assert!(!is_session_health_degraded(10, 10));
    }

    #[test]
    fn test_session_health_degraded_threshold_80_percent() {
        // Exactly 80% healthy - not degraded (threshold is < 80%)
        assert!(!is_session_health_degraded(4, 5)); // 80%
        assert!(!is_session_health_degraded(8, 10)); // 80%
    }

    #[test]
    fn test_session_health_degraded_below_threshold() {
        // Below 80% healthy - degraded
        assert!(is_session_health_degraded(3, 5)); // 60%
        assert!(is_session_health_degraded(7, 10)); // 70%
        assert!(is_session_health_degraded(0, 5)); // 0%
        assert!(is_session_health_degraded(1, 2)); // 50%
    }

    #[test]
    fn test_session_health_degraded_edge_cases() {
        // Edge cases near threshold
        assert!(is_session_health_degraded(79, 100)); // 79% - just below
        assert!(!is_session_health_degraded(80, 100)); // 80% - at threshold
        assert!(!is_session_health_degraded(81, 100)); // 81% - above threshold
    }

    #[test]
    fn test_format_health_warning() {
        let warning = format_health_warning(3, 5);
        assert!(warning.contains("3/5"));
        assert!(warning.contains("Session health degraded"));
        assert!(warning.contains("healthy sessions remaining"));
    }

    #[test]
    fn test_format_health_warning_zero_healthy() {
        let warning = format_health_warning(0, 5);
        assert!(warning.contains("0/5"));
        assert!(warning.contains("Session health degraded"));
    }

    #[test]
    fn test_format_health_warning_single_session() {
        let warning = format_health_warning(0, 1);
        assert!(warning.contains("0/1"));
    }
}
