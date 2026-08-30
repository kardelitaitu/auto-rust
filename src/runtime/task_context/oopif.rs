//! Minimal raw CDP client for attaching to OOPIF (cross-origin iframe) targets.
//!
//! chromiumoxide 0.6 holds a single CDP session bound to the main page and
//! cannot route commands to arbitrary target sessions — which is exactly what
//! out-of-process iframes (OOPIFs) require. This module opens a second
//! WebSocket to the browser's debug endpoint and drives the iframe target
//! directly: `Target.getTargets` to find it, `Target.attachToTarget` to get a
//! flattened session, and `Runtime.evaluate` (scoped to that session) so JS
//! runs *inside* the cross-origin iframe — the same mechanism DevTools uses
//! when you switch the console context to a frame.

use anyhow::{anyhow, Result};
use async_tungstenite::tungstenite::Message as WsMessage;
use futures::stream::StreamExt;
use futures::SinkExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Max time to wait for a CDP command response before failing.
const CDP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Capacity of the outgoing command queue shared by all callers of one client.
const OUTGOING_CHANNEL_CAPACITY: usize = 64;

/// Next JSON-RPC call id for this process.
fn next_call_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Split a URL into `(scheme, host)`. The single canonical URL parser for
/// the OOPIF machinery — `frame.rs` imports this instead of duplicating it.
pub(crate) fn scheme_host(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    Some((scheme, host))
}

/// Extract the host from a URL.
fn host_of(url: &str) -> Option<&str> {
    scheme_host(url).map(|(_, host)| host)
}

/// Route a JSON-RPC response to its pending oneshot by call id.
/// Returns `true` when a pending sender was found and notified.
fn route_response(pending: &mut HashMap<u64, oneshot::Sender<Value>>, msg: &Value) -> bool {
    if let Some(id) = msg.get("id").and_then(Value::as_u64) {
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(msg.clone());
            return true;
        }
    }
    false
}

/// Build a JSON-RPC call value (used by the writer task; testable in isolation).
fn build_json_rpc(id: u64, method: &str, params: &Value, session_id: Option<&str>) -> Value {
    let mut call = json!({ "id": id, "method": method, "params": params });
    if let Some(sid) = session_id {
        call["sessionId"] = json!(sid);
    }
    call
}

/// Extract the `result.value` from a CDP evaluate **inner result** (what
/// `call()` returns — the `result` object of the JSON-RPC response).
fn extract_result_value(resp: &Value) -> Value {
    resp.get("result")
        .and_then(|r| r.get("value"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Extract `exceptionDetails.text` from a CDP evaluate **inner result**.
fn extract_exception_text(resp: &Value) -> Option<String> {
    resp.get("exceptionDetails")
        .and_then(|e| e.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Extract `sessionId` from an attachToTarget **inner result**.
fn extract_session_id(resp: &Value) -> Option<String> {
    resp.get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Score how strongly a target URL corresponds to the iframe's `src`:
/// the length of their common prefix. A frame that navigated internally
/// still shares its origin; a blank or secondary sibling shares only the
/// scheme. Generic replacement for the old site-specific "miner"/"index.html"
/// preference — works for any mini-app.
fn url_affinity(url: &str, src_url: &str) -> usize {
    url.bytes()
        .zip(src_url.bytes())
        .take_while(|(a, b)| a == b)
        .count()
}

/// Find an iframe target whose URL host matches `host` (case-insensitive)
/// from a target list, preferring the one whose URL most closely matches
/// `src_url` (the iframe element's resolved src).
///
/// When NO host match exists but the page has exactly one iframe target,
/// that target is returned as a fallback: a mini-app that redirected to a
/// different host keeps its original `src` attribute, so host equality alone
/// would never find it. With several iframe targets this fallback is refused
/// — guessing between frames risks attaching to the wrong one.
fn find_iframe_target_in(infos: &[Value], host: &str, src_url: &str) -> Option<Value> {
    let host_lower = host.to_lowercase();
    let mut best: Option<(usize, Value)> = None;
    let mut iframe_total: usize = 0;
    let mut sole_iframe: Option<Value> = None;
    for info in infos {
        let ty = info.get("type").and_then(Value::as_str).unwrap_or("");
        let url = info.get("url").and_then(Value::as_str).unwrap_or("");
        if ty != "iframe" || url.is_empty() {
            continue;
        }
        iframe_total += 1;
        sole_iframe = Some(info.clone());
        let target_host_lower = host_of(url).unwrap_or("").to_lowercase();
        if target_host_lower == host_lower {
            let score = url_affinity(url, src_url);
            if best.as_ref().is_none_or(|(s, _)| score > *s) {
                best = Some((score, info.clone()));
            }
        }
    }
    if let Some((_, info)) = best {
        return Some(info);
    }
    if iframe_total == 1 {
        sole_iframe
    } else {
        None
    }
}

/// A CDP call queued for the writer task.
struct Outgoing {
    id: u64,
    method: String,
    params: Value,
    session_id: Option<String>,
}

/// Minimal CDP client over a second WebSocket to the browser debug endpoint.
#[derive(Clone)]
pub struct OopifClient {
    tx: mpsc::Sender<Outgoing>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
}

impl OopifClient {
    /// Open a WebSocket to the browser debug endpoint
    /// (`ws://127.0.0.1:<port>/devtools/browser/<id>`).
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (ws, _) = async_tungstenite::tokio::connect_async(ws_url)
            .await
            .map_err(|e| anyhow!("OOPIF CDP connect to '{ws_url}' failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();
        let (tx, mut rx) = mpsc::channel::<Outgoing>(OUTGOING_CHANNEL_CAPACITY);
        let pending = Arc::new(Mutex::new(HashMap::<u64, oneshot::Sender<Value>>::new()));

        // Writer task: drain queued commands to the socket.
        let writer = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let call =
                    build_json_rpc(msg.id, &msg.method, &msg.params, msg.session_id.as_deref());
                if sink.send(WsMessage::Text(call.to_string())).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: route `{"id": N, ...}` responses to pending oneshots.
        let pending_reader = pending.clone();
        let reader = tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        log::warn!("[oopif] CDP websocket read error, connection ending: {e}");
                        break;
                    }
                };
                let text = match msg {
                    WsMessage::Text(t) => t,
                    WsMessage::Binary(b) => String::from_utf8_lossy(&b).to_string(),
                    _ => continue,
                };
                let Ok(v) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                route_response(&mut *pending_reader.lock().await, &v);
            }
            // Stream ended or errored: drop pending oneshots so callers fail immediately
            // with "channel closed" instead of waiting 10s for CDP timeout.
            pending_reader.lock().await.clear();
        });
        let _ = (writer, reader);

        Ok(Self { tx, pending })
    }

    /// Send a CDP command and await its response. Scopes to a target session
    /// when `session_id` is provided (flattened attach).
    async fn call(&self, method: &str, params: Value, session_id: Option<&str>) -> Result<Value> {
        let id = next_call_id();
        let (rtx, rrx) = oneshot::channel();
        self.pending.lock().await.insert(id, rtx);
        if self
            .tx
            .send(Outgoing {
                id,
                method: method.to_string(),
                params,
                session_id: session_id.map(str::to_string),
            })
            .await
            .is_err()
        {
            self.pending.lock().await.remove(&id);
            return Err(anyhow!("CDP writer task stopped"));
        }
        let resp = match tokio::time::timeout(CDP_RESPONSE_TIMEOUT, rrx).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&id);
                return Err(anyhow!("CDP response channel closed for {method}"));
            }
            Err(_) => {
                self.pending.lock().await.remove(&id);
                return Err(anyhow!(
                    "CDP {method} timed out after {CDP_RESPONSE_TIMEOUT:?}"
                ));
            }
        };
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("CDP {method} error: {err}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Find an `iframe`-type target whose URL host matches the iframe `src`'s
    /// host exactly, preferring the target whose URL most closely matches
    /// `src_url`. Falls back to the sole iframe target when the frame
    /// redirected off its src host (see [`find_iframe_target_in`]).
    pub async fn find_iframe_target(&self, src_url: &str) -> Result<Value> {
        let host = host_of(src_url)
            .ok_or_else(|| anyhow!("Iframe src '{src_url}' has no host to match"))?;
        let result = self.call("Target.getTargets", json!({}), None).await?;
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let found = find_iframe_target_in(&infos, host, src_url)
            .ok_or_else(|| anyhow!("No iframe target found for host '{host}'"))?;
        if let Some(url) = found.get("url").and_then(Value::as_str) {
            if host_of(url).map(|h| h.eq_ignore_ascii_case(host)) != Some(true) {
                log::warn!(
                    "[oopif] no host match for '{host}' — using sole iframe target (url '{url}'); \
                     the frame likely redirected off its src host"
                );
            }
        }
        Ok(found)
    }

    /// Find a `page`-type target whose URL contains `url_fragment`.
    pub async fn find_page_target(&self, url_fragment: &str) -> Result<Value> {
        let result = self.call("Target.getTargets", json!({}), None).await?;
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for info in infos {
            let ty = info.get("type").and_then(Value::as_str).unwrap_or("");
            let url = info.get("url").and_then(Value::as_str).unwrap_or("");
            if ty == "page" && url.contains(url_fragment) {
                return Ok(info);
            }
        }
        Err(anyhow!(
            "No page target found for url fragment '{url_fragment}'"
        ))
    }

    /// Bring a target to the foreground (`Target.activateTarget`).
    pub async fn activate_target(&self, target_id: &str) -> Result<()> {
        self.call(
            "Target.activateTarget",
            json!({ "targetId": target_id }),
            None,
        )
        .await?;
        Ok(())
    }

    /// Close a target (`Target.closeTarget`).
    pub async fn close_target(&self, target_id: &str) -> Result<()> {
        self.call("Target.closeTarget", json!({ "targetId": target_id }), None)
            .await?;
        Ok(())
    }

    /// Close all page-type targets except `main_target_id`, then activate `main_target_id`.
    pub async fn close_other_tabs(&self, main_target_id: &str) -> Result<usize> {
        let result = match self.call("Target.getTargets", json!({}), None).await {
            Ok(v) => v,
            Err(_) => return Ok(0),
        };
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut closed_count = 0;
        for info in infos {
            let target_id = info.get("targetId").and_then(Value::as_str).unwrap_or("");
            let ty = info.get("type").and_then(Value::as_str).unwrap_or("");
            let url = info.get("url").and_then(Value::as_str).unwrap_or("");

            if ty == "page" && !target_id.is_empty() && target_id != main_target_id {
                log::info!("[close_other_tabs] closing spawned tab {target_id} ({url})");
                if let Err(e) = self.close_target(target_id).await {
                    log::warn!("[close_other_tabs] failed to close target {target_id}: {e}");
                } else {
                    closed_count += 1;
                }
            }
        }

        let _ = self.activate_target(main_target_id).await;
        Ok(closed_count)
    }

    /// Attach to a target and return its flattened session id.
    pub async fn attach(&self, target_id: &str) -> Result<String> {
        let result = self
            .call(
                "Target.attachToTarget",
                json!({ "targetId": target_id, "flatten": true }),
                None,
            )
            .await?;
        let session_id = extract_session_id(&result)
            .ok_or_else(|| anyhow!("attachToTarget returned no sessionId"))?;
        // Ensure the session reports execution contexts so Runtime.evaluate lands
        // in the page's main world — without Runtime.enable, a freshly re-created
        // OOPIF can leave evaluate stuck in a blank default context.
        let _ = self
            .call("Runtime.enable", json!({}), Some(&session_id))
            .await;
        Ok(session_id)
    }

    /// Evaluate JS in the given session and return its JSON value.
    ///
    /// `awaitPromise` makes an expression that returns a Promise resolve
    /// before the value comes back — without it, async JS handed to
    /// `iframe_eval` would silently serialize to `{}` instead of its result.
    pub async fn evaluate(&self, session_id: &str, expression: &str) -> Result<Value> {
        let result = self
            .call(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
                Some(session_id),
            )
            .await?;
        if let Some(text) = extract_exception_text(&result) {
            return Err(anyhow!("JS exception in iframe: {text}"));
        }
        Ok(extract_result_value(&result))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_json_rpc, extract_exception_text, extract_result_value, extract_session_id,
        find_iframe_target_in, host_of, next_call_id, route_response, scheme_host, url_affinity,
    };
    use serde_json::{json, Value};
    use tokio::sync::oneshot;

    #[test]
    fn call_ids_increment() {
        let a = next_call_id();
        let b = next_call_id();
        assert!(b > a);
    }

    #[test]
    fn host_of_extracts_host() {
        assert_eq!(
            host_of("https://atfminers.asloni.online/miner/index.html?v=1#x"),
            Some("atfminers.asloni.online")
        );
        assert_eq!(
            host_of("https://web.telegram.org/k/"),
            Some("web.telegram.org")
        );
        assert_eq!(
            host_of("ws://127.0.0.1:9222/devtools"),
            Some("127.0.0.1:9222")
        );
        assert_eq!(host_of("not a url"), None);
    }

    #[test]
    fn route_response_delivers_to_pending() {
        let mut pending = std::collections::HashMap::new();
        let (tx, rx) = oneshot::channel();
        pending.insert(7, tx);
        let msg = json!({ "id": 7, "result": { "ok": true } });
        assert!(route_response(&mut pending, &msg));
        assert!(pending.is_empty());
        let received = rx.blocking_recv().expect("response delivered");
        assert_eq!(received["result"]["ok"], true);
    }

    #[test]
    fn route_response_ignores_unknown_or_event_messages() {
        let mut pending = std::collections::HashMap::new();
        assert!(!route_response(
            &mut pending,
            &json!({ "id": 999, "result": {} })
        ));
        assert!(!route_response(
            &mut pending,
            &json!({ "method": "Target.targetCreated" })
        ));
        assert!(pending.is_empty());
    }

    #[test]
    fn build_json_rpc_without_session() {
        let call = build_json_rpc(5, "Target.getTargets", &json!({}), None);
        assert_eq!(call["id"], 5);
        assert_eq!(call["method"], "Target.getTargets");
        assert_eq!(call["params"], json!({}));
        assert!(call.get("sessionId").is_none());
    }

    #[test]
    fn build_json_rpc_with_session() {
        let call = build_json_rpc(
            9,
            "Runtime.evaluate",
            &json!({"expression": "1"}),
            Some("sess-1"),
        );
        assert_eq!(call["sessionId"], "sess-1");
        assert_eq!(call["params"]["expression"], "1");
    }

    #[test]
    fn extract_result_value_nested_path() {
        let resp = json!({
            "result": {
                "type": "object",
                "value": { "x": 42.0 }
            }
        });
        assert_eq!(extract_result_value(&resp), json!({ "x": 42.0 }));
    }

    #[test]
    fn extract_result_value_missing_paths() {
        assert_eq!(extract_result_value(&json!({ "id": 1 })), Value::Null);
        assert_eq!(extract_result_value(&json!({ "result": {} })), Value::Null);
        assert_eq!(
            extract_result_value(&json!({ "result": { "type": "string" } })),
            Value::Null
        );
    }

    #[test]
    fn extract_exception_text_present_and_absent() {
        let with_exc = json!({
            "result": { "type": "object" },
            "exceptionDetails": { "text": "ReferenceError: x is not defined" }
        });
        assert_eq!(
            extract_exception_text(&with_exc),
            Some("ReferenceError: x is not defined".to_string())
        );
        assert_eq!(extract_exception_text(&json!({ "result": {} })), None);
        assert_eq!(extract_exception_text(&json!({})), None);
    }

    #[test]
    fn extract_session_id_present_and_absent() {
        assert_eq!(
            extract_session_id(&json!({ "sessionId": "ABC123" })),
            Some("ABC123".to_string())
        );
        assert_eq!(extract_session_id(&json!({})), None);
        assert_eq!(extract_session_id(&json!(null)), None);
    }

    #[test]
    fn find_iframe_target_exact_host_match() {
        let infos = vec![
            json!({ "type": "page", "url": "https://web.telegram.org/k/", "targetId": "p1" }),
            json!({ "type": "iframe", "url": "https://atfminers.asloni.online/miner/index.html", "targetId": "f1" }),
        ];
        let found = find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://atfminers.asloni.online/miner/index.html",
        )
        .expect("found");
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_ignores_wrong_type_or_host() {
        // Two iframe targets on wrong hosts — no host match and the sole-iframe
        // fallback is refused (guessing between frames is unsafe).
        let infos = vec![
            json!({ "type": "page", "url": "https://atfminers.asloni.online/x", "targetId": "p1" }),
            json!({ "type": "iframe", "url": "https://other.example.com/y", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://third.example.com/z", "targetId": "f2" }),
        ];
        assert!(find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://atfminers.asloni.online/x"
        )
        .is_none());
    }

    #[test]
    fn find_iframe_target_no_substring_matching() {
        // "asloni.online" must NOT host-match "atfminers.asloni.online".
        // With a second iframe present there is also no sole-iframe fallback,
        // so the result must be None.
        let infos = vec![
            json!({ "type": "iframe", "url": "https://atfminers.asloni.online/miner", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://cdn.example.com/blank", "targetId": "f2" }),
        ];
        assert!(
            find_iframe_target_in(&infos, "asloni.online", "https://asloni.online/x").is_none()
        );
    }

    #[test]
    fn find_iframe_target_sole_iframe_fallback_on_redirect() {
        // The mini-app redirected off its src host: no host match, but exactly
        // one iframe target exists — use it (logged by the caller).
        let infos = vec![
            json!({ "type": "page", "url": "https://web.telegram.org/k/", "targetId": "p1" }),
            json!({ "type": "iframe", "url": "https://cdn.redirected.host/app", "targetId": "f1" }),
        ];
        let found = find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://atfminers.asloni.online/miner",
        )
        .expect("sole iframe fallback");
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_prefers_closest_src_url_among_same_host() {
        // Same-host siblings: the target whose URL shares the longest prefix
        // with the iframe src wins, regardless of list order.
        let infos = vec![
            json!({ "type": "iframe", "url": "https://a.example.com/other", "targetId": "f-other" }),
            json!({ "type": "iframe", "url": "https://a.example.com/miner/index.html?v=1", "targetId": "f-real" }),
        ];
        let found = find_iframe_target_in(
            &infos,
            "a.example.com",
            "https://a.example.com/miner/index.html?v=1",
        )
        .expect("found");
        assert_eq!(found["targetId"], "f-real");
    }

    #[test]
    fn find_iframe_target_empty_list() {
        assert!(find_iframe_target_in(&[], "any-host", "https://any-host/x").is_none());
    }

    // ── Additional edge-case coverage ────────────────────────────────

    #[test]
    fn host_of_with_port() {
        assert_eq!(
            host_of("http://127.0.0.1:9222/json"),
            Some("127.0.0.1:9222")
        );
    }

    #[test]
    fn host_of_empty_string() {
        assert_eq!(host_of(""), None);
    }

    #[test]
    fn host_of_no_scheme() {
        assert_eq!(host_of("just-a-host"), None);
    }

    #[test]
    fn host_of_ws_scheme() {
        assert_eq!(
            host_of("ws://127.0.0.1:40325/devtools/browser/abc"),
            Some("127.0.0.1:40325")
        );
    }

    // ── scheme_host (canonical URL splitter, shared with frame.rs) ───

    #[test]
    fn scheme_host_extracts_scheme_and_host() {
        assert_eq!(
            scheme_host("https://atfminers.asloni.online/miner/index.html?v=1#x"),
            Some(("https", "atfminers.asloni.online"))
        );
        assert_eq!(
            scheme_host("https://web.telegram.org/k/"),
            Some(("https", "web.telegram.org"))
        );
        assert_eq!(scheme_host("not a url"), None);
    }

    #[test]
    fn scheme_host_edge_cases() {
        assert_eq!(
            scheme_host("http://127.0.0.1:8080/path"),
            Some(("http", "127.0.0.1:8080"))
        );
        assert_eq!(
            scheme_host("https://example.com/page?q=1#top"),
            Some(("https", "example.com"))
        );
        assert_eq!(
            scheme_host("ws://127.0.0.1:9222"),
            Some(("ws", "127.0.0.1:9222"))
        );
        assert_eq!(scheme_host(""), None);
        assert_eq!(scheme_host("example.com/path"), None);
        assert_eq!(
            scheme_host("wss://secure.example.com/ws"),
            Some(("wss", "secure.example.com"))
        );
    }

    #[test]
    fn route_response_event_without_id() {
        let mut pending = std::collections::HashMap::new();
        let event = json!({ "method": "Target.targetCreated", "params": {} });
        assert!(!route_response(&mut pending, &event));
        assert!(pending.is_empty());
    }

    #[test]
    fn route_response_multiple_pending_ids() {
        let mut pending = std::collections::HashMap::new();
        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        pending.insert(1, tx1);
        pending.insert(2, tx2);

        // Route id=2 first
        assert!(route_response(
            &mut pending,
            &json!({"id": 2, "result": {"b": true}})
        ));
        assert_eq!(pending.len(), 1);
        assert!(pending.contains_key(&1));

        // Route id=1
        assert!(route_response(
            &mut pending,
            &json!({"id": 1, "result": {"a": true}})
        ));
        assert!(pending.is_empty());

        assert_eq!(rx2.blocking_recv().unwrap()["result"]["b"], true);
        assert_eq!(rx1.blocking_recv().unwrap()["result"]["a"], true);
    }

    #[test]
    fn build_json_rpc_empty_method() {
        let call = build_json_rpc(1, "", &json!({}), None);
        assert_eq!(call["method"], "");
    }

    #[test]
    fn build_json_rpc_nested_params() {
        let params = json!({"expression": "document.title", "returnByValue": true});
        let call = build_json_rpc(42, "Runtime.evaluate", &params, Some("sess-99"));
        assert_eq!(call["id"], 42);
        assert_eq!(call["params"]["expression"], "document.title");
        assert_eq!(call["sessionId"], "sess-99");
    }

    #[test]
    fn extract_result_value_string() {
        let resp = json!({
            "result": { "type": "string", "value": "hello" }
        });
        assert_eq!(extract_result_value(&resp), json!("hello"));
    }

    #[test]
    fn extract_result_value_number() {
        let resp = json!({
            "result": { "type": "number", "value": 2.75 }
        });
        assert_eq!(extract_result_value(&resp), json!(2.75));
    }

    #[test]
    fn extract_result_value_boolean() {
        let resp = json!({
            "result": { "type": "boolean", "value": true }
        });
        assert_eq!(extract_result_value(&resp), json!(true));
    }

    #[test]
    fn extract_result_value_null_value() {
        let resp = json!({
            "result": { "type": "object", "value": null }
        });
        assert_eq!(extract_result_value(&resp), Value::Null);
    }

    #[test]
    fn extract_exception_text_only_text_field() {
        // exceptionDetails present but no text field
        let resp = json!({
            "exceptionDetails": { "exceptionId": 1 }
        });
        assert_eq!(extract_exception_text(&resp), None);
    }

    #[test]
    fn extract_session_id_empty_string() {
        // sessionId present but empty — should return Some("")
        let resp = json!({ "sessionId": "" });
        assert_eq!(extract_session_id(&resp), Some("".to_string()));
    }

    #[test]
    fn find_iframe_target_multiple_iframes_first_match() {
        let infos = vec![
            json!({ "type": "iframe", "url": "https://a.example.com/x", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://a.example.com/y", "targetId": "f2" }),
        ];
        let found =
            find_iframe_target_in(&infos, "a.example.com", "https://a.example.com/x").unwrap();
        // src affinity picks the matching URL
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_different_hosts() {
        let infos = vec![
            json!({ "type": "iframe", "url": "https://a.com/x", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://b.com/y", "targetId": "f2" }),
        ];
        assert_eq!(
            find_iframe_target_in(&infos, "b.com", "https://b.com/y").unwrap()["targetId"],
            "f2"
        );
        assert_eq!(
            find_iframe_target_in(&infos, "a.com", "https://a.com/x").unwrap()["targetId"],
            "f1"
        );
    }

    #[test]
    fn find_iframe_target_no_url_field() {
        let infos = vec![json!({ "type": "iframe", "targetId": "f1" })];
        assert!(find_iframe_target_in(&infos, "any.com", "https://any.com/x").is_none());
    }

    #[test]
    fn find_iframe_target_empty_url() {
        let infos = vec![json!({ "type": "iframe", "url": "", "targetId": "f1" })];
        assert!(find_iframe_target_in(&infos, "any.com", "https://any.com/x").is_none());
    }

    #[test]
    fn extract_result_value_array() {
        let resp = json!({
            "result": { "type": "object", "value": [1, 2, 3] }
        });
        assert_eq!(extract_result_value(&resp), json!([1, 2, 3]));
    }

    #[test]
    fn find_iframe_target_prefers_mini_app_page_over_blank_sibling() {
        // After a re-render there can be a blank iframe first in the list —
        // the real mini-app page (…/miner/index.html) must win.
        let infos = vec![
            json!({ "type": "iframe", "url": "about:blank", "targetId": "blank" }),
            json!({
                "type": "iframe",
                "url": "https://atfminers.asloni.online/miner/index.html?v=1",
                "targetId": "real"
            }),
        ];
        let found = find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://atfminers.asloni.online/miner/index.html?v=1",
        )
        .expect("found");
        assert_eq!(found["targetId"], "real");
    }

    #[test]
    fn find_iframe_target_falls_back_to_any_host_iframe() {
        // No affinity competition — the only host match wins.
        let infos = vec![json!({
            "type": "iframe",
            "url": "https://atfminers.asloni.online/other",
            "targetId": "f1"
        })];
        let found = find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://atfminers.asloni.online/other",
        )
        .expect("found");
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_case_insensitive_host() {
        let infos = vec![json!({
            "type": "iframe",
            "url": "https://ATFMiners.Asloni.Online/miner/index.html",
            "targetId": "f1"
        })];
        let found = find_iframe_target_in(
            &infos,
            "atfminers.asloni.online",
            "https://ATFMiners.Asloni.Online/miner/index.html",
        )
        .expect("found");
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn url_affinity_scores_common_prefix() {
        assert_eq!(url_affinity("https://a.com/x", "https://a.com/x"), 15);
        assert_eq!(url_affinity("https://a.com/y", "https://a.com/x"), 14);
        assert_eq!(url_affinity("", "https://a.com"), 0);
    }
}
