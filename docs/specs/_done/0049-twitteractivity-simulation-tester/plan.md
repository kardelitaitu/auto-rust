## Plan

1. Define a simulation-only contract that never touches browser APIs.
2. Add a deterministic planner that rolls the same task decisions in memory.
3. Log a compact task timeline covering phase order, candidate budget, action rolls, and stop reason.
4. Add regression tests for repeatable output and no-side-effect behavior.
