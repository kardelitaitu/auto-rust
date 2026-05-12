use auto_rust::bacon_agent_ollama::cli::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = <Args as clap::Parser>::parse();
    auto_rust::bacon_agent_ollama::run(&args.prompt, args.role.as_deref()).await
}
