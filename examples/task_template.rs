use anyhow::Result;
use serde_json::Value;

use rust_orchestrator::prelude::*;

fn main() {
    println!("task_template is a starter example; copy the run() function into a task module.");
}

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<()> {
    let _ = payload;

    ctx.navigate_to("https://example.com", 30000).await?;
    ctx.pause(500).await;
    ctx.press("End").await?;
    ctx.type_text("hello").await?;
    ctx.random_scroll().await?;

    Ok(())
}
