use anyhow::{Context, Result};
use log::{info, warn};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bacon_core::{collect_source_context, read_role_prompt, GitSnapshot};

use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;

const MAX_ATTEMPTS: u32 = 4;

/// Result of auto-applying a queued patch.
struct AutoApplyReport {
    _applied_path: Option<PathBuf>,
    output: String,
}

/// A patch that passed verification and is queued for auto-apply.
struct QueuedPatch {
    patch_path: PathBuf,
    changed_paths: Vec<PathBuf>,
}

fn approved_patches_dir(root: &Path) -> PathBuf {
    root.join(".bacon")
        .join("sessions")
        .join("approved_patches")
}

fn safe_file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn changed_paths_from_patch(patch: &str) -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("diff --git a/") {
            if let Some(b_path) = path.find(" b/") {
                paths.insert(PathBuf::from(&path[..b_path]));
            }
        }
    }
    paths.into_iter().collect()
}

fn extract_unified_diff(response: &str) -> Result<String> {
    // Try fenced block first
    if let Some(diff) = extract_fenced_diff(response) {
        return Ok(diff);
    }
    // Fall back to raw diff --git extraction
    if response.contains("diff --git") {
        return Ok(response.to_string());
    }
    anyhow::bail!("No unified diff found in Coder output");
}

fn extract_fenced_diff(response: &str) -> Option<String> {
    let mut lines = response.lines();
    let mut in_diff = false;
    let mut diff_lines = Vec::new();
    for line in &mut lines {
        if line.trim_start().starts_with("```diff") || line.trim_start().starts_with("````diff") {
            in_diff = true;
            continue;
        }
        if in_diff
            && (line.trim_start().starts_with("```") || line.trim_start().starts_with("````"))
        {
            break;
        }
        if in_diff {
            diff_lines.push(line);
        }
    }
    if diff_lines.is_empty() {
        return None;
    }
    Some(diff_lines.join("\n"))
}

/// Verify a patch by applying to a temporary clone and running check-fast.ps1.
fn verify_patch_with_check_fast(root: &Path, patch_path: &Path) -> Result<String> {
    let check_output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "apply",
            "--check",
            "--recount",
            "--allow-overlap",
            "--ignore-whitespace",
            "--ignore-space-change",
        ])
        .arg(patch_path)
        .output()
        .context("failed to run git apply --check")?;
    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        anyhow::bail!("git apply --check failed:\n{}", stderr);
    }

    // Apply to temp worktree, run check-fast
    let worktree_dir = tempfile::tempdir().context("failed to create temp worktree")?;
    let worktree = worktree_dir.path();

    let clone_output = Command::new("git")
        .arg("clone")
        .arg("--shared")
        .arg(root)
        .arg(worktree)
        .output()
        .context("failed to clone to temp worktree")?;
    if !clone_output.status.success() {
        anyhow::bail!("failed to clone to temp worktree");
    }

    let apply_output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args([
            "apply",
            "--recount",
            "--allow-overlap",
            "--ignore-whitespace",
            "--ignore-space-change",
        ])
        .arg(patch_path)
        .output()
        .context("failed to apply patch in worktree")?;
    if !apply_output.status.success() {
        let stderr = String::from_utf8_lossy(&apply_output.stderr);
        anyhow::bail!("patch apply failed in worktree:\n{}", stderr);
    }

    // Run check-fast.ps1 in worktree
    let check_output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-File")
        .arg(worktree.join("check-fast.ps1"))
        .output()
        .context("failed to run check-fast.ps1 in worktree")?;
    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        let stdout = String::from_utf8_lossy(&check_output.stdout);
        anyhow::bail!(
            "check-fast.ps1 failed:\nstdout:\n{}\nstderr:\n{}",
            stdout,
            stderr
        );
    }

    Ok(String::from_utf8_lossy(&check_output.stdout).to_string())
}

/// Apply a queued patch to the main repository using GitSnapshot for rollback.
fn auto_apply_queued_patch(root: &Path, queued: &QueuedPatch) -> Result<AutoApplyReport> {
    info!(
        "Auto-applying patch: {} ({} files)",
        queued.patch_path.display(),
        queued.changed_paths.len()
    );

    // Capture pre-apply state via GitSnapshot
    let snapshot = GitSnapshot::create(root, &queued.changed_paths)
        .context("failed to create pre-apply snapshot")?;

    // Dry-run check
    let check_output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "apply",
            "--check",
            "--recount",
            "--allow-overlap",
            "--ignore-whitespace",
            "--ignore-space-change",
        ])
        .arg(&queued.patch_path)
        .output()
        .context("failed to run git apply --check")?;
    if !check_output.status.success() {
        let stderr = String::from_utf8_lossy(&check_output.stderr);
        snapshot
            .restore()
            .context("snapshot rollback failed after apply --check failure")?;
        anyhow::bail!(
            "git apply --check failed; rolled back via snapshot:\n{}",
            stderr
        );
    }

    // Apply
    let apply_output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "apply",
            "--recount",
            "--allow-overlap",
            "--ignore-whitespace",
            "--ignore-space-change",
        ])
        .arg(&queued.patch_path)
        .output()
        .context("failed to run git apply")?;
    if !apply_output.status.success() {
        let stderr = String::from_utf8_lossy(&apply_output.stderr);
        snapshot
            .restore()
            .context("snapshot rollback failed after apply failure")?;
        anyhow::bail!("git apply failed; rolled back via snapshot:\n{}", stderr);
    }

    // Validate
    let check_fast_output = match run_check_fast(root) {
        Ok(output) => output,
        Err(err) => {
            snapshot
                .restore()
                .context("snapshot rollback failed after check-fast failure")?;
            anyhow::bail!(
                "auto-apply check-fast.ps1 failed; rolled back via snapshot:\n{}",
                err
            );
        }
    };

    snapshot
        .mark_applied()
        .context("failed to mark snapshot as applied")?;

    info!("Auto-apply passed for {}", queued.patch_path.display());
    Ok(AutoApplyReport {
        _applied_path: Some(archive_approved_patch(root, &queued.patch_path)),
        output: format!(
            "auto-apply passed\nlocal check-fast:\n{}",
            check_fast_output
        ),
    })
}

fn run_check_fast(root: &Path) -> Result<String> {
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-File")
        .arg(root.join("check-fast.ps1"))
        .output()
        .context("failed to run check-fast.ps1")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "check-fast.ps1 exited with error:\nstdout:\n{}\nstderr:\n{}",
            stdout,
            stderr
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn archive_approved_patch(root: &Path, patch_path: &Path) -> PathBuf {
    let applied_dir = approved_patches_dir(root).join("applied");
    let _ = std::fs::create_dir_all(&applied_dir);
    let dest = applied_dir.join(
        patch_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("patch.diff")),
    );
    let _ = std::fs::copy(patch_path, &dest);
    dest
}

fn role_prompt() -> String {
    read_role_prompt("coder")
}

fn is_refusal(response: &str) -> bool {
    crate::bacon_core::is_refusal(response)
}

fn build_spec_context(ctx: &PipelineCtx) -> String {
    match &ctx.spec_path {
        Some(spec_path) => {
            let plan = spec_io::read_spec_file(spec_path, "plan.md");
            let api_outline = spec_io::read_spec_file(spec_path, "internal-api-outline.md");
            let validation_spec = spec_io::read_spec_file(spec_path, "validation.md");
            // Collect actual source file contents referenced throughout spec text
            let all_spec_text = format!("{}\n{}\n{}", plan, api_outline, validation_spec);
            let source_context = collect_source_context(&all_spec_text, 8, 100);
            format!(
                "## Plan\n{}\n\n## API Changes\n{}\n\n## Validation Criteria\n{}{}",
                plan, api_outline, validation_spec, source_context
            )
        }
        None => ctx.description.clone(),
    }
}

fn signal_scope_reduction(ctx: &PipelineCtx, errors: Vec<String>) -> PipelineCtx {
    let mut output = PipelineCtx::new(ctx.description.clone());
    output.scope_reduction_needed = true;
    output.coder_errors = errors;
    output.scope_reduction_count = ctx.scope_reduction_count + 1;
    output.spec_path = ctx.spec_path.clone();
    output.dry_run = ctx.dry_run;
    output.confidence = ctx.confidence;
    output
}

pub async fn run(_llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let config = crate::bacon_agent_nvidia::cli::nvidia_config_from_args(args);
    let system_prompt = role_prompt();

    let mut attempt: u32 = 1;
    let mut last_error = String::new();
    let mut errors: Vec<String> = Vec::new();
    let mut extracted_confidence: Option<crate::bacon_core::Confidence> = None;
    let mut consecutive_refusals: u32 = 0;
    let mut repeated_error_count: u32 = 0;

    loop {
        let spec_context = build_spec_context(ctx);

        let prompt = if attempt == 1 {
            format!(
                "Implement the following spec as a unified diff patch:\n\n{}\n\nProduce one unified diff patch starting with diff --git. Do not include SEARCH/REPLACE blocks.",
                spec_context
            )
        } else {
            format!(
                "Implement the following spec as a unified diff patch.\n\n\
                 The previous attempt had these issues:\n{}\n\n\
                 Spec context:\n{}\n\n\
                 Produce one unified diff patch starting with diff --git. \
                 Do not include SEARCH/REPLACE blocks. \
                 Do not refuse or explain why you can't do it — just produce the patch.",
                last_error, spec_context
            )
        };

        info!(
            "NVIDIA Coder calling API (attempt {}/{}) with model: {}",
            attempt, MAX_ATTEMPTS, config.model
        );

        let response = match nvidia_api::chat(&config, &system_prompt, &prompt).await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("NVIDIA API call failed on attempt {}: {}", attempt, e);
                warn!("{}", err_msg);
                errors.push(err_msg.clone());
                if attempt >= MAX_ATTEMPTS {
                    warn!("Max retries exhausted — signalling scope reduction needed");
                    return Ok(signal_scope_reduction(ctx, errors));
                }
                last_error = format!("API error: {}", e);
                attempt += 1;
                continue;
            }
        };

        // Extract and log confidence
        if let Some(conf) = crate::bacon_core::extract_confidence(&response) {
            info!(
                "NVIDIA Coder confidence (attempt {}): {}",
                attempt,
                conf.as_str()
            );
            extracted_confidence = Some(conf);
        }

        // Refusal detection
        if is_refusal(&response) {
            let refusal_log = response.chars().take(200).collect::<String>();
            consecutive_refusals += 1;
            warn!(
                "NVIDIA Coder refused on attempt {} (consecutive refusal #{})",
                attempt, consecutive_refusals
            );
            errors.push(format!("Attempt {} refused with: {}", attempt, refusal_log));

            // 2 consecutive refusals → abort pipeline, no scope reduction
            if consecutive_refusals >= 2 {
                let report = format!(
                    "NVIDIA Coder refused to implement after {} consecutive refusals.\n\n\
                     Attempt {}:\n{}\n\n\
                     Refusal chain ({} consecutive refusals).",
                    consecutive_refusals, attempt, refusal_log, consecutive_refusals
                );
                warn!("NVIDIA Coder: 2 consecutive refusals — aborting pipeline, needs human approval");
                if !ctx.dry_run {
                    if let Some(ref spec_path) = ctx.spec_path {
                        let validation_path = spec_path.join("validation.md");
                        let _ = std::fs::write(
                            &validation_path,
                            format!("# Coder Refusal Report\n\n{}", report),
                        );
                        let meta_path = spec_path.join("spec.yaml");
                        if let Ok(content) = std::fs::read_to_string(&meta_path) {
                            let updated = content.replace("in-progress", "needs-human-approval");
                            let _ = std::fs::write(&meta_path, updated);
                        }
                    }
                }
                let mut output = PipelineCtx::new(report).with_dry_run(ctx.dry_run);
                output.spec_path = ctx.spec_path.clone();
                output.confidence = extracted_confidence;
                output.coder_refused = true;
                return Ok(output);
            }

            if attempt >= MAX_ATTEMPTS {
                warn!("Max retries exhausted due to refusals — signalling scope reduction needed");
                let report = format!(
                    "LLM refused implementation after {} attempts. Last refusal:\n{}",
                    MAX_ATTEMPTS,
                    response.chars().take(500).collect::<String>()
                );
                let mut output = signal_scope_reduction(ctx, errors);
                output.description = report;
                return Ok(output);
            }
            last_error = format!(
                "The previous attempt produced a refusal instead of a patch. Provide a concrete diff. Refusal was: {}",
                refusal_log
            );
            attempt += 1;
            continue;
        }
        consecutive_refusals = 0;

        // Check for empty or non-diff response
        if response.trim().is_empty() || !response.contains("diff --git") {
            warn!(
                "NVIDIA Coder returned empty or non-diff response on attempt {}",
                attempt
            );
            errors.push(format!("Attempt {} produced no valid diff patch", attempt));
            if attempt >= MAX_ATTEMPTS {
                warn!("Max retries exhausted — signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = "The previous attempt did not produce a valid unified diff patch. Make sure to output a proper `diff --git` patch.".to_string();
            attempt += 1;
            continue;
        }

        info!("NVIDIA Coder produced valid patch on attempt {}", attempt);

        // Extract and queue the patch
        let patch = match extract_unified_diff(&response) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to extract unified diff: {}", e);
                errors.push(format!(
                    "Attempt {} produced diff content but extraction failed: {}",
                    attempt, e
                ));
                if attempt >= MAX_ATTEMPTS {
                    return Ok(signal_scope_reduction(ctx, errors));
                }
                last_error = format!("Failed to extract unified diff: {}", e);
                attempt += 1;
                continue;
            }
        };

        // Save patch to temp file and verify
        let temp_patch =
            tempfile::NamedTempFile::new().context("failed to create temp patch file")?;
        std::fs::write(temp_patch.path(), &patch)?;

        let changed_paths = changed_paths_from_patch(&patch);
        if changed_paths.is_empty() {
            warn!("Patch had no changed paths on attempt {}", attempt);
            errors.push(format!("Attempt {} patch had no changed paths", attempt));
            if attempt >= MAX_ATTEMPTS {
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = "The patch did not specify any changed file paths".to_string();
            attempt += 1;
            continue;
        }

        // Verify with check-fast.ps1 (unless dry run)
        if !ctx.dry_run {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            match verify_patch_with_check_fast(root, temp_patch.path()) {
                Ok(check_output) => {
                    info!("Patch verification passed on attempt {}", attempt);

                    // Queue the patch
                    let approved_dir = approved_patches_dir(root);
                    std::fs::create_dir_all(&approved_dir)
                        .context("failed to create approved_patches dir")?;
                    let spec_id = ctx
                        .spec_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown-spec".to_string());
                    let patch_name =
                        format!("{}_attempt_{}.diff", safe_file_stem(&spec_id), attempt);
                    let patch_path = approved_dir.join(patch_name);
                    std::fs::write(&patch_path, &patch)?;
                    let saved_patch_path = patch_path.clone();

                    // Gate: auto-apply only if --auto-apply flag is set
                    if args.auto_apply {
                        let queued = QueuedPatch {
                            patch_path: saved_patch_path.clone(),
                            changed_paths,
                        };
                        match auto_apply_queued_patch(root, &queued) {
                            Ok(report) => {
                                info!("Auto-apply succeeded: {}", report.output);
                                let mut result =
                                    PipelineCtx::new(report.output).with_dry_run(ctx.dry_run);
                                result.spec_path = ctx.spec_path.clone();
                                result.confidence = extracted_confidence;
                                result.patch_path = Some(saved_patch_path);
                                return Ok(result);
                            }
                            Err(e) => {
                                warn!("Auto-apply failed on attempt {}: {}", attempt, e);
                                errors
                                    .push(format!("Attempt {} auto-apply failed: {}", attempt, e));
                                if attempt >= MAX_ATTEMPTS {
                                    return Ok(signal_scope_reduction(ctx, errors));
                                }
                                last_error = format!(
                                    "Auto-apply failed: {}\nCheck output: {}\n\n\
---\n\nFix the patch and ensure check-fast.ps1 passes.",
                                    e, check_output
                                );
                                attempt += 1;
                                continue;
                            }
                        }
                    } else {
                        // Auto-apply disabled — return success with the queued patch path
                        info!(
                            "Patch queued (auto-apply disabled): {}",
                            saved_patch_path.display()
                        );
                        let mut result =
                            PipelineCtx::new("Patch queued; manual apply required".to_string())
                                .with_dry_run(ctx.dry_run);
                        result.spec_path = ctx.spec_path.clone();
                        result.confidence = extracted_confidence;
                        result.patch_path = Some(saved_patch_path);
                        return Ok(result);
                    }
                }
                Err(e) => {
                    warn!("Patch verification failed on attempt {}: {}", attempt, e);
                    errors.push(format!("Attempt {} verification failed: {}", attempt, e));
                    if attempt >= MAX_ATTEMPTS {
                        return Ok(signal_scope_reduction(ctx, errors));
                    }
                    let new_error = format!(
                        "Patch verification failed:\n{}\n\n\
                         ---\n\n\
                         Fix the patch and ensure check-fast.ps1 passes.",
                        e
                    );
                    if attempt > 1 && new_error == last_error {
                        repeated_error_count += 1;
                        if repeated_error_count >= 1 {
                            warn!("Same verification error repeated — skipping remaining retries");
                            return Ok(signal_scope_reduction(ctx, errors));
                        }
                    } else {
                        repeated_error_count = 0;
                    }
                    last_error = new_error;
                    attempt += 1;
                    continue;
                }
            }
        } else {
            // Dry run — just return the patch response
            info!("Dry run: skipping patch verification and queue, returning patch directly");
            let mut result = PipelineCtx::new(response).with_dry_run(true);
            result.spec_path = ctx.spec_path.clone();
            result.confidence = extracted_confidence;
            return Ok(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ========================================================================
    // is_refusal Tests
    // ========================================================================

    #[test]
    fn test_refusal_cannot_implement() {
        assert!(is_refusal("I cannot implement this feature"));
    }

    #[test]
    fn test_refusal_cannot_complete() {
        assert!(is_refusal("I cannot complete your request"));
    }

    #[test]
    fn test_refusal_unable_to_implement() {
        assert!(is_refusal("Sorry, I am unable to implement"));
    }

    #[test]
    fn test_refusal_i_cannot() {
        assert!(is_refusal("i cannot do this"));
    }

    #[test]
    fn test_refusal_wont_implement() {
        assert!(is_refusal("I won't implement that"));
    }

    #[test]
    fn test_refusal_outside_my() {
        assert!(is_refusal("This is outside my capabilities"));
    }

    #[test]
    fn test_refusal_not_possible() {
        assert!(is_refusal("It is not possible to implement"));
    }

    #[test]
    fn test_refusal_cant_implement() {
        assert!(is_refusal("I can't implement this right now"));
    }

    #[test]
    fn test_refusal_dont_know() {
        assert!(is_refusal("I don't know how to do that"));
    }

    #[test]
    fn test_refusal_case_insensitive() {
        assert!(is_refusal("I CANNOT IMPLEMENT this"));
    }

    #[test]
    fn test_refusal_no_match() {
        assert!(!is_refusal("Here is the patch you requested"));
    }

    #[test]
    fn test_refusal_empty_string() {
        assert!(!is_refusal(""));
    }

    #[test]
    fn test_refusal_partial_word_no_false_match() {
        assert!(!is_refusal("I am considering your request"));
    }

    #[test]
    fn test_refusal_all_phrases_detected() {
        for phrase in crate::bacon_core::REFUSAL_PHRASES {
            assert!(
                is_refusal(phrase),
                "Should detect refusal phrase: '{}'",
                phrase
            );
        }
    }

    // ========================================================================
    // signal_scope_reduction Tests
    // ========================================================================

    #[test]
    fn test_signal_scope_reduction_sets_flags() {
        let ctx = PipelineCtx::new("test".to_string());
        let errors = vec!["error1".to_string(), "error2".to_string()];
        let result = signal_scope_reduction(&ctx, errors.clone());
        assert!(result.scope_reduction_needed);
        assert_eq!(result.scope_reduction_count, 1);
        assert_eq!(result.coder_errors, errors);
    }

    #[test]
    fn test_signal_scope_reduction_increments_count() {
        let mut ctx = PipelineCtx::new("test".to_string());
        ctx.scope_reduction_count = 3;
        let result = signal_scope_reduction(&ctx, vec![]);
        assert_eq!(result.scope_reduction_count, 4);
    }

    #[test]
    fn test_signal_scope_reduction_preserves_fields() {
        let mut ctx = PipelineCtx::new("desc".to_string());
        ctx.dry_run = true;
        ctx.spec_path = Some(PathBuf::from("/tmp/spec"));
        let result = signal_scope_reduction(&ctx, vec![]);
        assert!(result.dry_run);
        assert_eq!(result.spec_path, Some(PathBuf::from("/tmp/spec")));
    }

    #[test]
    fn test_signal_scope_reduction_new_description() {
        let ctx = PipelineCtx::new("original".to_string());
        let result = signal_scope_reduction(&ctx, vec![]);
        assert_eq!(result.description, "original");
    }
}
