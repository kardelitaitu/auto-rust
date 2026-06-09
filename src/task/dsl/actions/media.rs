//! Screenshot/media actions for DSL executor.
//!
//! Handlers for: screenshot (full page or element, with optional
//! file path). Clears the selector cache after capture.

use anyhow::Result;

impl<T: super::super::DslApi> super::super::DslExecutor<'_, T> {
    pub(crate) async fn execute_screenshot(
        &mut self,
        path: Option<&str>,
        selector: Option<&str>,
    ) -> Result<()> {
        let resolved_selector = selector.map(|s| self.substitute_variables(s));
        let resolved_path = path.map(|p| self.substitute_variables(p));

        if let Some(ref sel) = resolved_selector {
            log::info!("Taking element screenshot of '{sel}'");
        } else {
            log::info!("Taking full page screenshot");
        }

        if let Some(ref sel) = resolved_selector {
            self.api.scroll_to(sel).await?;
        }

        let file_path = self.api.screenshot().await?;

        if let Some(ref dest) = resolved_path {
            log::info!("Screenshot saved to: {dest} (default: {file_path})");
        } else {
            log::info!("Screenshot saved to: {file_path}");
        }

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
    async fn test_execute_action_screenshot_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls.len(),
            1,
            "Screenshot should make exactly one API call"
        );
        assert_eq!(
            calls[0],
            MockCall::Screenshot,
            "Screenshot should call api.screenshot"
        );
    }

    #[tokio::test]
    async fn test_execute_action_screenshot_with_selector_scrolls_first() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: Some("#element".to_string()),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 2, "Should scroll to element then screenshot");
        assert_eq!(
            calls[0],
            MockCall::ScrollTo {
                selector: "#element".to_string()
            },
            "first call should scroll to the element"
        );
        assert_eq!(
            calls[1],
            MockCall::Screenshot,
            "second call should take the screenshot"
        );
    }

    #[tokio::test]
    async fn test_execute_action_screenshot_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(
            exec.cache_size() > 0,
            "cache should have entries before screenshot"
        );

        exec.execute_action(&Action::Screenshot {
            path: None,
            selector: None,
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            0,
            "cache should be cleared after screenshot"
        );
    }
}
