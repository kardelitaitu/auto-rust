//! Plugin Manifest
//!
//! Defines the manifest format for plugin metadata.

use serde::{Deserialize, Serialize};

/// Plugin capability types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Can add custom DSL actions
    CustomActions,
    /// Can hook into task lifecycle
    TaskHooks,
    /// Can provide custom validators
    Validators,
    /// Can modify page content
    PageManipulation,
    /// Can access external APIs
    ExternalApi,
    /// Can log custom metrics
    Metrics,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Plugin name
    pub name: String,
    /// Required version range (semver)
    pub version: Option<String>,
    /// Whether this is an optional dependency
    #[serde(default)]
    pub optional: bool,
}

/// Plugin manifest - metadata for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (must be unique)
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// API version this plugin targets
    pub api_version: String,
    /// Plugin author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Plugin description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Entry point WASM file
    pub entry_point: String,
    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// Plugin capabilities
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// Plugin-specific configuration schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl PluginManifest {
    /// Parse manifest from TOML string
    pub fn from_toml(toml_str: &str) -> anyhow::Result<Self> {
        let manifest: Self = toml::from_str(toml_str)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse manifest from YAML string
    pub fn from_yaml(yaml_str: &str) -> anyhow::Result<Self> {
        let manifest: Self = serde_yaml::from_str(yaml_str)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse manifest from JSON string
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let manifest: Self = serde_json::from_str(json_str)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check name
        if self.name.is_empty() {
            anyhow::bail!("Plugin name cannot be empty");
        }

        // Check version format (basic semver check)
        if !self.version.contains('.') {
            anyhow::bail!("Plugin version must follow semver (e.g., '1.0.0')");
        }

        // Check API version
        if self.api_version.is_empty() {
            anyhow::bail!("API version cannot be empty");
        }

        // Check entry point
        if self.entry_point.is_empty() {
            anyhow::bail!("Entry point cannot be empty");
        }

        // Check that entry point has correct extension
        if !self.entry_point.ends_with(".wasm") {
            anyhow::bail!("Entry point must be a .wasm file");
        }

        Ok(())
    }

    /// Check if manifest has a specific capability
    pub fn has_capability(&self, capability: PluginCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Check if manifest depends on a plugin
    pub fn depends_on(&self, plugin_name: &str) -> bool {
        self.dependencies.iter().any(|dep| dep.name == plugin_name)
    }

    /// Get manifest as TOML string
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string(self)?)
    }

    /// Get manifest as YAML string
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    /// Get manifest as JSON string
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Default manifest template
pub fn default_manifest_template(name: &str) -> String {
    format!(
        r##"name = "{name}"
version = "0.1.0"
api_version = "1.0.0"
author = "Your Name"
description = "A plugin for auto-rust"
entry_point = "{name}.wasm"

dependencies = []

capabilities = ["custom_actions", "task_hooks"]

[config]
# Plugin-specific configuration
debug = false
timeout_ms = 5000
"##,
        name = name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_from_toml() {
        let toml = r#"
name = "test-plugin"
version = "1.0.0"
api_version = "1.0.0"
author = "Test Author"
description = "A test plugin"
entry_point = "test.wasm"
capabilities = ["custom_actions", "task_hooks"]

[config]
debug = true
"#;

        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.api_version, "1.0.0");
        assert_eq!(manifest.author, Some("Test Author".to_string()));
        assert_eq!(manifest.description, Some("A test plugin".to_string()));
        assert_eq!(manifest.entry_point, "test.wasm");
        assert!(manifest.has_capability(PluginCapability::CustomActions));
        assert!(manifest.has_capability(PluginCapability::TaskHooks));
    }

    #[test]
    fn test_manifest_from_yaml() {
        let yaml = r#"
name: yaml-plugin
version: "2.0.0"
api_version: "1.0.0"
entry_point: yaml-plugin.wasm
capabilities:
  - validators
  - metrics
"#;

        let manifest = PluginManifest::from_yaml(yaml).unwrap();
        assert_eq!(manifest.name, "yaml-plugin");
        assert_eq!(manifest.version, "2.0.0");
        assert!(manifest.has_capability(PluginCapability::Validators));
        assert!(manifest.has_capability(PluginCapability::Metrics));
    }

    #[test]
    fn test_manifest_validation_empty_name() {
        let manifest = PluginManifest {
            name: "".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0.0".to_string(),
            author: None,
            description: None,
            entry_point: "test.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn test_manifest_validation_invalid_version() {
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "invalid".to_string(),
            api_version: "1.0.0".to_string(),
            author: None,
            description: None,
            entry_point: "test.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_manifest_validation_wrong_extension() {
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0.0".to_string(),
            author: None,
            description: None,
            entry_point: "test.js".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            config: None,
        };

        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wasm"));
    }

    #[test]
    fn test_manifest_valid() {
        let manifest = PluginManifest {
            name: "valid-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0.0".to_string(),
            author: Some("Author".to_string()),
            description: Some("Description".to_string()),
            entry_point: "plugin.wasm".to_string(),
            dependencies: vec![],
            capabilities: vec![PluginCapability::CustomActions],
            config: Some(serde_json::json!({"key": "value"})),
        };

        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_default_manifest_template() {
        let template = default_manifest_template("my-plugin");
        assert!(template.contains("name = \"my-plugin\""));
        assert!(template.contains("version = \"0.1.0\""));
        assert!(template.contains("entry_point = \"my-plugin.wasm\""));
    }
}
