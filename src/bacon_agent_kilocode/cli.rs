use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "kilocode", about = "Code quality inspector")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run code quality inspection
    Run(RunArgs),
}

#[derive(Debug, Parser)]
pub struct RunArgs {
    #[arg(help = "What to inspect (prompt for the LLM)")]
    pub text: String,

    #[arg(long, help = "Pipeline role (observer, strategist, coder, auditor)")]
    pub role: Option<String>,

    #[arg(long, help = "Dry-run mode")]
    pub dry_run: bool,
}
