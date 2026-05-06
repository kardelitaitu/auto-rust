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

/// Information about a task's validation requirements.
/// Used by CLI help system to display payload guidance.
#[derive(Debug, Clone)]
pub struct TaskValidationInfo {
    /// Human-readable description of expected payload
    pub description: String,
    /// Example CLI invocations
    pub examples: Vec<String>,
    /// Required field names
    pub required_fields: Vec<String>,
    /// Optional field names
    pub optional_fields: Vec<String>,
}

/// Get validation information for a specific task.
///
/// Returns structured info about the task's expected payload,
/// used by the `--help-task` CLI feature.
///
/// # Arguments
/// * `task_name` - The name of the task to get info for
///
/// # Returns
/// Option containing validation info if the task is known
pub fn get_task_validation_info(task_name: &str) -> Option<TaskValidationInfo> {
    match task_name {
        "cookiebot" => Some(TaskValidationInfo {
            description: "Object with optional configuration".to_string(),
            examples: vec![
                "cookiebot".to_string(),
                "cookiebot={\"data_file\": \"custom.txt\"}".to_string(),
            ],
            required_fields: vec![],
            optional_fields: vec!["data_file".to_string()],
        }),
        "pageview" => Some(TaskValidationInfo {
            description: "Object with url or value field".to_string(),
            examples: vec![
                "pageview=https://example.com".to_string(),
                "pageview=url=https://example.com".to_string(),
                "pageview={\"url\": \"https://example.com\"}".to_string(),
            ],
            required_fields: vec!["url or value".to_string()],
            optional_fields: vec![],
        }),
        "twitterfollow" => Some(TaskValidationInfo {
            description: "Object with username, url, or value field".to_string(),
            examples: vec![
                "twitterfollow=elonmusk".to_string(),
                "twitterfollow=https://x.com/elonmusk".to_string(),
                "twitterfollow={\"username\": \"elonmusk\"}".to_string(),
            ],
            required_fields: vec!["username, url, or value".to_string()],
            optional_fields: vec![],
        }),
        "twitterquote" => Some(TaskValidationInfo {
            description: "Object with url/value and optional quote_text".to_string(),
            examples: vec![
                "twitterquote=https://x.com/user/status/123".to_string(),
                "twitterquote={\"url\": \"...\", \"quote_text\": \"comment\"}".to_string(),
            ],
            required_fields: vec!["url or value".to_string()],
            optional_fields: vec!["quote_text (max 280 chars)".to_string()],
        }),
        "twitterreply" => Some(TaskValidationInfo {
            description: "Object with url or value pointing to tweet".to_string(),
            examples: vec!["twitterreply=https://x.com/user/status/123".to_string()],
            required_fields: vec!["url or value".to_string()],
            optional_fields: vec![],
        }),
        "twitteractivity" => Some(TaskValidationInfo {
            description: "Object with optional engagement configuration".to_string(),
            examples: vec![
                "twitteractivity".to_string(),
                "twitteractivity={\"duration_ms\": 120000}".to_string(),
            ],
            required_fields: vec![],
            optional_fields: vec!["duration_ms".to_string(), "weights".to_string()],
        }),
        "demoqa" | "demo-keyboard" | "demo-mouse" | "twitterdive" | "twitterlike"
        | "twitterretweet" | "twittertest" | "task-example" => Some(TaskValidationInfo {
            description: "Object with task-specific parameters".to_string(),
            examples: vec![
                format!("{}={{}}", task_name),
                format!("{}={{\"key\": \"value\"}}", task_name),
            ],
            required_fields: vec![],
            optional_fields: vec!["task-specific fields".to_string()],
        }),
        _ => None,
    }
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

    #[test]
    fn cookiebot_accepts_object_payload() {
        // cookiebot accepts any object payload
        assert!(validate_task("cookiebot", json!({})).is_ok());
        assert!(validate_task("cookiebot", json!({"data_file":"custom.txt"})).is_ok());
        // cookiebot rejects non-object payloads
        assert!(validate_task("cookiebot", json!([])).is_err());
        assert!(validate_task("cookiebot", json!("string")).is_err());
    }

    #[test]
    fn pageview_requires_url_or_value() {
        // Empty object should fail
        assert!(validate_task("pageview", json!({})).is_err());
        // Whitespace-only should fail
        assert!(validate_task("pageview", json!({"url":"   "})).is_err());
        assert!(validate_task("pageview", json!({"value":"   "})).is_err());
        // Valid URL should pass
        assert!(validate_task("pageview", json!({"url":"https://example.com"})).is_ok());
        // Value alias should also work
        assert!(validate_task("pageview", json!({"value":"https://example.com"})).is_ok());
    }

    #[test]
    fn twitterquote_requires_url_or_value() {
        assert!(validate_task("twitterquote", json!({})).is_err());
        assert!(validate_task("twitterquote", json!({"url":"https://x.com/a/status/1"})).is_ok());
        assert!(validate_task("twitterquote", json!({"value":"https://x.com/a/status/1"})).is_ok());
    }

    #[test]
    fn twitterquote_enforces_280_char_limit() {
        let long_text = "a".repeat(281);
        assert!(validate_task(
            "twitterquote",
            json!({
                "url": "https://x.com/a/status/1",
                "quote_text": long_text
            })
        )
        .is_err());

        // Exactly 280 chars should pass
        let exact_text = "a".repeat(280);
        assert!(validate_task(
            "twitterquote",
            json!({
                "url": "https://x.com/a/status/1",
                "quote_text": exact_text
            })
        )
        .is_ok());
    }

    #[test]
    fn twitterfollow_accepts_url_instead_of_username() {
        // Should accept URL as alternative to username
        assert!(validate_task("twitterfollow", json!({"url":"https://x.com/user"})).is_ok());
        // Should accept value alias
        assert!(validate_task("twitterfollow", json!({"value":"https://x.com/user"})).is_ok());
    }

    #[test]
    fn validate_task_payload_new() {
        use super::TaskPayload;
        let payload = TaskPayload::new("test-task".to_string(), json!({"key": "value"}));
        assert_eq!(payload.name, "test-task");
        assert!(payload.payload.is_object());
    }

    #[test]
    fn twitterretweet_requires_object() {
        assert!(validate_task("twitterretweet", json!([])).is_err());
        assert!(validate_task("twitterretweet", json!({})).is_ok());
    }

    #[test]
    fn twitterlike_requires_object() {
        assert!(validate_task("twitterlike", json!("string")).is_err());
        assert!(validate_task("twitterlike", json!({})).is_ok());
    }

    #[test]
    fn twitterdive_requires_object() {
        assert!(validate_task("twitterdive", json!(123)).is_err());
        assert!(validate_task("twitterdive", json!({"depth": 5})).is_ok());
    }

    #[test]
    fn demo_keyboard_requires_object() {
        assert!(validate_task("demo-keyboard", json!(null)).is_err());
        assert!(validate_task("demo-keyboard", json!({})).is_ok());
    }

    #[test]
    fn demo_mouse_requires_object() {
        assert!(validate_task("demo-mouse", json!(true)).is_err());
        assert!(validate_task("demo-mouse", json!({})).is_ok());
    }

    #[test]
    fn resolve_pageview_target_prefers_url_over_value() {
        // When both provided, url should be preferred
        let result = resolve_pageview_target(&json!({
            "url": "https://example.com",
            "value": "https://other.com"
        }))
        .unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn resolve_pageview_target_empty_object_fails() {
        assert!(resolve_pageview_target(&json!({})).is_err());
    }

    #[test]
    fn validate_non_object_payload_fails_for_all_tasks() {
        // All tasks that require object payload should reject arrays
        let tasks = [
            "cookiebot",
            "pageview",
            "twitterfollow",
            "twitterquote",
            "twitterreply",
            "twitteractivity",
            "demoqa",
            "demo-keyboard",
            "demo-mouse",
            "twitterlike",
            "twitterretweet",
            "twitterdive",
        ];

        for task in tasks {
            assert!(
                validate_task(task, json!([])).is_err(),
                "Task {} should reject array payload",
                task
            );
            assert!(
                validate_task(task, json!("string")).is_err(),
                "Task {} should reject string payload",
                task
            );
        }
    }
}
