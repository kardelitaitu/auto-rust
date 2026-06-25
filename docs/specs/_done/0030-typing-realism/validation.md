last audited 26-06-26 by antigravity

## Implementation Status: COMPLETE ✅

Implemented by Buffy on 2026-06-26. All acceptance criteria met:

1. `natural_typing_profiled` inserts a word pause after every space character.
   - Pause duration drawn from `behavior.word_pause_ms`, clamped to [0, 1500]ms, ±30% variance.
   - Pause placed **after the entire character-dispatch if/else block**, not inside one branch
     (ensures it fires regardless of typo path).
   - No pause inserted for non-space characters.

2. `get_similar_char` returns an adjacent key for all of:
   `z, x, c, v, b, g, h, j, k, l, u` — matching QWERTY physical adjacency.

3. `natural_typing` wrapper carries a `#[deprecated(since = "0.2.32")]` attribute.
   Function body and signature are unchanged.

4. `cargo check` passes with zero errors and zero warnings.

5. `cargo test utils::keyboard` passes with 53/53 tests (39 original + 14 new).

6. New unit tests added:
   - `test_word_pause_ms_clamp_upper_bound` — verifies clamp at 1500ms (not 2000ms from profile)
   - `test_word_pause_ms_clamp_lower_bound` — verifies clamp at 0
   - `test_word_pause_space_is_identity` — verifies space passes through `get_similar_char` unchanged
   - 11 per-character tests for each new `get_similar_char` mapping

## Refinements vs Original Plan

| Aspect | Original plan | Actual implementation |
|---|---|---|
| Word pause placement | After `type_character_profiled` branch | After the entire if-else block (handles all 3 dispatch paths: normal, typo-correct, typo-leave) |
| Deprecation version | `since = "0.2.3"` | `since = "0.2.32"` (matches current branch) |
| Proptest char range | `'a'..='z'` for unmapped identity test | `' '..='~'` (all letters now mapped; uses digits/symbols for unmapped cases) |

## Test Commands

```powershell
cargo check
cargo test utils::keyboard
cargo fmt --all --check
```

## Visual Inspection

After implementation, confirm in `src/utils/keyboard.rs`:

1. `natural_typing_profiled` — contains a `if ch == ' '` branch that calls `human_pause`
   with `behavior.word_pause_ms.clamp(0, 1500)`, placed after the character-dispatch block.

2. `get_similar_char` — match arms include:
   `'z' => 'x'`, `'x' => 'z'`, `'c' => 'v'`, `'v' => 'c'`, `'b' => 'v'`,
   `'g' => 'h'`, `'h' => 'g'`, `'j' => 'k'`, `'k' => 'j'`, `'l' => 'k'`, `'u' => 'y'`

3. `natural_typing` — has `#[deprecated(since = "0.2.32", note = "...")]` attribute
   directly above the `pub async fn` line.

4. No `natural_typing(` call sites outside the function's own definition (confirmed by
   `cargo check` with zero warnings — no existing callers emit deprecation warnings).
