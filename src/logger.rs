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
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_log_context_default() {
        let ctx = LogContext::default();
        assert!(ctx.session_id.is_none());
        assert!(ctx.profile_name.is_none());
        assert!(ctx.task_name.is_none());
    }

    #[test]
    fn test_log_context_creation() {
        let ctx = LogContext {
            session_id: Some("test-session".to_string()),
            profile_name: Some("test-profile".to_string()),
            task_name: Some("test-task".to_string()),
        };
        assert_eq!(ctx.session_id, Some("test-session".to_string()));
        assert_eq!(ctx.profile_name, Some("test-profile".to_string()));
        assert_eq!(ctx.task_name, Some("test-task".to_string()));
    }

    #[test]
    fn test_log_context_clone() {
        let ctx1 = LogContext {
            session_id: Some("test".to_string()),
            profile_name: Some("profile".to_string()),
            task_name: Some("task".to_string()),
        };
        let ctx2 = ctx1.clone();
        assert_eq!(ctx1.session_id, ctx2.session_id);
        assert_eq!(ctx1.profile_name, ctx2.profile_name);
        assert_eq!(ctx1.task_name, ctx2.task_name);
    }

    #[test]
    fn test_log_context_format_all_fields() {
        let ctx = LogContext {
            session_id: Some("brave-9002".to_string()),
            profile_name: Some("Teen".to_string()),
            task_name: Some("pageview".to_string()),
        };
        let formatted = ctx.format();
        assert_eq!(formatted, "[brave-9002][Teen][pageview]");
    }

    #[test]
    fn test_log_context_format_partial() {
        let ctx = LogContext {
            session_id: Some("brave-9002".to_string()),
            profile_name: None,
            task_name: Some("pageview".to_string()),
        };
        let formatted = ctx.format();
        assert_eq!(formatted, "[brave-9002][pageview]");
    }

    #[test]
    fn test_log_context_format_empty() {
        let ctx = LogContext::default();
        let formatted = ctx.format();
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_set_log_context() {
        let ctx = LogContext {
            session_id: Some("test".to_string()),
            profile_name: None,
            task_name: None,
        };
        set_log_context(ctx);
        let retrieved = get_log_context();
        assert_eq!(retrieved.session_id, Some("test".to_string()));
    }

    #[test]
    fn test_get_log_context_default() {
        clear_log_context();
        let ctx = get_log_context();
        assert!(ctx.session_id.is_none());
        assert!(ctx.profile_name.is_none());
        assert!(ctx.task_name.is_none());
    }

    #[test]
    fn test_clear_log_context() {
        set_log_context(LogContext {
            session_id: Some("test".to_string()),
            profile_name: None,
            task_name: None,
        });
        clear_log_context();
        let ctx = get_log_context();
        assert!(ctx.session_id.is_none());
    }

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

    #[test]
    fn test_file_logger_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Logger should be created successfully
        let _ = logger;
    }

    #[test]
    fn test_file_logger_enabled_info() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Default level is Info, so Info should be enabled
        let metadata = Metadata::builder().level(log::Level::Info).build();
        assert!(logger.enabled(&metadata));
    }

    #[test]
    fn test_file_logger_enabled_debug() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Default level is Info, so Debug should not be enabled
        let metadata = Metadata::builder().level(log::Level::Debug).build();
        assert!(!logger.enabled(&metadata));
    }

    #[test]
    fn test_file_logger_enabled_warn() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Default level is Info, so Warn should be enabled
        let metadata = Metadata::builder().level(log::Level::Warn).build();
        assert!(logger.enabled(&metadata));
    }

    #[test]
    fn test_file_logger_log_without_context() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("test message"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("test message"));
        assert!(content.contains("INFO"));
    }

    #[test]
    fn test_file_logger_log_with_context() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        set_log_context(LogContext {
            session_id: Some("test-session".to_string()),
            profile_name: None,
            task_name: None,
        });

        let record = Record::builder()
            .args(format_args!("test message"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("test message"));
        assert!(content.contains("[test-session]"));
    }

    #[test]
    fn test_file_logger_filters_chromiumoxide() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("chromiumoxide message"))
            .level(log::Level::Info)
            .target("chromiumoxide::handler")
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        // Chromiumoxide messages should be filtered out
        assert!(!content.contains("chromiumoxide message"));
    }

    #[test]
    fn test_file_logger_flush() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("test message"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush(); // Should not panic
    }
}
