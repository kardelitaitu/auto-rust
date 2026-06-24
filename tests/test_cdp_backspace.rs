use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::input::{DispatchKeyEventParams, DispatchKeyEventType};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut browser, mut handler) =
        Browser::launch(BrowserConfig::builder().with_head().build()?).await?;
    let handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });

    let page = browser.new_page("about:blank").await?;
    let params = DispatchKeyEventParams {
        r#type: DispatchKeyEventType::RawKeyDown,
        modifiers: None,
        timestamp: None,
        text: None,
        unmodified_text: None,
        key_identifier: None,
        code: Some("Backspace".to_string()),
        key: Some("Backspace".to_string()),
        windows_virtual_key_code: Some(8),
        native_virtual_key_code: Some(8),
        auto_repeat: None,
        is_keypad: None,
        is_system_key: None,
        location: None,
        commands: None,
    };
    match timeout(Duration::from_secs(2), page.execute(params)).await {
        Ok(_res) => println!("Result: Success"),
        Err(e) => println!("Timeout: {:?}", e),
    }

    browser.close().await?;
    handle.await?;
    Ok(())
}
