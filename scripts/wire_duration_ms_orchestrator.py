"""Wire DurationMs into 6 config fields in config/mod.rs.

Replaces:
- Field types: u64 -> DurationMs for connection_timeout_ms, half_open_time_ms,
  task_timeout_ms, group_timeout_ms, worker_wait_timeout_ms, retry_delay_ms
- Default constructors: DurationMs::new_const(...)
- env var override: parse u64 then DurationMs::new()
- Test constructors: DurationMs::new_const(...)
- Test assertions: .get() for u64 comparison

TOML inline test strings are left unchanged (serde transparent handles u64<->DurationMs).
Clone comparison assertions like assert_eq!(a, b) where both are DurationMs are left unchanged.
"""

import re

with open("src/config/mod.rs", "r", encoding="utf-8") as f:
    content = f.read()

# === Change 1: Field type declarations ===
replacements = [
    # BrowserConfig fields
    ("    pub connection_timeout_ms: u64,", "    pub connection_timeout_ms: DurationMs,"),
    # CircuitBreakerConfig field
    ("    pub half_open_time_ms: u64,", "    pub half_open_time_ms: DurationMs,"),
    # OrchestratorConfig fields
    ("    pub task_timeout_ms: u64,", "    pub task_timeout_ms: DurationMs,"),
    ("    pub group_timeout_ms: u64,", "    pub group_timeout_ms: DurationMs,"),
    ("    pub worker_wait_timeout_ms: u64,", "    pub worker_wait_timeout_ms: DurationMs,"),
    ("    pub retry_delay_ms: u64,", "    pub retry_delay_ms: DurationMs,"),
]

for old, new in replacements:
    assert old in content, f"Field declaration not found: {old}"
    content = content.replace(old, new)

# === Change 2: BrowserConfig::default() ===
content = content.replace(
    "            connection_timeout_ms: 30000,",
    "            connection_timeout_ms: DurationMs::new_const(30000),"
)

# === Change 3: OrchestratorConfig::default() ===
content = content.replace(
    "            task_timeout_ms: 60000,\n            group_timeout_ms: 300000,\n            worker_wait_timeout_ms: 10000,\n            task_stagger_delay_ms: 500,\n            max_retries: 3,\n            retry_delay_ms: 2000,",
    "            task_timeout_ms: DurationMs::new_const(60000),\n            group_timeout_ms: DurationMs::new_const(300000),\n            worker_wait_timeout_ms: DurationMs::new_const(10000),\n            task_stagger_delay_ms: 500,\n            max_retries: 3,\n            retry_delay_ms: DurationMs::new_const(2000),"
)

# === Change 4: CircuitBreakerConfig::default() ===
content = content.replace(
    "            half_open_time_ms: 30000,",
    "            half_open_time_ms: DurationMs::new_const(30000),"
)

# === Change 5: load_code_config() BrowserConfig ===
content = content.replace(
    "            connection_timeout_ms: 10000,",
    "            connection_timeout_ms: DurationMs::new_const(10000),"
)

# load_code_config OrchestratorConfig
content = content.replace(
    "            task_timeout_ms: 600_000,\n            group_timeout_ms: 600_000,\n            worker_wait_timeout_ms: 10000,\n            task_stagger_delay_ms: 2000,\n            max_retries: 2,\n            retry_delay_ms: 500,",
    "            task_timeout_ms: DurationMs::new_const(600_000),\n            group_timeout_ms: DurationMs::new_const(600_000),\n            worker_wait_timeout_ms: DurationMs::new_const(10000),\n            task_stagger_delay_ms: 2000,\n            max_retries: 2,\n            retry_delay_ms: DurationMs::new_const(500),"
)

# load_code_config CircuitBreakerConfig
content = content.replace(
    "                half_open_time_ms: 30000,",
    "                half_open_time_ms: DurationMs::new_const(30000),"
)

# === Change 6: Env var override for TASK_TIMEOUT_MS ===
content = content.replace(
    "        config.orchestrator.task_timeout_ms = timeout\n            .parse()\n            .unwrap_or(config.orchestrator.task_timeout_ms);",
    "        config.orchestrator.task_timeout_ms = timeout\n            .parse::<u64>()\n            .ok()\n            .and_then(DurationMs::new)\n            .unwrap_or(config.orchestrator.task_timeout_ms);"
)

# === Change 7: Test assertions comparing u64 literal ===
field_comparisons = [
    ("assert_eq!(config.task_timeout_ms, 60000)", "assert_eq!(config.task_timeout_ms.get(), 60000)"),
    ("assert_eq!(config.half_open_time_ms, 30000)", "assert_eq!(config.half_open_time_ms.get(), 30000)"),
    ("assert_eq!(config.half_open_time_ms, 60000)", "assert_eq!(config.half_open_time_ms.get(), 60000)"),
    ("assert_eq!(config.task_timeout_ms, 120000)", "assert_eq!(config.task_timeout_ms.get(), 120000)"),
    # Test in env override test file
    ("config.orchestrator.task_timeout_ms, 120000,", "config.orchestrator.task_timeout_ms.get(), 120000,"),
    ("config.orchestrator.task_timeout_ms, 99999,", "config.orchestrator.task_timeout_ms.get(), 99999,"),
]

for old, new in field_comparisons:
    assert old in content, f"Comparison not found: {old}"
    content = content.replace(old, new)

# === Change 8: Test struct constructions ===
content = content.replace(
    "            half_open_time_ms: 60000,",
    "            half_open_time_ms: DurationMs::new_const(60000),"
)

# Custom OrchestratorConfig in test
content = content.replace(
    "            task_timeout_ms: 120000,\n            group_timeout_ms: 600000,\n            worker_wait_timeout_ms: 20000,\n            task_stagger_delay_ms: 1000,\n            max_retries: 5,\n            retry_delay_ms: 5000,",
    "            task_timeout_ms: DurationMs::new_const(120000),\n            group_timeout_ms: DurationMs::new_const(600000),\n            worker_wait_timeout_ms: DurationMs::new_const(20000),\n            task_stagger_delay_ms: 1000,\n            max_retries: 5,\n            retry_delay_ms: DurationMs::new_const(5000),"
)

with open("src/config/mod.rs", "w", encoding="utf-8") as f:
    f.write(content)

print("OK config/mod.rs updated successfully")

# Verify key changes
count_checks = [
    ("connection_timeout_ms: DurationMs", "Field type"),
    ("half_open_time_ms: DurationMs", "Field type"),
    ("task_timeout_ms: DurationMs", "Field type"),
    ("group_timeout_ms: DurationMs", "Field type"),
    ("worker_wait_timeout_ms: DurationMs", "Field type"),
    ("retry_delay_ms: DurationMs", "Field type"),
]
for pattern, desc in count_checks:
    count = content.count(pattern)
    if count > 0:
        print(f"  OK {desc}: {count} occurrences")
    else:
        print(f"  FAIL {desc}: NOT FOUND")
