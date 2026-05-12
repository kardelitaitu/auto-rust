use auto_rust::bacon_agent_pi::cli::Args;
use auto_rust::bacon_agent_pi::pipeline::Pipeline;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = Args::parse_or_exit();
    let pipeline = Pipeline::new(args)?;
    pipeline.run().await
}
