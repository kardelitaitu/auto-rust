# Plan

## What Is the Solution
1. **Refactor `mouse.rs`**: Move the mathematical trajectory generation (Bezier curves, Fitts's Law) into `src/utils/mouse/trajectory.rs`.
2. Move the interaction logic (hovering, clicking) into `src/utils/mouse/interact.rs`.
3. Leave `mouse.rs` purely as an API facade (re-exporting the modules).
