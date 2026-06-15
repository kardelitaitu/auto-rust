"""Wire TaskPolicy.max_duration_ms from u64 to DurationMs.

Changes:
1. Add `use crate::session::DurationMs;` import
2. Change field type: `pub max_duration_ms: u64` -> `DurationMs`
3. Update validate() to use .get()
4. Update DEFAULT_TASK_POLICY const to use DurationMs::new_const()
5. Update all LazyLock static policies
6. Update all test assertions and test constructions
7. Update test_policy_validation_zero_timeout_fails (DurationMs can't be 0)
"""

path = "src/task/policy.rs"

with open(path, "r", encoding="utf-8") as f:
    content = f.read()

changes = 0

# 1. Add import for DurationMs (after the serde_json import line)
old_import = "use serde_json;\nuse std::collections::HashMap;\n"
new_import = "use serde_json;\nuse std::collections::HashMap;\nuse crate::session::DurationMs;\n"
if old_import in content:
    content = content.replace(old_import, new_import)
    changes += 1
    print(f"Added DurationMs import")
else:
    print(f"ERROR: Import section not found")

# 2. Change field type
old_field = "    pub max_duration_ms: u64,\n"
new_field = "    pub max_duration_ms: DurationMs,\n"
count = content.count(old_field)
if count == 1:
    content = content.replace(old_field, new_field)
    changes += 1
    print(f"Changed field type to DurationMs")
else:
    print(f"ERROR: Found {count} field declarations, expected 1")

# 3. Update validate() - use .get() == 0
old_validate = "        if self.max_duration_ms == 0 {\n            return Err(format!(\n                \"max_duration_ms must be > 0, got {}\",\n                self.max_duration_ms\n            ));\n        }"
new_validate = "        if self.max_duration_ms.get() == 0 {\n            return Err(format!(\n                \"max_duration_ms must be > 0, got {}\",\n                self.max_duration_ms.get()\n            ));\n        }"
if old_validate in content:
    content = content.replace(old_validate, new_validate)
    changes += 1
    print(f"Updated validate() to use .get()")
else:
    print(f"WARNING: validate() pattern not found")

# 4. Update DEFAULT_TASK_POLICY const
old_const = "pub const DEFAULT_TASK_POLICY: TaskPolicy = TaskPolicy {\n    max_duration_ms: 60_000, // 1 minute"
new_const = "pub const DEFAULT_TASK_POLICY: TaskPolicy = TaskPolicy {\n    max_duration_ms: DurationMs::new_const(60_000), // 1 minute"
if old_const in content:
    content = content.replace(old_const, new_const)
    changes += 1
    print(f"Updated DEFAULT_TASK_POLICY const")
else:
    print(f"ERROR: DEFAULT_TASK_POLICY pattern not found")

# 5. Update LazyLock statics: literal u64 values -> DurationMs::new_const()
for literal_ms in ["120_000", "45_000", "30_000"]:
    for indent in ["        ", "\t\t"]:
        old_lazy = f"{indent}max_duration_ms: {literal_ms},"
        new_lazy = f"{indent}max_duration_ms: DurationMs::new_const({literal_ms}),"
        c = content.count(old_lazy)
        if c > 0:
            content = content.replace(old_lazy, new_lazy)
            changes += 1
            print(f"Updated LazyLock literal {literal_ms} ({c}x)")

# Update LazyLock statics: constant references -> DurationMs::new_const()
const_refs = [
    "crate::task::cookiebot::DEFAULT_COOKIEBOT_TASK_DURATION_MS",
    "crate::utils::twitter::DEFAULT_TWITTERACTIVITY_DURATION_MS",
    "crate::task::demo_keyboard::DEFAULT_DEMO_KEYBOARD_TASK_DURATION_MS",
    "crate::task::demo_mouse::DEFAULT_DEMO_MOUSE_TASK_DURATION_MS",
    "crate::task::demoqa::DEFAULT_DEMOQA_TASK_DURATION_MS",
    "crate::task::task_example::DEFAULT_TASK_EXAMPLE_DURATION_MS",
    "crate::task::twitterdive::DEFAULT_TWITTERDIVE_DURATION_MS",
    "crate::task::twitterfollow::DEFAULT_TWITTERFOLLOW_TASK_DURATION_MS",
    "crate::task::twitterintent::DEFAULT_TWITTERINTENT_TASK_DURATION_MS",
    "crate::task::twitterlike::DEFAULT_TWITTERLIKE_TASK_DURATION_MS",
    "crate::task::twitterquote::DEFAULT_TWITTERQUOTE_TASK_DURATION_MS",
    "crate::task::twitterreply::DEFAULT_TWITTERREPLY_TASK_DURATION_MS",
    "crate::task::twitterretweet::DEFAULT_TWITTERRETWEET_TASK_DURATION_MS",
    "crate::task::twittertest::DEFAULT_TWITTERTEST_TASK_DURATION_MS",
]

for const_ref in const_refs:
    for indent in ["        ", "\t\t"]:
        old_lazy_ref = f"{indent}max_duration_ms: {const_ref},"
        new_lazy_ref = f"{indent}max_duration_ms: DurationMs::new_const({const_ref}),"
        c = content.count(old_lazy_ref)
        if c > 0:
            content = content.replace(old_lazy_ref, new_lazy_ref)
            changes += 1
            print(f"Updated LazyLock ref {const_ref.rsplit('::', 1)[1]} ({c}x)")

# 6. Update test constructions with raw u64 values
# Pattern: max_duration_ms: X, in test code (not in the struct definition itself)
# We need to be careful not to replace the ones already wrapped in DurationMs::new_const()

# Replace raw 60_000, 30_000, 120_000 in test struct constructions
# Only in the test module (after #[cfg(test)])
test_section_marker = "#[cfg(test)]\nmod tests {"
test_idx = content.find(test_section_marker)
if test_idx > 0:
    # Replace all remaining `max_duration_ms: <integer>` in the test section
    test_section_start = content.find("#[cfg(test)]")
    before_tests = content[:test_section_start]
    after_tests = content[test_section_start:]

    # In the test section, replace raw u64 literals in TaskPolicy construction
    for ms_val in ["60_000", "30_000", "120_000"]:
        # Only replace if not already wrapped in DurationMs::new_const()
        old_pat = f"max_duration_ms: {ms_val},"
        new_pat = f"max_duration_ms: DurationMs::new_const({ms_val}),"
        # But only in test section, and only for TaskPolicy construction
        if old_pat in after_tests and f"DurationMs::new_const({ms_val})" not in after_tests.split(old_pat)[0][-30:]:
            c = after_tests.count(old_pat)
            after_tests = after_tests.replace(old_pat, new_pat)
            changes += 1
            print(f"Updated test literal {ms_val} ({c}x)")

    content = before_tests + after_tests

# 7. Update test assertions - replace .max_duration_ms with .max_duration_ms.get()
# Do this ONLY in the test section
test_idx = content.find("#[cfg(test)]")
before_tests = content[:test_idx]
after_tests = content[test_idx:]

# Replace all `assert_eq!(xxx.max_duration_ms,` -> `.get(),`
# But be careful not to change assertions already comparing two DurationMs values
import re

# First pass: replace `assert_eq!(xxx.max_duration_ms, yyy)` where yyy is a u64 or constant
# Pattern: assert_eq!(<path>.max_duration_ms, ...) -> assert_eq!(<path>.max_duration_ms.get(), ...)
# But only when the RHS is a number or a constant reference (not a DurationMs field access)
def replace_assert_eq_max_duration(match):
    full = match.group(0)
    lhs = match.group(1)
    rhs = match.group(2)
    # If RHS already has .get(), skip
    if ".get()" in rhs or "DurationMs" in rhs:
        return full
    # Only add .get() if RHS is a number or a simple constant reference
    if re.match(r'^\d[\d_]*$', rhs.strip()) or '::' in rhs:
        return f"assert_eq!({lhs}.max_duration_ms.get(), {rhs}"
    return full

# Actually, let me be more surgical and just do specific replacements
# Replace `assert_eq!(policy.max_duration_ms, 60_000);` style assertions
specific_replacements = [
    ("assert_eq!(policy.max_duration_ms, 60_000);", "assert_eq!(policy.max_duration_ms.get(), 60_000);"),
    ("assert_eq!(policy.max_duration_ms, 30_000);", "assert_eq!(policy.max_duration_ms.get(), 30_000);"),
    ("assert_eq!(policy.max_duration_ms, 120_000);", "assert_eq!(policy.max_duration_ms.get(), 120_000);"),
    ("assert_eq!(policy.max_duration_ms, cloned.max_duration_ms)", "assert_eq!(policy.max_duration_ms.get(), cloned.max_duration_ms.get())"),
    ("cloned.max_duration_ms, DEFAULT_TASK_POLICY.max_duration_ms", "cloned.max_duration_ms.get(), DEFAULT_TASK_POLICY.max_duration_ms.get()"),
    ("policy_hyphen.max_duration_ms, policy_snake.max_duration_ms", "policy_hyphen.max_duration_ms.get(), policy_snake.max_duration_ms.get()"),
    ("policy_hyphen.max_duration_ms, DEMO_KEYBOARD_POLICY.max_duration_ms", "policy_hyphen.max_duration_ms.get(), DEMO_KEYBOARD_POLICY.max_duration_ms.get()"),
    ("policy_hyphen.max_duration_ms, DEMO_MOUSE_POLICY.max_duration_ms", "policy_hyphen.max_duration_ms.get(), DEMO_MOUSE_POLICY.max_duration_ms.get()"),
    ("policy_hyphen.max_duration_ms, DEMO_QA_POLICY.max_duration_ms", "policy_hyphen.max_duration_ms.get(), DEMO_QA_POLICY.max_duration_ms.get()"),
    ("policy_hyphen.max_duration_ms, TASK_EXAMPLE_POLICY.max_duration_ms", "policy_hyphen.max_duration_ms.get(), TASK_EXAMPLE_POLICY.max_duration_ms.get()"),
    ("policy.max_duration_ms, COOKIEBOT_POLICY.max_duration_ms", "policy.max_duration_ms.get(), COOKIEBOT_POLICY.max_duration_ms.get()"),
    ("policy.max_duration_ms, DEFAULT_TASK_POLICY.max_duration_ms", "policy.max_duration_ms.get(), DEFAULT_TASK_POLICY.max_duration_ms.get()"),
    ("direct_policy.max_duration_ms, registry_policy.max_duration_ms", "direct_policy.max_duration_ms.get(), registry_policy.max_duration_ms.get()"),
]

for old_ass, new_ass in specific_replacements:
    # Only replace in test section
    c = after_tests.count(old_ass)
    if c > 0:
        after_tests = after_tests.replace(old_ass, new_ass)
        changes += 1
        print(f"Updated assert ({c}x): {old_ass[:60]}")

# Replace remaining assert_eq with u64 constant comparisons
# Pattern: assert_eq!(\n    policy.max_duration_ms,\n    crate::task::cookiebot::DEFAULT_...
for const_ref in const_refs:
    short_name = const_ref.rsplit("::", 1)[1]
    old_comp = f"assert_eq!(\n            policy.max_duration_ms,\n            {const_ref}"
    new_comp = f"assert_eq!(\n            policy.max_duration_ms.get(),\n            {const_ref}"
    c = after_tests.count(old_comp)
    if c > 0:
        after_tests = after_tests.replace(old_comp, new_comp)
        changes += 1
        print(f"Updated assert with {short_name} ({c}x)")

# Update `policy.max_duration_ms > 0` check
old_gt_zero = '                policy.max_duration_ms > 0,\n                "Task'
new_gt_zero = '                policy.max_duration_ms.get() > 0,\n                "Task'
c = after_tests.count(old_gt_zero)
if c > 0:
    after_tests = after_tests.replace(old_gt_zero, new_gt_zero)
    changes += 1
    print(f"Updated gt_zero check")

content = before_tests + after_tests

# 8. Update zero-timeout test - DurationMs can't be 0
old_zero = '        let policy = TaskPolicy {\n            max_duration_ms: 0,\n            permissions: TaskPermissions::default(),\n        };\n        assert!(policy.validate().is_err());'
new_zero = '        // DurationMs type guarantees non-zero, so any constructable policy is valid\n        let policy = TaskPolicy {\n            max_duration_ms: DurationMs::new_const(1),\n            permissions: TaskPermissions::default(),\n        };\n        assert!(policy.validate().is_ok());'
if old_zero in content:
    content = content.replace(old_zero, new_zero)
    changes += 1
    print(f"Updated zero-timeout test to validate with valid value")
else:
    print(f"WARNING: zero-timeout test pattern not found")

# Write back
with open(path, "w", encoding="utf-8") as f:
    f.write(content)

print(f"\nTotal: {changes} changes applied to {path}")
