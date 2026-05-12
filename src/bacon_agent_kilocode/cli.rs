use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "kilocode", about = "Code quality inspector")]
pub struct Args {
    #[arg(short = 'p', long = "prompt", help = "What to inspect")]
    pub prompt: String,

    #[arg(long, help = "Pipeline role (observer, strategist, coder, auditor)")]
    pub role: Option<String>,

    #[arg(long, help = "Dry-run mode")]
    pub dry_run: bool,
}
