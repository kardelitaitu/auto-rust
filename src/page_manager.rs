use chromiumoxide::Page;
use std::sync::Arc;
use log::{info, warn};
use dashmap::DashSet;
use chromiumoxide::Browser;

pub struct PageManager {
    session_id: String,
    browser: Browser,
    active_pages: DashSet<u64>,
}

impl PageManager {
    pub fn new(session_id: String, browser: Browser) -> Self {
        Self {
            session_id,
            browser,
            active_pages: DashSet::new(),
        }
    }

    pub fn browser(&self) -> &Browser {
        &self.browser
    }

    pub fn register_page(&self, page_id: u64) {
        self.active_pages.insert(page_id);
    }

    pub fn unregister_page(&self, page_id: u64) {
        self.active_pages.remove(&page_id);
    }

    pub fn active_page_count(&self) -> usize {
        self.active_pages.len()
    }

    pub async fn acquire_page(&self) -> anyhow::Result<Arc<Page>> {
        let page = self.browser.new_page("about:blank").await?;
        Ok(Arc::new(page))
    }

    pub async fn release_page(&self, page: Arc<Page>) {
        if let Err(e) = Arc::try_unwrap(page)
            .expect("Failed to unwrap Arc<Page>")
            .close()
            .await
        {
            warn!("[{}] Error closing page: {}", self.session_id, e);
        }
    }

    pub async fn close_pages_only(&self) -> anyhow::Result<()> {
        info!("[{}] close_pages_only", self.session_id);
        let pages = self.browser.pages().await?;
        for (i, page) in pages.iter().enumerate() {
            let page_clone = page.clone();
            if let Err(e) = page_clone.close().await {
                warn!("[{}] Error closing page {}: {}", self.session_id, i, e);
            }
        }
        self.browser.new_page("about:blank").await?;
        Ok(())
    }

    pub async fn close_browser(&mut self) -> anyhow::Result<()> {
        self.browser.close().await?;
        Ok(())
    }
}
