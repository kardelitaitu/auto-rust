//! Session and browser data export/import methods for TaskContext.

use anyhow::Result;
use std::collections::HashMap;

use crate::runtime::task_context::{deserialize_evaluated_json, TaskContext};

impl TaskContext {
    pub async fn export_session(&self, url: &str) -> Result<crate::task::policy::SessionData> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_session' permission",
                self.session_id
            ));
        }
        let cookies_result = self
            .page
            .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
            .await;
        let cookies_json = match cookies_result {
            Ok(cookies) => {
                serde_json::to_value(&cookies.cookies).unwrap_or(serde_json::Value::Array(vec![]))
            }
            Err(e) => {
                log::warn!("Failed to export cookies: {}", e);
                serde_json::Value::Array(vec![])
            }
        };
        let cookies = cookies_json.as_array().unwrap_or(&vec![]).clone();
        let local_storage_js = r#"
            (function() {
                const data = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        "#;
        let local_storage_str = self
            .page
            .evaluate(local_storage_js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;
        let local_storage_value = local_storage_str
            .value()
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let local_storage: HashMap<String, String> =
            deserialize_evaluated_json(local_storage_value).unwrap_or_default();
        let session_data = crate::task::policy::SessionData {
            cookies,
            local_storage,
            exported_at: chrono::Utc::now(),
            url: url.to_string(),
        };
        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_export_session",
            url,
            session_data.cookies.len()
        );
        Ok(session_data)
    }

    pub async fn import_session(
        &self,
        session_data: &crate::task::policy::SessionData,
    ) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_session' permission",
                self.session_id
            ));
        }
        self.import_cookies(&session_data.cookies).await?;
        let local_storage_json = serde_json::to_string(&session_data.local_storage)
            .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage: {}", e))?;
        let js_code = format!(
            r#"
            (function() {{
                const data = {};
                Object.entries(data).forEach(([k, v]) => {{
                    localStorage.setItem(k, v);
                }});
                return 'localStorage restored';
            }})()
            "#,
            local_storage_json
        );
        self.page
            .evaluate(js_code)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;
        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_import_session",
            session_data.url,
            session_data.cookies.len()
        );
        Ok(())
    }
}
