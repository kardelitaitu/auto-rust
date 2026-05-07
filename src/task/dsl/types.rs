//! DSL types for task definitions.
//!
//! This module provides the type definitions for DSL tasks:
//! - `TaskDefinition` - main task structure
//! - `Action` - action enum with all supported actions
//! - `Condition` - condition enum for control flow
//! - `ParameterDef`, `ParameterType` - parameter definitions
//! - `DurationMs` - wrapper type for u64 that supports dereferencing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TaskDefinition {
    /// Task name (must be unique)
    pub name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Policy name for timeout/permission configuration
    #[serde(default = "default_policy")]
    pub policy: String,
    /// Task parameters/inputs
    #[serde(default)]
    pub parameters: HashMap<String, ParameterDef>,
    /// Included task files to merge
    #[serde(default)]
    pub include: Vec<IncludeSpec>,
    /// Sequence of actions to execute
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Specification for including another task file.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct IncludeSpec {
    /// Path to the task file to include (relative or absolute)
    pub path: String,
    /// Optional condition for conditional inclusion
    #[serde(default)]
    pub condition: Option<String>,
}

fn default_policy() -> String {
    "default".to_string()
}

/// Parameter definition for task inputs.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ParameterDef {
    /// Parameter type
    #[serde(default)]
    pub r#type: ParameterType,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Default value (optional)
    pub default: Option<serde_yaml::Value>,
    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,
}

/// Parameter types supported by the DSL.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    #[default]
    String,
    Integer,
    Boolean,
    Url,
    Selector,
}

/// A single action/step in a task.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Navigate to a URL
    Navigate { url: String },
    /// Click an element
    Click { selector: String },
    /// Type text into an element
    Type { selector: String, text: String },
    /// Wait for a duration
    Wait { duration_ms: u64 },
    /// Wait for an element to be visible
    WaitFor {
        selector: String,
        timeout_ms: Option<u64>,
    },
    /// Scroll to an element
    ScrollTo { selector: String },
    /// Extract text from an element
    Extract {
        selector: String,
        variable: Option<String>,
    },
    /// Execute JavaScript
    Execute { script: String },
    /// Conditional action
    If {
        condition: Condition,
        then: Vec<Action>,
        r#else: Option<Vec<Action>>,
    },
    /// Loop over actions
    Loop {
        count: Option<u32>,
        condition: Option<Condition>,
        actions: Vec<Action>,
    },
    /// Call another task
    Call {
        task: String,
        parameters: Option<HashMap<String, serde_yaml::Value>>,
    },
    /// Log a message
    Log {
        message: String,
        level: Option<LogLevel>,
    },
    /// Capture a screenshot of the page
    Screenshot {
        /// Optional path to save the screenshot (defaults to auto-generated)
        path: Option<String>,
        /// Optional selector to screenshot specific element (defaults to full page)
        selector: Option<String>,
    },
    /// Clear an input field
    Clear { selector: String },
    /// Hover over an element
    Hover { selector: String },
    /// Select an option from a dropdown
    Select {
        selector: String,
        /// Value to select (use text or value attribute)
        value: String,
        /// Whether to select by visible text (default) or value attribute
        by_value: Option<bool>,
    },
    /// Right-click on an element
    RightClick { selector: String },
    /// Double-click on an element
    DoubleClick { selector: String },
    /// Execute actions in parallel
    Parallel {
        /// Actions to execute concurrently
        actions: Vec<Action>,
        /// Maximum number of concurrent actions (default: all at once)
        max_concurrency: Option<usize>,
    },
    /// Retry actions with exponential backoff
    Retry {
        /// Actions to retry on failure
        actions: Vec<Action>,
        /// Maximum number of retry attempts (default: 3)
        max_attempts: Option<u32>,
        /// Initial delay in milliseconds (default: 1000)
        initial_delay_ms: Option<u64>,
        /// Maximum delay in milliseconds (default: 30000)
        max_delay_ms: Option<u64>,
        /// Multiplier for exponential backoff (default: 2.0)
        backoff_multiplier: Option<f64>,
        /// Add random jitter to prevent thundering herd (default: true)
        jitter: Option<bool>,
        /// Only retry on specific error patterns (default: retry all)
        retry_on: Option<Vec<String>>,
    },
    /// Iterate over a collection with a loop variable
    Foreach {
        /// Variable name to bind each iteration value (e.g., "item", "index")
        variable: String,
        /// Collection to iterate over: array, range, or selector for DOM elements
        collection: ForeachCollection,
        /// Actions to execute for each iteration
        actions: Vec<Action>,
        /// Maximum number of iterations (default: 100, safety limit)
        max_iterations: Option<u32>,
    },
    /// While loop with condition-based execution
    While {
        /// Condition to evaluate before each iteration
        condition: Condition,
        /// Actions to execute while condition is true
        actions: Vec<Action>,
        /// Maximum number of iterations (default: 1000, safety limit)
        max_iterations: Option<u32>,
    },
    /// Try-catch-finally for error handling
    Try {
        /// Actions to attempt (try block)
        try_actions: Vec<Action>,
        /// Actions to execute on error (catch block)
        catch_actions: Option<Vec<Action>>,
        /// Variable name to store error message
        error_variable: Option<String>,
        /// Actions to always execute (finally block)
        finally_actions: Option<Vec<Action>>,
    },
}

/// Collection types for the Foreach action.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ForeachCollection {
    /// Array of values to iterate over
    Array { values: Vec<serde_yaml::Value> },
    /// Range of integers (inclusive start, exclusive end)
    Range { start: i64, end: i64 },
    /// DOM elements matching a selector
    Elements { selector: String },
    /// Use a variable containing an array
    Variable { name: String },
}

/// Log levels for the Log action.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Warn,
    Error,
}

/// Condition for conditional/loop actions.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Check if element exists
    ElementExists { selector: String },
    /// Check if element is visible
    ElementVisible { selector: String },
    /// Check text content equals value
    TextEquals { selector: String, value: String },
    /// Check text matches regex pattern
    TextMatches { selector: String, pattern: String },
    /// Check if variable equals value
    VariableEquals {
        name: String,
        value: serde_yaml::Value,
    },
    /// Check if variable matches regex pattern
    VariableMatches { name: String, pattern: String },
    /// Check if numeric variable is greater than value
    NumericGreaterThan { name: String, value: f64 },
    /// Check if numeric variable is less than value
    NumericLessThan { name: String, value: f64 },
    /// Check numeric variable is within range (inclusive)
    NumericRange { name: String, min: f64, max: f64 },
    /// Check if date string is before reference date
    DateBefore {
        name: String,
        date: String,
        format: Option<String>,
    },
    /// Check if date string is after reference date
    DateAfter {
        name: String,
        date: String,
        format: Option<String>,
    },
    /// Check if array variable contains a value
    ArrayContains {
        name: String,
        value: serde_yaml::Value,
    },
    /// Check if array variable length matches
    ArrayLength {
        name: String,
        min: Option<usize>,
        max: Option<usize>,
        exact: Option<usize>,
    },
    /// Logical AND of multiple conditions
    And { conditions: Vec<Condition> },
    /// Logical OR of multiple conditions
    Or { conditions: Vec<Condition> },
    /// Negate a condition
    Not { condition: Box<Condition> },
    /// Always true
    True,
    /// Always false
    False,
    /// Check if variable is defined
    VariableDefined { name: String },
    /// Check if variable is not defined
    VariableNotDefined { name: String },
}
