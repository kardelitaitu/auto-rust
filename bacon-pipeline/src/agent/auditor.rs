use anyhow::Result;
use log::{info, warn};

use super::spec_io;
use super::types::PipelineCtx;
use crate::core::cli_types::RunArgs;

fn role_prompt() -> String {
    crate::core::read_role_prompt("auditor")
}

pub async fn run(
    llm: &dyn crate::core::LlmClient,
    _args: &RunArgs,
    ctx: &PipelineCtx,
) -> Result<PipelineCtx> {
    let system_prompt = role_prompt();

    // Resolve spec context: prefer files on disk, fall back to ctx.description
    let (_spec_path, meta, spec_name, plan, validation_spec) = if let Some(p) = &ctx.spec_path {
        let meta = spec_io::read_spec_meta(p)?;
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let plan = spec_io::read_spec_file(p, "plan.md");
        let validation_spec = spec_io::read_spec_file(p, "validation.md");
        (Some(p.clone()), meta, name, plan, validation_spec)
    } else {
        warn!("No spec path — using ctx.description as plan for audit");
        let meta = spec_io::SpecMeta {
            id: "adhoc".to_string(),
            title: "Ad-hoc task".to_string(),
            status: "implemented".to_string(),
            owner: "pipeline".to_string(),
            implementer: "auto".to_string(),
            priority: "medium".to_string(),
        };
        let plan = ctx.description.clone();
        let validation_spec = "See plan for validation criteria.".to_string();
        (None, meta, "adhoc".to_string(), plan, validation_spec)
    };

    // Read the approved patch file if available; fall back to git diff
    let diff = if let Some(patch_path) = &ctx.patch_path {
        match std::fs::read_to_string(patch_path) {
            Ok(content) => {
                info!("Reading approved patch from: {}", patch_path.display());
                if content.len() > 10000 {
                    format!("{}...\n[truncated at 10000 chars]", &content[..10000])
                } else {
                    content
                }
            }
            Err(e) => {
                warn!(
                    "Could not read patch file at {}: {}",
                    patch_path.display(),
                    e
                );
                "(no patch file available)".to_string()
            }
        }
    } else {
        warn!("No patch_path in context — falling back to working tree diff (may be empty)");
        std::process::Command::new("git")
            .args(["diff", "--", "src/"])
            .current_dir(crate::config::manifest_dir())
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let out = String::from_utf8_lossy(&o.stdout).to_string();
                    if out.len() > 10000 {
                        Some(format!("{}...\n[truncated at 10000 chars]", &out[..10000]))
                    } else if !out.is_empty() {
                        Some(out)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "(no diff available — no patch file present)".to_string())
    };

    let user_prompt = format!(
        "Audit this implemented spec: {} ({})\n\n\
         Title: {}\nStatus: {}\n\n\
         ## Plan (from plan.md)\n{}\n\n\
         ## Validation Criteria\n{}\n\n\
         ## Git Diff (working tree)\n```diff\n{}\n```\n\n\
         Does the implementation match the spec? \
         Are all acceptance criteria met? \
         Any missed edge cases, scope violations, or regressions?\n\n\
         Respond with PASS or FAIL as the first word, then explain your reasoning.\n\
         PASS → the spec should be marked done and moved to _done/.\n\
         FAIL → mark needs-human-approval and explain what's missing.",
        meta.title, spec_name, meta.title, meta.status, plan, validation_spec, diff
    );

    info!("NVIDIA Auditor calling API...");
    let messages = vec![
        crate::llm::ChatMessage::system(system_prompt),
        crate::llm::ChatMessage::user(user_prompt),
    ];
    let response = llm.chat(messages).await?;

    // Extract and log confidence
    let confidence = crate::core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Auditor confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Auditor Output ===");
    println!("{response}");
    println!("=============================");

    let mut output = PipelineCtx::new(
        response,
        ctx.fs.clone(),
        ctx.runner.clone(),
        ctx.llm.clone(),
    )
    .with_confidence(confidence)
    .with_dry_run(ctx.dry_run);
    output.spec_path = ctx.spec_path.clone();
    output.patch_path = ctx.patch_path.clone();

    let decision_first = output.description.split_whitespace().next().unwrap_or("");
    if let Some(ref spec_path) = output.spec_path {
        if decision_first.eq_ignore_ascii_case("PASS") {
            info!("NVIDIA Auditor PASS");
            if output.dry_run {
                info!("DRY RUN: would move spec to _done/");
            } else {
                let archived_path = promote_to_done(spec_path)?;
                output.spec_path = Some(archived_path);
            }
        } else {
            info!("NVIDIA Auditor FAIL — marking needs-human-approval");
            if !output.dry_run {
                write_audit_report(spec_path, &output.description)?;
            }
            output.set_needs_approval();
        }
    } else {
        // No spec path on disk — just log the audit result
        if decision_first.eq_ignore_ascii_case("PASS") {
            info!("NVIDIA Auditor PASS (adhoc — no spec to archive)");
        } else {
            warn!("NVIDIA Auditor found issues (adhoc — no spec to file)");
        }
    }

    Ok(output)
}

fn promote_to_done(path: &std::path::Path) -> Result<std::path::PathBuf> {
    // Gate: run spec-lint before archiving — catches structural errors
    let (passed, output) = crate::core::run_spec_lint(path)?;
    if !passed {
        anyhow::bail!(
            "spec-lint failed for {} — not moving to _done/:\n{}",
            path.display(),
            output
        );
    }

    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "done".to_string();
    spec_io::write_spec_meta(path, &meta)?;

    // Rewrite docs array in spec.yaml to point to _done/ instead of _active/
    let yaml_path = path.join("spec.yaml");
    match std::fs::read_to_string(&yaml_path) {
        Ok(content) => {
            let updated_content = content
                .replace("docs/specs/_active/", "docs/specs/_done/")
                .replace("docs\\specs\\_active\\", "docs\\specs\\_done\\");
            if let Err(e) = std::fs::write(&yaml_path, updated_content) {
                warn!("Failed to write updated spec.yaml paths: {e}");
            }
        }
        Err(e) => {
            warn!("Failed to read spec.yaml for path rewrite: {e}");
        }
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    info!("Moving {name} to _done/");
    let dest = spec_io::move_to_done(path)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
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
                std::env::temp_dir().join(format!("bacon-auditor-tests-{}", std::process::id()));
            let _ = std::fs::create_dir_all(root.join(".bacon/sessions"));
            crate::config::init(crate::config::ProjectConfig::with_defaults(root));
        });
    }

    fn make_ctx(description: &str, dry_run: bool) -> PipelineCtx {
        PipelineCtx::new(description.to_string(), None, None, None).with_dry_run(dry_run)
    }

    // ------------------------------------------------------------------
    // PASS — ad-hoc path (no spec on disk)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pass_response_returns_pass_description() {
        setup();
        let llm = MockLlmClient::ok("PASS - all acceptance criteria met. No issues found.");
        let ctx = make_ctx("implemented feature X", false);

        let result = run(&llm, &RunArgs::default(), &ctx).await.unwrap();

        assert!(
            result.description.starts_with("PASS"),
            "got: {}",
            result.description
        );
        assert!(!result.needs_human_approval);
    }

    // ------------------------------------------------------------------
    // FAIL — ad-hoc path (no spec on disk)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn fail_response_returns_fail_description_adhoc() {
        setup();
        let llm = MockLlmClient::ok("FAIL - missing error handling for edge case.");
        let ctx = make_ctx("implemented feature X", false);

        let result = run(&llm, &RunArgs::default(), &ctx).await.unwrap();

        assert!(
            result.description.starts_with("FAIL"),
            "got: {}",
            result.description
        );
        assert!(!result.needs_human_approval); // ad-hoc path doesn't set this
    }

    // ------------------------------------------------------------------
    // Error propagation
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn llm_error_propagates() {
        setup();
        let llm = MockLlmClient::err("API unavailable");
        let ctx = make_ctx("test", false);

        let err = run(&llm, &RunArgs::default(), &ctx).await.unwrap_err();

        let msg = format!("{err:#}");
        assert!(msg.contains("API unavailable"), "got: {msg}");
    }

    // ------------------------------------------------------------------
    // Confidence extraction
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn confidence_extracted_from_response() {
        setup();
        let llm = MockLlmClient::ok("PASS - looks good.\n\nConfidence: High");
        let ctx = make_ctx("test", false);

        let result = run(&llm, &RunArgs::default(), &ctx).await.unwrap();

        assert!(result.confidence.is_some(), "confidence should be parsed");
        assert_eq!(result.confidence.as_ref().unwrap().as_str(), "high");
    }

    #[tokio::test]
    async fn no_confidence_in_response_returns_none() {
        setup();
        let llm = MockLlmClient::ok("PASS - looks good.");
        let ctx = make_ctx("test", false);

        let result = run(&llm, &RunArgs::default(), &ctx).await.unwrap();

        assert!(result.confidence.is_none(), "got: {:?}", result.confidence);
    }

    // ------------------------------------------------------------------
    // Dry-run propagation
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn dry_run_flag_propagates() {
        setup();
        let llm = MockLlmClient::ok("PASS - looks good.");
        let ctx = make_ctx("test", true);

        let result = run(&llm, &RunArgs::default(), &ctx).await.unwrap();

        assert!(result.dry_run);
    }
}

fn write_audit_report(path: &std::path::Path, report: &str) -> Result<()> {
    let validation_path = path.join("validation.md");
    let existing = std::fs::read_to_string(&validation_path).unwrap_or_else(|e| {
        warn!("Failed to read existing validation.md: {e}");
        String::new()
    });
    std::fs::write(
        &validation_path,
        format!("# Audit Report\n\n{report}\n\n{existing}"),
    )?;

    info!("Wrote audit report to validation.md");
    Ok(())
}
