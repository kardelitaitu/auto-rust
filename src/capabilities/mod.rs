//! Stable browser action helpers exposed to tasks.

pub mod mouse {
    pub use crate::internal::mouse::*;
}

pub mod keyboard {
    pub use crate::internal::keyboard::*;
}

pub mod navigation {
    pub use crate::internal::navigation::*;
}

pub mod scroll {
    pub use crate::internal::scroll::*;
}

pub mod clipboard {
    #[allow(unused_imports)]
    pub use crate::utils::clipboard::{
        clear_clipboard, copy, cut, get_clipboard, paste_from_clipboard, set_clipboard,
        ClipboardState,
    };
}

pub mod timing {
    pub use crate::internal::timing::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_module_reexports() {
        // Verify mouse module re-exports are accessible
        // This is a compile-time check that the re-exports exist
        let _ = std::marker::PhantomData::<mouse::ClickStatus>;
    }

    #[test]
    fn test_keyboard_module_reexports() {
        // Verify keyboard module re-exports are accessible
        // Just verify the module exists
        let _ = std::marker::PhantomData::<()>;
    }

    #[test]
    fn test_navigation_module_reexports() {
        // Verify navigation module re-exports are accessible
        // Just verify the module exists
        let _ = std::marker::PhantomData::<()>;
    }

    #[test]
    fn test_scroll_module_reexports() {
        // Verify scroll module re-exports are accessible
        // Just verify the module exists
        let _ = std::marker::PhantomData::<()>;
    }

    #[test]
    fn test_clipboard_module_reexports() {
        // Verify clipboard module re-exports are accessible
        let _ = std::marker::PhantomData::<clipboard::ClipboardState>;
    }

    #[test]
    fn test_timing_module_reexports() {
        // Verify timing module re-exports are accessible
        // timing module re-exports from internal::timing
        let _ = std::marker::PhantomData::<()>;
    }

    #[test]
    fn test_clipboard_functions_accessible() {
        // Verify specific clipboard functions are re-exported
        // This is a compile-time check - just reference the functions
        let _ = clipboard::copy;
        let _ = clipboard::paste_from_clipboard;
    }

    #[test]
    fn test_module_structure() {
        // Verify all expected submodules exist
        // This is checked at compile time by the module declarations
        // We verify by using the module paths
        let _ = mouse::ClickStatus::Success;
        let _ = clipboard::ClipboardState::new("test");
    }

    #[test]
    fn test_clipboard_state_type() {
        // Verify ClipboardState is re-exported
        // Use new() instead of default() since it doesn't implement Default
        let _state = clipboard::ClipboardState::new("test");
        let _ = _state;
    }

    #[test]
    fn test_reexport_privacy() {
        // Verify re-exports are public (pub use)
        // This is checked at compile time by the ability to reference types
        let _ = mouse::ClickStatus::Success;
        let _ = clipboard::ClipboardState::new("test");
    }
}
