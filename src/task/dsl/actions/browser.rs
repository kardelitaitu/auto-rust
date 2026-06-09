//! Browser interaction actions for DSL executor.
//!
//! Handlers for: navigate, click, type, hover, select, scroll_to,
//! right_click, double_click, clear — all of which mutate the DOM
//! and clear the selector cache after execution.

use anyhow::Result;

impl<T: super::super::DslApi> super::super::DslExecutor<'_, T> {
    pub(crate) async fn execute_navigate(&mut self, url: &str) -> Result<()> {
        let resolved_url = self.substitute_variables(url);
        self.api.navigate(&resolved_url, 30000).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        self.api.click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_type(&mut self, selector: &str, text: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let resolved_text = self.substitute_variables(text);
        self.api.r#type(&resolved_selector, &resolved_text).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_scroll_to(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        self.api.scroll_to(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_clear(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Clearing input field '{resolved_selector}'");
        self.api.clear(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_hover(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Hovering over element '{resolved_selector}'");
        self.api.hover(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_select(
        &mut self,
        selector: &str,
        value: &str,
        by_value: Option<bool>,
    ) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let resolved_value = self.substitute_variables(value);
        let use_value_attr = by_value.unwrap_or(false);

        log::debug!(
            "Selecting '{resolved_value}' from dropdown '{resolved_selector}' (by_value={use_value_attr})"
        );

        let script = if use_value_attr {
            format!(r"document.querySelector('{resolved_selector}').value = '{resolved_value}';")
        } else {
            format!(
                r"const select = document.querySelector('{resolved_selector}');
                const options = Array.from(select.options);
                const option = options.find(o => o.text.trim() === '{resolved_value}');
                if (option) select.value = option.value;"
            )
        };

        self.api.execute_js(&script).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_right_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Right-clicking element '{resolved_selector}'");
        self.api.right_click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }

    pub(crate) async fn execute_double_click(&mut self, selector: &str) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        log::debug!("Double-clicking element '{resolved_selector}'");
        self.api.double_click(&resolved_selector).await?;
        self.clear_cache();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{Action, TaskDefinition};
    use crate::task::dsl::api::mock::{MockCall, MockDslApi};
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
    async fn test_execute_action_navigate_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Navigate {
            url: "https://example.com".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            MockCall::Navigate {
                url: "https://example.com".to_string(),
                timeout_ms: 30000,
            },
            "Navigate should call api.navigate with url and 30s timeout"
        );
    }

    #[tokio::test]
    async fn test_execute_action_navigate_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(exec.cache_size() > 0);

        exec.execute_action(&Action::Navigate {
            url: "https://example.com".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            0,
            "cache should be cleared after navigate"
        );
    }

    #[tokio::test]
    async fn test_execute_action_navigate_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Navigate {
                url: "https://fail.example.com".to_string(),
            })
            .await;

        assert!(result.is_err(), "navigate should propagate API error");
    }

    #[tokio::test]
    async fn test_execute_action_click_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Click {
            selector: "#submit-btn".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "should make exactly one API call");
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#submit-btn".to_string()
            },
            "Click should call api.click with the correct selector"
        );
    }

    #[tokio::test]
    async fn test_execute_action_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(
            exec.cache_size() > 0,
            "cache should have entries before click"
        );

        exec.execute_action(&Action::Click {
            selector: "#btn".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "cache should be cleared after click");
    }

    #[tokio::test]
    async fn test_execute_action_click_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Click {
                selector: "#broken".to_string(),
            })
            .await;

        assert!(result.is_err(), "click should propagate API error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MockDslApi forced failure"),
            "error message should come from mock"
        );
    }

    #[tokio::test]
    async fn test_execute_action_click_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("btn_id".to_string(), "dynamic-button".to_string());

        exec.execute_action(&Action::Click {
            selector: "#${btn_id}".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::Click {
                selector: "#dynamic-button".to_string()
            },
            "variable btn_id should be substituted before calling api.click"
        );
    }

    #[tokio::test]
    async fn test_execute_action_type_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Type {
            selector: "#username".to_string(),
            text: "test_user".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            MockCall::Type {
                selector: "#username".to_string(),
                text: "test_user".to_string(),
            },
            "Type should call api.r#type with correct selector and text"
        );
    }

    #[tokio::test]
    async fn test_execute_action_type_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(exec.cache_size() > 0);

        exec.execute_action(&Action::Type {
            selector: "#input".to_string(),
            text: "hello".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "cache should be cleared after type");
    }

    #[tokio::test]
    async fn test_execute_action_type_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Type {
                selector: "#input".to_string(),
                text: "fail".to_string(),
            })
            .await;

        assert!(result.is_err(), "type should propagate API error");
    }

    #[tokio::test]
    async fn test_execute_action_type_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("user".to_string(), "alice".to_string());

        exec.execute_action(&Action::Type {
            selector: "#input".to_string(),
            text: "Hello ${user}".to_string(),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::Type {
                selector: "#input".to_string(),
                text: "Hello alice".to_string(),
            },
            "variable user should be substituted in both selector and text"
        );
    }

    #[tokio::test]
    async fn test_execute_action_scroll_to_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::ScrollTo {
            selector: "#footer".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "ScrollTo should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_clear_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::Clear {
            selector: "#input".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "Clear should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_hover_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::Hover {
            selector: "#menu".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "Hover should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_right_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::RightClick {
            selector: "#item".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "RightClick should clear cache");
    }

    #[tokio::test]
    async fn test_execute_action_double_click_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );

        exec.execute_action(&Action::DoubleClick {
            selector: "#item".to_string(),
        })
        .await
        .unwrap();

        assert_eq!(exec.cache_size(), 0, "DoubleClick should clear cache");
    }

    // ── Select action tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_execute_action_select_by_text() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#country".to_string(),
            value: "United States".to_string(),
            by_value: Some(false),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Select should make exactly one API call");
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'United States')")),
            "Select by text should execute JS that finds option by label text"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_by_value() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#country".to_string(),
            value: "US".to_string(),
            by_value: Some(true),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Select should make exactly one API call");
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script == "document.querySelector('#country').value = 'US';"),
            "Select by value should execute JS that sets value directly"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("target".to_string(), "Canada".to_string());
        exec.variables
            .insert("country_sel".to_string(), "#country".to_string());

        exec.execute_action(&Action::Select {
            selector: "${country_sel}".to_string(),
            value: "${target}".to_string(),
            by_value: Some(false),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'Canada')") && script.contains("querySelector('#country')")),
            "variables should be substituted in both selector and value"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_defaults_to_text_lookup() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Select {
            selector: "#sel".to_string(),
            value: "option1".to_string(),
            by_value: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            matches!(&calls[0], MockCall::ExecuteJs { script } if script.contains("options.find(o => o.text.trim() === 'option1')")),
            "by_value=None should default to text lookup (false)"
        );
    }

    #[tokio::test]
    async fn test_execute_action_select_propagates_execute_js_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Select {
                selector: "#sel".to_string(),
                value: "x".to_string(),
                by_value: Some(false),
            })
            .await;

        assert!(result.is_err(), "Select should propagate execute_js error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MockDslApi forced failure"),
            "error message should come from mock"
        );
    }
}
