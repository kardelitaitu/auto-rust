"""Wire DurationMs into discovery_retry_delay_ms and feed_scan_duration_ms in config/mod.rs."""

import re

path = r"C:\My Script\auto-rust\src\config\mod.rs"

with open(path, encoding="utf-8") as f:
    content = f.read()

# 1. Add DurationMs import (after the last existing use line that starts with "use crate::")
# Find a good insertion point - after "use crate::session::DurationMs;" since it may already exist,
# or near other use statements
if "use crate::session::DurationMs;" not in content:
    # Find the last use crate statement
    last_use_match = list(re.finditer(r'^use crate::[^;]+;\n', content, re.MULTILINE))
    if last_use_match:
        pos = last_use_match[-1].end()
        content = content[:pos] + "use crate::session::DurationMs;\n" + content[pos:]
    else:
        # Fallback: add after the module-level use statements
        # Find the first non-empty line after imports - look for a line starting with pub fn or pub struct
        content = "use crate::session::DurationMs;\n" + content

# 2. Field type changes (struct definitions only, not TOML strings)
# BrowserConfig.discovery_retry_delay_ms
content = content.replace(
    "    pub discovery_retry_delay_ms: u64,\n    /// Circuit breaker configuration",
    "    pub discovery_retry_delay_ms: DurationMs,\n    /// Circuit breaker configuration"
)
# TwitterActivityConfig.feed_scan_duration_ms
content = content.replace(
    "    pub feed_scan_duration_ms: u64,\n    /// Number of scroll actions",
    "    pub feed_scan_duration_ms: DurationMs,\n    /// Number of scroll actions"
)

# 3. Default function
content = content.replace(
    "fn default_feed_scan_duration() -> u64 {\n    60000\n}",
    "fn default_feed_scan_duration() -> DurationMs {\n    DurationMs::new_const(60000)\n}"
)

# 4. Discovery retry delay default in BrowserConfig::default()
content = content.replace(
    "            discovery_retry_delay_ms: 500,",
    "            discovery_retry_delay_ms: DurationMs::new_const(500),"
)

# 5. Test assertion: assert_eq!(config.discovery_retry_delay_ms, 500)
# Only match in test assertion context, not in TOML strings
content = content.replace(
    "assert_eq!(config.discovery_retry_delay_ms, 500);\n        assert!(config.profiles.is_empty())",
    "assert_eq!(config.discovery_retry_delay_ms.get(), 500);\n        assert!(config.profiles.is_empty())"
)

# 6. Test assertion: assert_eq!(config.feed_scan_duration_ms, 60000)
content = content.replace(
    "assert_eq!(config.feed_scan_duration_ms, 60000);\n        assert_eq!(config.feed_scroll_count, 10);",
    "assert_eq!(config.feed_scan_duration_ms.get(), 60000);\n        assert_eq!(config.feed_scroll_count, 10);"
)

# 7. Clone comparison: assert_eq!(cloned.discovery_retry_delay_ms, config.discovery_retry_delay_ms)
# These compare DurationMs to DurationMs, no change needed (PartialEq is derived)

# 8. Clone comparison: assert_eq!(cloned.feed_scan_duration_ms, config.feed_scan_duration_ms)
# Same, no change needed

# 9. Validation: if config.discovery_retry_delay_ms == 0 { warn!(...) }
# This becomes unreachable since DurationMs can't be 0. Replace with a no-op or remove.
content = content.replace(
    "        if config.discovery_retry_delay_ms == 0 {\n            warn!(\"discovery_retry_delay_ms is 0. Consider adding a delay between retries.\");\n        }\n        if config.discovery_retry_delay_ms > 60_000 {",
    "        if config.discovery_retry_delay_ms.get() > 60_000 {"
)

# 10. Feed scan duration validation
content = content.replace(
    "if config.feed_scan_duration_ms < 10_000 {\n            warn!(\n                \"twitter_activity.feed_scan_duration_ms ({}) is very low (<10s). \\\n                 Feed scan may not capture enough content.\",\n                config.feed_scan_duration_ms\n            );\n        }\n        if config.feed_scan_duration_ms > 1_800_000 {\n            return Err(OrchestratorError::Config(ConfigError::InvalidValue {\n                field: \"twitter_activity.feed_scan_duration_ms\".to_string(),\n                value: config.feed_scan_duration_ms.to_string(),",
    'if config.feed_scan_duration_ms.get() < 10_000 {\n            warn!(\n                "twitter_activity.feed_scan_duration_ms ({}) is very low (<10s). \\\n                 Feed scan may not capture enough content.",\n                config.feed_scan_duration_ms.get()\n            );\n        }\n        if config.feed_scan_duration_ms.get() > 1_800_000 {\n            return Err(OrchestratorError::Config(ConfigError::InvalidValue {\n                field: "twitter_activity.feed_scan_duration_ms".to_string(),\n                value: config.feed_scan_duration_ms.get().to_string(),'
)

# 11. Warning for discovery_retry_delay_ms > 60_000
content = content.replace(
    "if config.discovery_retry_delay_ms > 60_000 {\n            warn!(\n                \"discovery_retry_delay_ms ({}) is very high. This may cause long startup delays.\",\n                config.discovery_retry_delay_ms\n            );\n        }",
    'if config.discovery_retry_delay_ms.get() > 60_000 {\n            warn!(\n                "discovery_retry_delay_ms ({}) is very high. This may cause long startup delays.",\n                config.discovery_retry_delay_ms.get()\n            );\n        }'
)

# 12. Test constructor in env override test that sets discovery_retry_delay_ms: 5000
content = content.replace(
    "            discovery_retry_delay_ms: 5000,",
    "            discovery_retry_delay_ms: DurationMs::new_const(5000),"
)

with open(path, "w", encoding="utf-8") as f:
    f.write(content)

print("config/mod.rs updated successfully")

# Count remaining u64 references to verify
remaining_u64 = content.count("discovery_retry_delay_ms: u64") + content.count("feed_scan_duration_ms: u64")
remaining_toml = content.count("discovery_retry_delay_ms =") + content.count("feed_scan_duration_ms =")
print(f"Remaining struct field 'u64' refs (should be 0): {remaining_u64}")
print(f"TOML string occurrences (should be unchanged, these are fine with serde transparent): {remaining_toml}")
