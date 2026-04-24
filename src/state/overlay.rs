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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_session_overlay_state_initialization() {
        let state = SessionOverlayState::new(true);
        assert!(state.is_enabled());

        let state = SessionOverlayState::new(false);
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_session_overlay_state_enable_disable() {
        let state = SessionOverlayState::new(false);
        assert!(!state.is_enabled());

        state.set_enabled(true);
        assert!(state.is_enabled());

        state.set_enabled(false);
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_cursor_position_storage_and_retrieval() {
        let state = SessionOverlayState::new(true);

        // Initially should return None
        assert!(state.cursor_position_snapshot().is_none());

        // Set cursor position
        state.set_cursor_position(100.5, 200.75);

        // Should return the set position
        let pos = state.cursor_position_snapshot();
        assert!(pos.is_some());
        let (x, y) = pos.unwrap();
        assert_eq!(x, 100.5);
        assert_eq!(y, 200.75);
    }

    #[test]
    fn test_cursor_start_position_in_bounds() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor within bounds
        state.set_cursor_position(400.0, 300.0);

        let (x, y) = state.cursor_start_position(&viewport);
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_cursor_start_position_out_of_bounds() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor outside bounds
        state.set_cursor_position(-100.0, 300.0);

        let (x, y) = state.cursor_start_position(&viewport);
        // Should fall back to center
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_cursor_start_position_not_initialized() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        let (x, y) = state.cursor_start_position(&viewport);
        // Should fall back to center
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_cursor_start_position_negative_values() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor with negative values
        state.set_cursor_position(-50.0, -100.0);

        let (x, y) = state.cursor_start_position(&viewport);
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_cursor_start_position_exceeds_viewport() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor beyond viewport
        state.set_cursor_position(1000.0, 700.0);

        let (x, y) = state.cursor_start_position(&viewport);
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_claim_sync_slot_success() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 100;

        // First claim should succeed
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));
    }

    #[test]
    fn test_claim_sync_slot_too_soon() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 100;

        // First claim
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));

        // Second claim too soon should fail
        assert!(!state.claim_sync_slot(now_ms + 50, false, min_interval_ms));
    }

    #[test]
    fn test_claim_sync_slot_after_interval() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 100;

        // First claim
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));

        // Claim after interval should succeed
        assert!(state.claim_sync_slot(now_ms + 150, false, min_interval_ms));
    }

    #[test]
    fn test_claim_sync_slot_force() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 100;

        // First claim
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));

        // Force claim should succeed regardless of interval
        assert!(state.claim_sync_slot(now_ms + 50, true, min_interval_ms));
    }

    #[test]
    fn test_bind_and_unbind_page_overlay() {
        let page_id = "test-page-1".to_string();
        let overlay_state = Arc::new(SessionOverlayState::new(true));

        // Bind overlay
        bind_page_overlay(page_id.clone(), overlay_state.clone());

        // Retrieve overlay
        let retrieved = overlay_for_page(&page_id);
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_enabled());

        // Unbind overlay
        unbind_page_overlay(&page_id);

        // Should no longer be retrievable
        let retrieved = overlay_for_page(&page_id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_overlay_for_page_not_found() {
        let retrieved = overlay_for_page("non-existent-page");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_set_overlay_enabled_for_all() {
        let page_id1 = "test-page-1".to_string();
        let page_id2 = "test-page-2".to_string();

        let overlay1 = Arc::new(SessionOverlayState::new(true));
        let overlay2 = Arc::new(SessionOverlayState::new(false));

        bind_page_overlay(page_id1.clone(), overlay1.clone());
        bind_page_overlay(page_id2.clone(), overlay2.clone());

        // Disable all
        set_overlay_enabled_for_all(false);
        assert!(!overlay1.is_enabled());
        assert!(!overlay2.is_enabled());

        // Enable all
        set_overlay_enabled_for_all(true);
        assert!(overlay1.is_enabled());
        assert!(overlay2.is_enabled());

        // Cleanup
        unbind_page_overlay(&page_id1);
        unbind_page_overlay(&page_id2);
    }

    #[test]
    fn test_are_all_overlays_enabled_true() {
        let page_id1 = "test-page-1".to_string();
        let page_id2 = "test-page-2".to_string();

        let overlay1 = Arc::new(SessionOverlayState::new(true));
        let overlay2 = Arc::new(SessionOverlayState::new(true));

        bind_page_overlay(page_id1.clone(), overlay1);
        bind_page_overlay(page_id2.clone(), overlay2);

        assert!(are_all_overlays_enabled());

        // Cleanup
        unbind_page_overlay(&page_id1);
        unbind_page_overlay(&page_id2);
    }

    #[test]
    fn test_are_all_overlays_enabled_false() {
        let page_id1 = "test-page-1".to_string();
        let page_id2 = "test-page-2".to_string();

        let overlay1 = Arc::new(SessionOverlayState::new(true));
        let overlay2 = Arc::new(SessionOverlayState::new(false));

        bind_page_overlay(page_id1.clone(), overlay1);
        bind_page_overlay(page_id2.clone(), overlay2);

        assert!(!are_all_overlays_enabled());

        // Cleanup
        unbind_page_overlay(&page_id1);
        unbind_page_overlay(&page_id2);
    }

    #[test]
    fn test_are_all_overlays_enabled_empty_registry() {
        // Empty registry should return true (vacuously true)
        assert!(are_all_overlays_enabled());
    }

    #[test]
    fn test_concurrent_cursor_position_access() {
        let state = Arc::new(SessionOverlayState::new(true));
        let mut handles = vec![];

        for i in 0..10 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let x = (i * 100 + j) as f64;
                    let y = (i * 100 + j * 2) as f64;
                    state_clone.set_cursor_position(x, y);
                    thread::sleep(Duration::from_micros(10));
                    let pos = state_clone.cursor_position_snapshot();
                    assert!(pos.is_some());
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Final position should be retrievable
        assert!(state.cursor_position_snapshot().is_some());
    }

    #[test]
    fn test_concurrent_enable_disable() {
        let state = Arc::new(SessionOverlayState::new(true));
        let mut handles = vec![];

        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    state_clone.set_enabled(true);
                    thread::sleep(Duration::from_micros(10));
                    state_clone.set_enabled(false);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Final state should be either enabled or disabled (no panic)
        let _ = state.is_enabled();
    }

    #[test]
    fn test_concurrent_claim_sync_slot() {
        let state = Arc::new(SessionOverlayState::new(true));
        let mut handles = vec![];
        let now_ms = 1000;
        let min_interval_ms = 10;

        for i in 0..10 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                let mut successes = 0;
                for j in 0..50 {
                    let time = now_ms + (i * 1000) + (j * min_interval_ms);
                    if state_clone.claim_sync_slot(time, false, min_interval_ms) {
                        successes += 1;
                    }
                }
                successes
            });
            handles.push(handle);
        }

        let mut total_successes = 0;
        for handle in handles {
            total_successes += handle.join().unwrap();
        }

        // Each thread should succeed on its own claims
        assert!(total_successes > 0);
    }

    #[test]
    fn test_active_page_management() {
        let state = SessionOverlayState::new(true);

        // Initially no active page
        assert!(state.active_page().is_none());

        // Note: We can't easily test with actual Page objects without mocking,
        // but we can test the locking mechanism works
        let _ = state.active_page();
    }

    #[test]
    fn test_cursor_position_infinity_handling() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor to infinity
        state.set_cursor_position(f64::INFINITY, f64::INFINITY);

        let (x, y) = state.cursor_start_position(&viewport);
        // Should fall back to center
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_cursor_position_nan_handling() {
        let state = SessionOverlayState::new(true);
        let viewport = Viewport {
            width: 800.0,
            height: 600.0,
        };

        // Set cursor to NaN
        state.set_cursor_position(f64::NAN, f64::NAN);

        let (x, y) = state.cursor_start_position(&viewport);
        // Should fall back to center
        assert_eq!(x, 400.0);
        assert_eq!(y, 300.0);
    }

    #[test]
    fn test_claim_sync_slot_with_zero_interval() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 0;

        // Should always succeed with zero interval
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));
    }

    #[test]
    fn test_claim_sync_slot_compare_and_swap() {
        let state = SessionOverlayState::new(true);
        let now_ms = 1000;
        let min_interval_ms = 100;

        // First claim
        assert!(state.claim_sync_slot(now_ms, false, min_interval_ms));

        // Simulate time passing
        let later_ms = now_ms + min_interval_ms;

        // Should succeed after interval
        assert!(state.claim_sync_slot(later_ms, false, min_interval_ms));

        // Check that the timestamp was updated
        assert!(!state.claim_sync_slot(later_ms, false, min_interval_ms));
    }
}
