//! Style and layout query methods for `TaskContext`.

use anyhow::Result;

use crate::runtime::task_context::{Rect, TaskContext};

impl TaskContext {
    pub async fn get_computed_style(&self, selector: &str, property: &str) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }
        let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let escaped_property = property.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r"
            (function() {{
                const el = document.querySelector('{escaped_selector}');
                if (!el) return null;
                return window.getComputedStyle(el).getPropertyValue('{escaped_property}');
            }})()
            "
        );
        let result = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        result
            .value()
            .and_then(|v| v.as_str().map(std::string::ToString::to_string))
            .ok_or_else(|| anyhow::anyhow!("Element not found: {selector}"))
    }

    pub async fn get_element_rect(&self, selector: &str) -> Result<Rect> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }
        let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r"
            (function() {{
                const el = document.querySelector('{escaped_selector}');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()
            "
        );
        let result = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        if let Some(obj) = result.value().and_then(|v| v.as_object()) {
            let x = obj
                .get("x")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let y = obj
                .get("y")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let width = obj
                .get("width")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let height = obj
                .get("height")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            Ok(Rect {
                x,
                y,
                width,
                height,
            })
        } else {
            Err(anyhow::anyhow!("Element not found: {selector}"))
        }
    }

    pub async fn get_scroll_position(&self) -> Result<(u32, u32)> {
        let js = r"
            (function() {
                return JSON.stringify({
                    x: window.scrollX || window.pageXOffset,
                    y: window.scrollY || window.pageYOffset
                });
            })()
        ";
        let result = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        if let Some(value) = result.value() {
            if let Some(s) = value.as_str() {
                if let Ok(pos) = serde_json::from_str::<serde_json::Value>(s) {
                    let x = pos
                        .get("x")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0) as u32;
                    let y = pos
                        .get("y")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0) as u32;
                    return Ok((x, y));
                }
            }
            if let Some(obj) = value.as_object() {
                let x = obj
                    .get("x")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0) as u32;
                let y = obj
                    .get("y")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0) as u32;
                return Ok((x, y));
            }
        }
        Ok((0, 0))
    }

    pub async fn count_elements(&self, selector: &str) -> Result<usize> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }
        let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r"
            (function() {{
                return document.querySelectorAll('{escaped_selector}').length;
            }})()
            "
        );
        let result = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        let count = result
            .value()
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
        Ok(count)
    }

    pub async fn is_in_viewport(&self, selector: &str) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }
        let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r"
            (function() {{
                const el = document.querySelector('{escaped_selector}');
                if (!el) return false;
                const r = el.getBoundingClientRect();
                const w = window.innerWidth || document.documentElement.clientWidth;
                const h = window.innerHeight || document.documentElement.clientHeight;
                return r.top < h && r.bottom > 0 && r.left < w && r.right > 0;
            }})()
            "
        );
        let result = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        Ok(result
            .value()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod fuzz_tests {
    use crate::runtime::task_context::Rect;
    use proptest::prelude::*;
    use serde_json::Value;

    /// Convert an arbitrary string to a Value: valid JSON or fallback to string.
    fn val(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    }

    /// Simulate the get_element_rect value processing logic.
    fn simulate_rect_from_value(value: &Value) -> Option<Rect> {
        let obj = value.as_object()?;
        let x = obj.get("x").and_then(Value::as_f64).unwrap_or(0.0);
        let y = obj.get("y").and_then(Value::as_f64).unwrap_or(0.0);
        let width = obj.get("width").and_then(Value::as_f64).unwrap_or(0.0);
        let height = obj.get("height").and_then(Value::as_f64).unwrap_or(0.0);
        Some(Rect {
            x,
            y,
            width,
            height,
        })
    }

    proptest! {
        #[test]
        fn fuzz_get_element_rect_processing(s: String) {
            let value = val(&s);
            let _ = simulate_rect_from_value(&value);
        }

        #[test]
        fn fuzz_get_element_rect_with_fields(x: String, y: String, width: String, height: String) {
            let obj = serde_json::json!({
                "x": val(&x), "y": val(&y),
                "width": val(&width), "height": val(&height)
            });
            let _ = simulate_rect_from_value(&obj);
        }

        #[test]
        fn fuzz_get_computed_style_value(s: String) {
            let value = val(&s);
            let _ = value.as_str().map(std::string::ToString::to_string);
        }

        #[test]
        fn fuzz_count_elements_value(s: String) {
            let value = val(&s);
            let _ = value.as_u64().unwrap_or(0) as usize;
        }

        #[test]
        fn fuzz_is_in_viewport_value(s: String) {
            let value = val(&s);
            let _ = value.as_bool().unwrap_or(false);
        }

        #[test]
        fn fuzz_scroll_position_processing(s: String) {
            let value = val(&s);
            if let Some(str_val) = value.as_str() {
                let _ = serde_json::from_str::<Value>(str_val);
            }
            if let Some(obj) = value.as_object() {
                let _x = obj.get("x").and_then(Value::as_f64).unwrap_or(0.0) as u32;
                let _y = obj.get("y").and_then(Value::as_f64).unwrap_or(0.0) as u32;
            }
        }
    }
}
