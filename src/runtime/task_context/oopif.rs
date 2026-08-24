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

/// Next JSON-RPC call id for this process.
fn next_call_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Extract the host from a URL.
fn host_of(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    Some(rest.split(['/', '?', '#']).next().unwrap_or(rest))
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

/// Extract the `result.result.value` chain from a CDP response, or Null.
fn extract_result_value(resp: &Value) -> Value {
    resp.get("result")
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("value"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Extract `result.exceptionDetails.text` from a CDP evaluate response, if present.
fn extract_exception_text(resp: &Value) -> Option<String> {
    resp.get("result")
        .and_then(|r| r.get("exceptionDetails"))
        .and_then(|e| e.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Extract the `result.sessionId` from an attach response.
fn extract_session_id(resp: &Value) -> Option<String> {
    resp.get("result")
        .and_then(|r| r.get("sessionId"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Find an iframe target whose URL host matches `host` exactly from a target list.
fn find_iframe_target_in(infos: &[Value], host: &str) -> Option<Value> {
    for info in infos {
        let ty = info.get("type").and_then(Value::as_str).unwrap_or("");
        let url = info.get("url").and_then(Value::as_str).unwrap_or("");
        if ty == "iframe" && host_of(url) == Some(host) {
            return Some(info.clone());
        }
    }
    None
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
        let (tx, mut rx) = mpsc::channel::<Outgoing>(64);
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
            while let Some(Ok(msg)) = stream.next().await {
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
        self.tx
            .send(Outgoing {
                id,
                method: method.to_string(),
                params,
                session_id: session_id.map(str::to_string),
            })
            .await
            .map_err(|_| anyhow!("CDP writer task stopped"))?;
        let resp = match tokio::time::timeout(CDP_RESPONSE_TIMEOUT, rrx).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&id);
                return Err(anyhow!("CDP response channel closed for {method}"));
            }
            Err(_) => {
                self.pending.lock().await.remove(&id);
                return Err(anyhow!("CDP {method} timed out after 10s"));
            }
        };
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("CDP {method} error: {err}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Find an `iframe`-type target whose URL host matches `host` exactly.
    pub async fn find_iframe_target(&self, host: &str) -> Result<Value> {
        let result = self.call("Target.getTargets", json!({}), None).await?;
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        find_iframe_target_in(&infos, host)
            .ok_or_else(|| anyhow!("No iframe target found for host '{host}'"))
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
        extract_session_id(&result).ok_or_else(|| anyhow!("attachToTarget returned no sessionId"))
    }

    /// Evaluate JS in the given session and return its JSON value.
    pub async fn evaluate(&self, session_id: &str, expression: &str) -> Result<Value> {
        let result = self
            .call(
                "Runtime.evaluate",
                json!({ "expression": expression, "returnByValue": true }),
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
        find_iframe_target_in, host_of, next_call_id, route_response,
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
            "id": 1,
            "result": {
                "result": {
                    "type": "object",
                    "value": { "x": 42.0 }
                }
            }
        });
        assert_eq!(extract_result_value(&resp), json!({ "x": 42.0 }));
    }

    #[test]
    fn extract_result_value_missing_paths() {
        assert_eq!(extract_result_value(&json!({ "id": 1 })), Value::Null);
        assert_eq!(extract_result_value(&json!({ "result": {} })), Value::Null);
        assert_eq!(
            extract_result_value(&json!({ "result": { "result": { "type": "string" } } })),
            Value::Null
        );
    }

    #[test]
    fn extract_exception_text_present_and_absent() {
        let with_exc = json!({
            "result": {
                "result": { "type": "object" },
                "exceptionDetails": { "text": "ReferenceError: x is not defined" }
            }
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
            extract_session_id(&json!({ "result": { "sessionId": "ABC123" } })),
            Some("ABC123".to_string())
        );
        assert_eq!(extract_session_id(&json!({ "result": {} })), None);
        assert_eq!(extract_session_id(&json!({})), None);
    }

    #[test]
    fn find_iframe_target_exact_host_match() {
        let infos = vec![
            json!({ "type": "page", "url": "https://web.telegram.org/k/", "targetId": "p1" }),
            json!({ "type": "iframe", "url": "https://atfminers.asloni.online/miner/index.html", "targetId": "f1" }),
        ];
        let found = find_iframe_target_in(&infos, "atfminers.asloni.online").expect("found");
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_ignores_wrong_type_or_host() {
        let infos = vec![
            json!({ "type": "page", "url": "https://atfminers.asloni.online/x", "targetId": "p1" }),
            json!({ "type": "iframe", "url": "https://other.example.com/y", "targetId": "f1" }),
        ];
        assert!(find_iframe_target_in(&infos, "atfminers.asloni.online").is_none());
    }

    #[test]
    fn find_iframe_target_no_substring_matching() {
        // "asloni.online" must NOT match "atfminers.asloni.online" — exact host only.
        let infos = vec![json!({
            "type": "iframe",
            "url": "https://atfminers.asloni.online/miner",
            "targetId": "f1"
        })];
        assert!(find_iframe_target_in(&infos, "asloni.online").is_none());
    }

    #[test]
    fn find_iframe_target_empty_list() {
        assert!(find_iframe_target_in(&[], "any-host").is_none());
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
            "result": {
                "result": { "type": "string", "value": "hello" }
            }
        });
        assert_eq!(extract_result_value(&resp), json!("hello"));
    }

    #[test]
    fn extract_result_value_number() {
        let resp = json!({
            "result": {
                "result": { "type": "number", "value": 2.75 }
            }
        });
        assert_eq!(extract_result_value(&resp), json!(2.75));
    }

    #[test]
    fn extract_result_value_boolean() {
        let resp = json!({
            "result": {
                "result": { "type": "boolean", "value": true }
            }
        });
        assert_eq!(extract_result_value(&resp), json!(true));
    }

    #[test]
    fn extract_result_value_null_value() {
        let resp = json!({
            "result": {
                "result": { "type": "object", "value": null }
            }
        });
        assert_eq!(extract_result_value(&resp), Value::Null);
    }

    #[test]
    fn extract_exception_text_only_text_field() {
        // exceptionDetails present but no text field
        let resp = json!({
            "result": {
                "exceptionDetails": { "exceptionId": 1 }
            }
        });
        assert_eq!(extract_exception_text(&resp), None);
    }

    #[test]
    fn extract_session_id_empty_string() {
        // sessionId present but empty — should return Some("")
        let resp = json!({ "result": { "sessionId": "" } });
        assert_eq!(extract_session_id(&resp), Some("".to_string()));
    }

    #[test]
    fn find_iframe_target_multiple_iframes_first_match() {
        let infos = vec![
            json!({ "type": "iframe", "url": "https://a.example.com/x", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://a.example.com/y", "targetId": "f2" }),
        ];
        let found = find_iframe_target_in(&infos, "a.example.com").unwrap();
        // Should return the first match
        assert_eq!(found["targetId"], "f1");
    }

    #[test]
    fn find_iframe_target_different_hosts() {
        let infos = vec![
            json!({ "type": "iframe", "url": "https://a.com/x", "targetId": "f1" }),
            json!({ "type": "iframe", "url": "https://b.com/y", "targetId": "f2" }),
        ];
        assert_eq!(
            find_iframe_target_in(&infos, "b.com").unwrap()["targetId"],
            "f2"
        );
        assert_eq!(
            find_iframe_target_in(&infos, "a.com").unwrap()["targetId"],
            "f1"
        );
    }

    #[test]
    fn find_iframe_target_no_url_field() {
        let infos = vec![json!({ "type": "iframe", "targetId": "f1" })];
        assert!(find_iframe_target_in(&infos, "any.com").is_none());
    }

    #[test]
    fn find_iframe_target_empty_url() {
        let infos = vec![json!({ "type": "iframe", "url": "", "targetId": "f1" })];
        assert!(find_iframe_target_in(&infos, "any.com").is_none());
    }

    #[test]
    fn extract_result_value_array() {
        let resp = json!({
            "result": {
                "result": { "type": "object", "value": [1, 2, 3] }
            }
        });
        assert_eq!(extract_result_value(&resp), json!([1, 2, 3]));
    }
}
