use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TaskMetrics {
    pub task_name: String,
    pub status: TaskStatus,
    pub duration_ms: u64,
    pub session_id: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskStatus {
    Success,
    Failed,
    Timeout,
}

#[allow(dead_code)]
pub struct MetricsCollector {
    total_tasks: Arc<AtomicUsize>,
    succeeded: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    timed_out: Arc<AtomicUsize>,
    total_duration_ms: Arc<AtomicUsize>,
    active_tasks: Arc<AtomicUsize>,
    task_history: Arc<RwLock<VecDeque<TaskMetrics>>>,
    max_history: usize,
}

impl MetricsCollector {
    #[allow(dead_code)]
    pub fn new(max_history: usize) -> Self {
        Self {
            total_tasks: Arc::new(AtomicUsize::new(0)),
            succeeded: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            timed_out: Arc::new(AtomicUsize::new(0)),
            total_duration_ms: Arc::new(AtomicUsize::new(0)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            task_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history,
        }
    }

    #[allow(dead_code)]
    pub fn task_started(&self) {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        self.total_tasks.fetch_add(1, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn task_completed(&self, metrics: TaskMetrics) {
        self.active_tasks.fetch_sub(1, Ordering::SeqCst);
        self.total_duration_ms
            .fetch_add(metrics.duration_ms as usize, Ordering::SeqCst);

        match metrics.status {
            TaskStatus::Success => {
                self.succeeded.fetch_add(1, Ordering::SeqCst);
            }
            TaskStatus::Failed => {
                self.failed.fetch_add(1, Ordering::SeqCst);
            }
            TaskStatus::Timeout => {
                self.timed_out.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut history = self.task_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_tasks: self.total_tasks.load(Ordering::SeqCst),
            succeeded: self.succeeded.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
            timed_out: self.timed_out.load(Ordering::SeqCst),
            active_tasks: self.active_tasks.load(Ordering::SeqCst),
            total_duration_ms: self.total_duration_ms.load(Ordering::SeqCst) as u64,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_tasks.load(Ordering::SeqCst);
        if total == 0 {
            return 0.0;
        }
        (self.succeeded.load(Ordering::SeqCst) as f64 / total as f64) * 100.0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub total_tasks: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub active_tasks: usize,
    pub total_duration_ms: u64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[derive(Debug, Serialize)]
pub struct RunSummary {
    pub timestamp: String,
    pub total_tasks: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub success_rate: f64,
    pub total_duration_ms: u64,
}

impl MetricsCollector {
    pub fn export_summary(&self) -> Result<(), std::io::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let stats = self.get_stats();

        let summary = RunSummary {
            timestamp: now,
            total_tasks: stats.total_tasks,
            succeeded: stats.succeeded,
            failed: stats.failed,
            timed_out: stats.timed_out,
            success_rate: self.success_rate(),
            total_duration_ms: stats.total_duration_ms,
        };

        let json = serde_json::to_string_pretty(&summary)?;
        let mut file = File::create("run-summary.json")?;
        file.write_all(json.as_bytes())?;

        log::info!("Exported run-summary.json");
        Ok(())
    }
}
