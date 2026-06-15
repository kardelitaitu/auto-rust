//! Pre-flight Task Validation
//!
//! Validates task definitions before execution to catch errors early.
//! This module provides comprehensive static analysis of task files.

mod types;
mod validator;

#[cfg(test)]
mod tests;

pub use types::{ValidationIssue, ValidationReport};
pub use validator::TaskValidator;

use crate::task::dsl::TaskDefinition;

impl Default for TaskValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick validation function for convenience.
#[must_use]
pub fn validate_task(def: &TaskDefinition) -> ValidationReport {
    TaskValidator::new().validate(def)
}

/// Validate a task with known task names (for Call validation).
pub fn validate_task_with_known_tasks(
    def: &TaskDefinition,
    known_tasks: impl IntoIterator<Item = impl Into<String>>,
) -> ValidationReport {
    TaskValidator::new()
        .with_known_tasks(known_tasks)
        .validate(def)
}
