use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn, LevelFilter};
use rust_orchestrator::session::Session;
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

    let browser_filters = cli::parse_browser_filters(args.browsers.as_deref());
    let groups = cli::parse_task_groups(&args.tasks);

    if !groups.is_empty() {
        cli::validate_task_groups_strict(&groups)?;
    }

    let mut sessions = browser::discover_browsers_with_filters(&config, &browser_filters).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        // Wait for shutdown signal in idle mode
        wait_for_shutdown(shutdown_tx.subscribe()).await;
        info!("Shutdown signal received. Closing browsers...");
        shutdown_sessions(&mut sessions).await;
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
    let group_index =
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

    if let Err(e) = metrics.export_summary(sessions.len(), healthy_sessions) {
        warn!("Failed to export run summary: {e}");
    }

    shutdown_sessions(&mut sessions).await;

    info!(
        "Tasks done (completed {}/{} groups) - browsers closed",
        group_index,
        groups.len()
    );
    Ok(())
}

/// Wait for shutdown signal
async fn wait_for_shutdown(mut shutdown_rx: broadcast::Receiver<()>) {
    let _ = shutdown_rx.recv().await;
}

#[async_trait(?Send)]
trait TaskGroupRunner {
    async fn run_group(&mut self, index: usize, group: &[cli::TaskDefinition]);
}

struct RuntimeGroupRunner<'a> {
    orchestrator: &'a mut orchestrator::Orchestrator,
    sessions: &'a [Session],
    metrics: Arc<metrics::MetricsCollector>,
    total_groups: usize,
}

#[async_trait(?Send)]
impl TaskGroupRunner for RuntimeGroupRunner<'_> {
    async fn run_group(&mut self, index: usize, group: &[cli::TaskDefinition]) {
        let task_groups_display = cli::format_task_groups(&[group.to_vec()]);
        info!(
            "Executing group {}/{}: {task_groups_display}",
            index + 1,
            self.total_groups
        );

        let result = self
            .orchestrator
            .execute_group(group, self.sessions, self.metrics.clone())
            .await;
        if let Err(e) = result {
            warn!("Group {} failed: {}", index + 1, e);
        }
    }
}

async fn execute_task_groups_with_shutdown<R>(
    groups: &[Vec<cli::TaskDefinition>],
    shutdown_rx: &mut broadcast::Receiver<()>,
    runner: &mut R,
) -> usize
where
    R: TaskGroupRunner + ?Sized,
{
    let mut group_index = 0;

    for (i, group) in groups.iter().enumerate() {
        if shutdown_rx.try_recv().is_ok() {
            info!("Shutdown requested, stopping before group {}", i + 1);
            break;
        }

        group_index = i + 1;
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested, stopping during group {}", i + 1);
                break;
            }
            _ = runner.run_group(i, group) => {}
        }
    }

    group_index
}

#[async_trait]
trait ShutdownSession {
    async fn shutdown(&mut self) -> anyhow::Result<()>;
}

#[async_trait]
impl ShutdownSession for Session {
    async fn shutdown(&mut self) -> anyhow::Result<()> {
        Session::graceful_shutdown(self).await
    }
}

async fn shutdown_sessions<T: ShutdownSession>(sessions: &mut [T]) {
    for session in sessions.iter_mut() {
        if let Err(e) = session.shutdown().await {
            warn!("Failed to shut down session: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::oneshot;

    struct MockShutdownSession {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    #[async_trait]
    impl ShutdownSession for MockShutdownSession {
        async fn shutdown(&mut self) -> anyhow::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                anyhow::bail!("shutdown failed");
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_shutdown_sessions_calls_all_entries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut sessions = vec![
            MockShutdownSession {
                calls: calls.clone(),
                fail: false,
            },
            MockShutdownSession {
                calls: calls.clone(),
                fail: true,
            },
            MockShutdownSession {
                calls: calls.clone(),
                fail: false,
            },
        ];

        shutdown_sessions(&mut sessions).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    struct MockTaskGroupRunner {
        started: Option<oneshot::Sender<()>>,
        finished: Arc<AtomicUsize>,
        delay_ms: u64,
    }

    #[async_trait(?Send)]
    impl TaskGroupRunner for MockTaskGroupRunner {
        async fn run_group(&mut self, _index: usize, _group: &[cli::TaskDefinition]) {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
            }
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            self.finished.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_normal_completion() {
        let groups = vec![
            vec![cli::TaskDefinition {
                name: "cookiebot".to_string(),
                payload: Default::default(),
            }],
            vec![cli::TaskDefinition {
                name: "pageview".to_string(),
                payload: Default::default(),
            }],
        ];

        let (_tx, mut rx) = broadcast::channel::<()>(1);
        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: None,
            finished: finished.clone(),
            delay_ms: 1,
        };
        let completed = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        assert_eq!(completed, 2);
        assert_eq!(finished.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_execute_task_groups_with_shutdown_ctrl_c_during_group() {
        let groups = vec![
            vec![cli::TaskDefinition {
                name: "cookiebot".to_string(),
                payload: Default::default(),
            }],
            vec![cli::TaskDefinition {
                name: "pageview".to_string(),
                payload: Default::default(),
            }],
        ];

        let (tx, mut rx) = broadcast::channel::<()>(1);
        let (started_tx, started_rx) = oneshot::channel::<()>();
        let finished = Arc::new(AtomicUsize::new(0));
        let mut runner = MockTaskGroupRunner {
            started: Some(started_tx),
            finished: finished.clone(),
            delay_ms: 250,
        };

        let sender_task = tokio::spawn(async move {
            let _ = started_rx.await;
            let _ = tx.send(());
        });

        let completed = execute_task_groups_with_shutdown(&groups, &mut rx, &mut runner).await;

        sender_task.await.expect("sender task should finish");
        assert_eq!(completed, 1);
        assert_eq!(finished.load(Ordering::SeqCst), 0);
    }
}
