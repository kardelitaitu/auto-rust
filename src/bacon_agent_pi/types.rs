use std::path::PathBuf;

#[derive(Debug)]
pub struct PipelineCtx {
    pub description: String,
    pub spec_path: Option<PathBuf>,
    pub dry_run: bool,
}

impl PipelineCtx {
    pub fn new(description: String) -> Self {
        Self {
            description,
            spec_path: None,
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}
