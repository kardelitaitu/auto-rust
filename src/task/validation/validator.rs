use std::collections::{HashMap, HashSet};

use crate::task::dsl::{Action, Condition, ForeachCollection, ParameterDef, TaskDefinition};
use crate::task::validation::ValidationReport;

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
    #[must_use]
    pub fn validate(&self, def: &TaskDefinition) -> ValidationReport {
        let mut report = ValidationReport::new(def.name.clone());

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

        validator.validate_task_structure(def, &mut report);

        for (idx, action) in def.actions.iter().enumerate() {
            let path = format!("actions[{idx}]");
            validator.validate_action(action, &path, 0, &mut report);
        }

        report.action_count = validator.count_actions(&def.actions);

        report
    }

    fn validate_task_structure(&self, def: &TaskDefinition, report: &mut ValidationReport) {
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

        if def.actions.is_empty() && def.include.is_empty() {
            report.error("Task must have at least one action or include");
        }

        for (name, param) in &def.parameters {
            self.validate_parameter_def(name, param, report);
        }

        for (idx, include) in def.include.iter().enumerate() {
            if include.path.is_empty() {
                report.error(format!("include[{idx}]: Path cannot be empty"));
            }
        }
    }

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

    fn validate_action(
        &self,
        action: &Action,
        path: &str,
        depth: usize,
        report: &mut ValidationReport,
    ) {
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
                    if let Some(ref current) = self.current_task {
                        if task == current {
                            report.error(format!(
                                "{path}: Task '{task}' calls itself (circular reference)"
                            ));
                        }
                    }

                    if !self.known_tasks.is_empty() && !self.known_tasks.contains(task) {
                        report.warning(format!(
                            "{path}: Task '{task}' is not in the known task list"
                        ));
                    }
                }

                if let Some(params) = parameters {
                    for value in params.values() {
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
                if min.is_none() && max.is_none() && exact.is_none() {
                    report.warning(format!(
                        "{path}: ArrayLength has no constraints (min/max/exact)"
                    ));
                }
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
            Condition::True | Condition::False => {}
            Condition::VariableDefined { name } | Condition::VariableNotDefined { name } => {
                if name.is_empty() {
                    report.error(format!("{path}: Variable name cannot be empty"));
                } else {
                    report.variables_referenced.insert(name.clone());
                }
            }
        }
    }

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

    fn validate_url(&self, url: &str, path: &str, report: &mut ValidationReport) {
        if url.is_empty() {
            report.error(format!("{path}: URL cannot be empty"));
            return;
        }

        self.extract_variables(url, report);

        if !url.contains("${") && !url.starts_with("http://") && !url.starts_with("https://") {
            report.warning(format!(
                "{path}: URL '{url}' does not start with http:// or https://"
            ));
        }
    }

    fn validate_selector(&self, selector: &str, path: &str, report: &mut ValidationReport) {
        if selector.is_empty() {
            report.error(format!("{path}: Selector cannot be empty"));
            return;
        }

        self.extract_variables(selector, report);

        if !selector.contains("${") {
            if selector.contains("  ") {
                report.warning(format!(
                    "{path}: Selector contains multiple consecutive spaces"
                ));
            }

            let open_brackets = selector.matches('[').count();
            let close_brackets = selector.matches(']').count();
            if open_brackets != close_brackets {
                report.error(format!(
                    "{path}: Selector has unbalanced brackets: '{selector}'"
                ));
            }

            let open_parens = selector.matches('(').count();
            let close_parens = selector.matches(')').count();
            if open_parens != close_parens {
                report.error(format!(
                    "{path}: Selector has unbalanced parentheses: '{selector}'"
                ));
            }

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

    fn validate_text(&self, text: &str, path: &str, context: &str, report: &mut ValidationReport) {
        self.extract_variables(text, report);

        if text.is_empty() {
            report.warning(format!("{path}: {context} is empty"));
        }
    }

    pub(crate) fn extract_variables(&self, text: &str, report: &mut ValidationReport) {
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

    #[allow(clippy::unused_self)]
    pub(crate) fn count_actions(&self, actions: &[Action]) -> usize {
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
                Action::Call { .. } => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::dsl::{
        Action, Condition, LogLevel, ParameterDef, ParameterType, TaskDefinition,
    };
    use std::collections::HashMap;

    fn make_def(name: &str, actions: Vec<Action>) -> TaskDefinition {
        TaskDefinition {
            name: name.to_string(),
            description: String::new(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions,
        }
    }

    fn valid_actions() -> Vec<Action> {
        vec![Action::Navigate {
            url: "https://example.com".to_string(),
        }]
    }

    // ── Task structure validation ────────────────────────────────────────

    #[test]
    fn empty_name_errors() {
        let v = TaskValidator::new();
        let report = v.validate(&make_def("", valid_actions()));
        assert!(!report.is_valid());
        assert!(report.issues.iter().any(|i| i.message().contains("empty")));
    }

    #[test]
    fn name_with_spaces_errors() {
        let v = TaskValidator::new();
        let report = v.validate(&make_def("my task", valid_actions()));
        assert!(!report.is_valid());
        assert!(report.issues.iter().any(|i| i.message().contains("spaces")));
    }

    #[test]
    fn empty_actions_and_includes_errors() {
        let v = TaskValidator::new();
        let report = v.validate(&make_def("good-name", vec![]));
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("at least one")));
    }

    #[test]
    fn valid_task_no_errors() {
        let v = TaskValidator::new();
        let report = v.validate(&make_def("my-task", valid_actions()));
        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
    }

    // ── Action validation ────────────────────────────────────────────────

    #[test]
    fn navigate_empty_url_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::Navigate { url: String::new() }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn click_empty_selector_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::Click {
            selector: String::new(),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn wait_zero_duration_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Wait { duration_ms: 0 }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid()); // warning, not error
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn wait_long_duration_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Wait {
            duration_ms: 120_000,
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn wait_for_zero_timeout_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::WaitFor {
            selector: "#el".to_string(),
            timeout_ms: Some(0),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn extract_empty_variable_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::Extract {
            selector: "#el".to_string(),
            variable: Some(String::new()),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn execute_empty_script_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Execute {
            script: String::new(),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn log_empty_message_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Log {
            message: String::new(),
            level: Some(LogLevel::Info),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    // ── Loop validation ──────────────────────────────────────────────────

    #[test]
    fn loop_without_count_or_condition_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::Loop {
            count: None,
            condition: None,
            actions: vec![],
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn loop_zero_count_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Loop {
            count: Some(0),
            condition: None,
            actions: valid_actions(),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn loop_large_count_warns() {
        let v = TaskValidator::new();
        let actions = vec![Action::Loop {
            count: Some(50_000),
            condition: None,
            actions: valid_actions(),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    // ── Call validation ──────────────────────────────────────────────────

    #[test]
    fn call_empty_task_name_errors() {
        let v = TaskValidator::new();
        let actions = vec![Action::Call {
            task: String::new(),
            parameters: None,
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(!report.is_valid());
    }

    #[test]
    fn call_self_circular_errors() {
        let v = TaskValidator::new().with_current_task("my-task");
        let actions = vec![Action::Call {
            task: "my-task".to_string(),
            parameters: None,
        }];
        let report = v.validate(&make_def("my-task", actions));
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("circular")));
    }

    #[test]
    fn call_unknown_task_warns_when_known_tasks_set() {
        let v = TaskValidator::new().with_known_tasks(vec!["known-task"]);
        let actions = vec![Action::Call {
            task: "unknown-task".to_string(),
            parameters: None,
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn call_known_task_no_warning() {
        let v = TaskValidator::new().with_known_tasks(vec!["known-task"]);
        let actions = vec![Action::Call {
            task: "known-task".to_string(),
            parameters: None,
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.is_valid());
        assert_eq!(report.warning_count(), 0);
    }

    // ── Nesting depth ────────────────────────────────────────────────────

    #[test]
    fn deep_nesting_exceeds_limit() {
        // Build a chain of nested if-else actions
        let mut action = Action::If {
            condition: Condition::True,
            then: vec![Action::Wait { duration_ms: 100 }],
            r#else: None,
        };
        for _ in 0..15 {
            action = Action::If {
                condition: Condition::True,
                then: vec![action],
                r#else: None,
            };
        }
        let v = TaskValidator::new().with_max_nesting_depth(10);
        let report = v.validate(&make_def("t", vec![action]));
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("nesting depth")));
    }

    // ── Parameter validation ─────────────────────────────────────────────

    #[test]
    fn parameter_empty_name_errors() {
        let mut params = HashMap::new();
        params.insert(
            String::new(),
            ParameterDef {
                r#type: ParameterType::String,
                required: true,
                default: None,
                description: String::new(),
            },
        );
        let v = TaskValidator::new();
        let mut def = make_def("t", valid_actions());
        def.parameters = params;
        let report = v.validate(&def);
        assert!(!report.is_valid());
    }

    #[test]
    fn parameter_name_with_spaces_errors() {
        let mut params = HashMap::new();
        params.insert(
            "my param".to_string(),
            ParameterDef {
                r#type: ParameterType::String,
                required: false,
                default: None,
                description: String::new(),
            },
        );
        let v = TaskValidator::new();
        let mut def = make_def("t", valid_actions());
        def.parameters = params;
        let report = v.validate(&def);
        assert!(!report.is_valid());
    }

    #[test]
    fn required_with_default_warns() {
        let mut params = HashMap::new();
        params.insert(
            "p".to_string(),
            ParameterDef {
                r#type: ParameterType::String,
                required: true,
                default: Some(serde_yaml::Value::String("x".to_string())),
                description: String::new(),
            },
        );
        let v = TaskValidator::new();
        let mut def = make_def("t", valid_actions());
        def.parameters = params;
        let report = v.validate(&def);
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    #[test]
    fn optional_without_default_warns() {
        let mut params = HashMap::new();
        params.insert(
            "p".to_string(),
            ParameterDef {
                r#type: ParameterType::String,
                required: false,
                default: None,
                description: String::new(),
            },
        );
        let v = TaskValidator::new();
        let mut def = make_def("t", valid_actions());
        def.parameters = params;
        let report = v.validate(&def);
        assert!(report.is_valid());
        assert!(report.warning_count() > 0);
    }

    // ── Include validation ───────────────────────────────────────────────

    #[test]
    fn include_empty_path_errors() {
        let v = TaskValidator::new();
        let mut def = make_def("t", valid_actions());
        def.include = vec![crate::task::dsl::IncludeSpec {
            path: String::new(),
            condition: None,
        }];
        let report = v.validate(&def);
        assert!(!report.is_valid());
    }

    // ── Action count ─────────────────────────────────────────────────────

    #[test]
    fn action_count_tracked() {
        let v = TaskValidator::new();
        let actions = vec![
            Action::Navigate {
                url: "https://a.com".to_string(),
            },
            Action::Click {
                selector: "#btn".to_string(),
            },
            Action::Wait { duration_ms: 100 },
        ];
        let report = v.validate(&make_def("t", actions));
        assert_eq!(report.action_count, 3);
    }

    #[test]
    fn nested_action_count() {
        let v = TaskValidator::new();
        let actions = vec![Action::Loop {
            count: Some(3),
            condition: None,
            actions: vec![
                Action::Wait { duration_ms: 100 },
                Action::Click {
                    selector: "#x".to_string(),
                },
            ],
        }];
        let report = v.validate(&make_def("t", actions));
        assert_eq!(report.action_count, 3); // 1 loop + 2 inner
    }

    // ── Variable tracking ────────────────────────────────────────────────

    #[test]
    fn extract_variable_tracked() {
        let v = TaskValidator::new();
        let actions = vec![Action::Extract {
            selector: "#el".to_string(),
            variable: Some("my_var".to_string()),
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.variables_referenced.contains("my_var"));
    }

    #[test]
    fn call_tracks_tasks() {
        let v = TaskValidator::new();
        let actions = vec![Action::Call {
            task: "other-task".to_string(),
            parameters: None,
        }];
        let report = v.validate(&make_def("t", actions));
        assert!(report.tasks_called.contains("other-task"));
    }
}

#[cfg(test)]
mod tests_extended {
    use super::*;
    use crate::task::dsl::{IncludeSpec, ParameterType, TaskDefinition};

    fn def(actions: Vec<Action>) -> TaskDefinition {
        TaskDefinition {
            name: "test-task".to_string(),
            description: String::new(),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions,
        }
    }

    fn report_for(actions: Vec<Action>) -> ValidationReport {
        TaskValidator::new().validate(&def(actions))
    }

    fn param(required: bool, default: Option<serde_yaml::Value>) -> ParameterDef {
        ParameterDef {
            r#type: ParameterType::String,
            description: String::new(),
            default,
            required,
        }
    }

    // ========================================================================
    // Selector edge cases
    // ========================================================================

    #[test]
    fn selector_unbalanced_brackets_is_error() {
        let report = report_for(vec![Action::Click {
            selector: "[data-x".into(),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("unbalanced brackets")));
    }

    #[test]
    fn selector_unbalanced_parens_is_error() {
        let report = report_for(vec![Action::Click {
            selector: "div(2".into(),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("unbalanced parentheses")));
    }

    #[test]
    fn selector_unbalanced_quotes_is_error() {
        let single = report_for(vec![Action::Click {
            selector: "a[title='x]".into(),
        }]);
        assert!(single
            .issues
            .iter()
            .any(|i| i.message().contains("unbalanced single quotes")));
        let double = report_for(vec![Action::Click {
            selector: "a[title=\"x]".into(),
        }]);
        assert!(double
            .issues
            .iter()
            .any(|i| i.message().contains("unbalanced double quotes")));
    }

    #[test]
    fn selector_consecutive_spaces_is_warning() {
        let report = report_for(vec![Action::Click {
            selector: "div  .btn".into(),
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("multiple consecutive spaces")));
    }

    // ========================================================================
    // More simple actions
    // ========================================================================

    #[test]
    fn navigate_non_http_url_is_warning() {
        let report = report_for(vec![Action::Navigate {
            url: "ftp://x.com".into(),
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("does not start with http")));
    }

    #[test]
    fn navigate_with_variable_skips_scheme_warning() {
        let report = report_for(vec![Action::Navigate {
            url: "${base_url}/page".into(),
        }]);
        assert_eq!(report.warning_count(), 0);
        assert!(report.variables_referenced.contains("base_url"));
    }

    #[test]
    fn type_empty_text_is_warning() {
        let report = report_for(vec![Action::Type {
            selector: "#in".into(),
            text: String::new(),
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("text is empty")));
    }

    #[test]
    fn extract_records_variable_reference() {
        let report = report_for(vec![Action::Extract {
            selector: "#x".into(),
            variable: Some("result".into()),
        }]);
        assert!(report.variables_referenced.contains("result"));
    }

    #[test]
    fn log_extracts_variables() {
        let report = report_for(vec![Action::Log {
            message: "done ${total}".into(),
            level: Some(crate::task::dsl::LogLevel::Info),
        }]);
        assert!(report.variables_referenced.contains("total"));
    }

    // ========================================================================
    // If / Parallel
    // ========================================================================

    #[test]
    fn if_paths_are_labeled() {
        let nested = report_for(vec![Action::If {
            condition: Condition::True,
            then: vec![Action::Click {
                selector: String::new(),
            }],
            r#else: Some(vec![Action::Click {
                selector: String::new(),
            }]),
        }]);
        assert!(nested
            .issues
            .iter()
            .any(|i| i.message().contains("actions[0].then[0]")));
        assert!(nested
            .issues
            .iter()
            .any(|i| i.message().contains("actions[0].else[0]")));
    }

    #[test]
    fn parallel_concurrency_zero_is_error() {
        let report = report_for(vec![Action::Parallel {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_concurrency: Some(0),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("max_concurrency cannot be 0")));
    }

    #[test]
    fn parallel_concurrency_above_count_warns() {
        let report = report_for(vec![Action::Parallel {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_concurrency: Some(5),
        }]);
        assert!(report.issues.iter().any(|i| i
            .message()
            .contains("max_concurrency (5) > action count (1)")));
    }

    #[test]
    fn parallel_empty_actions_warns() {
        let report = report_for(vec![Action::Parallel {
            actions: vec![],
            max_concurrency: None,
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("no actions")));
    }

    // ========================================================================
    // Retry
    // ========================================================================

    #[test]
    fn retry_zero_attempts_is_error() {
        let report = report_for(vec![Action::Retry {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_attempts: Some(0),
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("max_attempts cannot be 0")));
    }

    #[test]
    fn retry_inverted_delays_is_error() {
        let report = report_for(vec![Action::Retry {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_attempts: None,
            initial_delay_ms: Some(5000),
            max_delay_ms: Some(1000),
            backoff_multiplier: None,
            jitter: None,
            retry_on: None,
        }]);
        assert!(report.issues.iter().any(|i| i
            .message()
            .contains("initial_delay_ms (5000) > max_delay_ms (1000)")));
    }

    #[test]
    fn retry_low_multiplier_is_error() {
        let report = report_for(vec![Action::Retry {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_attempts: None,
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: Some(0.5),
            jitter: None,
            retry_on: None,
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("backoff_multiplier (0.5) < 1.0")));
    }

    #[test]
    fn retry_empty_patterns_is_warning() {
        let report = report_for(vec![Action::Retry {
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_attempts: None,
            initial_delay_ms: None,
            max_delay_ms: None,
            backoff_multiplier: None,
            jitter: None,
            retry_on: Some(vec![]),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("retry_on patterns are empty")));
    }

    // ========================================================================
    // Foreach / While / Try
    // ========================================================================

    #[test]
    fn foreach_empty_variable_is_error() {
        let report = report_for(vec![Action::Foreach {
            variable: String::new(),
            collection: ForeachCollection::Array {
                values: vec![serde_yaml::Value::String("a".into())],
            },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("Variable name cannot be empty")));
    }

    #[test]
    fn foreach_zero_max_iterations_is_error() {
        let report = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Array {
                values: vec![serde_yaml::Value::String("a".into())],
            },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: Some(0),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("max_iterations cannot be 0")));
    }

    #[test]
    fn while_zero_max_iterations_is_error() {
        let report = report_for(vec![Action::While {
            condition: Condition::True,
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: Some(0),
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("max_iterations cannot be 0")));
    }

    #[test]
    fn try_empty_blocks_and_error_var() {
        let empty_try = report_for(vec![Action::Try {
            try_actions: vec![],
            catch_actions: None,
            error_variable: None,
            finally_actions: None,
        }]);
        assert!(empty_try
            .issues
            .iter()
            .any(|i| i.message().contains("Try block has no actions")));

        let empty_catch = report_for(vec![Action::Try {
            try_actions: vec![Action::Wait { duration_ms: 1 }],
            catch_actions: Some(vec![]),
            error_variable: None,
            finally_actions: None,
        }]);
        assert!(empty_catch
            .issues
            .iter()
            .any(|i| i.message().contains("Catch block has no actions")));

        let empty_finally = report_for(vec![Action::Try {
            try_actions: vec![Action::Wait { duration_ms: 1 }],
            catch_actions: None,
            error_variable: None,
            finally_actions: Some(vec![]),
        }]);
        assert!(empty_finally
            .issues
            .iter()
            .any(|i| i.message().contains("Finally block has no actions")));

        let empty_err_var = report_for(vec![Action::Try {
            try_actions: vec![Action::Wait { duration_ms: 1 }],
            catch_actions: None,
            error_variable: Some(String::new()),
            finally_actions: None,
        }]);
        assert!(empty_err_var
            .issues
            .iter()
            .any(|i| i.message().contains("Error variable name cannot be empty")));
    }

    // ========================================================================
    // Conditions
    // ========================================================================

    #[test]
    fn condition_element_exists_empty_selector_is_error() {
        let report = report_for(vec![Action::If {
            condition: Condition::ElementExists {
                selector: String::new(),
            },
            then: vec![Action::Wait { duration_ms: 1 }],
            r#else: None,
        }]);
        assert!(report.has_errors());
    }

    #[test]
    fn condition_text_matches_invalid_regex_is_error() {
        let report = report_for(vec![Action::While {
            condition: Condition::TextMatches {
                selector: "#x".into(),
                pattern: "[invalid".into(),
            },
            actions: vec![],
            max_iterations: None,
        }]);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("pattern is invalid regex")));
    }

    #[test]
    fn condition_variable_equals_empty_name_is_error() {
        let report = report_for(vec![Action::If {
            condition: Condition::VariableEquals {
                name: String::new(),
                value: serde_yaml::Value::String("x".into()),
            },
            then: vec![],
            r#else: None,
        }]);
        assert!(report.has_errors());
    }

    #[test]
    fn condition_numeric_range_inverted_is_warning() {
        let report = report_for(vec![Action::While {
            condition: Condition::NumericRange {
                name: "n".into(),
                min: 10.0,
                max: 5.0,
            },
            actions: vec![],
            max_iterations: None,
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("min (10) is greater than max (5)")));
    }

    #[test]
    fn condition_date_empty_is_warning() {
        let report = report_for(vec![Action::If {
            condition: Condition::DateBefore {
                name: "d".into(),
                date: String::new(),
                format: Some(String::new()),
            },
            then: vec![],
            r#else: None,
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("date is empty")));
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("Date format is empty")));
    }

    #[test]
    fn condition_array_length_no_constraints_is_warning() {
        let report = report_for(vec![Action::If {
            condition: Condition::ArrayLength {
                name: "arr".into(),
                min: None,
                max: None,
                exact: None,
            },
            then: vec![],
            r#else: None,
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("has no constraints")));
    }

    #[test]
    fn condition_nested_and_or_not_validated() {
        let report = report_for(vec![Action::If {
            condition: Condition::And {
                conditions: vec![
                    Condition::Not {
                        condition: Box::new(Condition::ElementVisible {
                            selector: String::new(),
                        }),
                    },
                    Condition::Or { conditions: vec![] },
                ],
            },
            then: vec![],
            r#else: None,
        }]);
        assert!(report.has_errors());
    }

    #[test]
    fn condition_true_false_no_issues() {
        let t = report_for(vec![Action::If {
            condition: Condition::True,
            then: vec![Action::Wait { duration_ms: 1 }],
            r#else: None,
        }]);
        assert!(t.is_valid());
        let f = report_for(vec![Action::If {
            condition: Condition::False,
            then: vec![Action::Wait { duration_ms: 1 }],
            r#else: None,
        }]);
        assert!(f.is_valid());
    }

    // ========================================================================
    // Collections
    // ========================================================================

    #[test]
    fn collection_array_empty_is_warning() {
        let report = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Array { values: vec![] },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("Array collection is empty")));
    }

    #[test]
    fn collection_range_inverted_is_error_and_huge_warns() {
        let inverted = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Range { start: 5, end: 2 },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert!(inverted
            .issues
            .iter()
            .any(|i| i.message().contains("Range start (5) >= end (2)")));

        let huge = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Range {
                start: 0,
                end: 20_000,
            },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert_eq!(huge.error_count(), 0);
        assert!(huge.issues.iter().any(|i| i.message().contains("> 10000")));
    }

    #[test]
    fn collection_elements_empty_selector_is_error() {
        let report = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Elements {
                selector: String::new(),
            },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert!(report.has_errors());
    }

    #[test]
    fn collection_variable_empty_name_is_error() {
        let report = report_for(vec![Action::Foreach {
            variable: "x".into(),
            collection: ForeachCollection::Variable {
                name: String::new(),
            },
            actions: vec![Action::Wait { duration_ms: 1 }],
            max_iterations: None,
        }]);
        assert!(report.has_errors());
    }

    // ========================================================================
    // Variables / counts / includes
    // ========================================================================

    #[test]
    fn extract_variables_multiple_and_incomplete() {
        let validator = TaskValidator::new();
        let mut report = ValidationReport::new("t".to_string());
        validator.extract_variables("a ${x} b ${y} c ${", &mut report);
        assert!(report.variables_referenced.contains("x"));
        assert!(report.variables_referenced.contains("y"));
        assert!(!report.variables_referenced.contains(""));
    }

    #[test]
    fn count_actions_recursive() {
        let report = report_for(vec![Action::If {
            condition: Condition::True,
            then: vec![
                Action::Wait { duration_ms: 1 },
                Action::Loop {
                    count: Some(2),
                    condition: None,
                    actions: vec![Action::Click {
                        selector: "#n".into(),
                    }],
                },
            ],
            r#else: Some(vec![Action::Retry {
                actions: vec![
                    Action::Wait { duration_ms: 1 },
                    Action::Wait { duration_ms: 1 },
                ],
                max_attempts: None,
                initial_delay_ms: None,
                max_delay_ms: None,
                backoff_multiplier: None,
                jitter: None,
                retry_on: None,
            }]),
        }]);
        assert_eq!(report.action_count, 7);
    }

    #[test]
    fn include_empty_path_is_error() {
        let mut d = def(vec![Action::Wait { duration_ms: 100 }]);
        d.include = vec![IncludeSpec {
            path: String::new(),
            condition: None,
        }];
        let report = TaskValidator::new().validate(&d);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("Path cannot be empty")));
    }

    #[test]
    fn required_param_with_default_warns_and_optional_without_default_warns() {
        let mut d = def(vec![Action::Wait { duration_ms: 100 }]);
        d.parameters.insert(
            "a".to_string(),
            param(true, Some(serde_yaml::Value::String("x".into()))),
        );
        d.parameters.insert("b".to_string(), param(false, None));
        let report = TaskValidator::new().validate(&d);
        assert_eq!(report.error_count(), 0);
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("required but has a default")));
        assert!(report
            .issues
            .iter()
            .any(|i| i.message().contains("optional but has no default")));
    }
}
