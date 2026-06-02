//! Project configuration — replaces `CARGO_MANIFEST_DIR` with configurable paths.
//!
//! The host project calls [`init()`] once at startup with a [`ProjectConfig`]
//! describing its filesystem layout. All bacon-pipeline modules then read paths
//! from the global config instead of compile-time `env!` macros.

use std::path::PathBuf;
use std::sync::OnceLock;

static PROJECT_CONFIG: OnceLock<ProjectConfig> = OnceLock::new();

/// Initialize the global project configuration.
///
/// Must be called once before any bacon-pipeline function that reads paths.
/// Panics if called more than once.
pub fn init(config: ProjectConfig) {
    PROJECT_CONFIG
        .set(config)
        .expect("ProjectConfig already initialized — call bacon_pipeline::init() only once");
}

/// Access the global project configuration.
///
/// Panics if [`init()`] has not been called yet.
#[must_use]
pub fn project_config() -> &'static ProjectConfig {
    PROJECT_CONFIG
        .get()
        .expect("ProjectConfig not initialized — call bacon_pipeline::init() first")
}

/// All filesystem paths the bacon pipeline needs.
///
/// Use [`ProjectConfig::with_defaults()`] to create one with conventional layout.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Root of the host project (replaces `CARGO_MANIFEST_DIR`).
    pub project_root: PathBuf,
    /// Spec packages directory (default: `project_root/docs/specs`).
    pub specs_dir: PathBuf,
    /// Bacon config directory (default: `project_root/.bacon`).
    pub bacon_dir: PathBuf,
    /// Role prompts directory (default: `project_root/.bacon/roles`).
    pub roles_dir: PathBuf,
    /// `.env` file path (default: `project_root/.env`).
    pub env_file: PathBuf,
    /// Configurable validation commands.
    pub validation: ValidationCommands,
}

/// Cross-platform validation commands.
///
/// Defaults use `cargo check` / `cargo test` which work on all platforms.
/// Set `spec_lint` to empty to use the built-in Rust spec linter.
#[derive(Debug, Clone)]
pub struct ValidationCommands {    /// Quick validation gate — runs `check-fast.ps1` (rustfmt, cargo check, clippy).
    /// Override by setting this field to `["cargo", "check", "--lib", "--bins"]`
    /// or any other command for environments without pwsh.
    pub check_fast: Vec<String>,
    /// Full test suite (default: `["cargo", "test"]`).
    pub check_full: Vec<String>,
    /// Spec linter command (default: empty = use built-in Rust linter).
    pub spec_lint: Vec<String>,
}

impl Default for ValidationCommands {
    fn default() -> Self {
        Self {
            // check-fast.ps1 runs fmt + cargo check + clippy on changed paths;
            // requires pwsh (PowerShell Core). Falls back gracefully if not found.
            check_fast: vec![
                "pwsh".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-File".into(),
                "check-fast.ps1".into(),
            ],
            check_full: vec!["cargo".into(), "test".into()],
            spec_lint: Vec::new(),
        }
    }
}

impl ProjectConfig {
    /// Create a `ProjectConfig` with conventional default paths.
    #[must_use]
    pub fn with_defaults(root: PathBuf) -> Self {
        let bacon_dir = root.join(".bacon");
        Self {
            specs_dir: root.join("docs/specs"),
            roles_dir: bacon_dir.join("roles"),
            env_file: root.join(".env"),
            validation: ValidationCommands::default(),
            bacon_dir,
            project_root: root,
        }
    }

    /// Shortcut to get the project root from the global config.
    #[must_use]
    pub fn project_root() -> PathBuf {
        project_config().project_root.clone()
    }

    /// Shortcut: path to `.bacon/bacon.toml`.
    #[must_use]
    pub fn bacon_toml() -> PathBuf {
        project_config().bacon_dir.join("bacon.toml")
    }
}

/// Internal helper — equivalent to the old `manifest_dir()`.
#[must_use]
pub(crate) fn manifest_dir() -> PathBuf {
    project_config().project_root.clone()
}

/// Resolve bacon pipeline config path, allowing integration tests to isolate routing.
#[must_use]
pub(crate) fn bacon_config_path() -> PathBuf {
    std::env::var_os("BACON_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join(".bacon/bacon.toml"))
}
