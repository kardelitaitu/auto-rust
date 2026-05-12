use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "ollama",
    about = "Bacon-local Ollama agent (bypasses global LLM config)"
)]
pub struct Args {
    #[arg(short = 'p', long = "prompt", help = "Task description")]
    pub prompt: String,

    #[arg(long, help = "Pipeline role (observer, strategist, coder, auditor)")]
    pub role: Option<String>,

    #[arg(long, help = "Dry-run mode")]
    pub dry_run: bool,
}
