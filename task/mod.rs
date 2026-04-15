use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;

pub mod cookiebot;
pub mod pageview;

use crate::utils::{block_images, block_media};

pub async fn perform_task(page: &Page, session_id: &str, name: &str, payload: Value) -> Result<()> {
    let clean_name = name.strip_suffix(".js").unwrap_or(name);

    block_images(page).await?;
    block_media(page).await?;

    match clean_name {
        "cookiebot" => cookiebot::run(session_id, page, payload).await,
        "pageview" => pageview::run(session_id, page, payload).await,
        _ => Err(anyhow::anyhow!("Unknown task: {}. Add it to task/mod.rs", name)),
    }
}