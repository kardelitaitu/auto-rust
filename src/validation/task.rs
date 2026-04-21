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
            "twitterfollow" => self.validate_twitterfollow(),
            "twitterquote" => self.validate_twitterquote(),
            "twitterreply" => self.validate_twitterreply(),
            "twitteractivity" => self.validate_twitteractivity(),
            "demoqa" => self.validate_demoqa(),
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

    fn validate_twitterfollow(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("twitterfollow payload must be an object");
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
            bail!("twitterfollow payload requires username, url, or value");
        }

        Ok(())
    }

    fn validate_twitterquote(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("twitterquote payload must be an object");
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
            bail!("twitterquote payload requires url or value");
        }

        if has_quote_text {
            let text = self.payload.get("quote_text").unwrap().as_str().unwrap();
            if text.len() > 280 {
                bail!("twitterquote quote_text exceeds 280 characters");
            }
        }

        Ok(())
    }

    fn validate_twitterreply(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("twitterreply payload must be an object");
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
            bail!("twitterreply payload requires url or value");
        }

        Ok(())
    }

    fn validate_twitteractivity(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("twitteractivity payload must be an object");
        }
        Ok(())
    }

    fn validate_demoqa(&self) -> Result<()> {
        if !self.payload.is_object() {
            bail!("demoqa payload must be an object");
        }
        Ok(())
    }
}

pub fn validate_task(name: &str, payload: Value) -> Result<()> {
    TaskPayload::new(name.to_string(), payload).validate()
}

#[cfg(test)]
mod tests {
    use super::validate_task;
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
}
