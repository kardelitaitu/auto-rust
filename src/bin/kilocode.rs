use auto_rust::bacon_agent_kilocode::cli::{Cli, Command};
use auto_rust::llm::Llm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => {
            let llm = Llm::new()?;
            auto_rust::bacon_agent_kilocode::run(
                &llm,
                &args.text,
                args.role.as_deref(),
                args.path.as_deref(),
            )
            .await
        }
    }
}
