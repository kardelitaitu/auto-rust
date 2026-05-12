use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "pi", about = "Bacon autonomous pipeline")]
pub struct Args {
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

impl Args {
    pub fn parse_or_exit() -> Self {
        Self::parse()
    }
}
