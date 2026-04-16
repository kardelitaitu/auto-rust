//! Task payload validation module.
//!
//! Provides validation for task-specific payloads to ensure they conform to expected schemas:
//! - Validates task names against known task types
//! - Checks payload structure and required fields
//! - Provides detailed error messages for invalid inputs
//! - Delegates to task-specific validation functions

use anyhow::{bail, Result};
use log::info;
use serde_json::Value;

/// Represents a task definition with its associated payload for validation.
/// Used to validate that task inputs conform to expected schemas before execution.
pub struct TaskPayload {
    /// Name of the task (e.g., "cookiebot", "pageview")
    pub name: String,
    /// JSON payload containing task-specific parameters
    pub payload: Value,
}

impl TaskPayload {
    /// Creates a new TaskPayload with the given name and JSON payload.
    ///
    /// # Arguments
    /// * `name` - Name of the task (must match a known task type)
    /// * `payload` - JSON parameters for the task
    ///
    /// # Returns
    /// A new TaskPayload instance ready for validation
    pub fn new(name: String, payload: Value) -> Self {
        Self { name, payload }
    }

    /// Validates that the task payload conforms to the expected schema for this task type.
    /// Delegates to task-specific validation functions based on the task name.
    ///
    /// # Returns
    /// Ok(()) if the payload is valid, Err if validation fails
    ///
    /// # Details
    /// For unknown task types, logs an informational message and returns Ok
    /// (allowing execution to proceed but noting the lack of validation).
    pub fn validate(&self) -> Result<()> {
        match self.name.as_str() {
            "cookiebot" => self.validate_cookiebot(),
            "pageview" => self.validate_pageview(),
            _ => {
                info!("No validation schema for task: {}", self.name);
                Ok(())
            }
        }
    }

    fn validate_cookiebot(&self) -> Result<()> {
        // cookiebot doesn't require specific payload keys
        // Just verify it's a valid JSON object
        if !self.payload.is_object() {
            bail!("cookiebot payload must be an object");
        }
        Ok(())
    }

    fn validate_pageview(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("pageview payload must be an object");
        }

        // pageview requires 'url' field
        if !self
            .payload
            .get("url")
            .map(|v| !v.is_null())
            .unwrap_or(false)
        {
            bail!("pageview payload requires 'url' field");
        }

        Ok(())
    }
}

pub fn validate_task(name: &str, payload: Value) -> Result<()> {
    TaskPayload::new(name.to_string(), payload).validate()
}
