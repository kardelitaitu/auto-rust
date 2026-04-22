//! Task payload validation module.
//!
//! Provides validation for task-specific payloads to ensure they conform to expected schemas:
//! - Validates task names against known task types
//! - Checks payload structure and required fields
//! - Provides detailed error messages for invalid inputs
//! - Delegates to task-specific validation functions

use crate::error::{OrchestratorError, Result, TaskError};
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
            "demo-keyboard" => self.validate_object_payload("demo-keyboard"),
            "demo-mouse" => self.validate_object_payload("demo-mouse"),
            "twitterfollow" => self.validate_twitterfollow(),
            "twitterdive" => self.validate_object_payload("twitterdive"),
            "twitterlike" => self.validate_object_payload("twitterlike"),
            "twitterquote" => self.validate_twitterquote(),
            "twitterreply" => self.validate_twitterreply(),
            "twitterretweet" => self.validate_object_payload("twitterretweet"),
            "twittertest" => self.validate_object_payload("twittertest"),
            "twitteractivity" => self.validate_twitteractivity(),
            "demoqa" => self.validate_demoqa(),
            "task-example" => self.validate_object_payload("task-example"),
            _ => {
                info!("No validation schema for task: {}", self.name);
                Ok(())
            }
        }
    }

    fn validate_object_payload(&self, task_name: &str) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: task_name.to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }
        Ok(())
    }

    fn validate_cookiebot(&self) -> Result<()> {
        // cookiebot doesn't require specific payload keys
        // Just verify it's a valid JSON object
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "cookiebot".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }
        Ok(())
    }

    fn validate_pageview(&self) -> Result<()> {
        resolve_pageview_target(&self.payload).map(|_| ())
    }

    fn validate_twitterfollow(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterfollow".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }

        let has_username = self
            .payload
            .get("username")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_url = self
            .payload
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_value = self
            .payload
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !(has_username || has_url || has_value) {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterfollow".to_string(),
                reason: "requires username, url, or value".to_string(),
            }));
        }

        Ok(())
    }

    fn validate_twitterquote(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterquote".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }

        let has_url = self
            .payload
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_value = self
            .payload
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_quote_text = self
            .payload
            .get("quote_text")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !(has_url || has_value) {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterquote".to_string(),
                reason: "requires url or value".to_string(),
            }));
        }

        if has_quote_text {
            let text = self.payload.get("quote_text").unwrap().as_str().unwrap();
            if text.len() > 280 {
                return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                    task_name: "twitterquote".to_string(),
                    reason: "quote_text exceeds 280 characters".to_string(),
                }));
            }
        }

        Ok(())
    }

    fn validate_twitterreply(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterreply".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }

        let has_url = self
            .payload
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_value = self
            .payload
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !(has_url || has_value) {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitterreply".to_string(),
                reason: "requires url or value".to_string(),
            }));
        }

        Ok(())
    }

    fn validate_twitteractivity(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "twitteractivity".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }
        Ok(())
    }

    fn validate_demoqa(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(OrchestratorError::Task(TaskError::ValidationFailed {
                task_name: "demoqa".to_string(),
                reason: "payload must be an object".to_string(),
            }));
        }
        Ok(())
    }
}

pub fn validate_task(name: &str, payload: Value) -> Result<()> {
    TaskPayload::new(name.to_string(), payload).validate()
}

/// Resolves the target URL for `pageview`.
///
/// Accepts both `url` and the legacy `value` alias so validation and task
/// execution stay in sync.
pub fn resolve_pageview_target(payload: &Value) -> Result<String> {
    if !payload.is_object() {
        return Err(OrchestratorError::Task(TaskError::ValidationFailed {
            task_name: "pageview".to_string(),
            reason: "payload must be an object".to_string(),
        }));
    }

    for key in ["url", "value"] {
        if let Some(target) = payload.get(key).and_then(|v| v.as_str()) {
            if !target.trim().is_empty() {
                return Ok(target.to_string());
            }
        }
    }

    Err(OrchestratorError::Task(TaskError::ValidationFailed {
        task_name: "pageview".to_string(),
        reason: "requires 'url' or 'value'".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{resolve_pageview_target, validate_task};
    use serde_json::json;

    #[test]
    fn twitterfollow_requires_target() {
        assert!(validate_task("twitterfollow", json!({})).is_err());
        assert!(validate_task("twitterfollow", json!({"username":"dika"})).is_ok());
    }

    #[test]
    fn twitterreply_requires_url_or_value() {
        assert!(validate_task("twitterreply", json!({})).is_err());
        assert!(validate_task("twitterreply", json!({"url":"https://x.com/a/status/1"})).is_ok());
    }

    #[test]
    fn twitteractivity_requires_object() {
        assert!(validate_task("twitteractivity", json!([])).is_err());
        assert!(validate_task("twitteractivity", json!({})).is_ok());
    }

    #[test]
    fn demoqa_requires_object() {
        assert!(validate_task("demoqa", json!([])).is_err());
        assert!(validate_task("demoqa", json!({})).is_ok());
    }

    #[test]
    fn pageview_accepts_value_alias() {
        assert_eq!(
            resolve_pageview_target(&json!({"value":"https://example.com"})).unwrap(),
            "https://example.com"
        );
    }

    #[test]
    fn task_example_requires_object() {
        assert!(validate_task("task-example", json!([])).is_err());
        assert!(validate_task("task-example", json!({})).is_ok());
    }

    #[test]
    fn unknown_task_accepts_any_payload() {
        // Unknown tasks should pass validation with any payload (just logs info)
        assert!(validate_task("unknown-task", json!({})).is_ok());
        assert!(validate_task("unknown-task", json!({"any":"value"})).is_ok());
        assert!(validate_task("unknown-task", json!([])).is_ok());
        assert!(validate_task("unknown-task", json!(null)).is_ok());
    }
}
