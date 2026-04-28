//! Session-scoped state handles.

pub mod overlay;

pub use crate::internal::clipboard::ClipboardState;
pub use overlay::{
    are_all_overlays_enabled, bind_page_overlay, overlay_for_page, set_overlay_enabled_for_all,
    unbind_page_overlay, SessionOverlayState,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test to verify all re-exports are accessible.
    #[test]
    fn test_state_re_exports_exist() {
        // These just need to compile - verifies module structure
        let _: Option<ClipboardState> = None;
        let _: Option<SessionOverlayState> = None;
    }

    /// Smoke test to verify overlay functions are accessible.
    #[test]
    fn test_overlay_functions_exist() {
        // Just verify function signatures exist by referencing them
        let _: fn(&str) = unbind_page_overlay;
        let _: fn() -> bool = are_all_overlays_enabled;
    }
}
