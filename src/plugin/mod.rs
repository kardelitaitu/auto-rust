//! Plugin System for auto-rust
//!
//! Provides extensibility through WASM-based plugins that can:
//! - Add custom DSL actions
//! - Hook into task lifecycle events
//! - Provide custom validators
//! - Add custom loggers/metrics

pub mod loader;
pub mod manifest;
pub mod plugin;
pub mod registry;

pub use loader::PluginLoader;
pub use manifest::{PluginManifest, PluginCapability};
pub use plugin::{Plugin, PluginContext, PluginHook};
pub use registry::{PluginRegistry, RegistryError};

/// Version of the plugin API
pub const PLUGIN_API_VERSION: &str = "1.0.0";

/// Default directory for plugins
pub const DEFAULT_PLUGIN_DIR: &str = "./plugins";

/// Plugin file extension
pub const PLUGIN_EXTENSION: &str = "wasm";
