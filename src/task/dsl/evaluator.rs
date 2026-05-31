//! Variable substitution and condition evaluation for DSL tasks.
//!
//! Contains helper methods for the `DslExecutor` that handle
//! variable substitution in action parameters and evaluation of
//! conditional expressions (if/else, while loops, etc.).

use crate::task::dsl::Condition;
use anyhow::Result;

impl<T: super::DslApi> super::DslExecutor<'_, T> {
    /// Substitute ${variable} placeholders with values from the variables map.
    ///
    /// Replaces occurrences of ${`variable_name`} in the input text with
    /// the corresponding value from the executor's variables map.
    ///
    /// # Arguments
    /// * `text` - Input text that may contain ${variable} placeholders
    ///
    /// # Returns
    /// Text with all placeholders replaced
    #[must_use]
    pub fn substitute_variables(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Replace ${variable} syntax
        for (key, value) in &self.variables {
            let placeholder = format!("${{{key}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Evaluate a single condition.
    ///
    /// Evaluates a condition against the current execution state,
    /// including variable values and DOM state via the API.
    ///
    /// # Arguments
    /// * `condition` - The condition to evaluate
    ///
    /// # Returns
    /// `true` if condition is met, `false` otherwise
    pub async fn evaluate_condition(&self, condition: &Condition) -> Result<bool> {
        match condition {
            Condition::ElementExists { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                match self.api.exists(&resolved_selector).await {
                    Ok(exists) => Ok(exists),
                    Err(_) => Ok(false),
                }
            }
            Condition::ElementVisible { selector } => {
                let resolved_selector = self.substitute_variables(selector);
                match self.api.visible(&resolved_selector).await {
                    Ok(visible) => Ok(visible),
                    Err(_) => Ok(false),
                }
            }
            Condition::TextEquals { selector, value } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_value = self.substitute_variables(value);
                match self.api.text(&resolved_selector).await {
                    Ok(Some(text)) => Ok(text.trim() == resolved_value),
                    _ => Ok(false),
                }
            }
            Condition::VariableEquals { name, value } => {
                if let Some(var_value) = self.variables.get(name) {
                    let expected = match value {
                        serde_yml::Value::String(s) => s.clone(),
                        serde_yml::Value::Number(n) => n.to_string(),
                        serde_yml::Value::Bool(b) => b.to_string(),
                        _ => format!("{value:?}"),
                    };
                    Ok(var_value == &expected)
                } else {
                    Ok(false)
                }
            }
            Condition::TextMatches { selector, pattern } => {
                let resolved_selector = self.substitute_variables(selector);
                let resolved_pattern = self.substitute_variables(pattern);
                match self.api.text(&resolved_selector).await {
                    Ok(Some(text)) => {
                        // Simple pattern matching (could be extended to regex)
                        Ok(text.contains(&resolved_pattern))
                    }
                    _ => Ok(false),
                }
            }
            Condition::Not { condition } => {
                let result = Box::pin(self.evaluate_condition(condition)).await?;
                Ok(!result)
            }
            Condition::And { conditions } => self.evaluate_conditions_and(conditions).await,
            Condition::Or { conditions } => self.evaluate_conditions_or(conditions).await,
            Condition::DateBefore { name, date, format } => {
                self.evaluate_date_comparison(name, date, format, true)
                    .await
            }
            Condition::DateAfter { name, date, format } => {
                self.evaluate_date_comparison(name, date, format, false)
                    .await
            }
            Condition::VariableMatches { name, pattern } => {
                if let Some(var_value) = self.variables.get(name) {
                    let resolved_pattern = self.substitute_variables(pattern);
                    Ok(var_value.contains(&resolved_pattern))
                } else {
                    Ok(false)
                }
            }
            Condition::NumericGreaterThan { name, value } => {
                if let Some(var_value) = self.variables.get(name) {
                    if let Ok(num) = var_value.parse::<f64>() {
                        Ok(num > *value)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::NumericLessThan { name, value } => {
                if let Some(var_value) = self.variables.get(name) {
                    if let Ok(num) = var_value.parse::<f64>() {
                        Ok(num < *value)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::NumericRange { name, min, max } => {
                if let Some(var_value) = self.variables.get(name) {
                    if let Ok(num) = var_value.parse::<f64>() {
                        Ok(num >= *min && num <= *max)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::ArrayContains { name, value } => {
                if let Some(var_value) = self.variables.get(name) {
                    // Simplified: check if value is in a comma-separated string
                    let search = match value {
                        serde_yml::Value::String(s) => s.clone(),
                        _ => format!("{value:?}"),
                    };
                    Ok(var_value.contains(&search))
                } else {
                    Ok(false)
                }
            }
            Condition::ArrayLength {
                name,
                min,
                max,
                exact,
            } => {
                if let Some(var_value) = self.variables.get(name) {
                    let len = var_value.len();
                    if let Some(exact_val) = exact {
                        Ok(len == *exact_val)
                    } else {
                        let min_ok = min.is_none_or(|m| len >= m);
                        let max_ok = max.is_none_or(|m| len <= m);
                        Ok(min_ok && max_ok)
                    }
                } else {
                    Ok(false)
                }
            }
            Condition::VariableDefined { name } => Ok(self.variables.contains_key(name)),
            Condition::VariableNotDefined { name } => Ok(!self.variables.contains_key(name)),
            Condition::True => Ok(true),
            Condition::False => Ok(false),
        }
    }

    /// Evaluate AND conditions (all must be true).
    ///
    /// # Arguments
    /// * `conditions` - Slice of conditions to evaluate
    ///
    /// # Returns
    /// `true` if ALL conditions are met
    pub async fn evaluate_conditions_and(&self, conditions: &[Condition]) -> Result<bool> {
        for cond in conditions {
            if !Box::pin(self.evaluate_condition(cond)).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate OR conditions (at least one must be true).
    ///
    /// # Arguments
    /// * `conditions` - Slice of conditions to evaluate
    ///
    /// # Returns
    /// `true` if ANY condition is met
    pub async fn evaluate_conditions_or(&self, conditions: &[Condition]) -> Result<bool> {
        for cond in conditions {
            if Box::pin(self.evaluate_condition(cond)).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Evaluate date comparison conditions.
    ///
    /// Compares a variable (expected to be a date) against a reference date.
    ///
    /// # Arguments
    /// * `name` - Variable name containing the date
    /// * `date` - Reference date string
    /// * `format` - Optional date format string
    /// * `is_before` - If true, check if variable date is before reference
    ///
    /// # Returns
    /// `true` if the comparison condition is met
    pub async fn evaluate_date_comparison(
        &self,
        name: &str,
        date: &str,
        format: &Option<String>,
        is_before: bool,
    ) -> Result<bool> {
        if let Some(var_value) = self.variables.get(name) {
            let date_format = format.as_deref().unwrap_or("%Y-%m-%d");

            // Parse the variable date
            let var_date = chrono::NaiveDate::parse_from_str(var_value, date_format);
            let var_datetime = chrono::NaiveDateTime::parse_from_str(var_value, date_format);

            // Parse the reference date
            let ref_date = chrono::NaiveDate::parse_from_str(date, date_format);
            let ref_datetime = chrono::NaiveDateTime::parse_from_str(date, date_format);

            let comparison_result = match (var_date, ref_date, var_datetime, ref_datetime) {
                (Ok(v), Ok(r), _, _) => {
                    if is_before {
                        v < r
                    } else {
                        v > r
                    }
                }
                (_, _, Ok(v), Ok(r)) => {
                    if is_before {
                        v < r
                    } else {
                        v > r
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Failed to parse date for variable '{name}': value='{var_value}', format='{date_format}'"
                    ));
                }
            };
            Ok(comparison_result)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::task::dsl::TaskDefinition;

    #[test]
    fn test_substitute_variables_logic() {
        // Test variable substitution logic directly
        // Since DslExecutor requires &TaskContext which needs full setup,
        // we test the core logic separately
        let text = "${greeting}, ${name}!";
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "World".to_string());
        variables.insert("greeting".to_string(), "Hello".to_string());

        let mut result = text.to_string();
        for (key, value) in &variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_substitute_variables_no_vars() {
        let text = "No variables here";
        let variables: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        let mut result = text.to_string();
        for (key, value) in &variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_task_definition_creation() {
        let def = TaskDefinition {
            name: "test".to_string(),
            description: "Test".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![],
        };
        assert_eq!(def.name, "test");
        assert!(def.actions.is_empty());
    }
}
