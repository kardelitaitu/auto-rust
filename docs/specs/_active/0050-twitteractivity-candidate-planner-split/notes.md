# Notes

- The risk is not the extraction itself.
- The risk is letting planner logic drift from executor logic after the split.
- Keep the planner small enough to test without browser setup.
- Keep the executor small enough to inspect when behavior changes.
- If the planner needs browser-derived data, pass a snapshot, not a live task handle.
