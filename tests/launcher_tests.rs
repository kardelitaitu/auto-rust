//! Integration tests for the launcher script.
//! Tests path resolution and launcher behavior.

use std::env;
use std::path::Path;

/// Test that the launcher script exists at the expected path
#[test]
fn launcher_script_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher_path = root.join("config").join("auto.cmd");

    assert!(
        launcher_path.exists(),
        "Launcher script not found at {:?}",
        launcher_path
    );
}

/// Test that the launcher script contains expected commands
#[test]
fn launcher_script_has_expected_content() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher_path = root.join("config").join("auto.cmd");

    let content = std::fs::read_to_string(&launcher_path).expect("Failed to read launcher script");

    // Check for key components
    assert!(
        content.contains("cargo run"),
        "Launcher should invoke cargo run"
    );
    assert!(
        content.contains("CARGO_MANIFEST_DIR") || content.contains("%~dp0"),
        "Launcher should handle path resolution"
    );
    assert!(
        content.contains("%*"),
        "Launcher should pass through arguments"
    );
}

/// Test that Cargo.toml exists at project root (required by launcher)
#[test]
fn cargo_toml_exists_at_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = root.join("Cargo.toml");

    assert!(
        cargo_toml.exists(),
        "Cargo.toml not found at project root {:?}",
        root
    );
}

/// Test that the binary name is 'auto' in Cargo.toml
#[test]
fn binary_name_is_auto() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

    // Check for [[bin]] section with name = "auto"
    assert!(
        content.contains("name = \"auto\""),
        "Cargo.toml should define binary name as 'auto'"
    );
}

/// Test that setup-windows.bat exists
#[test]
fn setup_script_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let setup_path = root.join("setup-windows.bat");

    assert!(
        setup_path.exists(),
        "setup-windows.bat not found at project root"
    );
}

/// Test that setup script references the launcher
#[test]
fn setup_script_references_launcher() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let setup_path = root.join("setup-windows.bat");

    if setup_path.exists() {
        let content = std::fs::read_to_string(&setup_path).expect("Failed to read setup script");

        // Should reference config\auto or auto.cmd
        assert!(
            content.contains("config\\auto") || content.contains("auto.cmd"),
            "Setup script should reference the launcher"
        );
    }
}
