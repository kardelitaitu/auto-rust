//! Cookie management methods for TaskContext.

use anyhow::Result;
use serde_json::Value;

use crate::runtime::task_context::TaskContext;

impl TaskContext {
    pub async fn export_cookies(&self, _url: &str) -> Result<Vec<Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }
        let cookies = self
            .page
            .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Network.getCookies - {}", e))?;
        let json = serde_json::to_value(&cookies.cookies)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cookies: {}", e))?;
        Ok(json.as_array().unwrap_or(&vec![]).clone())
    }

    pub async fn export_cookies_for_domain(&self, domain: &str) -> Result<Vec<Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }
        let all_cookies = self.export_cookies("").await?;
        let filtered: Vec<Value> = all_cookies
            .into_iter()
            .filter(|cookie| {
                cookie
                    .get("domain")
                    .and_then(|d| d.as_str())
                    .map(|d| d == domain || d == format!(".{}", domain).as_str())
                    .unwrap_or(false)
            })
            .collect();
        log::warn!(
            "task_policy_audit: task={} permission={} domain={} count={}",
            self.session_id,
            "allow_export_cookies",
            domain,
            filtered.len()
        );
        Ok(filtered)
    }

    pub async fn export_session_cookies(&self, _url: &str) -> Result<Vec<Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }
        let all_cookies = self.export_cookies("").await?;
        let session_cookies: Vec<Value> = all_cookies
            .into_iter()
            .filter(|cookie| {
                cookie
                    .get("session")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false)
                    || cookie.get("expires").is_none()
                    || cookie
                        .get("expires")
                        .map(|e| e.is_null() || e.as_f64() == Some(0.0) || e.as_f64() == Some(-1.0))
                        .unwrap_or(true)
            })
            .collect();
        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id,
            "allow_export_cookies",
            _url,
            session_cookies.len()
        );
        Ok(session_cookies)
    }

    pub async fn has_cookie(&self, name: &str, domain: Option<&str>) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }
        let cookies = if let Some(d) = domain {
            self.export_cookies_for_domain(d).await?
        } else {
            self.export_cookies("").await?
        };
        let exists = cookies.iter().any(|cookie| {
            cookie
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == name)
                .unwrap_or(false)
        });
        Ok(exists)
    }

    pub async fn import_cookies(&self, cookies: &[Value]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_cookies' permission",
                self.session_id
            ));
        }
        for cookie in cookies {
            let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let domain = cookie.get("domain").and_then(|v| v.as_str());
            let path = cookie.get("path").and_then(|v| v.as_str());
            if name.is_empty() || value.is_empty() {
                continue;
            }
            let mut params =
                chromiumoxide::cdp::browser_protocol::network::SetCookieParams::builder()
                    .name(name)
                    .value(value);
            if let Some(d) = domain {
                params = params.domain(d);
            }
            if let Some(p) = path {
                params = params.path(p);
            }
            let params = params
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build SetCookieParams: {}", e))?;
            self.page
                .execute(params)
                .await
                .map_err(|e| anyhow::anyhow!("CDP error: Network.setCookie - {}", e))?;
        }
        log::warn!(
            "task_policy_audit: task={} permission={} count={}",
            self.session_id,
            "allow_import_cookies",
            cookies.len()
        );
        Ok(())
    }
}
