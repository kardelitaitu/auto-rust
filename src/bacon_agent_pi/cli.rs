use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "pi", about = "Bacon autonomous pipeline")]
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

    #[arg(long, help = "Process independent specs in parallel")]
    pub parallel: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the pipeline
    Run(RunArgs),
    /// Run test harness against throwaway fixtures
    Test(TestArgs),
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

    #[arg(long, help = "Process independent specs in parallel")]
    pub parallel: bool,
}

#[derive(Debug, Parser)]
pub struct TestArgs {
    #[arg(long, help = "Run a specific fixture by name")]
    pub fixture: Option<String>,

    #[arg(long, help = "List available fixtures")]
    pub list: bool,
}
