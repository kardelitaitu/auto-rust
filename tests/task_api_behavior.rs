use anyhow::Result;
use auto::config::{NativeClickCalibrationMode, NativeInteractionConfig};
use auto::metrics::{MetricsCollector, RUN_COUNTER_CLICK_FALLBACK_HIT};
use auto::result::{TaskErrorKind, TaskResult, TaskStatus};
use auto::runtime::task_context::{FocusStatus, TaskContext, WaitForVisibleStatus};
use auto::session::Session;
use auto::task::policy::{TaskPermissions, TaskPolicy, DEFAULT_TASK_POLICY};
use auto::utils::mouse::{
    clear_nativeclick_forced_calibration_for_tests, clear_nativeclick_trace_hooks,
    set_nativeclick_forced_calibration_for_tests, take_nativeclick_trace_hooks, ClickStatus,
    HoverStatus,
};
use chromiumoxide::Browser;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::instrument::WithSubscriber;
use tracing::Level;

struct TestServer {
    url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start(body: &'static str) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let response_body = body.to_string();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept = listener.accept() => {
                        let Ok((stream, _)) = accept else {
                            break;
                        };
                        let body = response_body.clone();
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, &body).await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            url: format!("http://127.0.0.1:{}/", addr.port()),
            shutdown_tx: Some(shutdown_tx),
            handle,
        })
    }

    fn url(&self) -> &str {
        &self.url
    }

    async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

async fn handle_connection(mut stream: TcpStream, body: &str) -> Result<()> {
    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf).await?;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn connect_test_session() -> Result<Option<Session>> {
    let Ok(ws_url) = env::var("TASK_API_TEST_WS") else {
        eprintln!("skipping task-api behavior tests: TASK_API_TEST_WS is not set");
        return Ok(None);
    };

    let (browser, handler) = Browser::connect(&ws_url).await?;
    let session = Session::new(
        "task-api-test".to_string(),
        "task-api-test".to_string(),
        "test".to_string(),
        browser,
        handler,
        1,
        0,
        None,
    );
    Ok(Some(session))
}

fn build_task_context(session: &Session, page: Arc<chromiumoxide::Page>) -> TaskContext {
    TaskContext::new(
        session.id.clone(),
        page,
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        &DEFAULT_TASK_POLICY,
        None,
    )
}

fn build_task_context_with_metrics(
    session: &Session,
    page: Arc<chromiumoxide::Page>,
    metrics: Arc<MetricsCollector>,
) -> TaskContext {
    TaskContext::new_with_metrics(
        session.id.clone(),
        page,
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        metrics,
        &DEFAULT_TASK_POLICY,
        None,
    )
}

fn build_task_context_with_policy(
    session: &Session,
    page: Arc<chromiumoxide::Page>,
    policy: &'static auto::task::policy::TaskPolicy,
) -> TaskContext {
    TaskContext::new(
        session.id.clone(),
        page,
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        policy,
        None,
    )
}

fn build_task_context_with_cancel_token(
    session: &Session,
    page: Arc<chromiumoxide::Page>,
    cancel_token: Option<CancellationToken>,
) -> TaskContext {
    TaskContext::new(
        session.id.clone(),
        page,
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        &DEFAULT_TASK_POLICY,
        cancel_token,
    )
}

#[cfg(feature = "accessibility-locator")]
#[derive(Clone, Default)]
struct SelectorTelemetryLogSink {
    bytes: Arc<Mutex<Vec<u8>>>,
}

#[cfg(feature = "accessibility-locator")]
struct SelectorTelemetryLogWriter {
    bytes: Arc<Mutex<Vec<u8>>>,
}

#[cfg(feature = "accessibility-locator")]
impl Write for SelectorTelemetryLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes
            .lock()
            .expect("selector telemetry log mutex poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "accessibility-locator")]
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SelectorTelemetryLogSink {
    type Writer = SelectorTelemetryLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SelectorTelemetryLogWriter {
            bytes: Arc::clone(&self.bytes),
        }
    }
}

#[cfg(feature = "accessibility-locator")]
fn selector_telemetry_logs(sink: &SelectorTelemetryLogSink) -> String {
    let bytes = sink
        .bytes
        .lock()
        .expect("selector telemetry log mutex poisoned")
        .clone();
    String::from_utf8(bytes).expect("selector telemetry logs must be utf-8")
}

async fn scroll_event_count(api: &TaskContext) -> Result<u64> {
    Ok(api
        .attr("body", "data-scroll-events")
        .await?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0))
}

const SCROLLING_PAGE: &str = r#"<!doctype html>
<html>
<head>
    <title>Scroll</title>
    <style>
        body {
            margin: 0;
            min-height: 5200px;
            font-family: sans-serif;
        }

        .spacer {
            height: 1600px;
        }

        .target {
            display: block;
            height: 88px;
            line-height: 88px;
            margin: 24px;
            background: #ddd;
        }

        #target-two {
            height: 120px;
            background: #cfc;
        }
    </style>
    <script>
        window.addEventListener('scroll', () => {
            const current = Number(document.body.dataset.scrollEvents || '0');
            document.body.dataset.scrollEvents = String(current + 1);
        });
    </script>
</head>
<body>
    <div id="top-marker" class="target">top</div>
    <div class="spacer"></div>
    <div id="target-one" class="target">target one</div>
    <div class="spacer" style="height: 1300px"></div>
    <div id="target-two" class="target">target two</div>
</body>
</html>"#;

fn sanitize_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed
    }
}

fn click_learning_path(profile_name: &str, session_id: &str) -> PathBuf {
    std::env::current_dir()
        .expect("current dir")
        .join("click-learning")
        .join(sanitize_component(profile_name))
        .join(format!("{}.json", sanitize_component(session_id)))
}

fn seed_click_learning(
    profile_name: &str,
    session_id: &str,
    selector: &str,
    attempts: u32,
    successes: u32,
    consecutive_failures: u32,
) -> Result<PathBuf> {
    let path = click_learning_path(profile_name, session_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::json!({
        "interaction_count": attempts,
        "total_attempts": attempts,
        "total_successes": successes,
        "recent_results": vec![false; attempts as usize],
        "selectors": {
            selector: {
                "attempts": attempts,
                "successes": successes,
                "consecutive_failures": consecutive_failures,
            }
        }
    });

    fs::write(&path, serde_json::to_string_pretty(&json)?)?;
    Ok(path)
}

#[tokio::test]
async fn navigate_loads_expected_content() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Task API</title></head><body><div id=\"loaded\">ready</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
    let api = build_task_context(&session, page.clone());

    api.navigate(server.url(), 10_000).await?;

    assert_eq!(api.title().await?, "Task API");
    assert!(api.exists("#loaded").await?);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn page_state_helpers_report_live_values() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>State</title></head><body><main id=\"root\">state</main></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    assert_eq!(api.url().await?, server.url());
    assert_eq!(api.title().await?, "State");

    let viewport = api.viewport().await?;
    assert!(viewport.width > 0.0);
    assert!(viewport.height > 0.0);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn session_export_import_roundtrip_restores_cookie_and_local_storage() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Session</title><script>localStorage.setItem('theme','dark'); document.cookie='session_token=abc123; path=/';</script></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_export_session: true,
            allow_import_session: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let exported = api.export_session(server.url()).await?;
    assert_eq!(exported.url, server.url());
    assert_eq!(
        exported.local_storage.get("theme").map(String::as_str),
        Some("dark")
    );
    assert!(exported
        .cookies
        .iter()
        .any(|cookie| cookie.get("name").and_then(|v| v.as_str()) == Some("session_token")));
    assert!(api.has_cookie("session_token", None).await?);

    page.evaluate(
        r#"(function() {
            localStorage.clear();
            document.cookie = 'session_token=; Max-Age=0; path=/';
            return true;
        })()"#,
    )
    .await?;

    let cleared = api.export_session(server.url()).await?;
    assert!(cleared.local_storage.is_empty());
    assert!(!cleared
        .cookies
        .iter()
        .any(|cookie| cookie.get("name").and_then(|v| v.as_str()) == Some("session_token")));

    api.import_session(&exported).await?;

    let restored = api.export_session(server.url()).await?;
    assert_eq!(
        restored.local_storage.get("theme").map(String::as_str),
        Some("dark")
    );
    assert!(restored
        .cookies
        .iter()
        .any(|cookie| cookie.get("name").and_then(|v| v.as_str()) == Some("session_token")));

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn browser_export_import_roundtrip_restores_storage_and_cookie() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Browser</title><script>localStorage.setItem('theme','dark'); sessionStorage.setItem('phase','one'); document.cookie='session_token=abc123; path=/';</script></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_browser_export: true,
            allow_browser_import: true,
            allow_import_cookies: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let exported = api.export_browser(server.url()).await?;
    assert!(exported
        .cookies
        .iter()
        .any(|cookie| cookie.get("name").and_then(|v| v.as_str()) == Some("session_token")));
    assert_eq!(
        exported
            .local_storage
            .get("127.0.0.1")
            .and_then(|origin| origin.get("theme").map(String::as_str)),
        Some("dark")
    );
    assert_eq!(
        exported
            .session_storage
            .get("127.0.0.1")
            .and_then(|origin| origin.get("phase").map(String::as_str)),
        Some("one")
    );

    page.evaluate(
        r#"(function() {
            localStorage.clear();
            sessionStorage.clear();
            document.cookie = 'session_token=; Max-Age=0; path=/';
            return true;
        })()"#,
    )
    .await?;

    let cleared = api.export_browser(server.url()).await?;
    assert!(cleared
        .cookies
        .iter()
        .all(|cookie| cookie.get("name").and_then(|v| v.as_str()) != Some("session_token")));
    assert!(cleared
        .local_storage
        .get("127.0.0.1")
        .map(|origin| origin.is_empty())
        .unwrap_or(true));
    assert!(cleared
        .session_storage
        .get("127.0.0.1")
        .map(|origin| origin.is_empty())
        .unwrap_or(true));

    api.import_browser(&exported).await?;

    let restored = api.export_browser(server.url()).await?;
    assert!(restored
        .cookies
        .iter()
        .any(|cookie| cookie.get("name").and_then(|v| v.as_str()) == Some("session_token")));
    assert_eq!(
        restored
            .local_storage
            .get("127.0.0.1")
            .and_then(|origin| origin.get("theme").map(String::as_str)),
        Some("dark")
    );
    assert_eq!(
        restored
            .session_storage
            .get("127.0.0.1")
            .and_then(|origin| origin.get("phase").map(String::as_str)),
        Some("one")
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn clipboard_session_state_roundtrip_uses_session_scope() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Clipboard</title></head><body><input id=\"field\" value=\"ready\" /></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_session_clipboard: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    api.write_clipboard("alpha")?;
    assert!(api.has_clipboard_content()?);
    assert_eq!(api.read_clipboard()?, "alpha");

    api.append_clipboard("beta", Some("|"))?;
    assert_eq!(api.read_clipboard()?, "alpha|beta");

    api.clear_clipboard()?;
    assert!(!api.has_clipboard_content()?);
    assert!(api.read_clipboard().is_err());

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn data_file_operations_roundtrip_and_cleanup() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Data</title></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_read_data: true,
            allow_write_data: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let unique = format!(
        "task-api-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let relative = format!("{}/sample.txt", unique);
    let data_root = std::path::Path::new("config").join(&unique);
    let file_path = data_root.join("sample.txt");

    api.write_data_file(&relative, b"hello")?;
    assert!(api.data_file_exists(&relative)?);
    assert_eq!(api.read_data_file(&relative)?, "hello");

    api.append_data_file(&relative, b" world")?;
    assert_eq!(api.read_data_file(&relative)?, "hello world");

    let metadata = api.data_file_metadata(&relative)?;
    assert!(metadata.size >= 11);

    let listed = api.list_data_files(Some(&unique))?;
    assert_eq!(listed, vec!["sample.txt".to_string()]);

    api.delete_data_file(&relative)?;
    assert!(!api.data_file_exists(&relative)?);

    let _ = fs::remove_file(&file_path);
    let _ = fs::remove_dir_all(&data_root);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn http_get_returns_status_body_and_headers() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Http</title></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_http_requests: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let response = api.http_get(server.url()).await?;
    assert_eq!(response.status, 200);
    assert!(response.body.contains("Http"));
    assert_eq!(
        response.headers.get("content-type").map(String::as_str),
        Some("text/html; charset=utf-8")
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn download_file_saves_response_bytes_and_cleans_up() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start("download payload").await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_http_requests: true,
            allow_write_data: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let unique = format!(
        "task-api-download-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let relative = format!("{}/payload.txt", unique);
    let file_path = std::path::Path::new("config").join(&relative);

    let size = api.download_file(server.url(), &relative).await?;
    assert_eq!(size, "download payload".len() as u64);
    assert!(file_path.exists());
    assert_eq!(std::fs::read_to_string(&file_path)?, "download payload");

    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir_all(std::path::Path::new("config").join(&unique));

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn dom_inspection_helpers_report_live_page_state() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>DOM</title><style>body{margin:0;} #target{color: rgb(12, 34, 56); width: 120px; height: 40px; } #hidden{display:none;}</style></head><body><div class=\"item\">one</div><div class=\"item\">two</div><div class=\"item\">three</div><div id=\"target\" data-role=\"hero\">Hello <span>world</span></div><input id=\"input\" value=\"typed value\" /><div id=\"hidden\">secret</div><div style=\"height:1800px\"></div><div id=\"offscreen\">below fold</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_dom_inspection: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    assert_eq!(
        api.get_computed_style("#target", "color").await?,
        "rgb(12, 34, 56)"
    );

    let rect = api.get_element_rect("#target").await?;
    assert!(rect.width > 0.0);
    assert!(rect.height > 0.0);

    assert_eq!(api.get_scroll_position().await?, (0, 0));
    page.evaluate("window.scrollTo(0, 180)").await?;
    let (scroll_x, scroll_y) = api.get_scroll_position().await?;
    assert_eq!(scroll_x, 0);
    assert!(scroll_y >= 180);

    assert_eq!(api.count_elements(".item").await?, 3);
    assert!(api.is_in_viewport("#target").await?);
    assert!(!api.is_in_viewport("#offscreen").await?);

    assert!(api.exists("#target").await?);
    assert!(!api.exists("#missing").await?);
    assert!(api.visible("#target").await?);
    assert!(!api.visible("#hidden").await?);

    assert_eq!(api.text("#target").await?, Some("Hello world".to_string()));
    assert_eq!(
        api.html("#target").await?,
        Some("Hello <span>world</span>".to_string())
    );
    assert_eq!(
        api.attr("#target", "data-role").await?,
        Some("hero".to_string())
    );
    assert_eq!(api.value("#input").await?, Some("typed value".to_string()));

    assert!(api.wait_for("#target", 500).await?);
    assert!(api.wait_for_visible("#target", 500).await?);
    assert!(!api.wait_for_visible("#hidden", 500).await?);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[cfg(feature = "accessibility-locator")]
#[tokio::test]
async fn browser_runtime_css_compatibility_matrix_under_feature_flag() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>CSS Compat</title><style>#hidden-css{display:none;}</style></head><body><div id=\"target\" data-role=\"hero\">Hello <span>world</span></div><input id=\"email\" aria-label=\"Email\" value=\"alice@example.com\" /><button id=\"save-btn\" aria-label=\"Save changes\">Save</button><button id=\"dup-a\" aria-label=\"Duplicate action\">One</button><button id=\"dup-b\" aria-label=\"Duplicate action\">Two</button><button id=\"hidden-css\" aria-label=\"Hidden action\">Hidden</button><div class=\"item\">one</div><div class=\"item\">two</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_dom_inspection: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    let matrix = [
        ("#target", true, true),
        ("#hidden-css", true, false),
        ("button[aria-label='Save changes']", true, true),
        ("[data-role='hero']", true, true),
        (".item", true, true),
    ];

    for (selector, exists_expected, visible_expected) in matrix {
        assert_eq!(
            api.exists(selector).await?,
            exists_expected,
            "exists mismatch for selector={selector}"
        );
        assert_eq!(
            api.visible(selector).await?,
            visible_expected,
            "visible mismatch for selector={selector}"
        );
    }

    assert_eq!(api.text("#target").await?, Some("Hello world".to_string()));
    assert_eq!(
        api.html("#target").await?,
        Some("Hello <span>world</span>".to_string())
    );
    assert_eq!(
        api.attr("#target", "data-role").await?,
        Some("hero".to_string())
    );
    assert_eq!(
        api.value("#email").await?,
        Some("alice@example.com".to_string())
    );

    assert!(api.wait_for("#target", 600).await?);
    assert!(api.wait_for_visible("#target", 600).await?);
    assert!(!api.wait_for_visible("#hidden-css", 600).await?);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[cfg(feature = "accessibility-locator")]
#[tokio::test]
async fn browser_runtime_accessibility_locator_integration_semantics() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>A11y Locator</title></head><body><button id=\"save-btn\" aria-label=\"Save changes\">Save</button><button id=\"dup-a\" aria-label=\"Duplicate action\">One</button><button id=\"dup-b\" aria-label=\"Duplicate action\">Two</button><input id=\"email\" aria-label=\"Email\" value=\"alice@example.com\" /></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let policy = Box::leak(Box::new(TaskPolicy {
        max_duration_ms: 30_000,
        permissions: TaskPermissions {
            allow_dom_inspection: true,
            ..Default::default()
        },
    }));
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    assert!(api.exists("role=button[name='Save changes']").await?);
    assert!(api.visible("role=button[name='Save changes']").await?);
    assert_eq!(
        api.text("role=button[name='Save changes']").await?,
        Some("Save changes".to_string())
    );
    assert_eq!(
        api.value("role=textbox[name='Email']").await?,
        Some("alice@example.com".to_string())
    );

    let not_found = api
        .exists("role=button[name='Does not exist']")
        .await
        .expect_err("expected locator_not_found");
    assert!(
        not_found.to_string().contains("locator_not_found"),
        "unexpected error: {}",
        not_found
    );

    let ambiguous = api
        .text("role=button[name='Duplicate action']")
        .await
        .expect_err("expected locator_ambiguous");
    assert!(
        ambiguous.to_string().contains("locator_ambiguous"),
        "unexpected error: {}",
        ambiguous
    );

    let parse_err = api
        .exists("role=button[name=\"Save changes\"]")
        .await
        .expect_err("expected locator_parse_error");
    assert!(
        parse_err.to_string().contains("locator_parse_error"),
        "unexpected error: {}",
        parse_err
    );

    let unsupported_html = api
        .html("role=button[name='Save changes']")
        .await
        .expect_err("expected locator_unsupported");
    assert!(
        unsupported_html.to_string().contains("locator_unsupported"),
        "unexpected error: {}",
        unsupported_html
    );

    let unsupported_attr = api
        .attr("role=button[name='Save changes']", "aria-label")
        .await
        .expect_err("expected locator_unsupported");
    assert!(
        unsupported_attr.to_string().contains("locator_unsupported"),
        "unexpected error: {}",
        unsupported_attr
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[cfg(feature = "accessibility-locator")]
#[tokio::test]
async fn browser_runtime_locator_action_paths_surface_errors_and_success() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>A11y Action Paths</title></head><body><button id=\"save-btn\" aria-label=\"Save changes\" ondblclick=\"document.body.setAttribute('data-dbl','yes')\" onmouseover=\"document.body.setAttribute('data-hover','yes')\" oncontextmenu=\"document.body.setAttribute('data-right','yes'); return false;\">Save</button><button id=\"dup-a\" aria-label=\"Duplicate action\">One</button><button id=\"dup-b\" aria-label=\"Duplicate action\">Two</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let clicked = api.click("role=button[name='Save changes']").await?;
    assert!(matches!(clicked.click, ClickStatus::Success));

    let hovered = api.hover("role=button[name='Save changes']").await?;
    assert!(matches!(hovered.hover, HoverStatus::Success));
    assert_eq!(
        api.attr("body", "data-hover").await?,
        Some("yes".to_string())
    );

    let right = api.right_click("role=button[name='Save changes']").await?;
    assert!(matches!(right.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-right").await?,
        Some("yes".to_string())
    );

    let doubled = api.double_click("role=button[name='Save changes']").await?;
    assert!(matches!(doubled.click, ClickStatus::Success));
    assert_eq!(api.attr("body", "data-dbl").await?, Some("yes".to_string()));

    let ambiguous = api
        .click("role=button[name='Duplicate action']")
        .await
        .expect_err("expected locator_ambiguous");
    assert!(
        ambiguous.to_string().contains("locator_ambiguous"),
        "unexpected error: {}",
        ambiguous
    );

    let bad_scope = api
        .click("role=button[name='Save changes'][scope='###bad']")
        .await
        .expect_err("expected locator_scope_invalid");
    assert!(
        bad_scope.to_string().contains("locator_scope_invalid"),
        "unexpected error: {}",
        bad_scope
    );

    let native_unsupported = api
        .nativeclick("role=button[name='Save changes']")
        .await
        .expect_err("expected locator_unsupported for nativeclick");
    assert!(
        native_unsupported
            .to_string()
            .contains("locator_unsupported"),
        "unexpected error: {}",
        native_unsupported
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[cfg(feature = "accessibility-locator")]
#[tokio::test]
async fn browser_runtime_locator_action_emits_selector_telemetry_fields() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>A11y Telemetry</title></head><body><button id=\"save-btn\" aria-label=\"Save changes\" onclick=\"document.body.setAttribute('data-clicked','yes')\">Save</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let sink = SelectorTelemetryLogSink::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_max_level(Level::DEBUG)
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .finish();

    let clicked = async { api.click("role=button[name='Save changes']").await }
        .with_subscriber(subscriber)
        .await?;
    assert!(matches!(clicked.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-clicked").await?,
        Some("yes".to_string())
    );

    let logs = selector_telemetry_logs(&sink);
    assert!(logs.contains("selector resolution"));
    assert!(logs.contains("selector_mode=a11y") || logs.contains("selector_mode=\"a11y\""));
    assert!(logs.contains("locator_result=ok") || logs.contains("locator_result=\"ok\""));
    assert!(logs.contains("locator_role=button") || logs.contains("locator_role=\"button\""));
    assert!(
        logs.contains("locator_match_mode=exact") || logs.contains("locator_match_mode=\"exact\"")
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn check_permission_respects_effective_policy() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Policy</title></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let policy = Box::leak(Box::new(auto::task::policy::TaskPolicy {
        max_duration_ms: 30_000,
        permissions: auto::task::policy::TaskPermissions {
            allow_screenshot: true,
            allow_session_clipboard: true,
            ..Default::default()
        },
    }));

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context_with_policy(&session, page.clone(), policy);

    assert!(api.check_permission("allow_screenshot").is_ok());
    assert!(api.check_permission("allow_session_clipboard").is_ok());
    assert!(api.check_permission("allow_export_cookies").is_err());
    assert!(api.check_permission("allow_unknown_permission").is_err());

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn check_page_connected_succeeds_on_live_page() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Connected</title></head><body><div id=\"ready\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.check_page_connected().await?;

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn wait_helpers_cover_visibility_and_load() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Wait</title></head><body><div id=\"hidden\" style=\"display:none\">no</div><div id=\"visible\">yes</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.wait_for_load(2_000).await?;
    assert!(
        api.wait_for_any_visible_selector(&["#hidden", "#visible"], 2_000)
            .await?
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn focus_reports_success_and_focus_event() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Focus</title></head><body><input id=\"target\" onfocus=\"document.body.setAttribute('data-focused', 'yes')\" /></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let outcome = api.focus("#target").await?;
    assert!(matches!(outcome.focus, FocusStatus::Success));
    assert!(outcome.x > 0.0);
    assert!(outcome.y > 0.0);
    assert_eq!(
        api.attr("body", "data-focused").await?,
        Some("yes".to_string())
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn pointer_helpers_cover_focus_hover_and_movement() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Pointer</title><style>body{margin:0;min-height:2000px;} #focus-target,#hover-target{display:block;margin:48px;width:220px;height:60px;line-height:60px;background:#ddd;} #hover-target{margin-top:32px;}</style></head><body onmousemove=\"document.body.setAttribute('data-moved','yes')\"><input id=\"focus-target\" value=\"focus\" onfocus=\"document.body.setAttribute('data-focused','yes')\" /><div id=\"hover-target\" onmouseover=\"document.body.setAttribute('data-hovered','yes')\">hover me</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let focus_outcome = api.focus("#focus-target").await?;
    assert!(matches!(focus_outcome.focus, FocusStatus::Success));
    assert_eq!(
        api.attr("body", "data-focused").await?,
        Some("yes".to_string())
    );

    let hover_outcome = api.hover("#hover-target").await?;
    assert!(matches!(hover_outcome.hover, HoverStatus::Success));
    assert_eq!(
        api.attr("body", "data-hovered").await?,
        Some("yes".to_string())
    );

    api.move_mouse_to(12.0, 12.0).await?;
    assert_eq!(
        api.attr("body", "data-moved").await?,
        Some("yes".to_string())
    );

    api.move_mouse_fast(24.0, 24.0).await?;
    api.sync_cursor_overlay().await?;
    assert!(api.exists("#__auto_rust_mouse_overlay").await?);

    let random = api.randomcursor().await?;
    let viewport = api.viewport().await?;
    assert!(random.x >= 0.0 && random.x <= viewport.width);
    assert!(random.y >= 0.0 && random.y <= viewport.height);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn keyboard_helpers_type_press_and_clear_text() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Keyboard</title></head><body onkeydown=\"document.body.setAttribute('data-last-keydown', event.key)\" onkeyup=\"document.body.setAttribute('data-last-keyup', event.key)\"><input id=\"first\" value=\"\" /><textarea id=\"second\"></textarea></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.focus("#first").await?;
    api.type_text("abc").await?;
    assert_eq!(api.value("#first").await?, Some("abc".to_string()));

    api.press("Enter").await?;
    assert_eq!(
        api.attr("body", "data-last-keydown").await?,
        Some("Enter".to_string())
    );
    assert_eq!(
        api.attr("body", "data-last-keyup").await?,
        Some("Enter".to_string())
    );

    api.press_with_modifiers("a", &["Control"]).await?;
    assert_eq!(
        api.attr("body", "data-last-keydown").await?,
        Some("a".to_string())
    );

    api.keyboard("#second", "hello").await?;
    assert_eq!(api.value("#second").await?, Some("hello".to_string()));

    api.type_into("#second", " world").await?;
    assert_eq!(api.value("#second").await?, Some("hello world".to_string()));

    api.select_all("#second").await?;
    let selection = page
        .evaluate(
            r#"(() => {
                const el = document.getElementById('second');
                return {
                    start: el.selectionStart,
                    end: el.selectionEnd,
                    length: el.value.length
                };
            })()"#,
        )
        .await?;
    let selection = selection.value().cloned().unwrap_or_default();
    assert_eq!(selection.get("start").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(
        selection.get("end").and_then(|v| v.as_u64()),
        Some("hello world".len() as u64)
    );

    api.clear("#second").await?;
    assert_eq!(api.value("#second").await?, Some(String::new()));

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn scrolling_helpers_cover_viewport_navigation() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(SCROLLING_PAGE).await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.scroll_to("#target-one").await?;
    assert!(api.is_in_viewport("#target-one").await?);
    let first_scroll = api.get_scroll_position().await?;
    assert!(first_scroll.1 > 0);

    api.scroll_into_view("#target-two").await?;
    assert!(api.is_in_viewport("#target-two").await?);
    let second_scroll = api.get_scroll_position().await?;
    assert!(second_scroll.1 >= first_scroll.1);

    page.evaluate("window.scrollTo(0, 1600)").await?;
    let before_back = api.get_scroll_position().await?;
    api.scroll_back(240).await?;
    let after_back = api.get_scroll_position().await?;
    assert!(after_back.1 < before_back.1);

    let before_top = scroll_event_count(&api).await?;
    api.scroll_to_top().await?;
    assert_eq!(api.get_scroll_position().await?, (0, 0));
    let after_top = scroll_event_count(&api).await?;
    assert!(after_top > before_top);

    let before_bottom = scroll_event_count(&api).await?;
    api.scroll_to_bottom().await?;
    let bottom_scroll = api.get_scroll_position().await?;
    assert!(bottom_scroll.1 > 0);
    let after_bottom = scroll_event_count(&api).await?;
    assert!(after_bottom > before_bottom);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn scrolling_reading_helpers_emit_scroll_activity() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(SCROLLING_PAGE).await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    page.evaluate("window.scrollTo(0, 1200)").await?;
    let before_random = scroll_event_count(&api).await?;
    api.random_scroll().await?;
    let after_random = scroll_event_count(&api).await?;
    assert!(after_random > before_random);

    page.evaluate("window.scrollTo(0, 1200)").await?;
    let before_read = scroll_event_count(&api).await?;
    api.scroll_read(2, 180, false, false).await?;
    let after_read = scroll_event_count(&api).await?;
    assert!(after_read > before_read);

    page.evaluate("window.scrollTo(0, 1200)").await?;
    let before_duration = scroll_event_count(&api).await?;
    api.scrollread(1_000).await?;
    let after_duration = scroll_event_count(&api).await?;
    assert!(after_duration > before_duration);

    page.evaluate("window.scrollTo(0, 200)").await?;
    let before_read_to = scroll_event_count(&api).await?;
    api.scroll_read_to("#target-two", 1, 180, false, false)
        .await?;
    let after_read_to = scroll_event_count(&api).await?;
    assert!(after_read_to > before_read_to);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn click_variants_trigger_selector_and_coordinate_events() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Clicks</title><style>body{margin:0;min-height:1200px;} button{position:absolute;width:120px;height:42px;} #click-at{left:20px;top:20px;} #left-click{left:170px;top:20px;} #left-fast{left:320px;top:20px;} #double{left:20px;top:100px;} #middle{left:170px;top:100px;} #right-at{left:320px;top:100px;} #right-fast{left:20px;top:180px;} #right-selector{left:170px;top:180px;}</style></head><body><button id=\"click-at\" onclick=\"document.body.setAttribute('data-click-at','yes')\">click at</button><button id=\"left-click\" onclick=\"document.body.setAttribute('data-left-click','yes')\">left click</button><button id=\"left-fast\" onclick=\"document.body.setAttribute('data-left-fast','yes')\">left fast</button><button id=\"double\" ondblclick=\"document.body.setAttribute('data-double','yes')\">double</button><button id=\"middle\" onauxclick=\"if (event.button===1) document.body.setAttribute('data-middle','yes')\">middle</button><button id=\"right-at\" oncontextmenu=\"event.preventDefault(); document.body.setAttribute('data-right-at','yes')\">right at</button><button id=\"right-fast\" oncontextmenu=\"event.preventDefault(); document.body.setAttribute('data-right-fast','yes')\">right fast</button><button id=\"right-selector\" oncontextmenu=\"event.preventDefault(); document.body.setAttribute('data-right-selector','yes')\">right selector</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.click_at(80.0, 41.0).await?;
    assert_eq!(
        api.attr("body", "data-click-at").await?,
        Some("yes".to_string())
    );

    api.left_click(230.0, 41.0).await?;
    assert_eq!(
        api.attr("body", "data-left-click").await?,
        Some("yes".to_string())
    );

    api.left_click_fast(380.0, 41.0).await?;
    assert_eq!(
        api.attr("body", "data-left-fast").await?,
        Some("yes".to_string())
    );

    let double = api.double_click("#double").await?;
    assert!(matches!(double.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-double").await?,
        Some("yes".to_string())
    );

    let middle = api.middle_click("#middle").await?;
    assert!(matches!(middle.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-middle").await?,
        Some("yes".to_string())
    );

    api.right_click_at(380.0, 121.0).await?;
    assert_eq!(
        api.attr("body", "data-right-at").await?,
        Some("yes".to_string())
    );

    api.right_click_fast(80.0, 201.0).await?;
    assert_eq!(
        api.attr("body", "data-right-fast").await?,
        Some("yes".to_string())
    );

    let right = api.right_click("#right-selector").await?;
    assert!(matches!(right.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-right-selector").await?,
        Some("yes".to_string())
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn drag_and_nativecursor_selector_report_expected_state() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Drag</title><style>body{margin:0;min-height:1200px;} #drag-source,#drop-target,#native-target{position:absolute;width:140px;height:56px;line-height:56px;text-align:center;} #drag-source{left:20px;top:20px;background:#cce; } #drop-target{left:220px;top:20px;background:#cec;} #native-target{left:20px;top:120px;background:#eee;}</style></head><body><div id=\"drag-source\" draggable=\"true\" ondragstart=\"event.dataTransfer.setData('text/plain','dragged')\">drag me</div><div id=\"drop-target\" ondragover=\"event.preventDefault()\" ondrop=\"event.preventDefault(); document.body.setAttribute('data-dropped', event.dataTransfer.getData('text/plain'))\">drop here</div><button id=\"native-target\">native target</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.drag("#drag-source", "#drop-target").await?;
    assert_eq!(
        api.attr("body", "data-dropped").await?,
        Some("dragged".to_string())
    );

    let native = api.nativecursor_selector("#native-target").await?;
    assert_eq!(native.target, "button#native-target");
    assert!(native.x > 0.0);
    assert!(native.y > 0.0);
    assert!(native.screen_x.is_some());
    assert!(native.screen_y.is_some());
    assert!(api.exists("#__auto_rust_mouse_overlay").await?);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn focus_returns_error_for_missing_selector() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Focus Missing</title></head><body><div id=\"present\">ok</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let result = api.focus("#missing").await;
    assert!(result.is_err());

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn click_and_wait_reports_visible_when_target_opens() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Click</title></head><body><button id=\"go\" onclick=\"document.getElementById('next').style.display='block'\">Go</button><div id=\"next\" style=\"display:none\">next</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let outcome = api.click_and_wait("#go", "#next", 5_000).await?;
    assert!(matches!(
        outcome.next_visible,
        WaitForVisibleStatus::Visible
    ));

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn click_and_wait_times_out_when_target_stays_hidden() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Click Timeout</title></head><body><button id=\"go\">Go</button><div id=\"next\" style=\"display:none\">next</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let outcome = api.click_and_wait("#go", "#next", 1_000).await?;
    assert!(matches!(
        outcome.next_visible,
        WaitForVisibleStatus::Timeout
    ));

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn cancelled_run_releases_page_for_reuse() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Cancel</title></head><body><div id=\"ready\">ready</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.wait_for_visible("#ready", 1_000).await?;
    let cancelled = TaskResult::cancelled(
        10,
        "Task cancelled during group shutdown".to_string(),
        TaskErrorKind::Timeout,
    );
    assert!(matches!(cancelled.status, TaskStatus::Cancelled));
    drop(api);
    session.release_page(page).await;

    let second_page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let second_api = build_task_context(&session, second_page.clone());
    assert_eq!(second_api.title().await?, "Cancel");
    drop(second_api);
    session.release_page(second_page).await;

    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn click_retries_and_fallback_when_target_detaches_mid_action() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Detach</title></head><body><button id=\"target\" onmouseover=\"this.remove(); document.body.setAttribute('data-hover', 'removed')\">Detach</button></body></html>",
    )
    .await?;

    let learning_path = seed_click_learning(
        &session.behavior_profile.name,
        &session.id,
        "#target",
        4,
        0,
        3,
    )?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let metrics = Arc::new(MetricsCollector::new(100));
    let api = build_task_context_with_metrics(&session, page.clone(), metrics.clone());

    let result = api.click("#target").await;
    assert!(result.is_err(), "detached target should fail cleanly");
    assert!(metrics.run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT) >= 1);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    let _ = fs::remove_file(&learning_path);
    if let Some(parent) = learning_path.parent() {
        let _ = fs::remove_dir(parent);
        if let Some(grand) = parent.parent() {
            let _ = fs::remove_dir(grand);
        }
    }

    Ok(())
}

#[tokio::test]
async fn click_timeout_storm_hits_fallback_without_panicking() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Storm</title></head><body><button id=\"target\" style=\"position:relative; left:0px; top:0px\">Storm</button><script>let d=1; setInterval(()=>{ const t=document.getElementById('target'); if (!t) return; t.style.left = (d*20)+'px'; d = d * -1; }, 30);</script></body></html>",
    )
    .await?;

    let learning_path = seed_click_learning(
        &session.behavior_profile.name,
        &session.id,
        "#target",
        6,
        0,
        5,
    )?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let metrics = Arc::new(MetricsCollector::new(100));
    let api = build_task_context_with_metrics(&session, page.clone(), metrics.clone());

    let result = api.click("#target").await;
    assert!(result.is_err(), "timeout storm should fail cleanly");
    assert!(metrics.run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT) >= 1);

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    let _ = fs::remove_file(&learning_path);
    if let Some(parent) = learning_path.parent() {
        let _ = fs::remove_dir(parent);
        if let Some(grand) = parent.parent() {
            let _ = fs::remove_dir(grand);
        }
    }

    Ok(())
}

#[tokio::test]
async fn nativeclick_pipeline_orders_scroll_before_dispatch() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Native Order</title></head><body style=\"height:2400px;\"><div style=\"height:1800px\"></div><button id=\"target\" onclick=\"document.body.setAttribute('data-native-clicked','yes')\">Click</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());
    clear_nativeclick_trace_hooks(&session.id);

    let outcome = api.nativeclick("#target").await?;
    assert!(matches!(outcome.click, ClickStatus::Success));
    assert_eq!(
        api.attr("body", "data-native-clicked").await?,
        Some("yes".to_string())
    );

    let phases = take_nativeclick_trace_hooks(&session.id);
    let scroll_idx = phases
        .iter()
        .position(|phase| phase == "scroll-into-view")
        .expect("scroll-into-view phase missing");
    let dispatch_idx = phases
        .iter()
        .position(|phase| phase == "dispatch")
        .expect("dispatch phase missing");
    assert!(
        scroll_idx < dispatch_idx,
        "expected scroll before dispatch, got phases={:?}",
        phases
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn nativeclick_verify_failure_returns_expected_error_class() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Native Verify Fail</title></head><body><button id=\"target\" onclick=\"this.style.display='none'; const o=document.createElement('div'); o.id='cover'; o.style.position='fixed'; o.style.left='0'; o.style.top='0'; o.style.width='100vw'; o.style.height='100vh'; o.style.zIndex='2147483647'; document.body.appendChild(o);\">Click</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    let err: anyhow::Error = api
        .nativeclick("#target")
        .await
        .expect_err("expected nativeclick verification failure");
    let msg = err.to_string();
    assert!(
        msg.contains("nativeclick verification failed"),
        "unexpected error class: {}",
        msg
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn nativeclick_mapping_failure_returns_expected_error_class() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Native Mapping Fail</title></head><body><button id=\"target\">Click</button></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    // Force invalid calibration input to assert mapping error class deterministically.
    set_nativeclick_forced_calibration_for_tests(
        &session.id,
        0.0,
        1.0,
        0.0,
        0.0,
        NativeClickCalibrationMode::Windows,
    );

    let err: anyhow::Error = api
        .nativeclick("#target")
        .await
        .expect_err("expected nativeclick mapping failure");
    let msg = err.to_string();
    assert!(
        msg.contains("nativeclick mapping failed"),
        "unexpected error class: {}",
        msg
    );

    clear_nativeclick_forced_calibration_for_tests(&session.id);
    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn screenshot_auto_saves_webp_with_correct_filename() -> Result<()> {
    let Some(session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Screenshot</title></head><body><div id=\"content\">Screenshot test</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;

    // Create custom policy with screenshot permission and leak it for static lifetime
    let policy = Box::leak(Box::new(auto::task::policy::TaskPolicy {
        max_duration_ms: 60_000,
        permissions: auto::task::policy::TaskPermissions {
            allow_screenshot: true,
            ..Default::default()
        },
    }));

    let api = TaskContext::new(
        session.id.clone(),
        page.clone(),
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        policy,
        None,
    );

    // Take screenshot
    let screenshot_path = api.screenshot().await?;

    // Verify file exists
    assert!(
        std::path::Path::new(&screenshot_path).exists(),
        "Screenshot file should exist"
    );

    // Verify file is in correct directory
    assert!(
        screenshot_path.starts_with("data/screenshot/"),
        "Screenshot should be in data/screenshot/"
    );

    // Verify filename format: yyyy-mm-dd-hh-mm-sessionid.webp
    assert!(
        screenshot_path.ends_with(".webp"),
        "Screenshot should be WebP"
    );
    assert!(
        screenshot_path.contains(&session.id),
        "Filename should contain session ID"
    );

    // Verify file is valid JPG by reading it
    let file_content = std::fs::read(&screenshot_path)?;
    assert!(
        !file_content.is_empty(),
        "Screenshot file should not be empty"
    );

    // WebP files start with RIFF ... WEBP
    assert_eq!(&file_content[0..4], b"RIFF");
    assert_eq!(&file_content[8..12], b"WEBP");

    // Cleanup
    let _ = std::fs::remove_file(&screenshot_path);
    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    drop(session);
    Ok(())
}

#[tokio::test]
async fn screenshot_permission_denied_without_allow_screenshot() -> Result<()> {
    let Some(session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Screenshot Deny</title></head><body><div id=\"content\">No permission</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;

    // Use DEFAULT_TASK_POLICY which has allow_screenshot = false
    let api = build_task_context(&session, page.clone());

    // Screenshot should fail with permission denied
    let err: anyhow::Error = api
        .screenshot()
        .await
        .expect_err("expected permission denied error");

    let msg = err.to_string();
    assert!(
        msg.contains("Permission denied") || msg.contains("allow_screenshot"),
        "error should mention permission denied: {}",
        msg
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    drop(session);
    Ok(())
}

#[tokio::test]
async fn screenshot_creates_directory_if_not_exists() -> Result<()> {
    let Some(session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Screenshot Dir</title></head><body><div id=\"content\">Directory test</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;

    // Remove directory if it exists
    let screenshot_dir = std::path::Path::new("data/screenshot");
    if screenshot_dir.exists() {
        let _ = std::fs::remove_dir_all(screenshot_dir);
    }

    // Create custom policy with screenshot permission and leak it for static lifetime
    let policy = Box::leak(Box::new(auto::task::policy::TaskPolicy {
        max_duration_ms: 60_000,
        permissions: auto::task::policy::TaskPermissions {
            allow_screenshot: true,
            ..Default::default()
        },
    }));

    let api = TaskContext::new(
        session.id.clone(),
        page.clone(),
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        policy,
        None,
    );

    // Take screenshot - should create directory
    let screenshot_path = api.screenshot().await?;

    // Verify directory was created
    assert!(
        screenshot_dir.exists(),
        "Screenshot directory should be created"
    );
    assert!(screenshot_dir.is_dir(), "Should be a directory");

    // Cleanup
    let _ = std::fs::remove_file(&screenshot_path);
    let _ = std::fs::remove_dir_all(screenshot_dir);
    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    drop(session);
    Ok(())
}

#[tokio::test]
async fn screenshot_with_quality_clamps_and_writes_webp() -> Result<()> {
    let Some(session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Screenshot Clamp</title></head><body><div id=\"content\">Clamp test</div></body></html>",
    )
    .await?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;

    let policy = Box::leak(Box::new(auto::task::policy::TaskPolicy {
        max_duration_ms: 60_000,
        permissions: auto::task::policy::TaskPermissions {
            allow_screenshot: true,
            ..Default::default()
        },
    }));

    let api = TaskContext::new(
        session.id.clone(),
        page.clone(),
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        policy,
        None,
    );

    let screenshot_path = api.screenshot_with_quality(0).await?;
    assert!(screenshot_path.ends_with(".webp"));
    assert!(std::path::Path::new(&screenshot_path).exists());

    let file_content = std::fs::read(&screenshot_path)?;
    assert_eq!(&file_content[0..4], b"RIFF");
    assert_eq!(&file_content[8..12], b"WEBP");

    let _ = std::fs::remove_file(&screenshot_path);
    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    drop(session);
    Ok(())
}

// ============================================================================
// Click Retry Cancellation and Error Propagation Tests
// ============================================================================

#[tokio::test]
async fn click_cancels_promptly_during_retry_loop() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    // Page with element that will cause click failures (rapidly moving)
    let server = TestServer::start(
        "<!doctype html><html><head><title>Cancel Test</title></head><body><button id=\"target\" style=\"position:relative; left:0px; top:0px\">Move</button><script>let d=1; setInterval(()=>{ const t=document.getElementById('target'); if (!t) return; t.style.left = (d*25)+'px'; d = d * -1; }, 25);</script></body></html>",
    )
    .await?;

    // Seed learning with high consecutive failures to force coordinate fallback
    let learning_path = seed_click_learning(
        &session.behavior_profile.name,
        &session.id,
        "#target",
        5,
        0,
        4,
    )?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;

    // Create cancellation token
    let cancel_token = CancellationToken::new();
    let api =
        build_task_context_with_cancel_token(&session, page.clone(), Some(cancel_token.clone()));

    // Start click operation
    let click_future = api.click("#target");

    // Cancel after brief delay (during first retry attempt)
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        cancel_token.cancel();
    });

    let start = Instant::now();
    let result = click_future.await;

    // Should complete quickly due to cancellation (not wait for full 12s timeout)
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 3000,
        "Click should cancel promptly, but took {}ms",
        elapsed.as_millis()
    );

    // Result should be an error (cancelled or click failure)
    assert!(result.is_err(), "Cancelled click should return error");

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    let _ = fs::remove_file(&learning_path);
    if let Some(parent) = learning_path.parent() {
        let _ = fs::remove_dir(parent);
        if let Some(grand) = parent.parent() {
            let _ = fs::remove_dir(grand);
        }
    }

    Ok(())
}

#[tokio::test]
async fn click_returns_last_error_after_all_attempts_fail() -> Result<()> {
    let Some(mut session): Option<Session> = connect_test_session().await? else {
        return Ok(());
    };

    // Page with element that will definitely fail all click attempts
    // Element removes itself on hover, causing consistent failures
    let server = TestServer::start(
        "<!doctype html><html><head><title>Error Propagation</title></head><body><button id=\"target\" onmouseover=\"this.remove(); document.body.setAttribute('data-error', 'removed')\">Remove</button></body></html>",
    )
    .await?;

    // Seed learning to force retry behavior
    let learning_path = seed_click_learning(
        &session.behavior_profile.name,
        &session.id,
        "#target",
        3,
        0,
        3,
    )?;

    let page: Arc<chromiumoxide::Page> = session.acquire_page_at(server.url()).await?;
    let metrics = Arc::new(MetricsCollector::new(100));
    let api = build_task_context_with_metrics(&session, page.clone(), metrics.clone());

    // Attempt click - all 3 attempts should fail, returning last error
    let start = Instant::now();
    let result = api.click("#target").await;
    let _elapsed = start.elapsed();

    // Should take at least 2 backoffs (attempt 1 succeeds quickly, attempts 2-3 have backoffs)
    // But we're primarily checking error propagation, not timing
    assert!(result.is_err(), "All attempts should fail");

    // Error message should indicate the final failure cause
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("removed") || err_msg.contains("detach") || err_msg.contains("element"),
        "Error should indicate element removal/detachment, got: {}",
        err_msg
    );

    // Should have attempted fallback after exhaustion
    assert!(
        metrics.run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT) >= 1,
        "Fallback should be triggered after retry exhaustion"
    );

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    let _ = fs::remove_file(&learning_path);
    if let Some(parent) = learning_path.parent() {
        let _ = fs::remove_dir(parent);
        if let Some(grand) = parent.parent() {
            let _ = fs::remove_dir(grand);
        }
    }

    Ok(())
}
