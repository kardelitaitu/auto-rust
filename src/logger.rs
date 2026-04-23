//! File-based logging implementation.
//!
//! Provides thread-safe logging to both stdout and a log file with:
//! - Configurable log levels
//! - Timestamped log entries
//! - Structured logging with session/profile/task context

use chrono::Local;
use log::LevelFilter;
use log::{Log, Metadata, Record};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

// Thread-local storage for logging context.
thread_local! {
    static LOG_CONTEXT: RefCell<LogContext> = RefCell::new(LogContext::default());
}

/// Logging context containing session and task information.
#[derive(Debug, Clone, Default)]
pub struct LogContext {
    pub session_id: Option<String>,
    pub profile_name: Option<String>,
    pub task_name: Option<String>,
}

impl LogContext {
    /// Returns formatted context string like "\[brave-9002\]\[Teen\]\[pageview\]"
    pub fn format(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref sid) = self.session_id {
            parts.push(format!("[{}]", sid));
        }

        if let Some(ref profile) = self.profile_name {
            parts.push(format!("[{}]", profile));
        }

        if let Some(ref task) = self.task_name {
            parts.push(format!("[{}]", task));
        }

        parts.join("")
    }
}

/// Sets the logging context for the current thread.
pub fn set_log_context(ctx: LogContext) {
    LOG_CONTEXT.with(|c| c.replace(ctx));
}

/// Gets the current logging context.
pub fn get_log_context() -> LogContext {
    LOG_CONTEXT.with(|c| c.borrow().clone())
}

/// Clears the logging context.
#[allow(dead_code)]
pub fn clear_log_context() {
    LOG_CONTEXT.with(|c| c.replace(LogContext::default()));
}

/// Scoped logging context guard that restores the previous thread-local context on drop.
pub struct LogContextGuard {
    previous: LogContext,
}

impl Drop for LogContextGuard {
    fn drop(&mut self) {
        set_log_context(self.previous.clone());
    }
}

/// Sets the logging context for the current scope and restores the previous context automatically.
pub fn scoped_log_context(ctx: LogContext) -> LogContextGuard {
    let previous = get_log_context();
    set_log_context(ctx);
    LogContextGuard { previous }
}

/// Logger implementation that writes log messages to both stdout and a file.
/// Provides thread-safe logging with configurable log levels.
pub struct FileLogger {
    /// Thread-safe file handle for writing log messages
    file: Mutex<File>,
    /// Minimum log level to output (messages below this level are ignored)
    level: LevelFilter,
}

impl FileLogger {
    /// Creates a new FileLogger that writes to the specified file path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(Self {
            file: Mutex::new(file),
            level: LevelFilter::Info,
        })
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Filter out chromiumoxide WebSocket deserialization errors
            if record.target().starts_with("chromiumoxide") {
                // Suppress chromiumoxide logs to reduce noise
                return;
            }

            let timestamp = Local::now().format("%H:%M:%S").to_string();

            // Get logging context
            let ctx = get_log_context();
            let context_str = ctx.format();

            let msg = if context_str.is_empty() {
                format!("{} {} {}\n", timestamp, record.level(), record.args())
            } else {
                format!(
                    "{} {} {} {}\n",
                    timestamp,
                    context_str,
                    record.level(),
                    record.args()
                )
            };

            // Write to stdout
            print!("{msg}");

            // Write to file
            if let Ok(mut file) = self.file.lock() {
                let _ = file.write_all(msg.as_bytes());
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_log_context_restores_previous_context_on_drop() {
        {
            let _guard = scoped_log_context(LogContext {
                session_id: Some("s1".to_string()),
                profile_name: Some("brave".to_string()),
                task_name: Some("pageview".to_string()),
            });
            let ctx = get_log_context();
            assert_eq!(ctx.session_id.as_deref(), Some("s1"));
        }

        let ctx = get_log_context();
        assert!(ctx.session_id.is_none());
        assert!(ctx.profile_name.is_none());
        assert!(ctx.task_name.is_none());
    }

    #[test]
    fn scoped_log_context_restores_outer_scope_when_nested() {
        let _outer = scoped_log_context(LogContext {
            session_id: Some("outer".to_string()),
            profile_name: Some("brave".to_string()),
            task_name: Some("demoqa".to_string()),
        });

        {
            let _inner = scoped_log_context(LogContext {
                session_id: Some("inner".to_string()),
                profile_name: Some("roxy".to_string()),
                task_name: Some("nativeclick".to_string()),
            });
            let ctx = get_log_context();
            assert_eq!(ctx.session_id.as_deref(), Some("inner"));
            assert_eq!(ctx.profile_name.as_deref(), Some("roxy"));
            assert_eq!(ctx.task_name.as_deref(), Some("nativeclick"));
        }

        let ctx = get_log_context();
        assert_eq!(ctx.session_id.as_deref(), Some("outer"));
        assert_eq!(ctx.profile_name.as_deref(), Some("brave"));
        assert_eq!(ctx.task_name.as_deref(), Some("demoqa"));
    }
}
