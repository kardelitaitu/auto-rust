//! Action handler implementations for the DSL executor.
//!
//! These modules contain the per-action handler functions extracted from
//! `executor.rs`. Each module corresponds to a domain of action types:
//!
//! - `browser`: Navigation, click, type, hover, select, scroll, right-click, double-click, clear
//! - `wait`: Duration wait and element wait-for
//! - `inspection`: Element text extraction
//! - `media`: Screenshot capture
//!
//! Handlers are `pub(super)` methods on `DslExecutor` and are dispatched from
//! `executor.rs` via `execute_action()`.

pub mod browser;
pub mod inspection;
pub mod media;
pub mod wait;
