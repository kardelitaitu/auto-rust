use crate::config::Config;
use crate::session::Session;
use crate::cli::TaskDefinition;
use anyhow::{Result, bail};
use log::{info, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

pub struct Orchestrator {
    config: Config,
    global_active_tasks: Arc<AtomicUsize>,
    global_semaphore: Arc<Semaphore>,
}

impl Orchestrator {
    pub fn new(config: Config) -> Self {
        Self {
            global_active_tasks: Arc::new(AtomicUsize::new(0)),
            global_semaphore: Arc::new(Semaphore::new(config.orchestrator.max_global_concurrency)),
            config,
        }
    }

    /// Execute a group of tasks across all sessions
    /// Tasks within a group run in parallel
    pub async fn execute_group(
        &mut self,
        group: &[TaskDefinition],
        sessions: &[Session],
    ) -> Result<()> {
        if sessions.is_empty() {
            bail!("No active sessions available");
        }

        if group.is_empty() {
            warn!("Empty task group, skipping");
            return Ok(());
        }

        let group_start = std::time::Instant::now();
        info!(
            "Executing group with {} task(s) across {} session(s)",
            group.len(),
            sessions.len()
        );

        // Apply group timeout
        let group_timeout = Duration::from_millis(self.config.orchestrator.group_timeout_ms);

        let task_futures: Vec<_> = group
            .iter()
            .cloned()
            .map(|task_def| {
                let sessions = sessions;
                let global_active = self.global_active_tasks.clone();
                let global_sem = self.global_semaphore.clone();
                let config = self.config.clone();

                async move {
                    // Global concurrency throttling
                    let _permit = global_sem.acquire().await?;
                    global_active.fetch_add(1, Ordering::SeqCst);

                    // Stagger task starts to prevent network spikes
                    tokio::time::sleep(Duration::from_millis(
                        config.orchestrator.task_stagger_delay_ms
                    ))
                    .await;

                    // Find an available session and execute the task
                    let result = execute_task_on_session(
                        &task_def,
                        &sessions,
                        &config,
                    )
                    .await;

                    global_active.fetch_sub(1, Ordering::SeqCst);
                    result
                }
            })
            .collect();

        // Execute all tasks in parallel within the group
        let results = timeout(group_timeout, async {
            futures::future::join_all(task_futures).await
        })
        .await;

        match results {
            Ok(results) => {
                let success_count = results.iter().filter(|r| r.is_ok()).count();
                let fail_count = results.len() - success_count;

                info!(
                    "Group complete: {} succeeded, {} failed ({}s)",
                    success_count,
                    fail_count,
                    group_start.elapsed().as_secs_f64()
                );

                if fail_count > 0 {
                    warn!("{} task(s) failed in group", fail_count);
                }

                Ok(())
            }
            Err(_) => {
                bail!(
                    "Group timeout exceeded ({}ms)",
                    self.config.orchestrator.group_timeout_ms
                );
            }
        }
    }
}

/// Execute a single task on ALL sessions in parallel
/// Wait for ALL sessions to complete (each runs independently)
async fn execute_task_on_session(
    task_def: &TaskDefinition,
    sessions: &[Session],
    config: &Config,
) -> Result<()> {
    if sessions.is_empty() {
        bail!("No sessions available");
    }

    info!(
        "[{}] Starting task on {} sessions",
        task_def.name,
        sessions.len()
    );

    // Create parallel tasks for each session
    let session_futures: Vec<_> = sessions
        .iter()
        .map(|session| {
            let task_def = task_def.clone();
            let config = config.clone();
            async move {
                let result = execute_task_with_retry(&task_def, &session, &config).await;
                (session.id.clone(), result)
            }
        })
        .collect();

    // Run ALL sessions in parallel and wait for ALL to complete
    let results = futures::future::join_all(session_futures).await;

    let mut success_count = 0;
    let mut failed_sessions = Vec::new();

    for (session_id, result) in results {
        match result {
            Ok(_) => {
                info!(
                    "[{}][{}] Completed",
                    session_id, task_def.name
                );
                success_count += 1;
            }
            Err(e) => {
                warn!(
                    "[{}][{}] Failed: {}",
                    session_id, task_def.name, e
                );
                failed_sessions.push(session_id);
            }
        }
    }

    if failed_sessions.is_empty() {
        info!(
            "[{}] All {} sessions completed successfully",
            task_def.name,
            sessions.len()
        );
        Ok(())
    } else {
        warn!(
            "[{}] {}/{} sessions failed: {}",
            task_def.name,
            failed_sessions.len(),
            sessions.len(),
            failed_sessions.join(", ")
        );
        // Return Ok if at least one session succeeded
        if success_count > 0 {
            Ok(())
        } else {
            bail!(
                "Task {} failed on all {} sessions",
                task_def.name,
                sessions.len()
            )
        }
    }
}

/// Execute a task with timeout and retry logic
async fn execute_task_with_retry(
    task_def: &TaskDefinition,
    session: &Session,
    config: &Config,
) -> Result<()> {
    let max_retries = config.orchestrator.max_retries;
    let task_timeout = Duration::from_millis(config.orchestrator.task_timeout_ms);
    let retry_delay = Duration::from_millis(config.orchestrator.retry_delay_ms);

    // Acquire worker permit
    let permit = session
        .acquire_worker(config.orchestrator.worker_wait_timeout_ms)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to acquire worker"))?;

    // Check session health
    if !session.is_healthy().await {
        session.mark_unhealthy();
        bail!("Session {} is unhealthy, skipping task", session.id);
    }

    // Acquire page
    let page = session.acquire_page().await?;

    let payload_json = serde_json::Value::Object(
        task_def
            .payload
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect(),
    );

    info!("[{}][{}] Executing task (timeout: {}ms, retries: {})...", 
        session.id, task_def.name, config.orchestrator.task_timeout_ms, max_retries);
    
    let result = tokio::time::timeout(task_timeout, 
        crate::task::perform_task(&page, &session.id, &task_def.name, payload_json, max_retries)
    ).await;

    // Release page
    session.release_page(page).await;

    // Drop permit (releases worker)
    drop(permit);

    match result {
        Ok(Ok(task_result)) => {
            if task_result.is_success() {
                session.mark_healthy();
                Ok(())
            } else {
                let error = task_result.last_error.clone().unwrap_or_else(|| "Unknown error".to_string());
                session.increment_failure();
                Err(anyhow::anyhow!("Task {} failed: {}", task_def.name, error))
            }
        }
        Ok(Err(e)) => {
            session.increment_failure();
            Err(anyhow::anyhow!("Task {} error: {}", task_def.name, e))
        }
        Err(_) => {
            session.increment_failure();
            session.mark_unhealthy();
            Err(anyhow::anyhow!("Task {} timed out after {}ms", task_def.name, config.orchestrator.task_timeout_ms))
        }
    }
}