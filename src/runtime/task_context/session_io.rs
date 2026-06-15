//! Session and browser data export/import methods for `TaskContext`.

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
                log::warn!("Failed to export cookies: {e}");
                serde_json::Value::Array(vec![])
            }
        };
        let cookies = cookies_json.as_array().unwrap_or(&vec![]).clone();
        let local_storage_js = r"
            (function() {
                const data = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        ";
        let local_storage_value = self
            .session_io_evaluate_with_retry(local_storage_js)
            .await
            .unwrap_or_else(|e| {
                log::warn!("Failed to export localStorage (after retries): {e}");
                serde_json::Value::Null
            });
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
            .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage: {e}"))?;
        let js_code = format!(
            r"
            (function() {{
                const data = {local_storage_json};
                Object.entries(data).forEach(([k, v]) => {{
                    localStorage.setItem(k, v);
                }});
                return 'localStorage restored';
            }})()
            "
        );
        self.session_io_evaluate_with_retry(&js_code)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;
        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_import_session",
            session_data.url,
            session_data.cookies.len()
        );
        Ok(())
    }

    // --- Browser-level export/import ---

    pub async fn export_browser(&self, url: &str) -> Result<crate::task::policy::BrowserData> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_browser_export {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_browser_export' permission",
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
                log::warn!("Failed to export cookies during browser export: {e}");
                serde_json::Value::Array(vec![])
            }
        };
        let cookies = cookies_json.as_array().unwrap_or(&vec![]).clone();

        let local_storage_js = r"
            (function() {
                const data = {};
                const hostname = window.location.hostname;
                data[hostname] = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[hostname][key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        ";
        let local_storage_value = self
            .session_io_evaluate_with_retry(local_storage_js)
            .await
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to export localStorage during browser export (after retries): {e}"
                );
                serde_json::Value::Null
            });
        let local_storage: std::collections::HashMap<
            String,
            std::collections::HashMap<String, String>,
        > = deserialize_evaluated_json(local_storage_value).unwrap_or_default();

        let session_storage_js = r"
            (function() {
                const data = {};
                const hostname = window.location.hostname;
                data[hostname] = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const key = sessionStorage.key(i);
                    data[hostname][key] = sessionStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        ";
        let session_storage_value = self
            .session_io_evaluate_with_retry(session_storage_js)
            .await
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to export sessionStorage during browser export (after retries): {e}"
                );
                serde_json::Value::Null
            });
        let session_storage: std::collections::HashMap<
            String,
            std::collections::HashMap<String, String>,
        > = deserialize_evaluated_json(session_storage_value).unwrap_or_default();

        let indexeddb_js = r"
            (function() {
                return new Promise((resolve) => {
                    const hostname = window.location.hostname;
                    const data = {};
                    data[hostname] = [];

                    if (!window.indexedDB) {
                        resolve(JSON.stringify(data));
                        return;
                    }

                    if (window.indexedDB.databases) {
                        window.indexedDB.databases().then(dbs => {
                            data[hostname] = dbs.map(db => db.name);
                            resolve(JSON.stringify(data));
                        }).catch(() => {
                            resolve(JSON.stringify(data));
                        });
                    } else {
                        resolve(JSON.stringify(data));
                    }
                });
            })()
        ";
        let indexeddb_names: std::collections::HashMap<String, Vec<String>> =
            match self.session_io_evaluate_with_retry(indexeddb_js).await {
                Ok(result) => serde_json::from_value(result).unwrap_or_default(),
                Err(e) => {
                    log::warn!("Failed to export IndexedDB names (after retries): {e}");
                    std::collections::HashMap::new()
                }
            };

        let browser_data = crate::task::policy::BrowserData {
            cookies,
            local_storage,
            session_storage,
            indexeddb_names,
            exported_at: chrono::Utc::now(),
            source: url.to_string(),
            browser_version: None,
        };

        log::warn!(
            "task_policy_audit: task={} permission={} url={} cookies={} origins={}",
            self.session_id,
            "allow_browser_export",
            url,
            browser_data.cookies.len(),
            browser_data.local_storage.len()
        );

        Ok(browser_data)
    }

    pub async fn import_browser(
        &self,
        browser_data: &crate::task::policy::BrowserData,
    ) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_browser_import {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_browser_import' permission",
                self.session_id
            ));
        }

        self.import_cookies(&browser_data.cookies).await?;

        for (origin, data) in &browser_data.local_storage {
            let local_storage_json = serde_json::to_string(data).map_err(|e| {
                anyhow::anyhow!("Failed to serialize localStorage for {origin}: {e}")
            })?;
            let js_code = format!(
                r"
                (function() {{
                    const data = {local_storage_json};
                    let count = 0;
                    Object.entries(data).forEach(([k, v]) => {{
                        try {{
                            localStorage.setItem(k, v);
                            count++;
                        }} catch (e) {{
                            console.warn('Failed to set localStorage item:', k, e);
                        }}
                    }});
                    return 'localStorage imported: ' + count + ' items for origin';
                }})()
                "
            );
            self.session_io_evaluate_with_retry(&js_code)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("CDP error: Runtime.evaluate for localStorage import - {e}")
                })?;
        }

        for (origin, data) in &browser_data.session_storage {
            let session_storage_json = serde_json::to_string(data).map_err(|e| {
                anyhow::anyhow!("Failed to serialize sessionStorage for {origin}: {e}")
            })?;
            let js_code = format!(
                r"
                (function() {{
                    const data = {session_storage_json};
                    let count = 0;
                    Object.entries(data).forEach(([k, v]) => {{
                        try {{
                            sessionStorage.setItem(k, v);
                            count++;
                        }} catch (e) {{
                            console.warn('Failed to set sessionStorage item:', k, e);
                        }}
                    }});
                    return 'sessionStorage imported: ' + count + ' items for origin';
                }})()
                "
            );
            self.session_io_evaluate_with_retry(&js_code)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("CDP error: Runtime.evaluate for sessionStorage import - {e}")
                })?;
        }

        log::warn!(
            "task_policy_audit: task={} permission={} source={} cookies={} origins={}",
            self.session_id,
            "allow_browser_import",
            browser_data.source,
            browser_data.cookies.len(),
            browser_data.local_storage.len()
        );

        Ok(())
    }

    pub async fn export_local_storage(
        &self,
        _url: &str,
    ) -> Result<std::collections::HashMap<String, String>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_session' permission",
                self.session_id
            ));
        }

        let local_storage_js = r"
            (function() {
                const data = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        ";
        let local_storage_value = self
            .session_io_evaluate_with_retry(local_storage_js)
            .await
            .unwrap_or_else(|e| {
                log::warn!("Failed to export localStorage (after retries): {e}");
                serde_json::Value::Null
            });
        let local_storage: std::collections::HashMap<String, String> =
            deserialize_evaluated_json(local_storage_value).unwrap_or_default();

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_export_session",
            _url,
            local_storage.len()
        );

        Ok(local_storage)
    }

    pub async fn import_local_storage(
        &self,
        _url: &str,
        data: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_session' permission",
                self.session_id
            ));
        }

        let local_storage_json = serde_json::to_string(data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage: {e}"))?;
        let js_code = format!(
            r"
            (function() {{
                const data = {local_storage_json};
                Object.entries(data).forEach(([k, v]) => {{
                    localStorage.setItem(k, v);
                }});
                return 'localStorage imported: ' + Object.keys(data).length + ' items';
            }})()
            "
        );
        self.session_io_evaluate_with_retry(&js_code)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {e}"))?;

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_import_session",
            _url,
            data.len()
        );

        Ok(())
    }

    #[must_use]
    pub fn validate_session_data_for_tests(data: &crate::task::policy::SessionData) -> Vec<String> {
        super::validate_session_data_impl(data)
    }

    /// Retry wrapper for CDP evaluate operations in session I/O.
    /// Delegates to the shared `with_retry` helper on TaskContext.
    async fn session_io_evaluate_with_retry(&self, js: &str) -> Result<serde_json::Value> {
        let result = self
            .with_retry(|| async { self.page.evaluate(js).await.map_err(anyhow::Error::from) })
            .await?;
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Null))
    }
}
