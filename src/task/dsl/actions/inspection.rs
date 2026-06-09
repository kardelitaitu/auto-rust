//! Element inspection actions for DSL executor.
//!
//! Handlers for: extract (reads element text and optionally stores in
//! a variable). Non-DOM-mutating — does NOT clear the selector cache.

use anyhow::Result;

impl<T: super::super::DslApi> super::super::DslExecutor<'_, T> {
    pub(crate) async fn execute_extract(
        &mut self,
        selector: &str,
        variable: Option<&str>,
    ) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let text = self.api.text(&resolved_selector).await?.unwrap_or_default();
        if let Some(var_name) = variable {
            log::debug!("Extracting variable '{var_name}': {text}");
            self.variables.insert(var_name.to_string(), text);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{Action, TaskDefinition};
    use crate::task::dsl::api::mock::MockDslApi;
    use crate::task::dsl::cache::SelectorCacheEntry;
    use crate::task::dsl::DslExecutor;
    use std::collections::HashMap;

    fn create_task_def(name: &str, actions: Vec<Action>) -> TaskDefinition {
        TaskDefinition {
            name: name.to_string(),
            description: format!("Test: {name}"),
            policy: "default".to_string(),
            parameters: HashMap::new(),
            include: vec![],
            actions,
        }
    }

    fn create_executor<'a>(
        mock: &'a MockDslApi,
        actions: Vec<Action>,
    ) -> DslExecutor<'a, MockDslApi> {
        DslExecutor::new(mock, create_task_def("test", actions))
    }

    #[tokio::test]
    async fn test_execute_action_extract_does_not_clear_cache() {
        let mock = MockDslApi::new();
        mock.set_text_result("#title", Some("Hello"));
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        let cache_before = exec.cache_size();

        exec.execute_action(&Action::Extract {
            selector: "#title".to_string(),
            variable: Some("title".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            cache_before,
            "Extract should NOT clear cache"
        );
    }

    #[tokio::test]
    async fn test_execute_extract_stores_variable() {
        let mock = MockDslApi::new();
        mock.set_text_result("#title", Some("Page Title"));
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Extract {
            selector: "#title".to_string(),
            variable: Some("page_title".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.variables.get("page_title").unwrap(),
            "Page Title",
            "Extract should store the text in the variable"
        );
    }

    #[tokio::test]
    async fn test_execute_extract_without_variable() {
        let mock = MockDslApi::new();
        mock.set_text_result("#title", Some("text"));
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Extract {
            selector: "#title".to_string(),
            variable: None,
        })
        .await
        .unwrap();

        assert!(
            exec.variables.is_empty(),
            "Extract without variable should not store anything"
        );
    }

    #[tokio::test]
    async fn test_execute_extract_api_returns_none() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Extract {
            selector: "#empty".to_string(),
            variable: Some("result".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.variables.get("result").unwrap(),
            "",
            "Extract should default to empty string when API returns None"
        );
    }

    #[tokio::test]
    async fn test_execute_extract_variable_substitution_in_selector() {
        let mock = MockDslApi::new();
        mock.set_text_result("#resolved", Some("found"));
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("id".to_string(), "resolved".to_string());

        exec.execute_action(&Action::Extract {
            selector: "#${id}".to_string(),
            variable: Some("data".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(exec.variables.get("data").unwrap(), "found");
        let calls = mock.get_calls();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::task::dsl::api::mock::MockCall::Text { selector } if selector == "#resolved")),
            "selector should be substituted"
        );
    }
}
