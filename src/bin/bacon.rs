#![deny(warnings)]

use auto::bacon_agent_nvidia::pipeline::Pipeline;
use auto::bacon_core::cli_types::{Cli, Command, RunArgs, TestArgs};
use clap::Parser;
use std::process::Command as ProcessCommand;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize bacon-pipeline configuration
    bacon_pipeline::config::init(bacon_pipeline::ProjectConfig::with_defaults(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ));

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
        max_attempts,
        ci,
    } = Cli::parse();

    // Handle test subcommand
    if let Some(Command::Test(test_args)) = &command {
        return run_tests(test_args);
    }

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
            max_attempts: max_attempts.or(run_args.max_attempts),
            ci: ci || run_args.ci,
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
            max_attempts,
            ci,
        },
    };

    let pipeline_dry_run = run_args.dry_run;
    let pipeline_auto = run_args.auto;

    if run_args.ci {
        bacon_pipeline::core::run_log::set_ci_mode(true);
    }

    let pipeline = Pipeline::new(run_args, pipeline_dry_run, pipeline_auto)?;
    pipeline.run().await
}

/// Run the bacon-pipeline test suite.
fn run_tests(args: &TestArgs) -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Handle --fixture clippy as a special case
    if let Some(ref fixture) = args.fixture {
        if fixture.to_lowercase() == "clippy" {
            eprintln!("Running clippy on bacon-pipeline...");
            let mut clippy = ProcessCommand::new("cargo");
            clippy.current_dir(&root);
            clippy.args(["clippy", "-p", "bacon-pipeline"]);
            let status = clippy.status()?;
            if !status.success() {
                anyhow::bail!("clippy found issues (exit code: {:?})", status.code());
            }
            return Ok(());
        }
    }

    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&root);
    cmd.arg("test");
    cmd.arg("-p");
    cmd.arg("bacon-pipeline");

    if args.list {
        cmd.arg("--list");
    }

    // Pass --fixture as a test name filter (e.g., "bacon test --fixture unit")
    if let Some(ref fixture) = args.fixture {
        if fixture.to_lowercase() != "unit" {
            cmd.arg("--");
            cmd.arg(fixture);
        }
    }

    // Stream output directly to the user's terminal
    eprintln!("Running bacon-pipeline tests...");
    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!(
            "bacon-pipeline tests failed (exit code: {:?})",
            status.code()
        );
    }
    Ok(())
}
