use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "gemini", about = "General-purpose coding assistant")]
pub struct Args {
    #[arg(short = 'p', long = "prompt", help = "Task description")]
    pub prompt: String,

    #[arg(long, help = "Pipeline role (observer, strategist, coder, auditor)")]
    pub role: Option<String>,

    #[arg(long, help = "Run in sandbox mode")]
    pub dry_run: bool,
}
