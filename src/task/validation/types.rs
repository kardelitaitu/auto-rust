use std::collections::HashSet;

/// A validation issue found during pre-flight checks.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationIssue {
    /// Error that will prevent execution
    Error(String),
    /// Warning that may indicate a problem
    Warning(String),
}

impl ValidationIssue {
    /// Get the message content.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            ValidationIssue::Error(msg) | ValidationIssue::Warning(msg) => msg,
        }
    }

    /// Check if this is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, ValidationIssue::Error(_))
    }
}

/// Result of validating a task definition.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Task name that was validated
    pub task_name: String,
    /// All validation issues found
    pub issues: Vec<ValidationIssue>,
    /// Number of actions validated
    pub action_count: usize,
    /// Number of unique variables referenced
    pub variables_referenced: HashSet<String>,
    /// Tasks called by this task
    pub tasks_called: HashSet<String>,
}

impl ValidationReport {
    /// Create a new validation report.
    #[must_use]
    pub fn new(task_name: String) -> Self {
        Self {
            task_name,
            issues: Vec::new(),
            action_count: 0,
            variables_referenced: HashSet::new(),
            tasks_called: HashSet::new(),
        }
    }

    /// Add an error to the report.
    pub fn error(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue::Error(message.into()));
    }

    /// Add a warning to the report.
    pub fn warning(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue::Warning(message.into()));
    }

    /// Check if validation passed (no errors, warnings allowed).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Check if there are any errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(ValidationIssue::is_error)
    }

    /// Get error count.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_error()).count()
    }

    /// Get warning count.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| !i.is_error()).count()
    }

    /// Get a human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        let errors = self.error_count();
        let warnings = self.warning_count();

        if errors == 0 && warnings == 0 {
            format!(
                "Task '{}' is valid ({} actions)",
                self.task_name, self.action_count
            )
        } else if errors == 0 {
            format!(
                "Task '{}' has {} warning(s) ({} actions)",
                self.task_name, warnings, self.action_count
            )
        } else {
            format!(
                "Task '{}' has {} error(s) and {} warning(s) ({} actions)",
                self.task_name, errors, warnings, self.action_count
            )
        }
    }
}
