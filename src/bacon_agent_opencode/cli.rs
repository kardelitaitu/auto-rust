use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "opencode", about = "Fast coding assistant (no spec gates)")]
pub struct Args {
    #[arg(short = 'p', long = "prompt", help = "Task description")]
    pub prompt: String,

    #[arg(long, help = "Pipeline role (observer, strategist, coder, auditor)")]
    pub role: Option<String>,

    #[arg(long, help = "Dry-run mode")]
    pub dry_run: bool,
}
