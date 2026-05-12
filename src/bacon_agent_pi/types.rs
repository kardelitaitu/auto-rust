use std::path::PathBuf;

#[derive(Debug)]
pub struct PipelineCtx {
    pub description: String,
    pub spec_path: Option<PathBuf>,
}

impl PipelineCtx {
    pub fn new(description: String) -> Self {
        Self {
            description,
            spec_path: None,
        }
    }
}
