//! HTTP request methods for `TaskContext`.

use anyhow::Result;

use crate::runtime::task_context::{HttpResponse, TaskContext};

impl TaskContext {
    pub async fn http_get(&self, url: &str) -> Result<HttpResponse> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {e}"))?;
        let status = response.status().as_u16();
        let headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {e}"))?;
        Ok(HttpResponse {
            status: status as u16,
            body,
            headers,
        })
    }

    pub async fn http_post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<HttpResponse> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;
        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP POST request failed: {e}"))?;
        let status = response.status().as_u16();
        let headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {e}"))?;
        Ok(HttpResponse {
            status: status as u16,
            body,
            headers,
        })
    }

    pub async fn download_file(&self, url: &str, relative_path: &str) -> Result<u64> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;
        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Download request failed: {e}"))?
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read download bytes: {e}"))?;
        let size = bytes.len() as u64;
        self.write_data_file(relative_path, &bytes)?;
        Ok(size)
    }
}
