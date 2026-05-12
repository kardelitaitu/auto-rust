use auto_rust::bacon_agent_opencode::cli::Args;
use auto_rust::llm::Llm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = <Args as clap::Parser>::parse();
    let llm = Llm::new()?;
    auto_rust::bacon_agent_opencode::run(&llm, &args.prompt, args.role.as_deref()).await
}
