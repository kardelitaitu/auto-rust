//! Navigation and page management methods for TaskContext.

use anyhow::{Context, Result};

use crate::capabilities::{navigation, timing};
use crate::runtime::task_context::TaskContext;

impl TaskContext {
    pub async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()> {
        let navigate_timeout_ms = timeout_ms.max(super::MIN_NAVIGATE_TIMEOUT_MS);
        navigation::goto(self.page(), url, navigate_timeout_ms)
            .await
            .with_context(|| {
                format!(
                    "navigate_timeout | stage=goto url={} timeout_ms={}",
                    url, navigate_timeout_ms
                )
            })?;
        let action_delay = &self.behavior_runtime.action_delay;
        timing::human_pause(
            action_delay.min_ms,
            action_delay.variance_pct.round() as u32,
        )
        .await;
        let settle_base = action_delay
            .min_ms
            .saturating_add(navigate_timeout_ms.min(2_000) / 4)
            .clamp(150, 4_000);
        let settle_variance = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
        timing::human_pause(settle_base, settle_variance).await;
        let settle_ms = navigate_timeout_ms.min(3_000);
        self.wait_for_load(settle_ms).await.with_context(|| {
            format!(
                "navigate_timeout | stage=settle_load url={} timeout_ms={}",
                url, settle_ms
            )
        })?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub fn check_permission(&self, permission: &'static str) -> crate::error::Result<()> {
        let perms = self.policy.effective_permissions();
        let has_permission = match permission {
            "allow_screenshot" => perms.allow_screenshot,
            "allow_export_cookies" => perms.allow_export_cookies,
            "allow_import_cookies" => perms.allow_import_cookies,
            "allow_export_session" => perms.allow_export_session,
            "allow_import_session" => perms.allow_import_session,
            "allow_session_clipboard" => perms.allow_session_clipboard,
            "allow_read_data" => perms.allow_read_data,
            "allow_write_data" => perms.allow_write_data,
            "allow_http_requests" => perms.allow_http_requests,
            "allow_dom_inspection" => perms.allow_dom_inspection,
            "allow_browser_export" => perms.allow_browser_export,
            "allow_browser_import" => perms.allow_browser_import,
            _ => {
                log::warn!("Unknown permission '{}' requested", permission);
                false
            }
        };
        if has_permission {
            Ok(())
        } else {
            Err(crate::error::TaskError::PermissionDenied {
                permission,
                task_name: self.session_id.clone(),
            }
            .into())
        }
    }

    pub async fn check_page_connected(&self) -> crate::error::Result<()> {
        match self.page.evaluate("1").await {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::TaskError::CdpError {
                operation: "Page.connection_check".to_string(),
                reason: format!("Page not responding to CDP: {}", e),
            }
            .into()),
        }
    }

    pub async fn screenshot(&self) -> Result<String> {
        self.screenshot_with_quality(50).await
    }

    pub async fn screenshot_with_quality(&self, quality: u8) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_screenshot {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_screenshot' permission",
                self.session_id
            ));
        }
        let quality = quality.clamp(1, 100);
        let png_bytes = self
            .page
            .screenshot(
                chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Page.captureScreenshot - {}", e))?;
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load PNG image: {}", e))?;
        let rgb_img = img.to_rgb8();
        let (width, height) = (rgb_img.width(), rgb_img.height());
        let encoder = webp::Encoder::new(rgb_img.as_raw(), webp::PixelLayout::Rgb, width, height);
        let webp_data = encoder.encode(quality as f32);
        let now = chrono::Utc::now();
        let filename = format!(
            "{}-{}-{}.webp",
            now.format("%Y-%m-%d"),
            now.format("%H-%M"),
            self.session_id
        );
        let screenshot_dir = std::path::Path::new("data/screenshot");
        std::fs::create_dir_all(screenshot_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create screenshot directory: {}", e))?;
        let file_path = screenshot_dir.join(&filename);
        std::fs::write(&file_path, &*webp_data)
            .map_err(|e| anyhow::anyhow!("Failed to write screenshot: {}", e))?;
        file_path
            .to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid screenshot path"))
    }
}
