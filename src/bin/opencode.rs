use auto::bacon_agent_opencode::cli::Args;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = Args::parse();
    // Pure CLI tool - no LLM required
    auto::bacon_agent_opencode::run(None, &args.prompt, args.role.as_deref(), args.dry_run).await?;
    Ok(())
}
