//! Task configuration parsing from JSON payload with helpers for reading validated numeric fields.

use crate::config::TwitterActivityConfig;
use crate::utils::payload as payload_util;
use crate::utils::payload::PayloadError;
use crate::utils::timing::duration_with_variance;
use rand::Rng;
use serde_json::Value;

use super::types::{SentimentTemplates, TaskValidationError};

/// Task configuration parsed from JSON payload.
#[derive(Debug, Clone, Default)]
pub struct TaskConfig {
    pub duration_ms: u64,
    pub candidate_count: u32,
    pub thread_depth: u32,
    pub max_actions_per_scan: u32,
    pub scroll_count: u32,
    pub weights: Option<Value>,
    pub llm_enabled: bool,
    pub llm_api_key: Option<String>,
    pub smart_decision_enabled: bool,
    pub sentiment_templates: SentimentTemplates,
    pub enhanced_sentiment_enabled: bool,
    pub dry_run_actions: bool,
    pub simulate_only: bool,
    pub seed: u64,
}

impl TaskConfig {
    /// Parse task configuration from JSON payload with defaults
    pub fn from_payload(
        payload: &Value,
        config: &TwitterActivityConfig,
    ) -> Result<Self, TaskValidationError> {
        let duration_ms =
            match read_u64(payload, "duration_ms", config.feed_scan_duration_ms.get())? {
                value if payload.get("duration_ms").is_some() => duration_with_variance(value, 20),
                value => value,
            };
        let candidate_count = read_u32(
            payload,
            "candidate_count",
            config.engagement_candidate_count,
        )?;
        let thread_depth = read_u32(payload, "thread_depth", 3)?;
        let max_actions_per_scan = read_u32(
            payload,
            "max_actions_per_scan",
            config.engagement_candidate_count,
        )?
        .max(1);
        let scroll_count = read_u32(payload, "scroll_count", config.feed_scroll_count)?;
        let weights = payload.get("weights").cloned();

        // Parse LLM config (V2 feature)
        let llm_enabled = payload
            .get("llm_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(config.llm.enabled);

        // Parse smart decision config (V3 feature - rule-based)
        let smart_decision_enabled = payload
            .get("smart_decision_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Sentiment templates use defaults for now
        let sentiment_templates = SentimentTemplates::default();

        // Parse enhanced sentiment config
        let enhanced_sentiment_enabled = payload
            .get("enhanced_sentiment_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true); // Enable by default for better analysis

        let dry_run_actions = payload
            .get("dry_run_actions")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let simulate_only = payload
            .get("simulate_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let llm_api_key = decision_llm_api_key();

        let seed = rand::thread_rng().gen::<u64>();

        Ok(Self {
            duration_ms,
            candidate_count,
            thread_depth,
            max_actions_per_scan,
            scroll_count,
            weights,
            llm_enabled,
            llm_api_key,
            smart_decision_enabled,
            sentiment_templates,
            enhanced_sentiment_enabled,
            dry_run_actions,
            simulate_only,
            seed,
        })
    }
}

fn decision_llm_api_key() -> Option<String> {
    std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| std::env::var("QWEN_API_KEY"))
        .ok()
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Helper: read numeric fields from payload with validation (u64)
pub fn read_u64(payload: &Value, key: &str, default: u64) -> Result<u64, TaskValidationError> {
    let raw = match payload_util::read_u64(payload, key) {
        Ok(v) => v,
        Err(PayloadError::Missing) => return Ok(default),
        Err(PayloadError::Invalid(_)) => {
            let kind = payload.get(key).map(value_kind).unwrap_or("unknown");
            return Err(TaskValidationError::InvalidFieldType {
                field: key.to_string(),
                expected: "positive integer",
                actual: kind,
            });
        }
    };
    if raw == 0 {
        Err(TaskValidationError::InvalidPositiveNumber {
            field: key.to_string(),
            value: 0,
        })
    } else {
        Ok(raw)
    }
}

/// Helper: read numeric fields from payload with validation (u32)
pub fn read_u32(payload: &Value, key: &str, default: u32) -> Result<u32, TaskValidationError> {
    let raw = match payload_util::read_u32(payload, key) {
        Ok(v) => v,
        Err(PayloadError::Missing) => return Ok(default),
        Err(PayloadError::Invalid(_)) => {
            let kind = payload.get(key).map(value_kind).unwrap_or("unknown");
            return Err(TaskValidationError::InvalidFieldType {
                field: key.to_string(),
                expected: "positive u32",
                actual: kind,
            });
        }
    };
    if raw == 0 {
        Err(TaskValidationError::InvalidPositiveNumber {
            field: key.to_string(),
            value: 0,
        })
    } else {
        Ok(raw)
    }
}

#[cfg(test)]
mod read_u64_tests {
    use super::read_u64;
    use serde_json::json;

    #[test]
    fn read_u64_returns_value_when_present() {
        let payload = json!({"duration_ms": 120000});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 120000);
    }

    #[test]
    fn read_u64_rejects_invalid() {
        let payload = json!({"duration_ms": -100});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duration_ms"));
    }

    #[test]
    fn read_u64_defaults_when_missing() {
        let payload = json!({});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 300000);
    }
}

#[cfg(test)]
mod read_u32_tests {
    use super::read_u32;
    use serde_json::json;

    #[test]
    fn read_u32_returns_value_when_present() {
        let payload = json!({"candidate_count": 10});
        let result = read_u32(&payload, "candidate_count", 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn read_u32_rejects_invalid() {
        let payload = json!({"candidate_count": -5});
        let result = read_u32(&payload, "candidate_count", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("candidate_count"));
    }
}

#[cfg(test)]
mod payload_tests {
    use super::TaskConfig;
    use serde_json::json;

    fn twitter_config() -> crate::config::TwitterActivityConfig {
        crate::config::TwitterActivityConfig::default()
    }

    fn full_payload() -> serde_json::Value {
        json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5
        })
    }

    fn duration_payload(value: i64) -> serde_json::Value {
        json!({"duration_ms": value})
    }

    #[test]
    fn from_payload_parses_core_fields() {
        let result = TaskConfig::from_payload(&full_payload(), &twitter_config());
        assert!(result.is_ok());
        let task_config = result.unwrap();
        assert!((96_000..=144_000).contains(&task_config.duration_ms));
        assert_eq!(task_config.candidate_count, 10);
        assert_eq!(task_config.thread_depth, 15);
        assert_eq!(task_config.max_actions_per_scan, 5);
        assert!(!task_config.simulate_only);
    }

    #[test]
    fn from_payload_rejects_invalid_duration() {
        let result = TaskConfig::from_payload(&duration_payload(-100), &twitter_config());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("duration_ms"),
            "Error should mention the field name: got {err}"
        );
        assert!(
            err.contains("positive"),
            "Error should mention 'positive': got {err}"
        );
    }

    #[test]
    fn from_payload_rejects_invalid_candidate_count_type() {
        let payload = json!({"candidate_count": "ten"});
        let result = TaskConfig::from_payload(&payload, &twitter_config());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("candidate_count"));
        assert_eq!(
            err,
            "Invalid value for 'candidate_count': string (must be positive u32)"
        );
    }

    #[test]
    fn from_payload_parses_simulation_fields() {
        let payload = json!({
            "simulate_only": true
        });
        let result = TaskConfig::from_payload(&payload, &twitter_config());
        assert!(result.is_ok());
        let task_config = result.unwrap();
        assert!(task_config.simulate_only);
    }
}

#[cfg(test)]
mod gap_tests {
    use super::{read_u32, read_u64, TaskConfig};
    use serde_json::json;

    #[test]
    fn read_u64_rejects_zero() {
        let payload = json!({"duration_ms": 0});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duration_ms"));
        assert!(err.contains("must be positive"));
    }

    #[test]
    fn read_u32_rejects_zero() {
        let payload = json!({"candidate_count": 0});
        let result = read_u32(&payload, "candidate_count", 5);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("candidate_count"));
        assert!(err.contains("must be positive"));
    }

    #[test]
    fn read_u64_rejects_string_type() {
        let payload = json!({"duration_ms": "fast"});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("string"));
    }

    #[test]
    fn read_u32_rejects_bool_type() {
        let payload = json!({"candidate_count": true});
        let result = read_u32(&payload, "candidate_count", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bool"));
    }

    #[test]
    fn read_u64_null_treated_as_missing_returns_default() {
        let payload = json!({"duration_ms": null});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 300000);
    }

    #[test]
    fn from_payload_max_actions_per_scan_minimum_is_one() {
        let payload = json!({});
        let config = crate::config::TwitterActivityConfig::default();
        let task_config = TaskConfig::from_payload(&payload, &config).unwrap();
        assert!(task_config.max_actions_per_scan >= 1);
    }

    #[test]
    fn from_payload_boolean_fields_default_correctly() {
        let payload = json!({});
        let config = crate::config::TwitterActivityConfig::default();
        let task_config = TaskConfig::from_payload(&payload, &config).unwrap();
        assert!(!task_config.simulate_only);
        assert!(!task_config.dry_run_actions);
        assert!(!task_config.smart_decision_enabled);
    }
}
