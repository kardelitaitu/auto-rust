//! Pre-flight Task Validation
//!
//! Validates task definitions before execution to catch errors early.
//! This module provides comprehensive static analysis of task files.

use std::collections::{HashMap, HashSet};

use crate::task::dsl::{Action, Condition, ForeachCollection, ParameterDef, TaskDefinition};

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

/// Comprehensive task validator.
pub struct TaskValidator {
    /// Maximum allowed recursion depth for nested actions
    max_nesting_depth: usize,
    /// Known task names for validating Call actions
    known_tasks: HashSet<String>,
    /// Parameters defined for this task
    parameters: HashMap<String, ParameterDef>,
    /// Current task name being validated (for circular reference detection)
    current_task: Option<String>,
}

impl TaskValidator {
    /// Create a new task validator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_nesting_depth: 10,
            known_tasks: HashSet::new(),
            parameters: HashMap::new(),
            current_task: None,
        }
    }

    /// Set the current task name (for circular reference detection).
    pub fn with_current_task(mut self, name: impl Into<String>) -> Self {
        self.current_task = Some(name.into());
        self
    }

    /// Set the maximum nesting depth.
    #[must_use]
    pub fn with_max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Register known tasks for validating Call actions.
    pub fn with_known_tasks(mut self, tasks: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.known_tasks = tasks.into_iter().map(std::convert::Into::into).collect();
        self
    }

    /// Add a parameter definition.
    pub fn with_parameter(mut self, name: impl Into<String>, param: ParameterDef) -> Self {
        self.parameters.insert(name.into(), param);
        self
    }

    /// Validate a complete task definition.
    ///
    /// Automatically sets the current task name for circular reference detection.
    #[must_use]
    pub fn validate(&self, def: &TaskDefinition) -> ValidationReport {
        let mut report = ValidationReport::new(def.name.clone());

        // Create a validator with current task name for circular detection
        let validator = if self.current_task.is_none() {
            TaskValidator {
                max_nesting_depth: self.max_nesting_depth,
                known_tasks: self.known_tasks.clone(),
                parameters: self.parameters.clone(),
                current_task: Some(def.name.clone()),
            }
        } else {
            TaskValidator {
                max_nesting_depth: self.max_nesting_depth,
                known_tasks: self.known_tasks.clone(),
                parameters: self.parameters.clone(),
                current_task: self.current_task.clone(),
            }
        };

        // Basic task structure validation
        validator.validate_task_structure(def, &mut report);

        // Validate all actions
        for (idx, action) in def.actions.iter().enumerate() {
            let path = format!("actions[{idx}]");
            validator.validate_action(action, &path, 0, &mut report);
        }

        report.action_count = validator.count_actions(&def.actions);

        report
    }

    /// Validate task structure (name, parameters, includes).
    fn validate_task_structure(&self, def: &TaskDefinition, report: &mut ValidationReport) {
        // Task name validation
        if def.name.is_empty() {
            report.error("Task name cannot be empty");
        } else if def.name.contains(' ') {
            report.error("Task name cannot contain spaces");
        } else if !def
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            report.warning(format!(
                "Task name '{}' contains special characters (recommend: alphanumeric, _, -)",
                def.name
            ));
        }

        // Must have at least one action or include
        if def.actions.is_empty() && def.include.is_empty() {
            report.error("Task must have at least one action or include");
        }

        // Validate parameters
        for (name, param) in &def.parameters {
            self.validate_parameter_def(name, param, report);
        }

        // Validate includes
        for (idx, include) in def.include.iter().enumerate() {
            if include.path.is_empty() {
                report.error(format!("include[{idx}]: Path cannot be empty"));
            }
        }
    }

    /// Validate a parameter definition.
    fn validate_parameter_def(
        &self,
        name: &str,
        param: &ParameterDef,
        report: &mut ValidationReport,
    ) {
        if name.is_empty() {
            report.error("Parameter name cannot be empty");
            return;
        }

        if name.contains(' ') {
            report.error(format!("Parameter '{name}' name cannot contain spaces"));
        }

        // Check for reasonable defaults
        if param.required && param.default.is_some() {
            report.warning(format!(
                "Parameter '{name}' is required but has a default value (redundant)"
            ));
        }

        if !param.required && param.default.is_none() {
            report.warning(format!(
                "Parameter '{name}' is optional but has no default (may cause issues)"
            ));
        }
    }

    /// Validate an action recursively.
    #[allow(clippy::unused_self)]
    fn validate_action(
        &self,
        action: &Action,
        path: &str,
        depth: usize,
        report: &mut ValidationReport,
    ) {
        // Check nesting depth
        if depth > self.max_nesting_depth {
            report.error(format!("{path}: Maximum nesting depth exceeded"));
            return;
        }

        match action {
            Action::Navigate { url } => {
                self.validate_url(url, path, report);
            }
            Action::Click { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::Type { selector, text } => {
                self.validate_selector(selector, path, report);
                self.validate_text(text, path, "text", report);
            }
            Action::Wait { duration_ms } => {
                if *duration_ms == 0 {
                    report.warning(format!("{path}: Wait duration is 0ms (no-op)"));
                } else if *duration_ms > 60000 {
                    report.warning(format!(
                        "{path}: Wait duration is {duration_ms}ms (> 60s), consider using a different approach"
                    ));
                }
            }
            Action::WaitFor {
                selector,
                timeout_ms,
            } => {
                self.validate_selector(selector, path, report);
                if let Some(timeout) = timeout_ms {
                    if *timeout == 0 {
                        report.error(format!("{path}: Timeout cannot be 0ms"));
                    } else if *timeout > 300000 {
                        report.warning(format!(
                            "{path}: Timeout is {timeout}ms (> 5min), consider a shorter timeout"
                        ));
                    }
                }
            }
            Action::ScrollTo { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::Extract { selector, variable } => {
                self.validate_selector(selector, path, report);
                if let Some(var) = variable {
                    if var.is_empty() {
                        report.error(format!("{path}: Variable name cannot be empty"));
                    } else {
                        report.variables_referenced.insert(var.clone());
                    }
                }
            }
            Action::Execute { script } => {
                if script.is_empty() {
                    report.warning(format!("{path}: Script is empty"));
                }
            }
            Action::Log { message, level: _ } => {
                if message.is_empty() {
                    report.warning(format!("{path}: Log message is empty"));
                }
                self.extract_variables(message, report);
            }
            Action::If {
                condition,
                then,
                r#else,
            } => {
                self.validate_condition(condition, path, report);

                if then.is_empty() {
                    report.warning(format!("{path}: 'then' block has no actions"));
                }
                for (idx, action) in then.iter().enumerate() {
                    self.validate_action(action, &format!("{path}.then[{idx}]"), depth + 1, report);
                }

                if let Some(else_actions) = r#else {
                    if else_actions.is_empty() {
                        report.warning(format!("{path}: 'else' block has no actions"));
                    }
                    for (idx, action) in else_actions.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{path}.else[{idx}]"),
                            depth + 1,
                            report,
                        );
                    }
                }
            }
            Action::Loop {
                count,
                condition,
                actions: loop_actions,
            } => {
                if let Some(c) = count {
                    if *c == 0 {
                        report.warning(format!("{path}: Loop count is 0 (no-op)"));
                    } else if *c > 10000 {
                        report.warning(format!(
                            "{path}: Loop count is {c} (> 10000), consider using a While loop"
                        ));
                    }
                }

                if let Some(cond) = condition {
                    self.validate_condition(cond, path, report);
                }

                if count.is_none() && condition.is_none() {
                    report.error(format!(
                        "{path}: Loop must have either 'count' or 'condition'"
                    ));
                }

                for (idx, action) in loop_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{path}.actions[{idx}]"),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::Call { task, parameters } => {
                if task.is_empty() {
                    report.error(format!("{path}: Task name cannot be empty"));
                } else {
                    // Check for direct circular reference (task calls itself)
                    if let Some(ref current) = self.current_task {
                        if task == current {
                            report.error(format!(
                                "{path}: Task '{task}' calls itself (circular reference)"
                            ));
                        }
                    }

                    // Check if task is in known list (if provided)
                    if !self.known_tasks.is_empty() && !self.known_tasks.contains(task) {
                        report.warning(format!(
                            "{path}: Task '{task}' is not in the known task list"
                        ));
                    }
                }

                // Extract variables from parameter values
                if let Some(params) = parameters {
                    for value in params.values() {
                        // Convert serde_yml::Value to string for variable extraction
                        if let Some(s) = value.as_str() {
                            self.extract_variables(s, report);
                        }
                    }
                }

                report.tasks_called.insert(task.clone());
            }
            Action::Screenshot {
                path: screenshot_path,
                selector,
            } => {
                if let Some(p) = screenshot_path {
                    if p.is_empty() {
                        report.warning(format!(
                            "{path}: Screenshot path is empty (will use auto-generated)"
                        ));
                    }
                    self.extract_variables(p, report);
                }
                if let Some(sel) = selector {
                    self.validate_selector(sel, path, report);
                }
            }
            Action::Clear { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::Hover { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::Select {
                selector,
                value,
                by_value: _,
            } => {
                self.validate_selector(selector, path, report);
                self.validate_text(value, path, "select value", report);
            }
            Action::RightClick { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::DoubleClick { selector } => {
                self.validate_selector(selector, path, report);
            }
            Action::Parallel {
                actions: parallel_actions,
                max_concurrency,
            } => {
                if parallel_actions.is_empty() {
                    report.warning(format!("{path}: Parallel block has no actions"));
                }

                if let Some(concurrency) = max_concurrency {
                    if *concurrency == 0 {
                        report.error(format!("{path}: max_concurrency cannot be 0"));
                    } else if *concurrency > parallel_actions.len() {
                        report.warning(format!(
                            "{}: max_concurrency ({}) > action count ({})",
                            path,
                            concurrency,
                            parallel_actions.len()
                        ));
                    }
                }

                for (idx, action) in parallel_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{path}.actions[{idx}]"),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::Retry {
                actions: retry_actions,
                max_attempts,
                initial_delay_ms,
                max_delay_ms,
                backoff_multiplier,
                jitter: _,
                retry_on,
            } => {
                if let Some(attempts) = max_attempts {
                    if *attempts == 0 {
                        report.error(format!("{path}: max_attempts cannot be 0"));
                    } else if *attempts > 100 {
                        report.warning(format!(
                            "{path}: max_attempts is {attempts} (> 100), consider if this is necessary"
                        ));
                    }
                }

                if let Some(delay) = initial_delay_ms {
                    if *delay == 0 {
                        report.warning(format!(
                            "{path}: initial_delay_ms is 0 (no delay between retries)"
                        ));
                    }
                }

                if let (Some(initial), Some(max)) = (initial_delay_ms, max_delay_ms) {
                    if *initial > *max {
                        report.error(format!(
                            "{path}: initial_delay_ms ({initial}) > max_delay_ms ({max})"
                        ));
                    }
                }

                if let Some(multiplier) = backoff_multiplier {
                    if *multiplier < 1.0 {
                        report.error(format!(
                            "{path}: backoff_multiplier ({multiplier}) < 1.0 (would decrease delay)"
                        ));
                    }
                }

                if let Some(patterns) = retry_on {
                    if patterns.is_empty() {
                        report.warning(format!(
                            "{path}: retry_on patterns are empty (will retry on all errors)"
                        ));
                    }
                }

                if retry_actions.is_empty() {
                    report.warning(format!("{path}: Retry block has no actions"));
                }

                for (idx, action) in retry_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{path}.actions[{idx}]"),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::Foreach {
                variable,
                collection,
                actions: foreach_actions,
                max_iterations,
            } => {
                if variable.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(variable.clone());
                }

                self.validate_collection(collection, path, report);

                if let Some(max) = max_iterations {
                    if *max == 0 {
                        report.error(format!("{path}: max_iterations cannot be 0"));
                    } else if *max > 10000 {
                        report.warning(format!("{path}: max_iterations is {max} (> 10000)"));
                    }
                }

                if foreach_actions.is_empty() {
                    report.warning(format!("{path}: Foreach block has no actions"));
                }

                for (idx, action) in foreach_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{path}.actions[{idx}]"),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::While {
                condition,
                actions: while_actions,
                max_iterations,
            } => {
                self.validate_condition(condition, path, report);

                if let Some(max) = max_iterations {
                    if *max == 0 {
                        report.error(format!("{path}: max_iterations cannot be 0"));
                    } else if *max > 100000 {
                        report.warning(format!(
                            "{path}: max_iterations is {max} (> 100000), this may run for a long time"
                        ));
                    }
                }

                if while_actions.is_empty() {
                    report.warning(format!("{path}: While block has no actions"));
                }

                for (idx, action) in while_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{path}.actions[{idx}]"),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::Try {
                try_actions,
                catch_actions,
                error_variable,
                finally_actions,
            } => {
                if try_actions.is_empty() {
                    report.warning(format!("{path}: Try block has no actions"));
                }

                for (idx, action) in try_actions.iter().enumerate() {
                    self.validate_action(action, &format!("{path}.try[{idx}]"), depth + 1, report);
                }

                if let Some(catch) = catch_actions {
                    if catch.is_empty() {
                        report.warning(format!("{path}: Catch block has no actions"));
                    }
                    for (idx, action) in catch.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{path}.catch[{idx}]"),
                            depth + 1,
                            report,
                        );
                    }
                }

                if let Some(var) = error_variable {
                    if var.is_empty() {
                        report.error(format!("{path}: Error variable name cannot be empty"));
                    } else {
                        report.variables_referenced.insert(var.clone());
                    }
                }

                if let Some(finally) = finally_actions {
                    if finally.is_empty() {
                        report.warning(format!("{path}: Finally block has no actions"));
                    }
                    for (idx, action) in finally.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{path}.finally[{idx}]"),
                            depth + 1,
                            report,
                        );
                    }
                }
            }
        }
    }

    /// Validate a condition.
    fn validate_condition(&self, condition: &Condition, path: &str, report: &mut ValidationReport) {
        match condition {
            Condition::ElementExists { selector } | Condition::ElementVisible { selector } => {
                self.validate_selector(selector, path, report);
            }
            Condition::TextEquals { selector, value } => {
                self.validate_selector(selector, path, report);
                if value.is_empty() {
                    report.warning(format!("{path}: TextEquals condition has empty value"));
                }
            }
            Condition::VariableEquals { name, value } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                // Convert serde_yml::Value to string for variable extraction
                if let Some(s) = value.as_str() {
                    self.extract_variables(s, report);
                }
            }
            Condition::TextMatches { selector, pattern } => {
                self.validate_selector(selector, path, report);
                if pattern.is_empty() {
                    report.warning(format!("{path}: TextMatches pattern is empty"));
                } else if regex::Regex::new(pattern).is_err() {
                    report.error(format!("{path}: TextMatches pattern is invalid regex"));
                }
            }
            Condition::VariableMatches { name, pattern } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                if pattern.is_empty() {
                    report.warning(format!("{path}: VariableMatches pattern is empty"));
                } else if regex::Regex::new(pattern).is_err() {
                    report.error(format!("{path}: VariableMatches pattern is invalid regex"));
                }
            }
            Condition::NumericGreaterThan { name, value: _ }
            | Condition::NumericLessThan { name, value: _ } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
            }
            Condition::NumericRange { name, min, max } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                if min > max {
                    report.warning(format!(
                        "{path}: NumericRange min ({min}) is greater than max ({max})"
                    ));
                }
            }
            Condition::DateBefore { name, date, format }
            | Condition::DateAfter { name, date, format } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                if date.is_empty() {
                    report.warning(format!("{path}: Date comparison date is empty"));
                }
                if let Some(fmt) = format {
                    if fmt.is_empty() {
                        report.warning(format!("{path}: Date format is empty (will use default)"));
                    }
                }
            }
            Condition::ArrayContains { name, value } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                if let Some(s) = value.as_str() {
                    self.extract_variables(s, report);
                }
            }
            Condition::ArrayLength {
                name,
                min,
                max,
                exact,
            } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                // Validate that at least one constraint is provided
                if min.is_none() && max.is_none() && exact.is_none() {
                    report.warning(format!(
                        "{path}: ArrayLength has no constraints (min/max/exact)"
                    ));
                }
                // Validate range consistency
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        report.warning(format!(
                            "{path}: ArrayLength min ({min_val}) is greater than max ({max_val})"
                        ));
                    }
                }
            }
            Condition::Not { condition: inner } => {
                self.validate_condition(inner, &format!("{path}[not]"), report);
            }
            Condition::And { conditions } | Condition::Or { conditions } => {
                for (idx, cond) in conditions.iter().enumerate() {
                    self.validate_condition(cond, &format!("{path}[{idx}]"), report);
                }
            }
            Condition::True | Condition::False => {
                // Always true or false, nothing to validate
            }
            Condition::VariableDefined { name } | Condition::VariableNotDefined { name } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
            }
        }
    }

    /// Validate a collection (for Foreach).
    fn validate_collection(
        &self,
        collection: &ForeachCollection,
        path: &str,
        report: &mut ValidationReport,
    ) {
        match collection {
            ForeachCollection::Array { values } => {
                if values.is_empty() {
                    report.warning(format!("{path}: Array collection is empty (no iterations)"));
                }
            }
            ForeachCollection::Range { start, end } => {
                if start >= end {
                    report.error(format!("{path}: Range start ({start}) >= end ({end})"));
                }
                if *end - *start > 10000 {
                    report.warning(format!(
                        "{}: Range has {} items (> 10000)",
                        path,
                        end - start
                    ));
                }
            }
            ForeachCollection::Elements { selector } => {
                self.validate_selector(selector, path, report);
            }
            ForeachCollection::Variable { name } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
            }
        }
    }

    /// Validate a URL string (with variable support).
    fn validate_url(&self, url: &str, path: &str, report: &mut ValidationReport) {
        if url.is_empty() {
            report.error(format!("{path}: URL cannot be empty"));
            return;
        }

        self.extract_variables(url, report);

        // If no variables, try to validate URL format
        if !url.contains("${") && !url.starts_with("http://") && !url.starts_with("https://") {
            report.warning(format!(
                "{path}: URL '{url}' does not start with http:// or https://"
            ));
        }
    }

    /// Validate a CSS selector (with variable support).
    fn validate_selector(&self, selector: &str, path: &str, report: &mut ValidationReport) {
        if selector.is_empty() {
            report.error(format!("{path}: Selector cannot be empty"));
            return;
        }

        self.extract_variables(selector, report);

        // Basic CSS selector validation (only if no variables)
        if !selector.contains("${") {
            // Check for common CSS selector issues
            if selector.contains("  ") {
                report.warning(format!(
                    "{path}: Selector contains multiple consecutive spaces"
                ));
            }

            // Check for balanced brackets
            let open_brackets = selector.matches('[').count();
            let close_brackets = selector.matches(']').count();
            if open_brackets != close_brackets {
                report.error(format!(
                    "{path}: Selector has unbalanced brackets: '{selector}'"
                ));
            }

            // Check for balanced parentheses
            let open_parens = selector.matches('(').count();
            let close_parens = selector.matches(')').count();
            if open_parens != close_parens {
                report.error(format!(
                    "{path}: Selector has unbalanced parentheses: '{selector}'"
                ));
            }

            // Check for balanced quotes
            let single_quotes = selector.matches('\'').count();
            let double_quotes = selector.matches('"').count();
            if !single_quotes.is_multiple_of(2) {
                report.error(format!(
                    "{path}: Selector has unbalanced single quotes: '{selector}'"
                ));
            }
            if !double_quotes.is_multiple_of(2) {
                report.error(format!(
                    "{path}: Selector has unbalanced double quotes: '{selector}'"
                ));
            }
        }
    }

    /// Validate text content.
    fn validate_text(&self, text: &str, path: &str, context: &str, report: &mut ValidationReport) {
        self.extract_variables(text, report);

        if text.is_empty() {
            report.warning(format!("{path}: {context} is empty"));
        }
    }

    /// Extract variable references from a string (e.g., "${variable}").
    fn extract_variables(&self, text: &str, report: &mut ValidationReport) {
        // Find all ${...} patterns
        let mut start = 0;
        while let Some(idx) = text[start..].find("${") {
            let var_start = start + idx + 2;
            if let Some(end_idx) = text[var_start..].find('}') {
                let var_name = &text[var_start..var_start + end_idx];
                if !var_name.is_empty() {
                    report.variables_referenced.insert(var_name.to_string());
                }
                start = var_start + end_idx + 1;
            } else {
                break;
            }
        }
    }

    /// Count total actions recursively.
    #[allow(clippy::unused_self)]
    fn count_actions(&self, actions: &[Action]) -> usize {
        let mut count = actions.len();

        for action in actions {
            match action {
                Action::If { then, r#else, .. } => {
                    count += self.count_actions(then);
                    if let Some(else_actions) = r#else {
                        count += self.count_actions(else_actions);
                    }
                }
                Action::Loop {
                    actions: loop_actions,
                    ..
                } => {
                    count += self.count_actions(loop_actions);
                }
                Action::Call { .. } => {
                    // Count as 1, actual size depends on called task
                }
                Action::Parallel {
                    actions: parallel_actions,
                    ..
                } => {
                    count += self.count_actions(parallel_actions);
                }
                Action::Retry {
                    actions: retry_actions,
                    ..
                } => {
                    count += self.count_actions(retry_actions);
                }
                Action::Foreach {
                    actions: foreach_actions,
                    ..
                } => {
                    count += self.count_actions(foreach_actions);
                }
                Action::While {
                    actions: while_actions,
                    ..
                } => {
                    count += self.count_actions(while_actions);
                }
                Action::Try {
                    try_actions,
                    catch_actions,
                    finally_actions,
                    ..
                } => {
                    count += self.count_actions(try_actions);
                    if let Some(catch) = catch_actions {
                        count += self.count_actions(catch);
                    }
                    if let Some(finally) = finally_actions {
                        count += self.count_actions(finally);
                    }
                }
                _ => {}
            }
        }

        count
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_basic_task() -> TaskDefinition {
        TaskDefinition {
            name: "test_task".to_string(),
            description: "Test task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Wait { duration_ms: 100 }],
        }
    }

    #[test]
    fn test_validate_empty_task_name() {
        let mut task = create_basic_task();
        task.name = "".to_string();

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("name cannot be empty")));
    }

    #[test]
    fn test_validate_task_name_with_spaces() {
        let mut task = create_basic_task();
        task.name = "test task".to_string();

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report.issues.iter().any(|i| i.message().contains("spaces")));
    }

    #[test]
    fn test_validate_empty_actions() {
        let mut task = create_basic_task();
        task.actions = vec![];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("at least one action")));
    }

    #[test]
    fn test_validate_empty_selector() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Click {
            selector: "".to_string(),
        }];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("Selector cannot be empty")));
    }

    #[test]
    fn test_validate_unbalanced_selector() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Click {
            selector: "div[class='test'".to_string(),
        }];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("unbalanced")));
    }

    #[test]
    fn test_validate_zero_wait_duration() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Wait { duration_ms: 0 }];

        let report = validate_task(&task);
        // Warning, not error
        assert!(report.is_valid());
        assert!(report.issues.iter().any(|i| i.message().contains("0ms")));
    }

    #[test]
    fn test_validate_valid_task() {
        let task = create_basic_task();

        let report = validate_task(&task);
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn test_validate_if_empty_then() {
        let mut task = create_basic_task();
        task.actions = vec![Action::If {
            condition: Condition::ElementExists {
                selector: "div".to_string(),
            },
            then: vec![],
            r#else: None,
        }];

        let report = validate_task(&task);
        assert!(report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("'then' block has no actions")));
    }

    #[test]
    fn test_validate_call_unknown_task() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Call {
            task: "unknown_task".to_string(),
            parameters: None,
        }];

        let known: HashSet<String> = vec!["known_task".to_string()].into_iter().collect();
        let report = TaskValidator::new().with_known_tasks(known).validate(&task);

        assert!(report.is_valid()); // Warning only
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("not in the known task list")));
    }

    #[test]
    fn test_validate_loop_without_count_or_condition() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Loop {
            count: None,
            condition: None,
            actions: vec![Action::Wait { duration_ms: 100 }],
        }];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("must have either")));
    }

    #[test]
    fn test_validate_retry_zero_attempts() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Retry {
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_attempts: Some(0),
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        }];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("cannot be 0")));
    }

    #[test]
    fn test_validate_foreach_invalid_range() {
        let mut task = create_basic_task();
        task.actions = vec![Action::Foreach {
            variable: "i".to_string(),
            collection: ForeachCollection::Range { start: 10, end: 5 },
            actions: vec![Action::Wait { duration_ms: 100 }],
            max_iterations: None,
        }];

        let report = validate_task(&task);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("start") && i.message().contains("end")));
    }

    #[test]
    fn test_extract_variables() {
        let validator = TaskValidator::new();
        let mut report = ValidationReport::new("test".to_string());

        validator.extract_variables("Hello ${name}, your id is ${id}", &mut report);

        assert!(report.variables_referenced.contains("name"));
        assert!(report.variables_referenced.contains("id"));
    }

    #[test]
    fn test_count_actions() {
        let validator = TaskValidator::new();

        let actions = vec![
            Action::Wait { duration_ms: 100 },
            Action::If {
                condition: Condition::ElementExists {
                    selector: "div".to_string(),
                },
                then: vec![
                    Action::Click {
                        selector: "button".to_string(),
                    },
                    Action::Wait { duration_ms: 500 },
                ],
                r#else: Some(vec![Action::Wait { duration_ms: 200 }]),
            },
        ];

        let count = validator.count_actions(&actions);
        assert_eq!(count, 5); // 1 Wait + 1 If + 2 in 'then' + 1 in 'else'
    }

    #[test]
    fn test_circular_reference_self_call() {
        // Task that calls itself (direct circular reference)
        let task = TaskDefinition {
            name: "self_calling".to_string(),
            description: "Calls itself".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Call {
                task: "self_calling".to_string(), // Calls itself!
                parameters: None,
            }],
        };

        let report = validate_task(&task);

        assert!(!report.is_valid(), "Self-calling task should be invalid");
        assert!(
            report.issues.iter().any(|i| {
                i.message().contains("circular reference") || i.message().contains("calls itself")
            }),
            "Should report circular reference error"
        );
    }

    #[test]
    fn test_no_false_circular_positive() {
        // Task that calls a DIFFERENT task (not circular)
        let task = TaskDefinition {
            name: "caller".to_string(),
            description: "Calls another task".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![Action::Call {
                task: "callee".to_string(), // Different task name
                parameters: None,
            }],
        };

        let report = validate_task(&task);

        // Should NOT have circular reference error
        assert!(!report.issues.iter().any(|i| {
            i.message().contains("circular reference") || i.message().contains("calls itself")
        }));
    }

    #[test]
    fn test_deep_nesting_limit() {
        // Create deeply nested If actions
        fn create_nested_if(depth: usize) -> Action {
            if depth == 0 {
                Action::Wait { duration_ms: 100 }
            } else {
                Action::If {
                    condition: Condition::ElementExists {
                        selector: format!("#level{}", depth),
                    },
                    then: vec![create_nested_if(depth - 1)],
                    r#else: None,
                }
            }
        }

        // Task with 12 levels of nesting (exceeds default limit of 10)
        let task = TaskDefinition {
            name: "deep_nested".to_string(),
            description: "Very deeply nested".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![create_nested_if(12)],
        };

        let report = validate_task(&task);

        assert!(!report.is_valid(), "Should fail due to nesting depth");
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.message().contains("nesting depth")),
            "Should report nesting depth error"
        );
    }

    #[test]
    fn test_custom_nesting_limit() {
        // Task with 8 levels of nesting
        fn create_nested_if(depth: usize) -> Action {
            if depth == 0 {
                Action::Wait { duration_ms: 100 }
            } else {
                Action::If {
                    condition: Condition::ElementExists {
                        selector: format!("#level{}", depth),
                    },
                    then: vec![create_nested_if(depth - 1)],
                    r#else: None,
                }
            }
        }

        let task = TaskDefinition {
            name: "medium_nested".to_string(),
            description: "Medium nesting".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![create_nested_if(8)], // 8 levels
        };

        // With default limit of 10, should pass
        let report = TaskValidator::new().validate(&task);
        assert!(report.is_valid(), "8 levels should pass with limit of 10");

        // With custom limit of 5, should fail
        let report = TaskValidator::new()
            .with_max_nesting_depth(5)
            .validate(&task);
        assert!(!report.is_valid(), "8 levels should fail with limit of 5");
    }

    #[test]
    fn test_multiple_call_actions_tracked() {
        // Task that calls multiple other tasks
        let task = TaskDefinition {
            name: "multi_caller".to_string(),
            description: "Calls multiple tasks".to_string(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions: vec![
                Action::Call {
                    task: "task_a".to_string(),
                    parameters: None,
                },
                Action::Call {
                    task: "task_b".to_string(),
                    parameters: None,
                },
                Action::Call {
                    task: "task_c".to_string(),
                    parameters: None,
                },
            ],
        };

        let report = validate_task(&task);

        assert!(report.tasks_called.contains("task_a"));
        assert!(report.tasks_called.contains("task_b"));
        assert!(report.tasks_called.contains("task_c"));
        assert_eq!(report.tasks_called.len(), 3);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn issue_error_message() {
        let e = ValidationIssue::Error("x".into());
        assert_eq!(e.message(), "x");
        assert!(e.is_error());
    }
    #[test]
    fn issue_warning_message() {
        let w = ValidationIssue::Warning("y".into());
        assert_eq!(w.message(), "y");
        assert!(!w.is_error());
    }
    #[test]
    fn report_new_empty() {
        let r = ValidationReport::new("t".into());
        assert!(r.is_valid());
        assert_eq!(r.error_count(), 0);
        assert_eq!(r.warning_count(), 0);
    }
    #[test]
    fn report_add_error() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        assert!(!r.is_valid());
        assert!(r.has_errors());
        assert_eq!(r.error_count(), 1);
    }
    #[test]
    fn report_add_warning() {
        let mut r = ValidationReport::new("t".into());
        r.warning("w");
        assert!(r.is_valid());
        assert_eq!(r.warning_count(), 1);
    }
    #[test]
    fn report_mixed_counts() {
        let mut r = ValidationReport::new("t".into());
        r.error("e1");
        r.error("e2");
        r.warning("w");
        assert_eq!(r.error_count(), 2);
        assert_eq!(r.warning_count(), 1);
    }
    #[test]
    fn report_summary_valid() {
        let r = ValidationReport::new("t".into());
        assert!(r.summary().contains("is valid"));
    }
    #[test]
    fn report_summary_warnings() {
        let mut r = ValidationReport::new("t".into());
        r.warning("w");
        assert!(r.summary().contains("warning(s)"));
    }
    #[test]
    fn report_summary_errors() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        assert!(r.summary().contains("error(s)"));
    }
    #[test]
    fn report_summary_mixed() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        r.warning("w");
        assert!(r.summary().contains("error(s)"));
        assert!(r.summary().contains("warning(s)"));
    }
    #[test]
    fn report_action_count_default() {
        let r = ValidationReport::new("t".into());
        assert_eq!(r.action_count, 0);
    }
    #[test]
    fn report_variables_empty() {
        let r = ValidationReport::new("t".into());
        assert!(r.variables_referenced.is_empty());
    }
    #[test]
    fn report_tasks_called_empty() {
        let r = ValidationReport::new("t".into());
        assert!(r.tasks_called.is_empty());
    }
    #[test]
    fn issue_partial_eq_error() {
        let a = ValidationIssue::Error("z".into());
        let b = ValidationIssue::Error("z".into());
        assert_eq!(a, b);
    }
    #[test]
    fn issue_partial_eq_warning() {
        let a = ValidationIssue::Warning("z".into());
        let b = ValidationIssue::Warning("z".into());
        assert_eq!(a, b);
    }
    #[test]
    fn report_clone() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        let c = r.clone();
        assert_eq!(c.error_count(), 1);
    }
    #[test]
    fn report_debug() {
        let r = ValidationReport::new("t".into());
        let _ = format!("{:?}", r);
    }
    #[test]
    fn issue_debug() {
        let e = ValidationIssue::Error("d".into());
        let _ = format!("{:?}", e);
    }
    #[test]
    fn report_multiple_warnings() {
        let mut r = ValidationReport::new("t".into());
        for i in 0..3 {
            r.warning(i.to_string());
        }
        assert_eq!(r.warning_count(), 3);
    }
    #[test]
    fn report_set_action_count_direct() {
        let mut r = ValidationReport::new("t".into());
        r.action_count = 42;
        assert_eq!(r.action_count, 42);
    }
    #[test]
    fn report_insert_variable() {
        let mut r = ValidationReport::new("t".into());
        r.variables_referenced.insert("v1".into());
        assert_eq!(r.variables_referenced.len(), 1);
    }
    #[test]
    fn report_insert_task_call() {
        let mut r = ValidationReport::new("t".into());
        r.tasks_called.insert("sub".into());
        assert!(r.tasks_called.contains("sub"));
    }
    #[test]
    fn issue_error_vs_warning_ne() {
        let e = ValidationIssue::Error("x".into());
        let w = ValidationIssue::Warning("x".into());
        assert_ne!(e, w);
    }
    #[test]
    fn report_name_preserved() {
        let r = ValidationReport::new("my_task".into());
        assert_eq!(r.task_name, "my_task");
    }
    #[test]
    fn report_error_twice() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        r.error("e2");
        assert_eq!(r.error_count(), 2);
    }
    #[test]
    fn report_warning_twice() {
        let mut r = ValidationReport::new("t".into());
        r.warning("w1");
        r.warning("w2");
        assert_eq!(r.warning_count(), 2);
    }
    #[test]
    fn report_zero_actions_summary() {
        let r = ValidationReport::new("t".into());
        assert!(r.summary().contains("0 actions"));
    }
    #[test]
    fn report_one_action_summary() {
        let mut r = ValidationReport::new("t".into());
        r.action_count = 1;
        assert!(r.summary().contains("1 actions"));
    }
    #[test]
    fn report_large_error_count() {
        let mut r = ValidationReport::new("t".into());
        for _ in 0..10 {
            r.error("e");
        }
        assert_eq!(r.error_count(), 10);
    }
    #[test]
    fn report_large_warning_count() {
        let mut r = ValidationReport::new("t".into());
        for _ in 0..10 {
            r.warning("w");
        }
        assert_eq!(r.warning_count(), 10);
    }
    #[test]
    fn issue_message_long() {
        let e = ValidationIssue::Error("a".repeat(100));
        assert_eq!(e.message().len(), 100);
    }
    #[test]
    fn report_clone_independent() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        let mut c = r.clone();
        c.warning("w");
        assert_eq!(r.warning_count(), 0);
        assert_eq!(c.warning_count(), 1);
    }
    #[test]
    fn report_debug_contains_name() {
        let r = ValidationReport::new("debug_t".into());
        assert!(format!("{:?}", r).contains("debug_t"));
    }
    #[test]
    fn issue_partial_eq_diff_msg() {
        let a = ValidationIssue::Error("1".into());
        let b = ValidationIssue::Error("2".into());
        assert_ne!(a, b);
    }
    #[test]
    fn report_empty_issues_vec() {
        let r = ValidationReport::new("t".into());
        assert!(r.issues.is_empty());
    }
    #[test]
    fn report_issues_len_after_add() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        r.warning("w");
        assert_eq!(r.issues.len(), 2);
    }
    #[test]
    fn report_is_valid_after_only_warnings() {
        let mut r = ValidationReport::new("t".into());
        r.warning("w");
        assert!(r.is_valid());
    }
    #[test]
    fn report_has_errors_false_initial() {
        let r = ValidationReport::new("t".into());
        assert!(!r.has_errors());
    }
    #[test]
    fn report_has_errors_true() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        assert!(r.has_errors());
    }
    #[test]
    fn report_variables_len() {
        let mut r = ValidationReport::new("t".into());
        r.variables_referenced.insert("x".into());
        r.variables_referenced.insert("y".into());
        assert_eq!(r.variables_referenced.len(), 2);
    }
    #[test]
    fn report_tasks_len() {
        let mut r = ValidationReport::new("t".into());
        r.tasks_called.insert("a".into());
        r.tasks_called.insert("b".into());
        assert_eq!(r.tasks_called.len(), 2);
    }
    #[test]
    fn issue_message_empty() {
        let e = ValidationIssue::Error("".into());
        assert_eq!(e.message(), "");
    }
    #[test]
    fn report_name_change() {
        let mut r = ValidationReport::new("old".into());
        r.task_name = "new".into();
        assert_eq!(r.task_name, "new");
    }
    #[test]
    fn report_summary_contains_task_name() {
        let r = ValidationReport::new("special".into());
        assert!(r.summary().contains("special"));
    }
    #[test]
    fn report_multiple_errors_summary() {
        let mut r = ValidationReport::new("t".into());
        r.error("e1");
        r.error("e2");
        assert!(r.summary().contains("2 error(s)"));
    }
    #[test]
    fn report_action_and_error_mix() {
        let mut r = ValidationReport::new("t".into());
        r.action_count = 5;
        r.error("e");
        assert!(r.summary().contains("5 actions"));
    }
    #[test]
    fn issue_clone() {
        let e = ValidationIssue::Error("c".into());
        let c = e.clone();
        assert_eq!(e, c);
    }
    #[test]
    fn report_default_action_zero() {
        let r = ValidationReport::new("t".into());
        assert_eq!(r.action_count, 0);
    }
    #[test]
    fn report_issues_push_direct() {
        let mut r = ValidationReport::new("t".into());
        r.issues.push(ValidationIssue::Error("p".into()));
        assert_eq!(r.issues.len(), 1);
    }
    #[test]
    fn report_variables_clear() {
        let mut r = ValidationReport::new("t".into());
        r.variables_referenced.insert("v".into());
        r.variables_referenced.clear();
        assert!(r.variables_referenced.is_empty());
    }
    #[test]
    fn report_tasks_clear() {
        let mut r = ValidationReport::new("t".into());
        r.tasks_called.insert("t".into());
        r.tasks_called.clear();
        assert!(r.tasks_called.is_empty());
    }
    #[test]
    fn issue_eq_self() {
        let e = ValidationIssue::Error("s".into());
        assert_eq!(e, e);
    }
    #[test]
    fn report_eq_name_only() {
        let r1 = ValidationReport::new("same".into());
        let r2 = ValidationReport::new("same".into());
        assert_eq!(r1.task_name, r2.task_name);
    }
    #[test]
    fn report_summary_no_warnings_errors() {
        let r = ValidationReport::new("t".into());
        let s = r.summary();
        assert!(!s.contains("error"));
        assert!(!s.contains("warning"));
    }
    #[test]
    fn report_error_count_zero() {
        let r = ValidationReport::new("t".into());
        assert_eq!(r.error_count(), 0);
    }
    #[test]
    fn report_warning_count_zero() {
        let r = ValidationReport::new("t".into());
        assert_eq!(r.warning_count(), 0);
    }
    #[test]
    fn report_issues_retain_errors() {
        let mut r = ValidationReport::new("t".into());
        r.error("e");
        r.warning("w");
        r.issues.retain(|i| i.is_error());
        assert_eq!(r.issues.len(), 1);
    }
    #[test]
    fn issue_message_unicode() {
        let e = ValidationIssue::Error(" café ".into());
        assert!(e.message().contains("café"));
    }
    #[test]
    fn report_tasks_contains_after_insert() {
        let mut r = ValidationReport::new("t".into());
        r.tasks_called.insert("subtask".into());
        assert!(r.tasks_called.contains("subtask"));
    }
    #[test]
    fn report_variables_contains() {
        let mut r = ValidationReport::new("t".into());
        r.variables_referenced.insert("varX".into());
        assert!(r.variables_referenced.contains("varX"));
    }
    #[test]
    fn report_summary_format_check() {
        let mut r = ValidationReport::new("fmt".into());
        r.error("e");
        let s = r.summary();
        assert!(s.starts_with("Task 'fmt'"));
    }
    #[test]
    fn report_new_with_special_chars() {
        let r = ValidationReport::new("t@#$".into());
        assert_eq!(r.task_name, "t@#$");
    }
    #[test]
    fn issue_warning_eq() {
        let w1 = ValidationIssue::Warning("w".into());
        let w2 = ValidationIssue::Warning("w".into());
        assert_eq!(w1, w2);
    }
    #[test]
    fn report_issues_iter_errors() {
        let mut r = ValidationReport::new("t".into());
        r.error("e1");
        r.error("e2");
        let errs: Vec<_> = r.issues.iter().filter(|i| i.is_error()).collect();
        assert_eq!(errs.len(), 2);
    }
    #[test]
    fn report_action_increment() {
        let mut r = ValidationReport::new("t".into());
        r.action_count += 1;
        assert_eq!(r.action_count, 1);
    }
    #[test]
    fn report_variables_insert_many() {
        let mut r = ValidationReport::new("t".into());
        ["a", "b", "c"].iter().for_each(|v| {
            r.variables_referenced.insert(v.to_string());
        });
        assert_eq!(r.variables_referenced.len(), 3);
    }
    #[test]
    fn report_debug_not_empty() {
        let r = ValidationReport::new("d".into());
        assert!(!format!("{:?}", r).is_empty());
    }
    #[test]
    fn issue_debug_not_empty() {
        let e = ValidationIssue::Error("d".into());
        assert!(!format!("{:?}", e).is_empty());
    }
    #[test]
    fn report_has_no_warnings_initial() {
        let r = ValidationReport::new("t".into());
        assert_eq!(r.warning_count(), 0);
    }
}
