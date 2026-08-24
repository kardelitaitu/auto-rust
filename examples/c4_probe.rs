//! Temporary probe: OOPIF client attach + evaluate inside the mini-app.
use auto::config::load_config;
use auto::runtime::task_context::oopif::OopifClient;
use auto::session::connector::{BrowserConnector, ShardBrowserConnector};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = load_config()?;
    let connector = ShardBrowserConnector::new();
    let caps = connector.discover(&config).await?;
    let Some(cap) = caps.first() else {
        println!("no shard profile with CDP");
        return Ok(());
    };
    println!("browser ws: {}", cap.ws_url);

    let client = OopifClient::connect(&cap.ws_url).await?;
    let target = client.find_iframe_target("atfminers").await?;
    let target_id = target
        .get("targetId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let target_url = target
        .get("url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    println!(
        "iframe target: {target_id} url={}",
        &target_url[..target_url.len().min(80)]
    );

    let session_id = client.attach(target_id).await?;
    println!("attached sessionId: {session_id}");

    let js = r#"(() => JSON.stringify({
        hasTasks: !!document.querySelector("[onclick=\"switchTab('tasks')\"]"),
        hasMine: !!document.querySelector("[onclick=\"switchTab('home')\"]"),
        goCnt: document.querySelectorAll('[id^="btn-telegram_"]').length
    }))()"#;
    let value = client.evaluate(&session_id, js).await?;
    println!("INSIDE MINI-APP: {value}");

    Ok(())
}
