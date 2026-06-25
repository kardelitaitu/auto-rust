last audited 26-06-26 by antigravity

# Typing Realism: Word Pauses, Full Keyboard Adjacency, Profile-Driven Speed

## Baseline

The typing system in `src/utils/keyboard.rs` has three concrete gaps:

**1. `word_pause_ms` is defined but never used**
`TypingBehavior` (defined in `src/utils/profile/mod.rs:142`) contains `word_pause_ms: u64`,
populated by `typing_behavior()` from the profile range `50–2000ms`. However,
`natural_typing_profiled` (keyboard.rs:174) iterates over characters without ever checking
for a space character and inserting the pause. The field is dead code.

**2. `get_similar_char` covers only 15 keys**
The current adjacency map (keyboard.rs:424) covers: `a,s,d,f,e,r,w,q,t,y,o,p,i,n,m`.
Missing the entire bottom row (`z,x,c,v,b`), common home-row keys (`g,h,j,k,l`),
and upper-row (`u`). Typos on any unrecognized character fall back to identity — the
"wrong" char is the correct char, producing no visible typo.

**3. `natural_typing` wrapper hardcodes `keystroke_mean_ms: 120`**
The public `natural_typing(page, selector, text, typo_rate)` function (keyboard.rs:160)
always constructs `TypingBehavior { keystroke_mean_ms: 120, keystroke_stddev_ms: 40, ... }`,
ignoring any per-session profile. Two callers exist:
- `src/utils/twitter/twitteractivity_interact.rs:432` — uses `natural_typing_profiled`
  (already correct).
- The wrapper itself is exported and could be picked up by future callers silently
  regressing to hardcoded 120ms.

## Implementation Steps

### Step 1 — Activate `word_pause_ms` in `natural_typing_profiled`
File: `src/utils/keyboard.rs`, function `natural_typing_profiled` (line 174).

After dispatching a space character (`ch == ' '`), insert:
```rust
if ch == ' ' {
    let word_pause = behavior.word_pause_ms.clamp(0, 1500);
    human_pause(word_pause, 30).await;
}
```
The 1500ms clamp prevents excessive latency on long texts. Place this **after**
`type_character_profiled` so the space itself is typed before the pause fires.

### Step 2 — Expand `get_similar_char` to full QWERTY adjacency
File: `src/utils/keyboard.rs`, function `get_similar_char` (line 424).

Add the following mappings (keyboard-adjacent, not phonetically similar):
```
'z' => 'x',   'x' => 'z',
'c' => 'v',   'v' => 'c',
'b' => 'v',   
'g' => 'h',   'h' => 'g',
'j' => 'k',   'k' => 'j',
'l' => 'k',
'u' => 'y',
```
Do NOT add mappings for space, digits, or punctuation — a typo on those
characters is too disruptive and hard for the "human" to notice naturally.

### Step 3 — Deprecate the hardcoded `natural_typing` wrapper
File: `src/utils/keyboard.rs`, function `natural_typing` (line 160).

Add a `#[deprecated]` attribute with a message directing callers to
`natural_typing_profiled`. Keep the function body intact so existing callers
compile — the deprecation is a warning gate, not a breaking change.

```rust
#[deprecated(
    since = "0.2.3",
    note = "Use `natural_typing_profiled` with the session profile's `typing_behavior()` \
            to get per-account speed variation. This wrapper hardcodes keystroke_mean_ms=120."
)]
pub async fn natural_typing(...) { ... }
```

### Step 4 — Audit call sites
Run: `cargo check 2>&1 | grep -i "natural_typing\|deprecated"`

Confirm `twitteractivity_interact.rs` already calls `natural_typing_profiled` (it does — line 432).
If any other call site references the deprecated wrapper, migrate it.

### Step 5 — Verify and test
```
cargo check
cargo test utils::keyboard
cargo fmt --all
```

## API Changes

- `natural_typing` — marked `#[deprecated]`, no signature change
- `natural_typing_profiled` — no signature change, behavior extended: now inserts word pauses
- `get_similar_char` — extended match arms, no signature change

No public API breakage.

## Validation

See `validation.md`.

## Design Decisions and Risks

**Word pause placement:** The pause fires after the space is typed (not before).
This mirrors real behavior — humans finish typing the previous word, hit space,
then pause before starting the next word. Confidence: **High**.

**Clamp at 1500ms:** Profile-derived `word_pause_ms` can reach 2000ms. A long
reply with 40 spaces at 2000ms = 80s added latency, risking the task timeout.
1500ms is a safe ceiling that preserves variation without danger. Confidence: **High**.

**No adjacency for space/digits:** A typo replacing a space with a random letter
would produce garbled text that looks nothing like a human mistake. Real typo maps
only cover key adjacency on the QWERTY layout. Confidence: **High**.

**Deprecation over removal:** Removing `natural_typing` immediately would break
any future-added callers silently. The `#[deprecated]` attribute surfaces warnings
at compile time without forcing immediate migration. Confidence: **High**.
