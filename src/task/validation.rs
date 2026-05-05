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
    pub fn message(&self) -> &str {
        match self {
            ValidationIssue::Error(msg) | ValidationIssue::Warning(msg) => msg,
        }
    }

    /// Check if this is an error.
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
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.is_error())
    }

    /// Get error count.
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.is_error()).count()
    }

    /// Get warning count.
    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| !i.is_error()).count()
    }

    /// Get a human-readable summary.
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
    pub fn with_max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Register known tasks for validating Call actions.
    pub fn with_known_tasks(mut self, tasks: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.known_tasks = tasks.into_iter().map(|t| t.into()).collect();
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
            let path = format!("actions[{}]", idx);
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
                report.error(format!("include[{}]: Path cannot be empty", idx));
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
            report.error(format!("Parameter '{}' name cannot contain spaces", name));
        }

        // Check for reasonable defaults
        if param.required && param.default.is_some() {
            report.warning(format!(
                "Parameter '{}' is required but has a default value (redundant)",
                name
            ));
        }

        if !param.required && param.default.is_none() {
            report.warning(format!(
                "Parameter '{}' is optional but has no default (may cause issues)",
                name
            ));
        }
    }

    /// Validate an action recursively.
    fn validate_action(
        &self,
        action: &Action,
        path: &str,
        depth: usize,
        report: &mut ValidationReport,
    ) {
        // Check nesting depth
        if depth > self.max_nesting_depth {
            report.error(format!("{}: Maximum nesting depth exceeded", path));
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
                    report.warning(format!("{}: Wait duration is 0ms (no-op)", path));
                } else if *duration_ms > 60000 {
                    report.warning(format!(
                        "{}: Wait duration is {}ms (> 60s), consider using a different approach",
                        path, duration_ms
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
                        report.error(format!("{}: Timeout cannot be 0ms", path));
                    } else if *timeout > 300000 {
                        report.warning(format!(
                            "{}: Timeout is {}ms (> 5min), consider a shorter timeout",
                            path, timeout
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
                        report.error(format!("{}: Variable name cannot be empty", path));
                    } else {
                        report.variables_referenced.insert(var.clone());
                    }
                }
            }
            Action::Execute { script } => {
                if script.is_empty() {
                    report.warning(format!("{}: Script is empty", path));
                }
            }
            Action::Log { message, level: _ } => {
                if message.is_empty() {
                    report.warning(format!("{}: Log message is empty", path));
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
                    report.warning(format!("{}: 'then' block has no actions", path));
                }
                for (idx, action) in then.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.then[{}]", path, idx),
                        depth + 1,
                        report,
                    );
                }

                if let Some(else_actions) = r#else {
                    if else_actions.is_empty() {
                        report.warning(format!("{}: 'else' block has no actions", path));
                    }
                    for (idx, action) in else_actions.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{}.else[{}]", path, idx),
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
                        report.warning(format!("{}: Loop count is 0 (no-op)", path));
                    } else if *c > 10000 {
                        report.warning(format!(
                            "{}: Loop count is {} (> 10000), consider using a While loop",
                            path, c
                        ));
                    }
                }

                if let Some(cond) = condition {
                    self.validate_condition(cond, path, report);
                }

                if count.is_none() && condition.is_none() {
                    report.error(format!(
                        "{}: Loop must have either 'count' or 'condition'",
                        path
                    ));
                }

                for (idx, action) in loop_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.actions[{}]", path, idx),
                        depth + 1,
                        report,
                    );
                }
            }
            Action::Call { task, parameters } => {
                if task.is_empty() {
                    report.error(format!("{}: Task name cannot be empty", path));
                } else {
                    // Check for direct circular reference (task calls itself)
                    if let Some(ref current) = self.current_task {
                        if task == current {
                            report.error(format!(
                                "{}: Task '{}' calls itself (circular reference)",
                                path, task
                            ));
                        }
                    }

                    // Check if task is in known list (if provided)
                    if !self.known_tasks.is_empty() && !self.known_tasks.contains(task) {
                        report.warning(format!(
                            "{}: Task '{}' is not in the known task list",
                            path, task
                        ));
                    }
                }

                // Extract variables from parameter values
                if let Some(params) = parameters {
                    for value in params.values() {
                        // Convert serde_yaml::Value to string for variable extraction
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
                            "{}: Screenshot path is empty (will use auto-generated)",
                            path
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
                    report.warning(format!("{}: Parallel block has no actions", path));
                }

                if let Some(concurrency) = max_concurrency {
                    if *concurrency == 0 {
                        report.error(format!("{}: max_concurrency cannot be 0", path));
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
                        &format!("{}.actions[{}]", path, idx),
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
                        report.error(format!("{}: max_attempts cannot be 0", path));
                    } else if *attempts > 100 {
                        report.warning(format!(
                            "{}: max_attempts is {} (> 100), consider if this is necessary",
                            path, attempts
                        ));
                    }
                }

                if let Some(delay) = initial_delay_ms {
                    if *delay == 0 {
                        report.warning(format!(
                            "{}: initial_delay_ms is 0 (no delay between retries)",
                            path
                        ));
                    }
                }

                if let (Some(initial), Some(max)) = (initial_delay_ms, max_delay_ms) {
                    if *initial > *max {
                        report.error(format!(
                            "{}: initial_delay_ms ({}) > max_delay_ms ({})",
                            path, initial, max
                        ));
                    }
                }

                if let Some(multiplier) = backoff_multiplier {
                    if *multiplier < 1.0 {
                        report.error(format!(
                            "{}: backoff_multiplier ({}) < 1.0 (would decrease delay)",
                            path, multiplier
                        ));
                    }
                }

                if let Some(patterns) = retry_on {
                    if patterns.is_empty() {
                        report.warning(format!(
                            "{}: retry_on patterns are empty (will retry on all errors)",
                            path
                        ));
                    }
                }

                if retry_actions.is_empty() {
                    report.warning(format!("{}: Retry block has no actions", path));
                }

                for (idx, action) in retry_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.actions[{}]", path, idx),
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
                    report.error(format!("{}: Variable name cannot be empty", path));
                } else {
                    report.variables_referenced.insert(variable.clone());
                }

                self.validate_collection(collection, path, report);

                if let Some(max) = max_iterations {
                    if *max == 0 {
                        report.error(format!("{}: max_iterations cannot be 0", path));
                    } else if *max > 10000 {
                        report.warning(format!("{}: max_iterations is {} (> 10000)", path, max));
                    }
                }

                if foreach_actions.is_empty() {
                    report.warning(format!("{}: Foreach block has no actions", path));
                }

                for (idx, action) in foreach_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.actions[{}]", path, idx),
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
                        report.error(format!("{}: max_iterations cannot be 0", path));
                    } else if *max > 100000 {
                        report.warning(format!(
                            "{}: max_iterations is {} (> 100000), this may run for a long time",
                            path, max
                        ));
                    }
                }

                if while_actions.is_empty() {
                    report.warning(format!("{}: While block has no actions", path));
                }

                for (idx, action) in while_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.actions[{}]", path, idx),
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
                    report.warning(format!("{}: Try block has no actions", path));
                }

                for (idx, action) in try_actions.iter().enumerate() {
                    self.validate_action(
                        action,
                        &format!("{}.try[{}]", path, idx),
                        depth + 1,
                        report,
                    );
                }

                if let Some(catch) = catch_actions {
                    if catch.is_empty() {
                        report.warning(format!("{}: Catch block has no actions", path));
                    }
                    for (idx, action) in catch.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{}.catch[{}]", path, idx),
                            depth + 1,
                            report,
                        );
                    }
                }

                if let Some(var) = error_variable {
                    if var.is_empty() {
                        report.error(format!("{}: Error variable name cannot be empty", path));
                    } else {
                        report.variables_referenced.insert(var.clone());
                    }
                }

                if let Some(finally) = finally_actions {
                    if finally.is_empty() {
                        report.warning(format!("{}: Finally block has no actions", path));
                    }
                    for (idx, action) in finally.iter().enumerate() {
                        self.validate_action(
                            action,
                            &format!("{}.finally[{}]", path, idx),
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
                    report.warning(format!("{}: TextEquals condition has empty value", path));
                }
            }
            Condition::VariableEquals { name, value } => {
                if name.is_empty() {
                    report.error(format!("{}: Variable name cannot be empty", path));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
                // Convert serde_yaml::Value to string for variable extraction
                if let Some(s) = value.as_str() {
                    self.extract_variables(s, report);
                }
            }
            Condition::Not { condition: inner } => {
                self.validate_condition(inner, &format!("{}[not]", path), report);
            }
            Condition::And { conditions } | Condition::Or { conditions } => {
                for (idx, cond) in conditions.iter().enumerate() {
                    self.validate_condition(cond, &format!("{}[{}]", path, idx), report);
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
                    report.warning(format!(
                        "{}: Array collection is empty (no iterations)",
                        path
                    ));
                }
            }
            ForeachCollection::Range { start, end } => {
                if start >= end {
                    report.error(format!(
                        "{}: Range start ({}) >= end ({})",
                        path, start, end
                    ));
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
                    report.error(format!("{}: Variable name cannot be empty", path));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
            }
        }
    }

    /// Validate a URL string (with variable support).
    fn validate_url(&self, url: &str, path: &str, report: &mut ValidationReport) {
        if url.is_empty() {
            report.error(format!("{}: URL cannot be empty", path));
            return;
        }

        self.extract_variables(url, report);

        // If no variables, try to validate URL format
        if !url.contains("${") && !url.starts_with("http://") && !url.starts_with("https://") {
            report.warning(format!(
                "{}: URL '{}' does not start with http:// or https://",
                path, url
            ));
        }
    }

    /// Validate a CSS selector (with variable support).
    fn validate_selector(&self, selector: &str, path: &str, report: &mut ValidationReport) {
        if selector.is_empty() {
            report.error(format!("{}: Selector cannot be empty", path));
            return;
        }

        self.extract_variables(selector, report);

        // Basic CSS selector validation (only if no variables)
        if !selector.contains("${") {
            // Check for common CSS selector issues
            if selector.contains("  ") {
                report.warning(format!(
                    "{}: Selector contains multiple consecutive spaces",
                    path
                ));
            }

            // Check for balanced brackets
            let open_brackets = selector.matches('[').count();
            let close_brackets = selector.matches(']').count();
            if open_brackets != close_brackets {
                report.error(format!(
                    "{}: Selector has unbalanced brackets: '{}'",
                    path, selector
                ));
            }

            // Check for balanced parentheses
            let open_parens = selector.matches('(').count();
            let close_parens = selector.matches(')').count();
            if open_parens != close_parens {
                report.error(format!(
                    "{}: Selector has unbalanced parentheses: '{}'",
                    path, selector
                ));
            }

            // Check for balanced quotes
            let single_quotes = selector.matches('\'').count();
            let double_quotes = selector.matches('"').count();
            if !single_quotes.is_multiple_of(2) {
                report.error(format!(
                    "{}: Selector has unbalanced single quotes: '{}'",
                    path, selector
                ));
            }
            if !double_quotes.is_multiple_of(2) {
                report.error(format!(
                    "{}: Selector has unbalanced double quotes: '{}'",
                    path, selector
                ));
            }
        }
    }

    /// Validate text content.
    fn validate_text(&self, text: &str, path: &str, context: &str, report: &mut ValidationReport) {
        self.extract_variables(text, report);

        if text.is_empty() {
            report.warning(format!("{}: {} is empty", path, context));
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
