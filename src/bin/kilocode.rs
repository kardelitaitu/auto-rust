use auto::bacon_agent_kilocode::cli::{Cli, Command};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => {
            // Pure CLI tool - no LLM required
            auto::bacon_agent_kilocode::run(None, &args.text, args.role.as_deref(), args.dry_run)
                .await?;
            Ok(())
        }
    }
}
