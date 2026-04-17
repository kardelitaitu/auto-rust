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
    pub use crate::internal::clipboard::{
        clear_clipboard, copy, cut, get_clipboard, paste_from_clipboard, set_clipboard,
        ClipboardState,
    };
}

pub mod timing {
    pub use crate::internal::timing::*;
}
