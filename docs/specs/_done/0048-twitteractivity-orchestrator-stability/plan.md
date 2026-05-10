## Plan

1. Lock the current task contract by checking payload parsing, timeout behavior, and summary formatting.
2. Verify the helper-module wiring stays thin and the task still delegates navigation, scanning, and candidate handling.
3. Add or tighten regression coverage where the task shell can drift from helper behavior.
4. Run the package checks and only then move implementation work forward.
