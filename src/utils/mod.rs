#[cfg(feature = "accessibility-locator")]
pub mod accessibility_locator;
pub mod blockmedia;
pub mod clipboard;
pub mod geometry;
pub mod keyboard;
pub mod math;
pub mod mouse;
pub(crate) mod native_input;
pub mod navigation;
pub mod page_size;
pub mod profile;
pub mod scroll;
pub mod text;
pub mod timing;
pub mod twitter;
pub mod zoom;

// Internal implementation module; tasks should import `crate::prelude::*` instead.
#[cfg(feature = "accessibility-locator")]
#[allow(unused_imports)]
pub use accessibility_locator::*;
#[allow(unused_imports)]
pub use blockmedia::*;
#[allow(unused_imports)]
pub use clipboard::*;
#[allow(unused_imports)]
pub use geometry::*;
#[allow(unused_imports)]
pub use keyboard::*;
#[allow(unused_imports)]
pub use math::*;
#[allow(unused_imports)]
pub use mouse::*;
#[allow(unused_imports)]
pub use navigation::*;
#[allow(unused_imports)]
pub use page_size::*;
#[allow(unused_imports)]
pub use profile::*;
#[allow(unused_imports)]
pub use scroll::*;
#[allow(unused_imports)]
pub use text::*;
#[allow(unused_imports)]
pub use timing::*;
#[allow(unused_imports)]
pub use zoom::*;

#[cfg(test)]
mod tests {
    /// Smoke test to verify utils module compiles.
    #[test]
    fn test_utils_module_compiles() {
        // Module structure verification - just needs to compile
        assert!(true);
    }
}
