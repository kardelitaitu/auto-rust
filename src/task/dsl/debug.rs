//! Debug/tracing infrastructure for DSL execution.
//!
//! Provides debug event types, breakpoint configuration,
//! and execution tracing for DSL tasks.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
/// Debug event type for execution tracing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugEventType {
    /// Action execution started
    ActionStart,
    /// Action execution completed
    ActionComplete,
    /// Action execution failed
    ActionError,
    /// Breakpoint hit
    Breakpoint,
    /// Variable set/changed
    VariableSet,
    /// Task call started
    TaskCallStart,
    /// Task call completed
    TaskCallComplete,
    /// Condition evaluated
    ConditionEvaluated,
}

/// Debug event for execution tracing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugEvent {
    /// Event timestamp
    pub timestamp: String,
    /// Event type
    pub event_type: DebugEventType,
    /// Action index (if applicable)
    pub action_index: Option<usize>,
    /// Action type (if applicable)
    pub action_type: Option<String>,
    /// Variable name (if applicable)
    pub variable_name: Option<String>,
    /// Variable value (if applicable)
    pub variable_value: Option<String>,
    /// Condition result (if applicable)
    pub condition_result: Option<bool>,
    /// Error message (if applicable)
    pub error: Option<String>,
}

/// Breakpoint configuration.
pub struct Breakpoint {
    /// Action index to break on (if None, applies to all actions)
    pub action_index: Option<usize>,
    /// Action type to break on (if None, applies to all types)
    pub action_type: Option<String>,
    /// Variable name to watch (if None, no variable watching)
    pub watch_variable: Option<String>,
    /// Condition that must be true for breakpoint to trigger
    #[allow(clippy::type_complexity)]
    condition: Option<Arc<dyn Fn(&HashMap<String, String>) -> bool + Send + Sync>>,
}

impl fmt::Debug for Breakpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Breakpoint")
            .field("action_index", &self.action_index)
            .field("action_type", &self.action_type)
            .field("watch_variable", &self.watch_variable)
            .field("has_condition", &self.condition.is_some())
            .finish()
    }
}

impl Clone for Breakpoint {
    fn clone(&self) -> Self {
        // Note: We cannot clone the closure itself, so we create a new Breakpoint
        // without the condition. This is a limitation for breakpoints with custom conditions.
        Self {
            action_index: self.action_index,
            action_type: self.action_type.clone(),
            watch_variable: self.watch_variable.clone(),
            condition: None, // Cannot clone closures
        }
    }
}

impl Breakpoint {
    /// Create a breakpoint on a specific action index.
    #[must_use]
    pub fn on_action(index: usize) -> Self {
        Self {
            action_index: Some(index),
            action_type: None,
            watch_variable: None,
            condition: None,
        }
    }

    /// Create a breakpoint on any action of a specific type.
    pub fn on_action_type(action_type: impl Into<String>) -> Self {
        Self {
            action_index: None,
            action_type: Some(action_type.into()),
            watch_variable: None,
            condition: None,
        }
    }

    /// Create a watch breakpoint that triggers when a variable changes.
    pub fn watch_variable(name: impl Into<String>) -> Self {
        Self {
            action_index: None,
            action_type: None,
            watch_variable: Some(name.into()),
            condition: None,
        }
    }

    /// Check if this breakpoint should trigger for the given action.
    #[must_use]
    pub fn should_trigger(
        &self,
        action_index: usize,
        action_type: &str,
        variables: &HashMap<String, String>,
    ) -> bool {
        // Check action index if specified
        if let Some(idx) = self.action_index {
            if idx != action_index {
                return false;
            }
        }

        // Check action type if specified
        if let Some(ref atype) = self.action_type {
            if atype != action_type {
                return false;
            }
        }

        // Check condition if specified
        if let Some(ref cond) = self.condition {
            if !cond(variables) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_event_type_variants() {
        let types = [
            DebugEventType::ActionStart,
            DebugEventType::ActionComplete,
            DebugEventType::ActionError,
            DebugEventType::Breakpoint,
            DebugEventType::VariableSet,
            DebugEventType::TaskCallStart,
            DebugEventType::TaskCallComplete,
            DebugEventType::ConditionEvaluated,
        ];
        assert_eq!(types.len(), 8);
    }

    #[test]
    fn test_debug_event_creation() {
        let event = DebugEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            event_type: DebugEventType::ActionStart,
            action_index: Some(0),
            action_type: Some("Click".to_string()),
            variable_name: None,
            variable_value: None,
            condition_result: None,
            error: None,
        };
        assert_eq!(event.action_index, Some(0));
    }

    #[test]
    fn test_breakpoint_on_action() {
        let bp = Breakpoint::on_action(5);
        assert_eq!(bp.action_index, Some(5));
        assert!(bp.action_type.is_none());
    }

    #[test]
    fn test_breakpoint_on_action_type() {
        let bp = Breakpoint::on_action_type("Click");
        assert_eq!(bp.action_type, Some("Click".to_string()));
        assert!(bp.action_index.is_none());
    }

    #[test]
    fn test_breakpoint_should_trigger() {
        let bp = Breakpoint::on_action(3);
        let vars = HashMap::new();

        // Should trigger for action 3
        assert!(bp.should_trigger(3, "Click", &vars));
        // Should not trigger for action 4
        assert!(!bp.should_trigger(4, "Click", &vars));
    }

    #[test]
    fn test_breakpoint_watch_variable() {
        let bp = Breakpoint::watch_variable("myVar");
        assert_eq!(bp.watch_variable, Some("myVar".to_string()));
    }

    #[test]
    fn test_breakpoint_action_type_trigger() {
        let bp = Breakpoint::on_action_type("Click");
        let vars = HashMap::new();

        // Should trigger for any action with type "Click"
        assert!(bp.should_trigger(0, "Click", &vars));
        assert!(bp.should_trigger(5, "Click", &vars));
        assert!(bp.should_trigger(99, "Click", &vars));

        // Should NOT trigger for different action types
        assert!(!bp.should_trigger(0, "Navigate", &vars));
        assert!(!bp.should_trigger(0, "Type", &vars));
    }

    #[test]
    fn test_breakpoint_combined_index_and_type() {
        // Breakpoint with both index and type filter
        let bp = Breakpoint {
            action_index: Some(2),
            action_type: Some("Click".to_string()),
            watch_variable: None,
            condition: None,
        };
        let vars = HashMap::new();

        // Must match BOTH index AND type
        assert!(bp.should_trigger(2, "Click", &vars));
        assert!(!bp.should_trigger(3, "Click", &vars)); // wrong index
        assert!(!bp.should_trigger(2, "Navigate", &vars)); // wrong type
        assert!(!bp.should_trigger(3, "Navigate", &vars)); // both wrong
    }

    #[test]
    fn test_breakpoint_no_filter_triggers_all() {
        let bp = Breakpoint {
            action_index: None,
            action_type: None,
            watch_variable: None,
            condition: None,
        };
        let vars = HashMap::new();

        // No filters means trigger on everything
        assert!(bp.should_trigger(0, "Click", &vars));
        assert!(bp.should_trigger(100, "Navigate", &vars));
        assert!(bp.should_trigger(999, "Wait", &vars));
    }

    #[test]
    fn test_breakpoint_clone_drops_condition() {
        use std::sync::Arc;

        let bp = Breakpoint {
            action_index: Some(1),
            action_type: Some("Click".to_string()),
            watch_variable: Some("x".to_string()),
            condition: Some(Arc::new(|_vars: &HashMap<String, String>| true)),
        };

        let cloned = bp.clone();
        assert_eq!(cloned.action_index, Some(1));
        assert_eq!(cloned.action_type, Some("Click".to_string()));
        assert_eq!(cloned.watch_variable, Some("x".to_string()));
        // Condition is dropped during clone
        assert!(cloned.condition.is_none());
    }

    #[test]
    fn test_breakpoint_with_custom_condition() {
        use std::sync::Arc;

        let bp = Breakpoint {
            action_index: None,
            action_type: None,
            watch_variable: None,
            condition: Some(Arc::new(|vars: &HashMap<String, String>| {
                vars.get("status")
                    .map(|v| v == "active")
                    .unwrap_or(false)
            })),
        };

        let mut vars = HashMap::new();

        // Condition not met
        assert!(!bp.should_trigger(0, "Click", &vars));

        // Condition met
        vars.insert("status".to_string(), "active".to_string());
        assert!(bp.should_trigger(0, "Click", &vars));

        // Condition no longer met
        vars.insert("status".to_string(), "inactive".to_string());
        assert!(!bp.should_trigger(0, "Click", &vars));
    }

    #[test]
    fn test_debug_event_serialization() {
        let event = DebugEvent {
            timestamp: "2024-06-15T10:30:00Z".to_string(),
            event_type: DebugEventType::ActionComplete,
            action_index: Some(2),
            action_type: Some("Navigate".to_string()),
            variable_name: None,
            variable_value: None,
            condition_result: Some(true),
            error: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let back: DebugEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action_index, Some(2));
        assert_eq!(back.condition_result, Some(true));
    }

    #[test]
    fn test_debug_event_type_serialization() {
        let types = vec![
            DebugEventType::ActionStart,
            DebugEventType::ActionComplete,
            DebugEventType::ActionError,
            DebugEventType::Breakpoint,
            DebugEventType::VariableSet,
            DebugEventType::TaskCallStart,
            DebugEventType::TaskCallComplete,
            DebugEventType::ConditionEvaluated,
        ];

        for event_type in &types {
            let json = serde_json::to_string(&event_type).unwrap();
            let back: DebugEventType = serde_json::from_str(&json).unwrap();
            // Verify round-trip by comparing serialized forms
            assert_eq!(
                serde_json::to_string(&back).unwrap(),
                serde_json::to_string(event_type).unwrap()
            );
        }
    }

    #[test]
    fn test_breakpoint_debug_format() {
        let bp = Breakpoint::on_action(3);
        let debug_str = format!("{:?}", bp);
        assert!(debug_str.contains("action_index: Some(3)"));
        assert!(debug_str.contains("has_condition: false"));
    }
}
