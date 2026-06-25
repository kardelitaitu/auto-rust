# CDP Browser Automation

> last audited 26-06-26 by docs-auditor

Skill for understanding and modifying the browser automation layer — the core abstraction that connects `TaskContext` (task-facing API) to Chrome DevTools Protocol (CDP) for mouse, keyboard, scroll, navigation, and native OS input.

## Architecture Overview

```
Task (Rust or DSL)
  │
  ▼
TaskContext (src/runtime/task_context/)
  │  High-level API verbs: click(), keyboard(), navigate(), hover(), scroll(), etc.
  │
  ├──► interaction_pipeline.rs  —  Unified pipeline: preflight → execute → verify → postflight
  │
  ├──► click.rs                 —  Click pipeline with learning system, retry, fallback
  ├──► pointer.rs               —  Hover, drag, keyboard/type, nativecursor
  ├──► page_nav.rs              —  Navigate, focus, wait_for_load, CDP retry
  ├──► click_learning.rs        —  Adaptive click timing (fatigue, page context, selector stats)
  │
  ├──► interaction.rs           —  Re-exports keyboard, clipboard, scroll helpers
  ├──► types.rs                 —  InteractionRequest, InteractionResult, shared types
  ├──► query.rs                 —  exists(), visible(), text(), wait_for(), viewport()
  └──► dom_verify.rs            —  Post-interaction verification helpers
       │
       ▼
  Capabilities layer (src/capabilities/ and src/internal/)
       │  Re-exports from internal modules — stable boundary for task code
       │
       ▼
  Utilities layer (src/utils/)
       │
       ├── mouse/   —  Human-like cursor movement and click simulation
       ├── keyboard.rs  —  Keystroke simulation, CDP dispatch
       ├── native_input.rs  —  OS-level input via enigo
       ├── scroll.rs  —  Smooth scrolling with easing
       ├── dom.rs  —  DOM query helpers
       └── timing.rs  —  Human-like pause distributions
```

## File Map

### TaskContext API entry points (`src/runtime/task_context/`)

| File | Purpose |
|---|---|
| `mod.rs` | `TaskContext` struct definition, API verb signatures (click, type, navigate, scroll, etc.) |
| `click.rs` | Primary click pipeline: retry loop, adaptive timing, coordinate fallback, strict verification |
| `click_learning.rs` | `ClickLearningState`, `ClickTimingContext`, `ClickTimingProfile`, selector performance tracking |
| `pointer.rs` | Hover, drag, keyboard/type text, nativecursor, press, cursor movement |
| `page_nav.rs` | Navigate, focus, wait_for_load, CDP retry with exponential backoff, screenshot |
| `interaction.rs` | Re-exports from capabilities (keyboard press, clipboard, scroll) |
| `interaction_pipeline.rs` | Unified pipeline: `execute_interaction()` — preflight, execute by kind, postflight |
| `types.rs` | `InteractionRequest`, `InteractionResult`, `InteractionKind`, outcome types |
| `query.rs` | DOM query helpers: exists, visible, text, html, attr, value, wait_for |
| `dom_verify.rs` | Post-interaction element verification, coordinate hit-testing |
| `validation.rs` | Session data validation |

### Mouse simulation (`src/utils/mouse/`)

| File | Purpose |
|---|---|
| `mod.rs` | High-level functions: `click_selector_human`, `hover_selector_human`, `native_click_selector_human`, coordinate mapping |
| `overlay.rs` | Visual cursor overlay (dot + ring + ghost trail), `cursor_move_to` with path style dispatch, click flash |
| `trajectory.rs` | Path generation: Bezier, arc, zigzag, overshoot, stopped, muscle |
| `curves.rs` | Click dispatch: `dispatch_click` fires pointer events → mousedown → pause → mouseup → pointer events |
| `cdp.rs` | Low-level CDP event dispatch: `dispatch_mouse_event_cdp`, `dispatch_pointer_event`, `dispatch_single_mouse_event` |
| `native.rs` | Native click calibration, fingerprint-based caching, screen coordinate mapping, lock management |
| `adaptive.rs` | Element type detection, hover dwell customization, collision-avoidant movement, stability checks |
| `types.rs` | `ClickOutcome`, `HoverOutcome`, `NativeCursorOutcome`, `MouseButton`, status enums |

### Keyboard simulation (`src/utils/`)

| File | Purpose |
|---|---|
| `keyboard.rs` | `press()`, `press_with_modifiers()`, `type_text_profiled()`, `natural_typing_profiled()`, CDP char dispatch |
| `native_input.rs` | OS-level input via `enigo` library, `jittered_delay_ms()`, backend selection |

### Scroll (`src/utils/`)

| File | Purpose |
|---|---|
| `scroll.rs` | `read()`, `scroll_into_view`, `scroll_to_top`, `scroll_to_bottom`, `human_scroll`, smooth easing |

## Key Concepts

### 1. TaskContext API Verbs

Every task action goes through a `TaskContext` method. The full API surface:

**Click methods:**
- `click(selector)` — Primary click with learning system, scroll, retry, fallback
- `nativeclick(selector)` — OS-level click via enigo (bypasses CDP)
- `click_at(x, y)` — Fast cursor move + click at raw coordinates
- `click_and_wait(selector, next, timeout_ms)` — Click then wait for next element
- `double_click(selector)`, `middle_click(selector)`, `right_click(selector)`
- `left_click(x, y)`, `left_click_fast(x, y)`, `right_click_at(x, y)`, `right_click_fast(x, y)`

**Keyboard methods:**
- `keyboard(selector, text)` or `r#type(selector, text)` — Focus + type with profiled timing
- `type_text(text)` — Type into currently focused element
- `press(key)`, `press_with_modifiers(key, modifiers)` — Single key press
- `select_all(selector)`, `clear(selector)`

**Cursor methods:**
- `hover(selector)` — Human-like hover with configurable timing
- `move_mouse_to(x, y)`, `move_mouse_fast(x, y)`
- `randomcursor()` — Move to random viewport position
- `nativecursor()`, `nativecursor_query(query)` — OS-level cursor move
- `drag(from, to)` — Drag from one selector to another
- `sync_cursor_overlay()`

**Navigation:**
- `navigate(url, timeout_ms)` — Navigate to URL with settle timing
- `wait_for_load(timeout_ms)`
- `set_user_agent(ua)`

**Scroll methods:**
- `scroll_to(selector)`, `scroll_into_view(selector)`
- `scroll_read(pauses, scroll_amount, variable_speed, back_scroll)`
- `scrollread(duration_ms)`
- `scroll_to_top()`, `scroll_to_bottom()`
- `scroll_back(distance)`

**Query methods:**
- `exists(selector)`, `visible(selector)`
- `text(selector)`, `html(selector)`, `attr(selector, name)`, `value(selector)`
- `wait_for(selector, timeout_ms)`, `wait_for_visible(selector, timeout_ms)`
- `url()`, `title()`, `viewport()`

**Other:**
- `interact(request)` — Unified pipeline entry point
- `screenshot()`, `screenshot_with_quality(quality)`
- `pause(base_ms)`, `pause_with_variance(base, pct)`, `pause_human(base, pct)`
- `copy()`, `cut()`, `paste()`

### 2. Interaction Pipeline

The unified pipeline (`interaction_pipeline.rs`) provides consistent preflight, execution, verification, and postflight:

```
execute_interaction(ctx, request)
  │
  ├── Preflight: element exists? visible? (skip visibility for keyboard/type/selectall/clear)
  ├── Execute by kind:
  │   ├── Click → click_internal → retry loop → fallback
  │   ├── Type → focus + keyboard_internal
  │   ├── NativeClick → nativeclick_internal → coordinate fallback
  │   ├── Focus, Hover, SelectAll, Clear, Keyboard
  └── Postflight: pause with configured delay
```

`InteractionRequest` builder pattern:
```rust
let result = api.interact(
    InteractionRequest::click("#submit")
        .without_verification()
        .without_fallback()
        .with_pause(200)
).await?;
```

### 3. Click Pipeline (the most complex interaction)

The click pipeline (`click.rs`) has multiple layers:

```
click(selector)
  │
  ├── Accessibility locator? → Direct coordinate click (skip human simulation)
  ├── Compute timing profile from learning system:
  │   - Page type (Social, Form, Home, Commerce, Content, Other)
  │   - Element priority (Critical, Normal, Optional)
  │   - Fatigue level (Rested < 15 clicks, Normal < 50, Tired >= 50)
  │   - Recent success rate (last 32 clicks)
  │   - Adaptation from selector history
  ├── Attention pause (pre-move)
  ├── Primary click attempts (MAX_ATTEMPTS = 3):
  │   - Attempt 1: base delay
  │   - Attempt 2: delay * 1.18
  │   - Attempt 3: delay * 1.36
  │   - Backoff between attempts: (150 + attempt * 180) + extra_stability_wait/2
  ├── Fallback click (if primary exhausted):
  │   - Focus element → click_at coordinates
  │   - Extra stability wait if adaptation suggests
  ├── Strict verification (if recent failures):
  │   - Hit-test elementFromPoint at click coordinates
  │   - If failed, another fallback attempt
  └── Record outcome to learning system (persisted to click-learning/ dir)
```

Constants:
- `CLICK_TOTAL_TIMEOUT_SECS = 12`
- `CLICK_MAX_ATTEMPTS = 3`

### 4. Mouse Movement Pipeline

```
cursor_move_to(page, target_x, target_y) (overlay.rs)
  │
  ├── Get start position from overlay state (or viewport center)
  ├── Degenerate path guard: if distance < 0.5px, dispatch single mousemove
  ├── Generate path based on PathStyle:
  │   ├── Bezier (default) — cubic bezier with Gaussian control points
  │   ├── Arc — curved path with upward/downward arc
  │   ├── Zigzag — perpendicular zigzag for erratic movement
  │   ├── Overshoot — goes past target then corrects
  │   ├── Stopped — intermediate pause points
  │   └── Muscle — simulation-based movement with jitter
  ├── Dispatch mousemove events along path
  │   - Each step: CDP DispatchMouseEvent (MouseMoved) → sync cursor overlay
  │   - Between steps: human pause with configurable delay and variance
  │   - 10% chance of extra micro-pause per step
  └── Sync overlay to final position
```

### 5. Click Simulation Pipeline

A single click fires a lifecycle of events:

```
dispatch_click(page, x, y, button) (curves.rs)
  │
  ├── pointerover  (JS PointerEvent dispatch)
  ├── pointerenter (JS PointerEvent dispatch)
  ├── [pause ~15ms]
  ├── pointermove  (last position update)
  ├── mousedown    (CDP DispatchMouseEvent)
  ├── [pause ~80ms — humans don't release immediately]
  ├── mouseup      (CDP DispatchMouseEvent)
  ├── pointerout   (cleanup)
  └── trigger_click_flash (visual ripple on overlay)
```

### 6. Hover Micro-Hesitation

When hovering before a click (`adaptive.rs::hover_before_click`), the system simulates human micro-movements:

```
hover_before_click(page, x, y, element_type)
  │
  ├── Determine dwell time based on element type:
  │   - "engagement" (like, retweet, follow): 500-1500ms
  │   - "button": 300-800ms
  │   - "link": 400-900ms
  │   - "input": 50-150ms
  │   - "checkbox": 200-500ms
  │   - default: 80-300ms
  ├── Fire pointerenter
  ├── Phase 1: half dwell (wait)
  ├── Micro-fidget: 1-2 small cursor shifts (Gaussian ~1.5px std dev)
  ├── Phase 2: return to target + remaining dwell
  └── Fire pointerleave
```

### 7. Native OS Input (enigo)

When CDP is insufficient (e.g., native OS dialogs, system-level interactions), the native input backend is used:

```
Native click pipeline (mod.rs::native_click_selector_human)
  │
  ├── Acquire native input lock (global mutex — prevents concurrent OS input)
  ├── Bring browser window to front
  ├── Scroll element into view (CDP)
  ├── Resolve content coordinates → map to screen coordinates
  │   - Get browser window metrics (screenX/Y, inner/outer dimensions, DPR, viewport scale)
  │   - Compute calibration: scale_x/y = DPR / viewport_scale
  │   - Origin offset: screen_x + chrome_x + viewport_offset * scale
  │   - Screen point = origin + content_point * scale + adjustment
  ├── Verify point hits selector via elementFromPoint
  ├── Sync overlay to match native position
  ├── Dispatch via enigo:
  │   - Generate eased trajectory from current mouse position to target
  │   - Steps: (distance / 85).ceil() steps, clamped [6, 16]
  │   - Each step: cubic ease-out + random jitter + per-step delay
  │   - Click: button press + ~45ms hold + button release
  └── Verify click target via elementFromPoint
```

Calibration modes:
- **Windows**: chrome_x = (outer_width - inner_width) / 2
- **Mac/Linux**: chrome_x = 0
- Calibration cached by fingerprint (mode, window metrics, DPR, viewport scale)

Backend selection (`NativeInputBackend`):
- `Enigo` (default, works on all platforms)
- `Sendinput` / `Rdev` — fall back to enigo with a warning

### 8. Click Learning System

The adaptive learning system (`click_learning.rs`) tracks click performance and adjusts timing:

```
ClickLearningState
  ├── interaction_count: u64
  ├── total_attempts, total_successes
  ├── recent_results: VecDeque<bool> (last 32)
  └── selectors: HashMap<String, SelectorLearningStats>
        ├── attempts, successes, consecutive_failures
        └── last_updated: Option<DateTime>

Calculated adaptations:
  ├── After 3+ attempts with < 75% success:
  │   - extra_stability_wait += 250ms
  │   - reaction_delay *= 1.20
  │   - prefer_coordinate_fallback = true
  │   - require_strict_verification = true
  ├── After 2+ consecutive failures:
  │   - extra_stability_wait += 380ms
  │   - reaction_delay *= 1.22
  │   - click_offset += 2px
  ├── Complex selectors (nth-child, data-testid, >):
  │   - extra_stability_wait += 120ms
  │   - reaction_delay *= 1.08
  └── Fatigue level "Tired" (50+ interactions):
      - reaction_delay *= 1.15
      - extra_stability_wait += 140ms
```

### 9. CDP Retry with Exponential Backoff

For CDP operations that fail transiently (`page_nav.rs`):

```
with_retry(op)
  │
  ├── Max attempts: 3
  ├── Base delay: 50ms
  ├── Backoff: base_delay * (1 << attempt.min(6))
  │   - Attempt 1: 100ms
  │   - Attempt 2: 200ms
  │   - Attempt 3: 400ms
  │   - Max: 3200ms (capped at shift 6)
  └── Only retries TRANSIENT errors:
      - Timeout ✓
      - Connection ✓
      - Temporary ✓
      - Network ✓
      - RateLimited ✓
      - Cancelled ✓
      - Disconnected ✓
      - NOT_FOUND ✗ (permanent)
      - PERMISSION_DENIED ✗ (permanent)
      - TARGET_TERMINATED ✗ (permanent)
```

### 10. Keyboard Input

Two dispatch methods:

**Standard keys** (letter keys, Enter, Tab, etc.): Uses `Input.dispatchKeyEvent` CDP method
```
rawKeyDown → char → keyUp per character
```
This fires `isTrusted=true` events that React cannot distinguish from real user input.

**Backspace**: Same CDP method with `windows_virtual_key_code=8`

**Profile-driven typing** (`natural_typing_profiled`):
- Keystroke delay: Gaussian(mean, stddev, min=50ms, max=500ms)
- Word pauses: configurable (clamped to 1500ms max)
- Typos: configurable rate, with optional correction
  - Typo: types a keyboard-adjacent character
  - Correction: Backspace + retype the correct character
  - Recovery chance: configurable (default 80%)

### 11. Cursor Overlay

Visual cursor overlay injected into every browser page:

- **Dot**: white circle with orange (`#ff6600`) border, 12px default
- **Ring**: transparent circle with orange border, 24px
- **Ghost trail**: 3 fading dots behind cursor (configurable with `CURSOR_OVERLAY_SHOW_TRAIL`)
- **Click flash**: dot scales to 1.8x, turns red, then ripple ring expands and fades
- Configured via env vars: `MOUSE_OVERLAY_SIZE_PX`, `CURSOR_OVERLAY_COLOR`, `CURSOR_OVERLAY_SHOW_TRAIL`
- Hides native cursor via `cursor: none !important`

## Common Modification Patterns

### Adding a new TaskContext API method

1. Add method signature in `src/runtime/task_context/mod.rs`
2. Implement logic in the appropriate submodule or inline
3. If it's an interaction kind, add to `InteractionKind` enum in `types.rs`
4. Wire into the pipeline in `interaction_pipeline.rs`
5. Add tests

### Modifying click behavior

1. Locate the timing parameters in `click_learning.rs` (`ClickTimingContext`, `ClickTimingProfile`)
2. Adjust the relevant multiplier/clamp
3. Verify in `click.rs` that the pipeline uses the new value
4. Check `choose_click_point` in `mod.rs` for offset logic
5. Run click-specific tests: `cargo test --lib click`

### Adding a new cursor path style

1. Add variant to `PathStyle` enum in `overlay.rs`
2. Implement generation function in `trajectory.rs`
3. Wire into `cursor_move_to_with_config` in `overlay.rs`
4. Add tests in `trajectory.rs`

### Adjusting native click calibration

1. Modify `browser_content_origin()` in `native.rs` for platform-specific chrome offsets
2. Adjust `browser_scale()` for DPR/viewport scaling
3. Update calibration validation bounds in `validate_native_calibration()`
4. Run native click tests: `cargo test --lib native`

## Test Locations

| Test | Location | Command |
|---|---|---|
| Click pipeline unit tests | `src/runtime/task_context/click.rs` | `cargo test --lib click` |
| Click learning tests | `src/runtime/task_context/click_learning.rs` | `cargo test --lib click_learning` |
| Mouse simulation tests | `src/utils/mouse/mod.rs` | `cargo test --lib mouse` |
| Trajectory tests | `src/utils/mouse/trajectory.rs` | `cargo test --lib trajectory` |
| Native click tests | `src/utils/mouse/native.rs` | `cargo test --lib native` |
| Navigation/page tests | `src/runtime/task_context/page_nav.rs` | `cargo test --lib page_nav` |
| Keyboard tests | `src/utils/keyboard.rs` | `cargo test --lib keyboard` |
| Scroll tests | `src/utils/scroll.rs` | `cargo test --lib scroll` |
| Interaction pipeline tests | `src/runtime/task_context/interaction_pipeline.rs` | `cargo test --lib interaction_pipeline` |
| Type tests | `src/runtime/task_context/types.rs` | `cargo test --lib task_context_types` |

## Pitfalls

1. **Don't bypass the pipeline** — Always use `TaskContext` methods or `interact()` instead of calling CDP directly, unless you have a very specific reason
2. **Native click is blocking** — `enigo` operations run on `spawn_blocking`, so they tie up the async runtime threadpool. Don't call in a tight loop
3. **Overlay state is per-page** — Each page has its own `SessionOverlayState`. If you switch pages, cursor position resets to viewport center
4. **Click learning is persistent** — The `click-learning/` directory stores JSON state. Delete it to reset adaptations
5. **CDP retry doesn't apply everywhere** — Only `page_nav.rs` uses `with_retry()`. Other CDP calls fail immediately. If adding CDP calls in a new location, consider wrapping in retry for transient errors
6. **Keyboard dispatch varies by key** — Standard keys use CDP `Input.dispatchKeyEvent` (trusted). Backspace uses same but with virtual key code 8. Don't mix JS `KeyboardEvent` (untrusted by React) with CDP dispatch
7. **Hover micro-hesitation is element-aware** — Element type detection is heuristic-based (string matching on selectors). Adding new engagement types requires updating `detect_element_type()` in `adaptive.rs`
8. **Native click calibration can drift** — When browser zoom, DPI, or window position changes, the fingerprint no longer matches and calibration is recomputed. If you see inaccurate native clicks, check `browser_window_metrics()` output
9. **The interaction pipeline runs visibility check for clicks but not type** — `interaction_needs_visibility()` in `interaction_pipeline.rs` defines which interactions require visible elements. Don't change this without understanding the implications
