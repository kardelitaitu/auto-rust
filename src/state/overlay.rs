use crate::utils::page_size::Viewport;
use chromiumoxide::Page;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

static PAGE_OVERLAY_REGISTRY: Lazy<DashMap<String, Arc<SessionOverlayState>>> =
    Lazy::new(DashMap::new);

#[derive(Debug)]
pub struct SessionOverlayState {
    enabled: AtomicBool,
    cursor_initialized: AtomicBool,
    cursor_x: AtomicU64,
    cursor_y: AtomicU64,
    last_sync_ms: AtomicU64,
    active_page: Mutex<Option<Arc<Page>>>,
}

impl SessionOverlayState {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            cursor_initialized: AtomicBool::new(false),
            cursor_x: AtomicU64::new(0),
            cursor_y: AtomicU64::new(0),
            last_sync_ms: AtomicU64::new(0),
            active_page: Mutex::new(None),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_cursor_position(&self, x: f64, y: f64) {
        self.cursor_x.store(x.to_bits(), Ordering::Relaxed);
        self.cursor_y.store(y.to_bits(), Ordering::Relaxed);
        self.cursor_initialized.store(true, Ordering::Relaxed);
    }

    pub fn cursor_position_snapshot(&self) -> Option<(f64, f64)> {
        if !self.cursor_initialized.load(Ordering::Relaxed) {
            return None;
        }

        Some((
            f64::from_bits(self.cursor_x.load(Ordering::Relaxed)),
            f64::from_bits(self.cursor_y.load(Ordering::Relaxed)),
        ))
    }

    pub fn cursor_start_position(&self, viewport: &Viewport) -> (f64, f64) {
        if let Some((x, y)) = self.cursor_position_snapshot() {
            if x.is_finite()
                && y.is_finite()
                && x >= 0.0
                && y >= 0.0
                && x <= viewport.width
                && y <= viewport.height
            {
                return (x, y);
            }
        }

        (viewport.width / 2.0, viewport.height / 2.0)
    }

    pub fn claim_sync_slot(&self, now_ms: u64, force: bool, min_interval_ms: u64) -> bool {
        loop {
            let last = self.last_sync_ms.load(Ordering::Relaxed);
            if !force && now_ms.saturating_sub(last) < min_interval_ms {
                return false;
            }

            if self
                .last_sync_ms
                .compare_exchange(last, now_ms, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    pub fn set_active_page(&self, page: Arc<Page>) {
        *self.active_page.lock() = Some(page);
    }

    pub fn clear_active_page_if(&self, page_id: &str) {
        let mut active = self.active_page.lock();
        if active
            .as_ref()
            .map(|page| page.target_id().as_ref() == page_id)
            .unwrap_or(false)
        {
            *active = None;
        }
    }

    pub fn active_page(&self) -> Option<Arc<Page>> {
        self.active_page.lock().as_ref().cloned()
    }
}

pub fn bind_page_overlay(page_id: String, overlay_state: Arc<SessionOverlayState>) {
    PAGE_OVERLAY_REGISTRY.insert(page_id, overlay_state);
}

pub fn unbind_page_overlay(page_id: &str) {
    PAGE_OVERLAY_REGISTRY.remove(page_id);
}

pub fn overlay_for_page(page_id: &str) -> Option<Arc<SessionOverlayState>> {
    PAGE_OVERLAY_REGISTRY
        .get(page_id)
        .map(|entry| Arc::clone(entry.value()))
}

pub fn set_overlay_enabled_for_all(enabled: bool) {
    let mut seen = HashSet::new();
    for entry in PAGE_OVERLAY_REGISTRY.iter() {
        let overlay_state = Arc::clone(entry.value());
        let overlay_key = Arc::as_ptr(&overlay_state) as usize;
        if seen.insert(overlay_key) {
            overlay_state.set_enabled(enabled);
        }
    }
}

pub fn are_all_overlays_enabled() -> bool {
    let mut seen = HashSet::new();

    for entry in PAGE_OVERLAY_REGISTRY.iter() {
        let overlay_state = Arc::clone(entry.value());
        let overlay_key = Arc::as_ptr(&overlay_state) as usize;
        if !seen.insert(overlay_key) {
            continue;
        }
        if !overlay_state.is_enabled() {
            return false;
        }
    }

    true
}
