#[cfg(feature = "accessibility-locator")]
pub mod accessibility_locator;
pub mod blockmedia;
pub mod clipboard;
pub mod dom;
pub mod geometry;
pub mod keyboard;
pub mod math;
pub mod mouse;
pub(crate) mod native_input;
pub mod navigation;
pub mod page_size;
pub mod payload;
pub mod profile;
pub mod retry;
pub mod scroll;
pub mod text;
pub mod timing;
pub mod twitter;
pub mod url;
pub mod zoom;

// Re-exports: keep only modules whose items are accessed via `crate::utils::*`.
// Items accessed through `crate::prelude::*` or direct submodule paths don't need glob re-exports here.
// Removing a glob re-export does NOT remove access — use the submodule path instead.
//
// To find which modules actually need glob re-exports, temporarily remove
// #[allow(unused_imports)] and run `cargo check` — any new warnings indicate
// modules that are NOT accessed through `crate::utils::*` and can drop the glob.
#[cfg(feature = "accessibility-locator")]
pub use accessibility_locator::*;
pub use blockmedia::*;
pub use clipboard::*;
pub use dom::*;
pub use geometry::*;
pub use keyboard::*;
pub use math::*;
pub use mouse::*;
pub use navigation::*;
pub use page_size::*;
pub use payload::*;
pub use profile::*;
pub use scroll::*;
pub use text::{normalize_browser_token, *};
pub use timing::*;
pub use url::*;
pub use zoom::*;

#[cfg(test)]
mod tests {
    /// Smoke test to verify utils module compiles.
    #[test]
    fn test_utils_module_compiles() {
        // Module structure verification - just needs to compile
        // Module tests placeholder
    }
}
