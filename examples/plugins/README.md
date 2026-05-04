# Plugin Development Guide

This guide explains how to create plugins for auto-rust to extend its functionality.

## Overview

Plugins are WASM modules that can:
- Add custom DSL actions
- Hook into task lifecycle events
- Provide custom validators
- Add custom metrics/loggers

## Plugin Structure

A plugin consists of:
1. **Manifest file** (`*.toml`, `*.yaml`, or `*.json`) - Plugin metadata
2. **WASM binary** (`*.wasm`) - The compiled plugin code

## Quick Start

### 1. Create Plugin Manifest

```toml
name = "my-plugin"
version = "0.1.0"
api_version = "1.0.0"
author = "Your Name"
description = "My custom plugin"
entry_point = "my-plugin.wasm"

dependencies = []

capabilities = ["custom_actions", "task_hooks"]

[config]
debug = false
timeout_ms = 5000
```

### 2. Plugin Implementation (Rust)

```rust
use auto::plugin::{Plugin, PluginContext, PluginHook, HookResult};
use async_trait::async_trait;

pub struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn supports_hook(&self, hook: PluginHook) -> bool {
        matches!(hook, PluginHook::BeforeTask | PluginHook::AfterTask)
    }

    async fn execute_hook(
        &self,
        hook: PluginHook,
        context: &PluginContext,
        data: &serde_json::Value,
    ) -> anyhow::Result<HookResult> {
        match hook {
            PluginHook::BeforeTask => {
                log::info!("Task {} starting...", context.task_name);
                Ok(HookResult::Continue)
            }
            PluginHook::AfterTask => {
                log::info!("Task {} completed!", context.task_name);
                Ok(HookResult::Continue)
            }
            _ => Ok(HookResult::Continue)
        }
    }

    fn custom_actions(&self) -> Vec<String> {
        vec!["my_custom_action".to_string()]
    }

    async fn execute_custom_action(
        &self,
        action_name: &str,
        context: &PluginContext,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        if action_name == "my_custom_action" {
            // Implement your custom action
            log::info!("Executing custom action with params: {:?}", params);
            
            return Ok(serde_json::json!({
                "status": "success",
                "message": "Custom action executed!"
            }));
        }
        
        Err(anyhow::anyhow!("Unknown action: {}", action_name))
    }
}
```

### 3. Build for WASM

```bash
# Add wasm32 target
rustup target add wasm32-wasi

# Build
cargo build --target wasm32-wasi --release
```

### 4. Install Plugin

```bash
# Create plugin directory
mkdir -p ./plugins/my-plugin

# Copy files
cp target/wasm32-wasi/release/my-plugin.wasm ./plugins/my-plugin/
cp my-plugin.toml ./plugins/my-plugin/
```

### 5. Use in DSL

```yaml
name: task-with-custom-action
description: "Use plugin's custom action"

actions:
  - my_custom_action:
      param1: "value1"
      param2: 42
```

## Manifest Reference

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique plugin identifier |
| `version` | string | Semver version (e.g., "1.0.0") |
| `api_version` | string | Target plugin API version |
| `entry_point` | string | Path to WASM file |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `author` | string | Plugin author name |
| `description` | string | Plugin description |
| `dependencies` | array | Other plugins this depends on |
| `capabilities` | array | What this plugin can do |
| `config` | table | Plugin-specific configuration |

### Capabilities

- `custom_actions` - Can add new DSL actions
- `task_hooks` - Can hook into task lifecycle
- `validators` - Can provide custom validation
- `page_manipulation` - Can modify page content
- `external_api` - Can access external APIs
- `metrics` - Can log custom metrics

## Hook Points

Plugins can hook into these execution points:

| Hook | When Called |
|------|-------------|
| `before_task` | Before task execution starts |
| `after_task` | After task execution completes |
| `before_action` | Before each action executes |
| `after_action` | After each action executes |
| `on_error` | When an error occurs |
| `validate` | For custom validation |
| `custom_action` | For custom action execution |

## Hook Results

Hooks return one of:

- `Continue` - Proceed with normal execution
- `Skip` - Skip the current operation
- `Replace(data)` - Replace with custom data
- `Abort(reason)` - Stop execution with error

## Example Plugins

### Hello World Plugin

```rust
use auto::plugin::{Plugin, PluginContext, PluginHook, HookResult};

pub struct HelloPlugin;

#[async_trait::async_trait]
impl Plugin for HelloPlugin {
    fn name(&self) -> &str { "hello" }
    fn version(&self) -> &str { "1.0.0" }
    
    fn supports_hook(&self, hook: PluginHook) -> bool {
        matches!(hook, PluginHook::BeforeTask)
    }
    
    async fn execute_hook(
        &self,
        _hook: PluginHook,
        context: &PluginContext,
        _data: &serde_json::Value,
    ) -> anyhow::Result<HookResult> {
        log::info!("Hello from plugin! Task: {}", context.task_name);
        Ok(HookResult::Continue)
    }
}
```

### Validation Plugin

```rust
use auto::plugin::{Plugin, PluginContext, PluginHook, HookResult};

pub struct ValidationPlugin;

#[async_trait::async_trait]
impl Plugin for Plugin for ValidationPlugin {
    fn name(&self) -> &str { "validator" }
    fn version(&self) -> &str { "1.0.0" }
    
    fn supports_hook(&self, hook: PluginHook) -> bool {
        matches!(hook, PluginHook::Validate)
    }
    
    async fn execute_hook(
        &self,
        _hook: PluginHook,
        context: &PluginContext,
        data: &serde_json::Value,
    ) -> anyhow::Result<HookResult> {
        // Custom validation logic
        if let Some(url) = context.current_url.as_ref() {
            if !url.starts_with("https://") {
                return Ok(HookResult::Abort(
                    "Insecure URL detected".to_string()
                ));
            }
        }
        Ok(HookResult::Continue)
    }
}
```

## Configuration

Load plugins from custom directory:

```rust
use auto::plugin::{PluginLoader, PluginLoaderConfig, PluginRegistry};

let config = PluginLoaderConfig::with_dir("/path/to/plugins");
let loader = PluginLoader::new(config);
let mut registry = PluginRegistry::new();

loader.load_all(&mut registry).await?;
registry.initialize_all().await?;
```

## Best Practices

1. **Use descriptive names** - Prefix with your org/name
2. **Version properly** - Follow semver
3. **Handle errors gracefully** - Don't panic
4. **Log appropriately** - Use `log::info!`, `log::debug!`
5. **Minimize dependencies** - Keep WASM size small
6. **Document capabilities** - List all hooks and actions

## Troubleshooting

### Plugin not loading
- Check manifest syntax (valid TOML/YAML/JSON)
- Verify WASM file exists at entry_point path
- Check api_version compatibility

### Hook not called
- Verify `supports_hook()` returns true
- Check registry is initialized

### Custom action not found
- Verify `custom_actions()` returns correct names
- Check action name matches exactly
