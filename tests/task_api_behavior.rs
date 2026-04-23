use anyhow::Result;
use chromiumoxide::Browser;
use rust_orchestrator::metrics::{MetricsCollector, RUN_COUNTER_CLICK_FALLBACK_HIT};
use rust_orchestrator::runtime::task_context::{FocusStatus, TaskContext, WaitForVisibleStatus};
use rust_orchestrator::session::Session;
use rust_orchestrator::{result::TaskErrorKind, result::TaskResult};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

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
        metrics,
    )
}

fn sanitize_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
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
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Task API</title></head><body><div id=\"loaded\">ready</div></body></html>",
    )
    .await?;

    let page = session.acquire_page().await?;
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
async fn focus_reports_success_and_focus_event() -> Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Focus</title></head><body><input id=\"target\" onfocus=\"document.body.setAttribute('data-focused', 'yes')\" /></body></html>",
    )
    .await?;

    let page = session.acquire_page_at(server.url()).await?;
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
async fn focus_returns_error_for_missing_selector() -> Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Focus Missing</title></head><body><div id=\"present\">ok</div></body></html>",
    )
    .await?;

    let page = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    assert!(api.focus("#missing").await.is_err());

    drop(api);
    session.release_page(page).await;
    server.shutdown().await;
    session.graceful_shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn click_and_wait_reports_visible_when_target_opens() -> Result<()> {
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Click</title></head><body><button id=\"go\" onclick=\"document.getElementById('next').style.display='block'\">Go</button><div id=\"next\" style=\"display:none\">next</div></body></html>",
    )
    .await?;

    let page = session.acquire_page_at(server.url()).await?;
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
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Click Timeout</title></head><body><button id=\"go\">Go</button><div id=\"next\" style=\"display:none\">next</div></body></html>",
    )
    .await?;

    let page = session.acquire_page_at(server.url()).await?;
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
    let Some(mut session) = connect_test_session().await? else {
        return Ok(());
    };

    let server = TestServer::start(
        "<!doctype html><html><head><title>Cancel</title></head><body><div id=\"ready\">ready</div></body></html>",
    )
    .await?;

    let page = session.acquire_page_at(server.url()).await?;
    let api = build_task_context(&session, page.clone());

    api.wait_for_visible("#ready", 1_000).await?;
    let cancelled = TaskResult::cancelled(
        10,
        "Task cancelled during group shutdown".to_string(),
        TaskErrorKind::Timeout,
    );
    assert!(matches!(
        cancelled.status,
        rust_orchestrator::result::TaskStatus::Cancelled
    ));
    drop(api);
    session.release_page(page).await;

    let second_page = session.acquire_page_at(server.url()).await?;
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
    let Some(mut session) = connect_test_session().await? else {
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

    let page = session.acquire_page_at(server.url()).await?;
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
    let Some(mut session) = connect_test_session().await? else {
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

    let page = session.acquire_page_at(server.url()).await?;
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
