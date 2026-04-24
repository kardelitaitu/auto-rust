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
#[derive(Debug, Clone, PartialEq, Default)]
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

    #[test]
    fn test_log_context_with_special_characters() {
        let ctx = LogContext {
            session_id: Some("session-123_abc".to_string()),
            profile_name: Some("Profile@Name".to_string()),
            task_name: Some("task#name".to_string()),
        };
        let formatted = ctx.format();
        assert!(formatted.contains("session-123_abc"));
        assert!(formatted.contains("Profile@Name"));
        assert!(formatted.contains("task#name"));
    }

    #[test]
    fn test_log_context_with_empty_strings() {
        let ctx = LogContext {
            session_id: Some("".to_string()),
            profile_name: Some("".to_string()),
            task_name: Some("".to_string()),
        };
        let formatted = ctx.format();
        // Empty strings should still produce brackets
        assert_eq!(formatted, "[][][]");
    }

    #[test]
    fn test_log_context_format_ordering() {
        let ctx = LogContext {
            session_id: Some("s1".to_string()),
            profile_name: Some("p1".to_string()),
            task_name: Some("t1".to_string()),
        };
        let formatted = ctx.format();
        // Order should be session_id, profile_name, task_name
        assert_eq!(formatted, "[s1][p1][t1]");
    }

    #[test]
    fn test_log_context_with_only_session_id() {
        let ctx = LogContext {
            session_id: Some("session-1".to_string()),
            profile_name: None,
            task_name: None,
        };
        let formatted = ctx.format();
        assert_eq!(formatted, "[session-1]");
    }

    #[test]
    fn test_log_context_with_only_profile_name() {
        let ctx = LogContext {
            session_id: None,
            profile_name: Some("Profile1".to_string()),
            task_name: None,
        };
        let formatted = ctx.format();
        assert_eq!(formatted, "[Profile1]");
    }

    #[test]
    fn test_log_context_with_only_task_name() {
        let ctx = LogContext {
            session_id: None,
            profile_name: None,
            task_name: Some("Task1".to_string()),
        };
        let formatted = ctx.format();
        assert_eq!(formatted, "[Task1]");
    }

    #[test]
    fn test_file_logger_enabled_error() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Default level is Info, so Error should be enabled
        let metadata = Metadata::builder().level(log::Level::Error).build();
        assert!(logger.enabled(&metadata));
    }

    #[test]
    fn test_file_logger_enabled_trace() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        // Default level is Info, so Trace should not be enabled
        let metadata = Metadata::builder().level(log::Level::Trace).build();
        assert!(!logger.enabled(&metadata));
    }

    #[test]
    fn test_file_logger_log_error_level() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("error message"))
            .level(log::Level::Error)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("error message"));
        assert!(content.contains("ERROR"));
    }

    #[test]
    fn test_file_logger_log_warn_level() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("warn message"))
            .level(log::Level::Warn)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("warn message"));
        assert!(content.contains("WARN"));
    }

    #[test]
    fn test_file_logger_log_multiple_messages() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        for i in 0..5 {
            let msg = format!("message {}", i);
            let args = format_args!("{}", msg);
            let record = Record::builder()
                .args(args)
                .level(log::Level::Info)
                .build();
            logger.log(&record);
        }
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("message 0"));
        assert!(content.contains("message 4"));
    }

    #[test]
    fn test_file_logger_log_with_long_message() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let long_msg = "a".repeat(1000);
        let args = format_args!("{}", long_msg);
        let record = Record::builder()
            .args(args)
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.len() > 1000);
    }

    #[test]
    fn test_file_logger_log_with_special_characters() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("test with \n\t\r special chars"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("test with"));
    }

    #[test]
    fn test_file_logger_truncates_existing_file() {
        let temp_file = NamedTempFile::new().unwrap();
        
        // Write some initial content
        std::fs::write(temp_file.path(), "old content").unwrap();
        
        let logger = FileLogger::new(temp_file.path()).unwrap();
        let record = Record::builder()
            .args(format_args!("new content"))
            .level(log::Level::Info)
            .build();
        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(!content.contains("old content"));
        assert!(content.contains("new content"));
    }

    #[test]
    fn test_file_logger_log_with_full_context() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        set_log_context(LogContext {
            session_id: Some("session-123".to_string()),
            profile_name: Some("Brave".to_string()),
            task_name: Some("pageview".to_string()),
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

        assert!(content.contains("[session-123][Brave][pageview]"));
        assert!(content.contains("test message"));
    }

    #[test]
    fn test_file_logger_debug_level_not_logged() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("debug message"))
            .level(log::Level::Debug)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        // Debug messages should not be logged with default Info level
        assert!(!content.contains("debug message"));
    }

    #[test]
    fn test_file_logger_filters_chromiumoxide_submodules() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("chromiumoxide message"))
            .level(log::Level::Info)
            .target("chromiumoxide::handler::websocket")
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(!content.contains("chromiumoxide message"));
    }

    #[test]
    fn test_file_logger_non_chromiumoxide_logged() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("other message"))
            .level(log::Level::Info)
            .target("other::module")
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        assert!(content.contains("other message"));
    }

    #[test]
    fn test_log_context_debug() {
        let ctx = LogContext {
            session_id: Some("test".to_string()),
            profile_name: Some("profile".to_string()),
            task_name: Some("task".to_string()),
        };
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("LogContext"));
    }

    #[test]
    fn test_log_context_guard_drop_restores() {
        let original = LogContext {
            session_id: Some("original".to_string()),
            profile_name: None,
            task_name: None,
        };
        set_log_context(original.clone());

        {
            let _guard = scoped_log_context(LogContext {
                session_id: Some("scoped".to_string()),
                profile_name: None,
                task_name: None,
            });
            let ctx = get_log_context();
            assert_eq!(ctx.session_id, Some("scoped".to_string()));
        }

        let ctx = get_log_context();
        assert_eq!(ctx.session_id, original.session_id);
    }

    #[test]
    fn test_file_logger_timestamp_format() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("test"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        // Timestamp should be in HH:MM:SS format
        assert!(content.chars().any(|c| c == ':'));
    }

    #[test]
    fn test_file_logger_multiple_flushes() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!("test"))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();
        logger.flush();
        logger.flush(); // Multiple flushes should not panic
    }

    #[test]
    fn test_log_context_unicode() {
        let ctx = LogContext {
            session_id: Some("セッション".to_string()),
            profile_name: Some("プロファイル".to_string()),
            task_name: Some("タスク".to_string()),
        };
        let formatted = ctx.format();
        assert!(formatted.contains("セッション"));
        assert!(formatted.contains("プロファイル"));
        assert!(formatted.contains("タスク"));
    }

    #[test]
    fn test_file_logger_empty_message() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();

        let record = Record::builder()
            .args(format_args!(""))
            .level(log::Level::Info)
            .build();

        logger.log(&record);
        logger.flush();

        let mut content = String::new();
        let mut file = std::fs::File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut content).unwrap();

        // Should still log even with empty message
        assert!(content.contains("INFO"));
    }

    #[test]
    fn test_set_log_context_overwrites() {
        set_log_context(LogContext {
            session_id: Some("first".to_string()),
            profile_name: None,
            task_name: None,
        });

        set_log_context(LogContext {
            session_id: Some("second".to_string()),
            profile_name: None,
            task_name: None,
        });

        let ctx = get_log_context();
        assert_eq!(ctx.session_id, Some("second".to_string()));
    }

    #[test]
    fn test_log_context_eq() {
        let ctx1 = LogContext {
            session_id: Some("test".to_string()),
            profile_name: Some("profile".to_string()),
            task_name: Some("task".to_string()),
        };
        let ctx2 = LogContext {
            session_id: Some("test".to_string()),
            profile_name: Some("profile".to_string()),
            task_name: Some("task".to_string()),
        };
        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_log_context_ne() {
        let ctx1 = LogContext {
            session_id: Some("test1".to_string()),
            profile_name: None,
            task_name: None,
        };
        let ctx2 = LogContext {
            session_id: Some("test2".to_string()),
            profile_name: None,
            task_name: None,
        };
        assert_ne!(ctx1, ctx2);
    }
}
