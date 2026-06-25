last audited 26-06-26 by antigravity

## Acceptance Criteria

1. `natural_typing_profiled` inserts a word pause after every space character.
   - Pause duration drawn from `behavior.word_pause_ms`, clamped to [0, 1500]ms, ±30% variance.
   - No pause inserted for non-space characters.

2. `get_similar_char` returns an adjacent key (not the original character) for all of:
   `z, x, c, v, b, g, h, j, k, l, u` — matching QWERTY physical adjacency.

3. `natural_typing` wrapper carries a `#[deprecated]` attribute that emits a compiler
   warning when called. The function body and signature are unchanged.

4. `cargo check` completes with zero errors and zero new warnings (deprecation warning
   is expected and acceptable at call sites; no other warnings introduced).

5. `cargo test utils::keyboard` passes with zero regressions.

6. New unit tests added:
   - `word_pause_fires_on_space` — asserts that `word_pause_ms` path is reachable
     (via a unit test on the logic, not async execution).
   - One test per newly added character in `get_similar_char` (11 new mappings minimum).

## Test Commands

```powershell
cargo check
cargo test utils::keyboard
cargo clippy -- -D warnings
cargo fmt --all --check
```

## Visual Inspection

After implementation, confirm in `src/utils/keyboard.rs`:

1. `natural_typing_profiled` — contains a `if ch == ' '` branch that calls `human_pause`
   with `behavior.word_pause_ms.clamp(0, 1500)`.

2. `get_similar_char` — match arms include at minimum:
   `'z' => 'x'`, `'x' => 'z'`, `'c' => 'v'`, `'v' => 'c'`, `'b' => 'v'`,
   `'g' => 'h'`, `'h' => 'g'`, `'j' => 'k'`, `'k' => 'j'`, `'l' => 'k'`, `'u' => 'y'`

3. `natural_typing` — has `#[deprecated(since = "0.2.3", note = "...")]` attribute
   directly above the `pub async fn` line.

4. No `natural_typing(` call sites outside the function's own definition (confirmed by
   `grep -rn "natural_typing(" src/` showing only `keyboard.rs` and `natural_typing_profiled`).
