"""Wire NativeInteractionConfig.stability_wait_ms and resolve_timeout_ms to DurationMs."""

import re

# ====== 1. config/mod.rs ======
with open("src/config/mod.rs", "r", encoding="utf-8") as f:
    content = f.read()

changes = 0

# stability_wait_ms field type
old = "    pub stability_wait_ms: u64,"
new = "    pub stability_wait_ms: DurationMs,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# resolve_timeout_ms field type
old = "    pub resolve_timeout_ms: u64,"
new = "    pub resolve_timeout_ms: DurationMs,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Default function for stability_wait_ms
old = "fn default_native_interaction_stability_wait_ms() -> u64 {\n    5_000\n}"
new = "fn default_native_interaction_stability_wait_ms() -> DurationMs {\n    DurationMs::new_const(5_000)\n}"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Default function for resolve_timeout_ms  
old = "fn default_native_interaction_resolve_timeout_ms() -> u64 {\n    2_000\n}"
new = "fn default_native_interaction_resolve_timeout_ms() -> DurationMs {\n    DurationMs::new_const(2_000)\n}"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test: assert_eq!(config.native_interaction.stability_wait_ms, 5000);
old = "assert_eq!(config.native_interaction.stability_wait_ms, 5000);"
new = "assert_eq!(config.native_interaction.stability_wait_ms.get(), 5000);"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test: assert_eq!(config.native_interaction.resolve_timeout_ms, 2000);
old = "assert_eq!(config.native_interaction.resolve_timeout_ms, 2000);"
new = "assert_eq!(config.native_interaction.resolve_timeout_ms.get(), 2000);"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test: assert_eq!(config.stability_wait_ms, 5000);
old = "assert_eq!(config.stability_wait_ms, 5000);"
new = "assert_eq!(config.stability_wait_ms.get(), 5000);"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test: assert_eq!(config.resolve_timeout_ms, 2000);
old = "assert_eq!(config.resolve_timeout_ms, 2000);"
new = "assert_eq!(config.resolve_timeout_ms.get(), 2000);"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test env override: assert_eq!(config.browser.native_interaction.stability_wait_ms, 3000,
old = "config.browser.native_interaction.stability_wait_ms, 3000,"
new = "config.browser.native_interaction.stability_wait_ms.get(), 3000,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test env override: assert_eq!(config.browser.native_interaction.stability_wait_ms, 9999,
old = "config.browser.native_interaction.stability_wait_ms, 9999,"
new = "config.browser.native_interaction.stability_wait_ms.get(), 9999,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test env override: assert_eq!(config.browser.native_interaction.resolve_timeout_ms, 1500,
old = "config.browser.native_interaction.resolve_timeout_ms, 1500,"
new = "config.browser.native_interaction.resolve_timeout_ms.get(), 1500,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Test env override: assert_eq!(config.browser.native_interaction.resolve_timeout_ms, 8888,
old = "config.browser.native_interaction.resolve_timeout_ms, 8888,"
new = "config.browser.native_interaction.resolve_timeout_ms.get(), 8888,"
if old in content:
    content = content.replace(old, new)
    changes += 1

# Env override for stability_wait_ms
old = """if let Ok(stability_wait_ms) = env::var(\"NATIVE_INTERACTION_STABILITY_WAIT_MS\") {
        config.browser.native_interaction.stability_wait_ms = stability_wait_ms
            .parse()
            .unwrap_or(config.browser.native_interaction.stability_wait_ms);
    }"""
new = """if let Ok(stability_wait_ms) = env::var(\"NATIVE_INTERACTION_STABILITY_WAIT_MS\") {
        config.browser.native_interaction.stability_wait_ms = stability_wait_ms
            .parse::<u64>()
            .ok()
            .and_then(DurationMs::new)
            .unwrap_or(config.browser.native_interaction.stability_wait_ms);
    }"""
if old in content:
    content = content.replace(old, new)
    changes += 1

# Env override for resolve_timeout_ms
old = """if let Ok(resolve_timeout_ms) = env::var(\"NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS\") {
        config.browser.native_interaction.resolve_timeout_ms = resolve_timeout_ms
            .parse()
            .unwrap_or(config.browser.native_interaction.resolve_timeout_ms);
    }"""
new = """if let Ok(resolve_timeout_ms) = env::var(\"NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS\") {
        config.browser.native_interaction.resolve_timeout_ms = resolve_timeout_ms
            .parse::<u64>()
            .ok()
            .and_then(DurationMs::new)
            .unwrap_or(config.browser.native_interaction.resolve_timeout_ms);
    }"""
if old in content:
    content = content.replace(old, new)
    changes += 1

with open("src/config/mod.rs", "w", encoding="utf-8") as f:
    f.write(content)

print(f"config/mod.rs: {changes} replacements made")

# ====== 2. mouse.rs ======
with open("src/utils/mouse.rs", "r", encoding="utf-8") as f:
    mouse_content = f.read()

mouse_changes = 0

# stability_wait_ms.clamp
old = "native_interaction.stability_wait_ms.clamp(1_000, 30_000)"
new = "native_interaction.stability_wait_ms.get().clamp(1_000, 30_000)"
if old in mouse_content:
    mouse_content = mouse_content.replace(old, new)
    mouse_changes += 1

# resolve_timeout_ms.clamp (occurs 2 times)
old = "native_interaction.resolve_timeout_ms.clamp(250, 30_000)"
new = "native_interaction.resolve_timeout_ms.get().clamp(250, 30_000)"
if old in mouse_content:
    mouse_content = mouse_content.replace(old, new)
    mouse_changes += 1

with open("src/utils/mouse.rs", "w", encoding="utf-8") as f:
    f.write(mouse_content)

print(f"mouse.rs: {mouse_changes} replacements made")

# ====== 3. Validation checks in config/mod.rs ======
# Re-read in case earlier changes shifted lines
with open("src/config/mod.rs", "r", encoding="utf-8") as f:
    content = f.read()

val_changes = 0

# validation: config.stability_wait_ms < 1_000 (in validate_browser_config or similar)
if "config.stability_wait_ms < 1_000" in content:
    content = content.replace("config.stability_wait_ms < 1_000", "config.stability_wait_ms.get() < 1_000")
    val_changes += 1

if "config.stability_wait_ms > 30_000" in content:
    content = content.replace("config.stability_wait_ms > 30_000", "config.stability_wait_ms.get() > 30_000")
    val_changes += 1

if "config.resolve_timeout_ms < 250" in content:  
    content = content.replace("config.resolve_timeout_ms < 250", "config.resolve_timeout_ms.get() < 250")
    val_changes += 1

if "config.resolve_timeout_ms > 30_000" in content:
    content = content.replace("config.resolve_timeout_ms > 30_000", "config.resolve_timeout_ms.get() > 30_000")
    val_changes += 1

if "config.stability_wait_ms == 0" in content:
    content = content.replace("config.stability_wait_ms == 0", "config.stability_wait_ms.get() == 0")
    val_changes += 1

if "config.resolve_timeout_ms == 0" in content:
    content = content.replace("config.resolve_timeout_ms == 0", "config.resolve_timeout_ms.get() == 0")
    val_changes += 1

with open("src/config/mod.rs", "w", encoding="utf-8") as f:
    f.write(content)

print(f"config/mod.rs validation: {val_changes} replacements made")
print(f"\nTotal: {changes + mouse_changes + val_changes} replacements")
