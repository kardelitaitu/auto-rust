use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "codex", about = "Codebase Q&A and documentation assistant")]
pub struct Args {
    #[arg(short = 'p', long, help = "Question about the codebase")]
    pub prompt: String,

    #[arg(
        long,
        help = "Pipeline role (observer, strategist, coder, auditor) — uses role prompt instead of default"
    )]
    pub role: Option<String>,

    #[arg(long, help = "Run in sandbox mode")]
    pub dry_run: bool,
}
