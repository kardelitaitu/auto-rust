//! Wait/pause actions for DSL executor.
//!
//! Handlers for: wait (duration-based sleep), wait_for (element existence
//! with timeout). These are non-DOM-mutating actions and do NOT clear
//! the selector cache.

use anyhow::Result;

impl<T: super::super::DslApi> super::super::DslExecutor<'_, T> {
    pub(crate) async fn execute_wait(&mut self, duration_ms: u64) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    pub(crate) async fn execute_wait_for(
        &mut self,
        selector: &str,
        timeout_ms: Option<u64>,
    ) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        let timeout = timeout_ms.unwrap_or(5000);
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout);
        while tokio::time::Instant::now() < deadline {
            if self.cached_exists(&resolved_selector).await? {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Err(anyhow::anyhow!(
            "Timeout waiting for element: {resolved_selector}"
        ))
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
    async fn test_execute_action_wait_does_not_clear_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            SelectorCacheEntry::new(true, true, None, 0),
        );
        let cache_before = exec.cache_size();

        exec.execute_action(&Action::Wait { duration_ms: 1 })
            .await
            .unwrap();

        assert_eq!(
            exec.cache_size(),
            cache_before,
            "Wait should NOT clear cache"
        );
    }

    #[tokio::test]
    async fn test_execute_wait_for_succeeds_when_element_exists() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#loaded", true);
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::WaitFor {
            selector: "#loaded".to_string(),
            timeout_ms: Some(500),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::task::dsl::api::mock::MockCall::Exists { selector } if selector == "#loaded")),
            "WaitFor should call exists with the correct selector"
        );
    }

    #[tokio::test]
    async fn test_execute_wait_for_timeout_when_element_missing() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#never", false);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::WaitFor {
                selector: "#never".to_string(),
                timeout_ms: Some(200),
            })
            .await;

        assert!(
            result.is_err(),
            "WaitFor should timeout when element never appears"
        );
        assert!(
            result.unwrap_err().to_string().contains("Timeout"),
            "error should mention timeout"
        );
    }

    #[tokio::test]
    async fn test_execute_wait_for_variable_substitution() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#dynamic", true);
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("target".to_string(), "dynamic".to_string());

        exec.execute_action(&Action::WaitFor {
            selector: "#${target}".to_string(),
            timeout_ms: Some(500),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, crate::task::dsl::api::mock::MockCall::Exists { selector } if selector == "#dynamic")),
            "selector should be substituted before checking"
        );
    }

    #[tokio::test]
    async fn test_execute_wait_for_default_timeout() {
        let mock = MockDslApi::new();
        mock.set_exists_result("#absent", false);
        let mut exec = create_executor(&mock, vec![]);

        let start = std::time::Instant::now();
        let result = exec
            .execute_action(&Action::WaitFor {
                selector: "#absent".to_string(),
                timeout_ms: Some(150),
            })
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(
            elapsed.as_millis() >= 100,
            "should wait at least close to the timeout duration"
        );
    }
}
