use anyhow::Result;
use chromiumoxide::Browser;
use rust_orchestrator::runtime::task_context::{FocusStatus, TaskContext, WaitForVisibleStatus};
use rust_orchestrator::session::Session;
use rust_orchestrator::{result::TaskErrorKind, result::TaskResult};
use std::env;
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
