use anyhow::{bail, Result};
use log::info;
use serde_json::Value;

pub struct TaskPayload {
    pub name: String,
    pub payload: Value,
}

impl TaskPayload {
    pub fn new(name: String, payload: Value) -> Self {
        Self { name, payload }
    }

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
