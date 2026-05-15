use auto::bacon_agent_nvidia::pipeline::Pipeline;
use auto::bacon_core::cli_types::{Cli, Command, RunArgs};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let Cli {
        command,
        prompt,
        stage,
        spec,
        fast,
        dry_run,
        auto,
        auto_apply,
        parallel,
    } = Cli::parse();

    let run_args = match command {
        Some(Command::Run(run_args)) => RunArgs {
            prompt: prompt.or(run_args.prompt),
            stage: stage.or(run_args.stage),
            spec: spec.or(run_args.spec),
            fast: fast || run_args.fast,
            dry_run: dry_run || run_args.dry_run,
            auto: auto || run_args.auto,
            auto_apply: auto_apply || run_args.auto_apply,
            parallel: parallel || run_args.parallel,
        },
        _ => RunArgs {
            prompt,
            stage,
            spec,
            fast,
            dry_run,
            auto,
            auto_apply,
            parallel,
        },
    };

    let pipeline_dry_run = run_args.dry_run;
    let pipeline_auto = run_args.auto;

    let pipeline = Pipeline::new(run_args, pipeline_dry_run, pipeline_auto)?;
    pipeline.run().await
}
