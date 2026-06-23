//! Browser API trait for DSL executor.
//!
//! Defines `DslApi` — a trait abstracting the browser API operations needed by
//! `DslExecutor`. This allows mock-based unit testing of the DSL executor
//! without a real browser connection.
//!
//! # Structure
//! - `DslApi` trait — 12 async methods for DOM interaction and querying
//! - `impl DslApi for TaskContext` — delegates to inherent TaskContext methods
//! - `MockDslApi` (test-only) — record-and-respond mock for unit tests

use anyhow::Result;

/// Browser API operations needed by DslExecutor.
///
/// Each method corresponds to a `TaskContext` inherent method used during
/// DSL task execution. The trait enables substituting a mock for testing.
#[async_trait::async_trait]
pub trait DslApi {
    /// Navigate to a URL with a timeout.
    async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()>;
    /// Click an element identified by selector.
    async fn click(&self, selector: &str) -> Result<()>;
    /// Type text into an element.
    async fn r#type(&self, selector: &str, text: &str) -> Result<()>;
    /// Scroll to an element.
    async fn scroll_to(&self, selector: &str) -> Result<()>;
    /// Get the text content of an element.
    async fn text(&self, selector: &str) -> Result<Option<String>>;
    /// Check if an element exists in the DOM.
    async fn exists(&self, selector: &str) -> Result<bool>;
    /// Check if an element is visible.
    async fn visible(&self, selector: &str) -> Result<bool>;
    /// Clear an input field.
    async fn clear(&self, selector: &str) -> Result<()>;
    /// Hover over an element.
    async fn hover(&self, selector: &str) -> Result<()>;
    /// Right-click on an element.
    async fn right_click(&self, selector: &str) -> Result<()>;
    /// Double-click on an element.
    async fn double_click(&self, selector: &str) -> Result<()>;
    /// Count elements matching a selector.
    async fn count_elements(&self, selector: &str) -> Result<usize>;
    /// Execute JavaScript in the page and return the result as a string.
    async fn execute_js(&self, script: &str) -> Result<String>;
    /// Take a screenshot of the current page. Returns the file path.
    async fn screenshot(&self) -> Result<String>;
}

// ── Real implementation for TaskContext ────────────────────────────────────

use crate::TaskContext;

#[async_trait::async_trait]
impl DslApi for TaskContext {
    async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()> {
        TaskContext::navigate(self, url, timeout_ms).await?;
        Ok(())
    }
    async fn click(&self, selector: &str) -> Result<()> {
        TaskContext::click(self, selector).await?;
        Ok(())
    }
    async fn r#type(&self, selector: &str, text: &str) -> Result<()> {
        TaskContext::r#type(self, selector, text).await?;
        Ok(())
    }
    async fn scroll_to(&self, selector: &str) -> Result<()> {
        TaskContext::scroll_to(self, selector).await?;
        Ok(())
    }
    async fn text(&self, selector: &str) -> Result<Option<String>> {
        TaskContext::text(self, selector).await
    }
    async fn exists(&self, selector: &str) -> Result<bool> {
        TaskContext::exists(self, selector).await
    }
    async fn visible(&self, selector: &str) -> Result<bool> {
        TaskContext::visible(self, selector).await
    }
    async fn clear(&self, selector: &str) -> Result<()> {
        TaskContext::clear(self, selector).await?;
        Ok(())
    }
    async fn hover(&self, selector: &str) -> Result<()> {
        TaskContext::hover(self, selector).await?;
        Ok(())
    }
    async fn right_click(&self, selector: &str) -> Result<()> {
        TaskContext::right_click(self, selector).await?;
        Ok(())
    }
    async fn double_click(&self, selector: &str) -> Result<()> {
        TaskContext::double_click(self, selector).await?;
        Ok(())
    }
    async fn count_elements(&self, selector: &str) -> Result<usize> {
        TaskContext::count_elements(self, selector).await
    }
    async fn execute_js(&self, script: &str) -> Result<String> {
        let result = self
            .page()
            .evaluate(script)
            .await
            .map_err(|e| anyhow::anyhow!("JS evaluation failed: {e}"))?;
        Ok(format!("{result:?}"))
    }
    async fn screenshot(&self) -> Result<String> {
        TaskContext::screenshot(self).await
    }
}

// ── Mock implementation for testing (only available in test builds) ────────

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Record of a single mock API call for test assertions.
    #[derive(Debug, Clone, PartialEq)]
    pub enum MockCall {
        Navigate { url: String, timeout_ms: u64 },
        Click { selector: String },
        Type { selector: String, text: String },
        ScrollTo { selector: String },
        Text { selector: String },
        Exists { selector: String },
        Visible { selector: String },
        Clear { selector: String },
        Hover { selector: String },
        RightClick { selector: String },
        DoubleClick { selector: String },
        CountElements { selector: String },
        ExecuteJs { script: String },
        Screenshot,
    }

    /// Mock implementation of `DslApi` for unit testing.
    ///
    /// Records all calls made to it and returns preconfigured results.
    /// Default behavior: all actions succeed unless overridden via result maps.
    pub struct MockDslApi {
        /// Record of all calls made to this mock.
        pub calls: Arc<Mutex<Vec<MockCall>>>,
        /// Predefined text results (selector → text).
        pub text_results: Arc<Mutex<HashMap<String, Option<String>>>>,
        /// Predefined exists results (selector → exists).
        pub exists_results: Arc<Mutex<HashMap<String, bool>>>,
        /// Predefined visible results (selector → visible).
        pub visible_results: Arc<Mutex<HashMap<String, bool>>>,
        /// Predefined count results (selector → count).
        pub count_results: Arc<Mutex<HashMap<String, usize>>>,
        /// Predefined JS execution results (script → result string).
        pub js_results: Arc<Mutex<HashMap<String, String>>>,
        /// If true, all mutating actions (click, type, etc.) will return Err.
        pub fail_all: Arc<Mutex<bool>>,
    }

    impl MockDslApi {
        /// Create a new MockDslApi with default (success) behavior.
        #[must_use]
        pub fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                text_results: Arc::new(Mutex::new(HashMap::new())),
                exists_results: Arc::new(Mutex::new(HashMap::new())),
                visible_results: Arc::new(Mutex::new(HashMap::new())),
                count_results: Arc::new(Mutex::new(HashMap::new())),
                js_results: Arc::new(Mutex::new(HashMap::new())),
                fail_all: Arc::new(Mutex::new(false)),
            }
        }

        /// Set a custom return value for `text()` on a specific selector.
        pub fn set_text_result(&self, selector: &str, text: Option<&str>) {
            self.text_results
                .lock()
                .unwrap()
                .insert(selector.to_string(), text.map(|s| s.to_string()));
        }

        /// Set a custom return value for `exists()` on a specific selector.
        pub fn set_exists_result(&self, selector: &str, exists: bool) {
            self.exists_results
                .lock()
                .unwrap()
                .insert(selector.to_string(), exists);
        }

        /// Set a custom return value for `visible()` on a specific selector.
        pub fn set_visible_result(&self, selector: &str, visible: bool) {
            self.visible_results
                .lock()
                .unwrap()
                .insert(selector.to_string(), visible);
        }

        /// Make all mutating actions (click, type, etc.) fail with an error.
        pub fn set_fail_all(&self, fail: bool) {
            *self.fail_all.lock().unwrap() = fail;
        }

        /// Return the list of recorded calls.
        pub fn get_calls(&self) -> Vec<MockCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    fn fail_or_ok(fail: bool) -> Result<()> {
        if fail {
            Err(anyhow::anyhow!("MockDslApi forced failure"))
        } else {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl DslApi for MockDslApi {
        async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::Navigate {
                url: url.to_string(),
                timeout_ms,
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn click(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::Click {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn r#type(&self, selector: &str, text: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::Type {
                selector: selector.to_string(),
                text: text.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn scroll_to(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::ScrollTo {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn text(&self, selector: &str) -> Result<Option<String>> {
            self.calls.lock().unwrap().push(MockCall::Text {
                selector: selector.to_string(),
            });
            let map = self.text_results.lock().unwrap();
            Ok(map.get(selector).cloned().flatten())
        }

        async fn exists(&self, selector: &str) -> Result<bool> {
            self.calls.lock().unwrap().push(MockCall::Exists {
                selector: selector.to_string(),
            });
            let map = self.exists_results.lock().unwrap();
            Ok(*map.get(selector).unwrap_or(&true))
        }

        async fn visible(&self, selector: &str) -> Result<bool> {
            self.calls.lock().unwrap().push(MockCall::Visible {
                selector: selector.to_string(),
            });
            let map = self.visible_results.lock().unwrap();
            Ok(*map.get(selector).unwrap_or(&true))
        }

        async fn clear(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::Clear {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn hover(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::Hover {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn right_click(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::RightClick {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn double_click(&self, selector: &str) -> Result<()> {
            self.calls.lock().unwrap().push(MockCall::DoubleClick {
                selector: selector.to_string(),
            });
            fail_or_ok(*self.fail_all.lock().unwrap())
        }

        async fn count_elements(&self, selector: &str) -> Result<usize> {
            self.calls.lock().unwrap().push(MockCall::CountElements {
                selector: selector.to_string(),
            });
            let map = self.count_results.lock().unwrap();
            Ok(*map.get(selector).unwrap_or(&5))
        }

        async fn execute_js(&self, script: &str) -> Result<String> {
            self.calls.lock().unwrap().push(MockCall::ExecuteJs {
                script: script.to_string(),
            });
            if *self.fail_all.lock().unwrap() {
                return Err(anyhow::anyhow!("MockDslApi forced failure"));
            }
            let map = self.js_results.lock().unwrap();
            Ok(map.get(script).cloned().unwrap_or_default())
        }

        async fn screenshot(&self) -> Result<String> {
            self.calls.lock().unwrap().push(MockCall::Screenshot);
            Ok("mock_screenshot.webp".to_string())
        }
    }
}
