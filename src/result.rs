use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Success,
    Failed(String),
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub status: TaskStatus,
    pub attempt: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub duration_ms: u64,
}

impl TaskResult {
    pub fn success(duration_ms: u64) -> Self {
        Self {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            duration_ms,
        }
    }

    pub fn failed(error: String, duration_ms: u64) -> Self {
        Self {
            status: TaskStatus::Failed(error),
            attempt: 1,
            max_retries: 0,
            last_error: None,
            duration_ms,
        }
    }

    pub fn timeout(duration_ms: u64) -> Self {
        Self {
            status: TaskStatus::Timeout,
            attempt: 1,
            max_retries: 0,
            last_error: Some("Task timed out".to_string()),
            duration_ms,
        }
    }

    pub fn with_retry(mut self, attempt: u32, max_retries: u32, last_error: String) -> Self {
        self.attempt = attempt;
        self.max_retries = max_retries;
        self.last_error = Some(last_error);
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Success)
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.status, TaskStatus::Failed(_) | TaskStatus::Timeout)
            && self.attempt < self.max_retries
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskErrorKind {
    Timeout,
    Validation,
    Navigation,
    Session,
    Browser,
    Unknown,
}

impl TaskErrorKind {
    pub fn classify(error: &str) -> Self {
        let e = error.to_lowercase();
        if e.contains("timeout") || e.contains("deadline") {
            TaskErrorKind::Timeout
        } else if e.contains("validation") || e.contains("invalid") || e.contains("schema") {
            TaskErrorKind::Validation
        } else if e.contains("navigat") || e.contains("goto") || e.contains("load") {
            TaskErrorKind::Navigation
        } else if e.contains("session") || e.contains("worker") || e.contains("page") {
            TaskErrorKind::Session
        } else if e.contains("browser") || e.contains("chromium") || e.contains("brave") {
            TaskErrorKind::Browser
        } else {
            TaskErrorKind::Unknown
        }
    }
}

pub type TaskResultFn = Box<dyn Fn() -> TaskResult + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub total_tasks: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub total_duration_ms: u64,
    pub results: Vec<TaskResult>,
}

impl RunSummary {
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            succeeded: 0,
            failed: 0,
            timed_out: 0,
            total_duration_ms: 0,
            results: Vec::new(),
        }
    }

    pub fn add(&mut self, result: TaskResult) {
        self.total_tasks += 1;
        self.total_duration_ms += result.duration_ms;

        match result.status {
            TaskStatus::Success => self.succeeded += 1,
            TaskStatus::Failed(_) => self.failed += 1,
            TaskStatus::Timeout => self.timed_out += 1,
        }

        self.results.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            return 0.0;
        }
        (self.succeeded as f64 / self.total_tasks as f64) * 100.0
    }
}

impl Default for RunSummary {
    fn default() -> Self {
        Self::new()
    }
}
