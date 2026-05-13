use auto::bacon_agent_pi::cli::RunArgs;
use auto::bacon_agent_pi::cli::{Cli, Command};
use auto::bacon_agent_pi::pipeline::Pipeline;
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

    match command {
        Some(Command::Test(args)) => auto::bacon_agent_pi::test_harness::run(&args).await,
        Some(Command::Run(run_args)) => {
            let args = RunArgs {
                prompt: prompt.or(run_args.prompt),
                stage: stage.or(run_args.stage),
                spec: spec.or(run_args.spec),
                fast: fast || run_args.fast,
                dry_run: dry_run || run_args.dry_run,
                auto: auto || run_args.auto,
                auto_apply: auto_apply || run_args.auto_apply,
                parallel: parallel || run_args.parallel,
            };
            let pipeline_dry_run = args.dry_run;
            let pipeline_auto = args.auto;

            let pipeline = Pipeline::new(args, pipeline_dry_run, pipeline_auto)?;
            pipeline.run().await
        }
        _ => {
            let args = RunArgs {
                prompt,
                stage,
                spec,
                fast,
                dry_run,
                auto,
                auto_apply,
                parallel,
            };
            let pipeline_dry_run = args.dry_run;
            let pipeline_auto = args.auto;

            let pipeline = Pipeline::new(args, pipeline_dry_run, pipeline_auto)?;
            pipeline.run().await
        }
    }
}
