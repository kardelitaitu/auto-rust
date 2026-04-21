//! Session-scoped state handles.

pub mod overlay;

pub use crate::internal::clipboard::ClipboardState;
pub use overlay::{
    are_all_overlays_enabled, bind_page_overlay, overlay_for_page, set_overlay_enabled_for_all,
    unbind_page_overlay, SessionOverlayState,
};
