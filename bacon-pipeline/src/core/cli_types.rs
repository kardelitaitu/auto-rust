//! Shared command-line interface types for the Bacon autonomous pipeline.
//!
//! These types define the CLI entry point for the `bacon` binary.
//! They are shared across all pipeline agents to avoid duplication.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "bacon", about = "Bacon autonomous pipeline")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short = 'p', long, help = "Prompt describing the task")]
    pub prompt: Option<String>,

    #[arg(
        long,
        help = "Resume from a specific stage (observer, strategist, coder, auditor)"
    )]
    pub stage: Option<String>,

    #[arg(long, help = "Target a specific spec number")]
    pub spec: Option<u32>,

    #[arg(
        long,
        help = "Force fast path (local LLM only, skip strategist + auditor)"
    )]
    pub fast: bool,

    #[arg(long, help = "Run in sandbox, don't touch real files")]
    pub dry_run: bool,

    #[arg(long, short = 'y', help = "Skip all interactive gates")]
    pub auto: bool,

    #[arg(long, help = "Apply verified Coder patches after full gating")]
    pub auto_apply: bool,

    #[arg(long, help = "Process independent specs in parallel")]
    pub parallel: bool,

    #[arg(long, help = "Override max retry attempts per stage (default: 4)")]
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the pipeline
    Run(RunArgs),
    /// Run bacon-pipeline test suite
    Test(TestArgs),
}

#[derive(Debug, Default, Parser)]
pub struct TestArgs {
    /// List available test targets
    #[arg(long)]
    pub list: bool,

    /// Run a specific test fixture (e.g. "clippy", "unit")
    #[arg(long)]
    pub fixture: Option<String>,
}

#[derive(Debug, Default, Parser)]
pub struct RunArgs {
    #[arg(short = 'p', long, help = "Prompt describing the task")]
    pub prompt: Option<String>,

    #[arg(
        long,
        help = "Resume from a specific stage (observer, strategist, coder, auditor)"
    )]
    pub stage: Option<String>,

    #[arg(long, help = "Target a specific spec number")]
    pub spec: Option<u32>,

    #[arg(
        long,
        help = "Force fast path (local LLM only, skip strategist + auditor)"
    )]
    pub fast: bool,

    #[arg(long, help = "Run in sandbox, don't touch real files")]
    pub dry_run: bool,

    #[arg(long, short = 'y', help = "Skip all interactive gates")]
    pub auto: bool,

    #[arg(long, help = "Apply verified Coder patches after full gating")]
    pub auto_apply: bool,

    #[arg(long, help = "Process independent specs in parallel")]
    pub parallel: bool,

    #[arg(long, help = "Override max retry attempts per stage (default: 4)")]
    pub max_attempts: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_top_level_auto_apply_flag() {
        let cli = Cli::try_parse_from(["bacon", "--auto-apply"]).expect("valid cli");
        assert!(cli.auto_apply);
    }

    #[test]
    fn parses_run_auto_apply_flag() {
        let cli = Cli::try_parse_from(["bacon", "run", "--auto-apply"]).expect("valid cli");
        let Some(Command::Run(args)) = cli.command else {
            panic!("expected run command");
        };
        assert!(args.auto_apply);
    }

    #[test]
    fn parses_top_level_max_attempts_flag() {
        let cli = Cli::try_parse_from(["bacon", "--max-attempts", "8"]).expect("valid cli");
        assert_eq!(cli.max_attempts, Some(8));
    }

    #[test]
    fn parses_run_max_attempts_flag() {
        let cli = Cli::try_parse_from(["bacon", "run", "--max-attempts", "3"]).expect("valid cli");
        let Some(Command::Run(args)) = cli.command else {
            panic!("expected run command");
        };
        assert_eq!(args.max_attempts, Some(3));
    }

    #[test]
    fn max_attempts_omitted_is_none() {
        let cli = Cli::try_parse_from(["bacon"]).expect("valid cli");
        assert_eq!(cli.max_attempts, None);
    }

    #[test]
    fn parses_test_subcommand() {
        let cli = Cli::try_parse_from(["bacon", "test"]).expect("valid cli");
        assert!(cli.command.is_some());
        assert!(matches!(cli.command.unwrap(), Command::Test(_)));
    }

    #[test]
    fn parses_test_list_flag() {
        let cli = Cli::try_parse_from(["bacon", "test", "--list"]).expect("valid cli");
        let Some(Command::Test(args)) = cli.command else {
            panic!("expected test command");
        };
        assert!(args.list);
    }

    #[test]
    fn parses_test_fixture_flag() {
        let cli =
            Cli::try_parse_from(["bacon", "test", "--fixture", "clippy"]).expect("valid cli");
        let Some(Command::Test(args)) = cli.command else {
            panic!("expected test command");
        };
        assert_eq!(args.fixture.as_deref(), Some("clippy"));
    }
}
