use anyhow::Result;
use log::info;

use super::spec_io;
use super::types::PipelineCtx;
use crate::core::cli_types::RunArgs;

fn system_prompt() -> String {
    crate::core::read_role_prompt("observer")
}

pub async fn run(
    llm: &dyn crate::core::LlmClient,
    args: &RunArgs,
    ctx: &PipelineCtx,
) -> Result<PipelineCtx> {
    // If a user prompt was provided, skip spec scan and go straight to LLM
    if args.prompt.is_none() {
        // Check for an approved spec — fast-path if one exists
        if let Some((spec_path, meta)) = spec_io::find_approved_spec()? {
            let name = spec_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("(unknown)")
                .to_string();
            info!(
                "Found approved spec — fast-tracking: {} ({})",
                meta.title, name
            );
            let mut result = PipelineCtx::new(
                format!("Implement spec: {} ({})", meta.title, name),
                ctx.fs.clone(),
                ctx.runner.clone(),
                ctx.llm.clone(),
            );
            result.spec_path = Some(spec_path);
            result.dry_run = ctx.dry_run;
            return Ok(result);
        }
        info!("No approved specs pending; scanning for new improvements");
    }

    // Duplicate check: scan _done/ and _abandoned/ for matching specs
    if let Some(ref prompt) = args.prompt {
        if let Ok(duplicates) = spec_io::find_specs_matching(prompt) {
            if !duplicates.is_empty() {
                let paths: Vec<String> = duplicates
                    .iter()
                    .map(|(p, t)| format!("{} ({})", p.display(), t))
                    .collect();
                info!(
                    "Found {} existing specs matching prompt — skipping LLM scan:\n{}",
                    duplicates.len(),
                    paths.join("\n")
                );
            }
        }
    }

    let prompt = args.prompt.as_deref().unwrap_or(
        "Scan codebase for one grounded, low-risk maintenance improvement. \
         Only propose work proven by the included source excerpts. \
         Do not propose performance tuning, dependency changes, or rewrites. \
         If no concrete issue is visible, respond exactly: No clear improvement found",
    );

    info!("NVIDIA Observer calling API...");
    let context = crate::core::gather_project_context();
    let enriched_prompt = format!("{context}\n\n## Task\n\n{prompt}");
    let messages = vec![
        crate::llm::ChatMessage::system(system_prompt()),
        crate::llm::ChatMessage::user(enriched_prompt),
    ];
    let response = llm.chat(messages).await?;

    // Extract and log confidence
    let confidence = crate::core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Observer confidence: {}", conf.as_str());
    }

    Ok(PipelineCtx::new(
        response,
        ctx.fs.clone(),
        ctx.runner.clone(),
        ctx.llm.clone(),
    )
    .with_dry_run(ctx.dry_run)
    .with_confidence(confidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::path::PathBuf;
    use std::sync::Once;

    struct MockLlmClient {
        response_text: String,
        should_fail: bool,
        fail_msg: String,
    }

    impl MockLlmClient {
        fn ok(response: &str) -> Self {
            Self {
                response_text: response.to_string(),
                should_fail: false,
                fail_msg: String::new(),
            }
        }

        fn err(msg: &str) -> Self {
            Self {
                response_text: String::new(),
                should_fail: true,
                fail_msg: msg.to_string(),
            }
        }
    }

    #[async_trait]
    impl crate::core::LlmClient for MockLlmClient {
        async fn chat(&self, _messages: Vec<crate::llm::ChatMessage>) -> anyhow::Result<String> {
            if self.should_fail {
                Err(anyhow::anyhow!("{}", self.fail_msg))
            } else {
                Ok(self.response_text.clone())
            }
        }
    }

    fn setup() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let root =
                std::env::temp_dir().join(format!("bacon-observer-tests-{}", std::process::id()));
            let _ = std::fs::create_dir_all(root.join(".bacon/sessions"));
            crate::config::init(crate::config::ProjectConfig::with_defaults(root));
            crate::core::run_log::set_ci_mode(false);
        });
    }

    /// Create a minimal approved spec in `_active/<name>/spec.yaml`.
    fn create_approved_spec(name: &str) -> PathBuf {
        let active = crate::core::spec_io::active_dir();
        let spec_dir = active.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let meta = crate::core::spec_io::SpecMeta {
            id: name.to_string(),
            title: name.to_string(),
            status: "approved".to_string(),
            owner: "test".to_string(),
            implementer: "pending".to_string(),
            priority: "medium".to_string(),
        };
        let yaml = serde_yml::to_string(&meta).unwrap();
        std::fs::write(spec_dir.join("spec.yaml"), &yaml).unwrap();
        spec_dir
    }

    fn make_args(prompt: Option<&str>, dry_run: bool) -> RunArgs {
        RunArgs {
            prompt: prompt.map(String::from),
            stage: None,
            spec: None,
            fast: false,
            dry_run,
            auto: true,
            auto_apply: false,
            parallel: false,
            max_attempts: None,
            ci: false,
        }
    }

    // ------------------------------------------------------------------
    // Approved spec fast-track
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn no_prompt_with_approved_spec_fast_tracks() {
        setup();
        let spec_dir = create_approved_spec("test-approved-001");
        let llm = MockLlmClient::ok("should not be called");
        let ctx = PipelineCtx::new("initial context".to_string(), None, None, None);
        let args = make_args(None, false);

        let result = run(&llm, &args, &ctx).await.unwrap();

        assert!(result.description.contains("Implement spec:"));
        assert!(result.description.contains("test-approved-001"));
        assert_eq!(result.spec_path, Some(spec_dir));
    }

    #[tokio::test]
    async fn no_prompt_no_approved_spec_calls_llm() {
        setup();
        let llm = MockLlmClient::ok("No clear improvement found");
        let ctx = PipelineCtx::new("scan for issues".to_string(), None, None, None);
        let args = make_args(None, false);

        let result = run(&llm, &args, &ctx).await.unwrap();

        assert_eq!(result.description, "No clear improvement found");
        assert!(result.spec_path.is_none());
    }

    // ------------------------------------------------------------------
    // LLM path with user prompt
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn with_prompt_calls_llm_and_returns_response() {
        setup();
        let llm = MockLlmClient::ok("Found: unused function in audit.rs");
        let ctx = PipelineCtx::new("irrelevant".to_string(), None, None, None);
        let args = make_args(Some("find dead code in audit.rs"), false);

        let result = run(&llm, &args, &ctx).await.unwrap();

        assert_eq!(result.description, "Found: unused function in audit.rs");
    }

    // ------------------------------------------------------------------
    // Error propagation
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn llm_error_propagates() {
        setup();
        let llm = MockLlmClient::err("API rate limited");
        let ctx = PipelineCtx::new("test".to_string(), None, None, None);
        let args = make_args(None, false);

        let err = run(&llm, &args, &ctx).await.unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("API rate limited"), "got: {msg}");
    }

    // ------------------------------------------------------------------
    // Dry-run propagation
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn dry_run_flag_propagates_through_llm_path() {
        setup();
        let llm = MockLlmClient::ok("Some improvement");
        let mut ctx = PipelineCtx::new("scan".to_string(), None, None, None);
        ctx.dry_run = true;
        let args = make_args(None, false);

        let result = run(&llm, &args, &ctx).await.unwrap();

        assert!(result.dry_run);
    }

    #[tokio::test]
    async fn dry_run_flag_propagates_through_fast_track() {
        setup();
        let _spec_dir = create_approved_spec("test-dry-run-001");
        let llm = MockLlmClient::ok("should not be called");
        let mut ctx = PipelineCtx::new("scan".to_string(), None, None, None);
        ctx.dry_run = true;
        let args = make_args(None, false);

        let result = run(&llm, &args, &ctx).await.unwrap();

        assert!(result.description.contains("Implement spec:"));
        assert!(result.dry_run);
    }
}
