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
use tokio::sync::{mpsc, oneshot, Mutex};

/// Next JSON-RPC call id for this process.
fn next_call_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
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
                let mut call = json!({
                    "id": msg.id,
                    "method": msg.method,
                    "params": msg.params,
                });
                if let Some(sid) = &msg.session_id {
                    call["sessionId"] = json!(sid);
                }
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
                if let Some(id) = v.get("id").and_then(Value::as_u64) {
                    if let Some(tx) = pending_reader.lock().await.remove(&id) {
                        let _ = tx.send(v.clone());
                    }
                }
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
        let resp = rrx
            .await
            .map_err(|_| anyhow!("CDP response channel closed for {method}"))?;
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("CDP {method} error: {err}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Find an `iframe`-type target whose URL contains `host_hint`.
    pub async fn find_iframe_target(&self, host_hint: &str) -> Result<Value> {
        let result = self.call("Target.getTargets", json!({}), None).await?;
        let infos = result
            .get("targetInfos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for info in infos {
            let ty = info.get("type").and_then(Value::as_str).unwrap_or("");
            let url = info.get("url").and_then(Value::as_str).unwrap_or("");
            if ty == "iframe" && url.contains(host_hint) {
                return Ok(info);
            }
        }
        Err(anyhow!("No iframe target found for host '{host_hint}'"))
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
        result
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow!("attachToTarget returned no sessionId"))
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
        if let Some(exc) = result.get("exceptionDetails") {
            let text = exc.get("text").and_then(Value::as_str).unwrap_or("unknown");
            return Err(anyhow!("JS exception in iframe: {text}"));
        }
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::next_call_id;

    #[test]
    fn call_ids_increment() {
        let a = next_call_id();
        let b = next_call_id();
        assert!(b > a);
    }
}
