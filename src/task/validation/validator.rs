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

    #[allow(clippy::unused_self)]
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
