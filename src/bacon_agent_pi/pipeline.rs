use anyhow::Result;
use log::info;

use crate::llm::Llm;

use super::cli::Args;
use super::types::PipelineCtx;

#[derive(Debug)]
pub enum Stage {
    Observer,
    Strategist,
    Coder,
    Auditor,
}

impl Stage {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "observer" => Some(Self::Observer),
            "strategist" => Some(Self::Strategist),
            "coder" => Some(Self::Coder),
            "auditor" => Some(Self::Auditor),
            _ => None,
        }
    }
}

pub struct Pipeline {
    pub args: Args,
    pub llm: Llm,
}

impl Pipeline {
    pub fn new(args: Args) -> Result<Self> {
        let llm = Llm::new().map_err(|e| anyhow::anyhow!("failed to initialize LLM: {}", e))?;
        Ok(Self { args, llm })
    }

    pub async fn run(&self) -> Result<()> {
        let resume_stage = self.args.stage.as_deref().and_then(Stage::from_name);

        if let Some(stage) = &resume_stage {
            info!("Resuming from stage: {:?}", stage);
        }

        if self.args.dry_run {
            info!("DRY RUN — no files will be modified");
        }

        // Observer
        let mut ctx = if should_run(&resume_stage, Stage::Observer) {
            info!("=== Stage 1: Observer ===");
            super::observer::run(&self.llm, &self.args).await?
        } else {
            PipelineCtx::new(String::new())
        };

        // Strategist
        if should_run(&resume_stage, Stage::Strategist) {
            info!("=== Stage 2: Strategist ===");
            ctx = super::strategist::run(&self.llm, &self.args, &ctx).await?;
        }

        // Coder
        if should_run(&resume_stage, Stage::Coder) {
            info!("=== Stage 3: Coder ===");
            ctx = super::coder::run(&self.llm, &self.args, &ctx).await?;
        }

        // Auditor
        if should_run(&resume_stage, Stage::Auditor) {
            info!("=== Stage 4: Auditor ===");
            super::auditor::run(&self.llm, &self.args, &ctx).await?;
        }

        info!("Pipeline complete");
        Ok(())
    }
}

fn should_run(resume: &Option<Stage>, current: Stage) -> bool {
    match resume {
        None => true,
        Some(_) => {
            let order = [
                Stage::Observer,
                Stage::Strategist,
                Stage::Coder,
                Stage::Auditor,
            ];
            let resume_idx = order
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(&current));
            let current_idx = order
                .iter()
                .position(|s| std::mem::discriminant(s) == std::mem::discriminant(&current));
            match (resume_idx, current_idx) {
                (Some(r), Some(c)) => c >= r,
                _ => true,
            }
        }
    }
}
