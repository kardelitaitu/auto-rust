# Plan: Cross-Origin OOPIF Iframe Interaction (Rust CDP Client)

**Status:** Draft — awaiting approval to implement
**Date:** 2026-08-23
**Owner:** auto-rust (Rust orchestrator)
**Branch:** `ollama-queue`

---

## ✅ Checklist

- [x] **C1. Add `async-tungstenite` direct dependency**
- [x] **C2. Create `src/runtime/task_context/oopif.rs` — minimal CDP client**
- [x] **C3. Thread the browser WS URL through Session → TaskContext**
- [x] **C4. Prove the concept with a live probe (before wiring into the API)**
- [x] **C5. Rewrite `iframe_click` internals to use the OOPIF client**
- [x] **C6. Add element-listing helper for multiple/scrollable elements**
  - `element_local_center` iterates all matches, scrolls first visible into view
  - atf task loops `iframe_click` to click every "Go" button
- [x] **C7. Update the `atf` task to use the working iframe interaction**
- [x] **C8. Docs**
- [ ] **C9. Verification**
  - `cargo check`, `cargo test --lib`, `cargo clippy --lib -- -D warnings`, `cargo fmt --all -- --check` ✅
  - Full `.\check.ps1` gate — running
  - Live end-to-end: `cargo run --bin auto -- atf` against a CDP-enabled profile ⏳ (launcher offline — needs user)
- [ ] **C10. Commit & push** (logical commits, repo-style messages)

---

## 1. Problem Statement

The `atf` task (Telegram mini-app mining) cannot click elements inside the
Telegram mini-app. The mini-app runs in a cross-origin iframe
(`atfminers.asloni.online` inside `web.telegram.org`), and none of the current
interaction paths can reach its content:

- Top-document CSS selectors: blocked by the browser same-origin policy.
- `Page.getFrameTree` + `frame_execution_context`: the iframe is an OOPIF and
  does **not** appear as a child frame in the main page's tree (verified live).
- `DOM.getDocument`/`describeNode`/`performSearch` with `pierce: true`: do not
  expose OOPIF content in this Chromium version (verified live: 0 results).
- `browser.get_page(iframe_target_id)`: chromiumoxide 0.6 only manages
  page-type targets; iframe targets return `Requested value not found`.
- Coordinate-based clicking (`iframe_click_at`): window-size dependent and
  breaks for the 6 scrollable "Go" buttons.

## 2. Root Cause

Cross-origin iframes are **OOPIFs** — separate browser processes/targets.
Interacting with their content requires a **separate CDP session attached to
the iframe target**. Our driver (chromiumoxide 0.6) holds one CDP session bound
to the main page and does not expose arbitrary-session routing in its public
API. The frame-tree, DOM, and page-creation APIs all operate on the main-page
session only.

## 3. Solution Overview

Build a **minimal custom CDP client** that opens a second WebSocket to the
browser's debug endpoint and drives the iframe target directly:

1. `Target.getTargets` → locate the mini-app target (`type=iframe`, host match).
2. `Target.attachToTarget { targetId, flatten: true }` → get a `sessionId`.
3. `Runtime.evaluate` (sent **with** that `sessionId`) → runs inside the
   mini-app, exactly like DevTools when you switch the console context to the
   iframe.
4. Read the element's local rect (with `scrollIntoView` for scrollable lists).
5. Absolute viewport point = iframe's current absolute rect + local point.
6. Dispatch the click with the existing chromiumoxide `Input` domain
   (browser-level — works across origins).

This keeps everything in Rust, is window-size independent (iframe rect read at
runtime), and supports scrolling + multiple matching elements via a single JS
evaluation.

## 4. Key APIs (CDP)

| Method | Purpose |
|---|---|
| `Target.getTargets` | List all targets; find `type=iframe` with our host |
| `Target.attachToTarget` | Attach to the iframe target; returns `sessionId` (flatten: true) |
| `Runtime.evaluate` | Evaluate JS in the target's default execution context |
| (existing) `Input.dispatchMouseEvent` | Click at viewport coords (via `mouse::left_click_at`) |

## 5. Module Design — `src/runtime/task_context/oopif.rs`

```rust
/// Minimal CDP client over a second WebSocket to the browser debug endpoint.
pub struct OopifClient {
    sender: mpsc::Sender<Outgoing>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
}

impl OopifClient {
    pub async fn connect(browser_ws_url: &str) -> Result<Self>;
    /// Send a JSON-RPC call; optionally scoped to a target session.
    pub async fn call(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value>;
    /// Convenience: find iframe target by host substring.
    pub async fn find_iframe_target(&self, host: &str) -> Result<TargetInfo>;
    /// Attach + return sessionId.
    pub async fn attach(&self, target_id: &str) -> Result<String>;
    /// Evaluate JS in the given session; returns the JSON value.
    pub async fn evaluate(&self, session_id: &str, expression: &str) -> Result<Value>;
}
```

- **Writer task**: owns the WebSocket, sends outgoing frames.
- **Reader task**: parses incoming messages; `{"id":N,...}` → pending oneshot;
  events ignored.
- **Response correlation**: incrementing call id + `HashMap` of oneshot senders
  (guarded by `tokio::sync::Mutex`).

## 6. Plumbing — Browser WS URL

- `Session` gains `browser_ws_url: String`.
- `Session::new(...)` gains a parameter; each connector's `connect()` passes
  `capability.ws_url` (the browser-level `ws://…/devtools/browser/<id>`).
- `TaskContext` receives it so the frame module can build an `OopifClient`.

## 7. `iframe_click` Rewrite (behaviour)

```
1. resolve_iframe(iframe_selector)          → rect (x,y,w,h) + src   [existing, works]
2. OopifClient::connect(browser_ws_url)
3. find_iframe_target(host_from_src)
4. attach(target_id)                        → session_id
5. evaluate(session_id, js)                 → element local center (scrollIntoView first)
6. absolute = (rect.x + local.x, rect.y + local.y)
7. mouse::left_click_at(page, absolute)     [existing chromiumoxide Input]
8. fallback: if any step fails → existing frame-tree path (best effort)
```

JS for step 5 (handles scrolling + first-visible element):

```js
(() => {
  const els = document.querySelectorAll(SELECTOR);
  for (const el of els) {
    el.scrollIntoView({ block: 'center' });
    const r = el.getBoundingClientRect();
    if (r.width > 0 && r.height > 0) {
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    }
  }
  return null;
})()
```

## 8. Effort & Risk

| Item | Estimate |
|---|---|
| C2 client | ~150 lines |
| C3 plumbing | ~50 lines across Session/TaskContext/connectors |
| C4 probe + C5 rewrite | ~100 lines |
| C6 listing helper | ~40 lines |
| Total new code | ~250–350 lines |
| New direct dependency | `async-tungstenite = "0.25.1"` (already in tree) |

**Risks / mitigations**
- WebSocket framing & reader/writer races → keep the client minimal, unit-test
  correlation logic with a mocked stream.
- Attach lifecycle (detach on page reload) → re-attach per `iframe_click` call.
- Session id collision → monotonic counter per client instance.
- `Runtime.evaluate` may need `Runtime.enable` in edge cases → enable
  idempotently on attach if needed (verify in C4).

## 9. Alternatives

| Option | Pros | Cons |
|---|---|---|
| **A. Custom Rust CDP client (this plan)** | All-Rust; no process handoff; full control | ~1–2 hrs; new dep; CDP plumbing |
| **B. Playwright (Node.js)** | ~30 min; native iframe support (`frameLocator`) | Node dependency; Rust↔Node handoff; two runtimes |
| **C. Coordinate clicking** | Trivial | Window-size + scroll fragile; fails 6-button case |

**Recommendation: A**, with B as a fallback if the OOPIF attach hits an
unexpected Chromium restriction during C4.

## 10. Definition of Done

- [ ] C1–C10 all checked
- [ ] Live proof: automation clicks the Tasks tab and a Go button inside the
      mini-app at any window size, including scrolling to a Go button
- [ ] Full `.\check.ps1` green
- [ ] Committed & pushed to `ollama-queue`
