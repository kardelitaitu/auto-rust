//! Internal implementation helpers.
//!
//! Framework modules use this instead of depending on `utils` directly.

pub mod profile {
    pub use crate::utils::profile::*;
}

pub mod mouse {
    pub use crate::utils::mouse::*;
}

pub mod geometry {
    pub use crate::utils::geometry::*;
}

pub mod keyboard {
    pub use crate::utils::keyboard::*;
}

pub mod navigation {
    pub use crate::utils::navigation::*;
}

pub mod scroll {
    pub use crate::utils::scroll::*;
}

pub mod timing {
    pub use crate::utils::timing::*;
}

pub mod clipboard {
    pub use crate::utils::clipboard::*;
}

pub mod text {
    pub use crate::utils::text::*;
}

pub mod blockmedia {
    pub use crate::utils::blockmedia::*;
}

pub mod page_size {
    pub use crate::utils::page_size::*;
}
