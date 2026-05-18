use anyhow::{Context, Result};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bacon_core::{collect_source_context, read_role_prompt, GitSnapshot};

use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;

/// Default max retry attempts when no `--max-attempts` CLI flag is provided.
const DEFAULT_MAX_ATTEMPTS: u32 = 4;

/// Resolve the effective max attempt count from CLI flag (if set) or default.
fn effective_max_attempts(cli_value: Option<u32>) -> u32 {
    cli_value.unwrap_or(DEFAULT_MAX_ATTEMPTS)
}

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

/// Minimum length (in bytes) for SEARCH text to be considered valid.
/// Blocks with shorter SEARCH text are almost certainly hallucinations
/// or refusal noise that happened to match the regex pattern.
const MIN_SEARCH_LENGTH: usize = 10;

/// Minimum length (in bytes) for REPLACE text to be considered valid.
const MIN_REPLACE_LENGTH: usize = 1;

/// Minimum total response length to even attempt parsing.
/// Responses shorter than this are guaranteed not to contain real
/// SEARCH/REPLACE blocks (noise, refusals, or empty output).
const MIN_RESPONSE_LENGTH: usize = 60;

/// Parse SEARCH/REPLACE blocks from LLM output (Aider-style format).
///
/// Format:
/// ```
/// path/to/file.ext
/// <<<<<<< SEARCH
/// existing content
/// =======
/// new content
/// >>>>>>> REPLACE
/// ```
///
/// Returns a list of (file_path, search_text, replace_text) tuples.
fn parse_search_replace_blocks(response: &str) -> Vec<(String, String, String)> {
    // Match blocks where the file path is on a line before <<<<<<< SEARCH
    let re = regex::Regex::new(
        r"(?ms)(?:^|\n)\s*([^\n]+?\.(?:rs|toml|md|json|ps1|sh|js|ts|css|html))\s*\n\s*<<<<<<< SEARCH\s*\n(.*?)\n\s*=======\s*\n(.*?)\n\s*>>>>>>> REPLACE"
    ).expect("invalid SEARCH/REPLACE regex");

    let mut blocks = Vec::new();
    for cap in re.captures_iter(response) {
        let file_path = cap[1].trim().to_string();
        let search = cap[2].to_string();
        let replace = cap[3].to_string();
        if !file_path.is_empty() {
            blocks.push((file_path, search, replace));
        }
    }
    blocks
}

/// Validate parsed SEARCH/REPLACE blocks for quality and correctness.
///
/// Checks performed:
/// - **Response too short**: Raw response is below MIN_RESPONSE_LENGTH
/// - **Trivial SEARCH text**: SEARCH content is below MIN_SEARCH_LENGTH bytes
/// - **Trivial REPLACE text**: REPLACE content is below MIN_REPLACE_LENGTH bytes
/// - **Duplicate blocks**: Same file+search appears more than once (hallucination signal)
///
/// Returns `Ok(())` if all blocks pass validation, or `Err` with a description
/// of the first failing check.
fn validate_search_replace_blocks(
    response: &str,
    blocks: &[(String, String, String)],
) -> Result<()> {
    // 1. Response-level: reject trivially short responses
    if response.trim().len() < MIN_RESPONSE_LENGTH {
        anyhow::bail!(
            "Response too short ({} bytes, minimum {}) — likely not a real SEARCH/REPLACE output",
            response.trim().len(),
            MIN_RESPONSE_LENGTH
        );
    }

    // 2. Block-level: reject blocks with trivial content
    for (i, (file_path, search, replace)) in blocks.iter().enumerate() {
        let search_trimmed = search.trim();
        let replace_trimmed = replace.trim();

        if search_trimmed.len() < MIN_SEARCH_LENGTH {
            anyhow::bail!(
                "Block #{} ({}): SEARCH text too short ({} bytes, minimum {}) — likely hallucinated",
                i + 1,
                file_path,
                search_trimmed.len(),
                MIN_SEARCH_LENGTH
            );
        }

        if replace_trimmed.len() < MIN_REPLACE_LENGTH {
            anyhow::bail!(
                "Block #{} ({}): REPLACE text is empty — no change to apply",
                i + 1,
                file_path,
            );
        }
    }

    // 3. Deduplication: reject repeated identical blocks (hallucination signal)
    let mut seen = std::collections::HashSet::new();
    for (file_path, search, _) in blocks.iter() {
        let key = format!("{}::{}", file_path, search);
        if !seen.insert(key.clone()) {
            anyhow::bail!(
                "Duplicate SEARCH block found for {} — LLM likely hallucinated or repeated output",
                file_path
            );
        }
    }

    Ok(())
}

/// Apply SEARCH/REPLACE blocks directly to working tree files with GitSnapshot rollback.
///
/// 1. Creates a GitSnapshot for safe rollback.
/// 2. For each block: reads the file, performs SEARCH→REPLACE, writes back.
/// 3. Runs `check-fast.ps1` to verify compilation.
/// 4. On success: generates unified diff via `git diff HEAD -- files...`.
/// 5. On failure: rolls back via snapshot.
///
/// Returns the unified diff patch and list of changed file paths (relative).
fn apply_search_replace_blocks(
    root: &Path,
    blocks: &[(String, String, String)],
) -> Result<(String, Vec<PathBuf>)> {
    // Collect relative file paths for snapshot and diff
    let mut rel_paths: Vec<PathBuf> = Vec::new();
    for (file_path, _, _) in blocks {
        let clean = file_path.trim_start_matches(&['/', '\\'][..]);
        let rel = PathBuf::from(clean);
        let abs = root.join(&rel);
        if !abs.exists() {
            anyhow::bail!(
                "SEARCH/REPLACE target file not found: {} (resolved to {})",
                file_path,
                abs.display()
            );
        }
        rel_paths.push(rel);
    }

    // Create snapshot for safe rollback (needs relative paths)
    let snapshot = GitSnapshot::create(root, &rel_paths)?;

    // Apply each SEARCH→REPLACE
    for (file_path, search, replace) in blocks {
        let clean = file_path.trim_start_matches(&['/', '\\'][..]);
        let abs_path = root.join(clean);
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("failed to read {}", abs_path.display()))?;

        // Try exact match first
        if let Some(pos) = content.find(search.as_str()) {
            let new_content = format!(
                "{}{}{}",
                &content[..pos],
                replace,
                &content[pos + search.len()..]
            );
            std::fs::write(&abs_path, &new_content)
                .with_context(|| format!("failed to write {}", abs_path.display()))?;
            info!("SEARCH/REPLACE applied to {} (exact match)", clean);
            continue;
        }

        // Fallback: whitespace-insensitive matching
        // Normalize both search and content by stripping all whitespace
        let search_flat: String = search.chars().filter(|c| !c.is_whitespace()).collect();
        if search_flat.is_empty() {
            anyhow::bail!(
                "SEARCH text for {} is empty after whitespace normalization",
                clean
            );
        }
        let content_flat: String = content.chars().filter(|c| !c.is_whitespace()).collect();
        if let Some(flat_pos) = content_flat.find(&search_flat) {
            // Map flat position back to original content position by counting chars
            let mut orig_pos = 0;
            let mut flat_idx = 0;
            for (i, ch) in content.char_indices() {
                if flat_idx >= flat_pos {
                    orig_pos = i;
                    break;
                }
                if !ch.is_whitespace() {
                    flat_idx += ch.len_utf8();
                }
            }
            // Find end byte offset in original content by walking forward
            // consuming search_flat.len() bytes of non-whitespace characters
            let mut end_pos = content.len();
            let mut consumed = 0usize;
            for (i, ch) in content[orig_pos..].char_indices() {
                if consumed >= search_flat.len() {
                    end_pos = orig_pos + i;
                    break;
                }
                if !ch.is_whitespace() {
                    consumed += ch.len_utf8();
                }
            }
            let new_content = format!("{}{}{}", &content[..orig_pos], replace, &content[end_pos..]);
            std::fs::write(&abs_path, &new_content)
                .with_context(|| format!("failed to write {}", abs_path.display()))?;
            info!(
                "SEARCH/REPLACE applied to {} (whitespace-insensitive match)",
                clean
            );
        } else {
            // Rollback before returning error (warn if restore fails)
            if let Err(e) = snapshot.restore() {
                warn!(
                    "Failed to restore snapshot during SEARCH/REPLACE error recovery: {}",
                    e
                );
            }
            anyhow::bail!(
                "SEARCH text not found in {} — even after whitespace-insensitive matching",
                clean
            );
        }
    }

    // Run check-fast.ps1 to verify
    match run_check_fast(root) {
        Ok(output) => info!(
            "check-fast.ps1 passed after SEARCH/REPLACE: {}",
            output.trim()
        ),
        Err(e) => {
            warn!("check-fast.ps1 failed after SEARCH/REPLACE: {}", e);
            snapshot.restore()?;
            anyhow::bail!(
                "SEARCH/REPLACE changes failed check-fast.ps1; rolled back: {}",
                e
            );
        }
    }

    // Generate unified diff
    let diff_output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("diff")
        .arg("HEAD")
        .arg("--")
        .args(&rel_paths)
        .output()
        .context("failed to run git diff HEAD")?;
    if !diff_output.status.success() {
        snapshot.restore()?;
        anyhow::bail!("git diff HEAD failed; rolled back");
    }
    let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();
    if diff.trim().is_empty() {
        snapshot.restore()?;
        anyhow::bail!("SEARCH/REPLACE produced no changes; rolled back");
    }

    // Mark snapshot as applied
    snapshot.mark_applied()?;

    info!(
        "SEARCH/REPLACE applied successfully: {} files changed",
        rel_paths.len()
    );
    Ok((diff, rel_paths))
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
            let validation_spec = spec_io::read_spec_file(spec_path, "validation.md");
            // Collect actual source file contents referenced throughout spec text
            let all_spec_text = format!("{}\n{}", plan, validation_spec);
            let source_context = collect_source_context(&all_spec_text, 8, 300);
            format!(
                "## Plan\n{}\n\n## Validation Criteria\n{}{}",
                plan, validation_spec, source_context
            )
        }
        None => {
            // When the external Strategist didn't write spec files to disk,
            // fall back to ctx.description but still extract file paths from
            // the plan text and include their source content. This ensures the
            // LLM has the actual code to match against in SEARCH/REPLACE blocks.
            let source_context = collect_source_context(&ctx.description, 8, 300);
            if source_context.is_empty() {
                ctx.description.clone()
            } else {
                format!("{}\n\n{}", ctx.description, source_context)
            }
        }
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

    // Mark spec as in-progress at the start of implementation
    if !ctx.dry_run {
        if let Some(ref spec_path) = ctx.spec_path {
            if let Ok(mut meta) = spec_io::read_spec_meta(spec_path) {
                if meta.status == "approved" {
                    meta.status = "in-progress".to_string();
                    if let Err(e) = spec_io::write_spec_meta(spec_path, &meta) {
                        warn!("Failed to set spec status to in-progress: {}", e);
                    } else {
                        info!("Set spec status to in-progress");
                    }
                }
            }
        }
    }

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
                "Implement the following spec using SEARCH/REPLACE blocks.\n\n{}\n\n\
                 For each file you need to change, output the file path, then:\n\
                 <<<<<<< SEARCH\n\
                 [exact lines to replace]\n\
                 =======\n\
                 [new lines]\n\
                 >>>>>>> REPLACE\n\
                 CRITICAL: Copy SEARCH lines EXACTLY from the source files — character for character.\n\
                 Do NOT output unified diff patches (diff --git). Only output SEARCH/REPLACE blocks.",
                spec_context
            )
        } else {
            format!(
                "Implement the following spec using SEARCH/REPLACE blocks.\n\n\
                 The previous attempt had these issues:\n{}\n\n\
                 Spec context:\n{}\n\n\
                 For each file you need to change, output the file path, then:\n\
                 <<<<<<< SEARCH\n\
                 [exact lines to replace]\n\
                 =======\n\
                 [new lines]\n\
                 >>>>>>> REPLACE\n\
                 CRITICAL: Copy SEARCH lines EXACTLY from the source files — character for character.\n\
                 Do NOT output unified diff patches (diff --git). Only output SEARCH/REPLACE blocks. \
                 Do not refuse or explain why you can't do it — just produce the blocks.",
                last_error, spec_context
            )
        };

        let max_attempts = effective_max_attempts(args.max_attempts);
        info!(
            "NVIDIA Coder calling API (attempt {}/{}) with model: {}",
            attempt, max_attempts, config.model
        );

        let response = match nvidia_api::chat(&config, &system_prompt, &prompt).await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("NVIDIA API call failed on attempt {}: {}", attempt, e);
                warn!("{}", err_msg);
                errors.push(err_msg.clone());
                let max_attempts = effective_max_attempts(args.max_attempts);
                if attempt >= max_attempts {
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
                        if let Ok(mut meta) = spec_io::read_spec_meta(spec_path) {
                            meta.status = "needs-human-approval".to_string();
                            if let Err(e) = spec_io::write_spec_meta(spec_path, &meta) {
                                warn!("Failed to persist needs-human-approval to spec.yaml: {}", e);
                            }
                        }
                    }
                }
                let mut output = PipelineCtx::new(report).with_dry_run(ctx.dry_run);
                output.spec_path = ctx.spec_path.clone();
                output.confidence = extracted_confidence;
                output.coder_refused = true;
                return Ok(output);
            }

            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted due to refusals — signalling scope reduction needed");
                let report = format!(
                    "LLM refused implementation after {} attempts. Last refusal:\n{}",
                    max_attempts,
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

        // Parse SEARCH/REPLACE blocks from LLM response
        let search_replace_blocks = parse_search_replace_blocks(&response);

        if search_replace_blocks.is_empty() {
            warn!(
                "NVIDIA Coder returned response with no SEARCH/REPLACE blocks on attempt {}",
                attempt
            );
            errors.push(format!(
                "Attempt {} produced no valid SEARCH/REPLACE blocks (response length: {})",
                attempt,
                response.len()
            ));
            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted — signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = "The previous attempt did not produce valid SEARCH/REPLACE blocks. Output SEARCH/REPLACE blocks with the file path, <<<<<<< SEARCH, =======, >>>>>>> REPLACE format. Do NOT output unified diff patches.".to_string();
            attempt += 1;
            continue;
        }

        // Validate block quality before attempting to apply
        if let Err(e) = validate_search_replace_blocks(&response, &search_replace_blocks) {
            warn!(
                "NVIDIA Coder SEARCH/REPLACE validation failed on attempt {}: {}",
                attempt, e
            );
            errors.push(format!("Attempt {} validation failed: {}", attempt, e));
            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted — signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = format!(
                "The previous attempt produced invalid SEARCH/REPLACE blocks: {}\n\n\
                 Make sure each block has meaningful SEARCH content (at least {} characters) \
                 and non-empty REPLACE content.",
                e, MIN_SEARCH_LENGTH
            );
            attempt += 1;
            continue;
        }

        info!(
            "NVIDIA Coder produced {} SEARCH/REPLACE blocks on attempt {}",
            search_replace_blocks.len(),
            attempt
        );

        // Apply directly via SEARCH/REPLACE with GitSnapshot rollback
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (patch, changed_paths) = match apply_search_replace_blocks(root, &search_replace_blocks)
        {
            Ok(result) => result,
            Err(e) => {
                warn!("SEARCH/REPLACE failed on attempt {}: {}", attempt, e);
                errors.push(format!("Attempt {} SEARCH/REPLACE failed: {}", attempt, e));
                let max_attempts = effective_max_attempts(args.max_attempts);
                if attempt >= max_attempts {
                    return Ok(signal_scope_reduction(ctx, errors));
                }
                last_error = format!(
                    "SEARCH/REPLACE failed: {}\n\n\
                     ---\n\n\
                     Make sure SEARCH lines match the source files EXACTLY.",
                    e
                );
                attempt += 1;
                continue;
            }
        };

        // Write patch to temp file for verification/queueing
        let temp_patch =
            tempfile::NamedTempFile::new().context("failed to create temp patch file")?;
        std::fs::write(temp_patch.path(), &patch)
            .context("failed to write patch from SEARCH/REPLACE")?;

        info!("SEARCH/REPLACE applied successfully, continuing to verification");

        // Verify with check-fast.ps1 (unless dry run)
        if !ctx.dry_run {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            match run_check_fast(root) {
                Ok(ref check_output) => {
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

                    // Gate: auto-apply if --auto-apply flag OR config's enable_auto_apply
                    let config_auto_apply =
                        crate::bacon_core::PipelineConfig::from_bacon_toml().enable_auto_apply;
                    let should_auto_apply = args.auto_apply || config_auto_apply;
                    if should_auto_apply {
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
                                let max_attempts = effective_max_attempts(args.max_attempts);
                                if attempt >= max_attempts {
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
                    let max_attempts = effective_max_attempts(args.max_attempts);
                    if attempt >= max_attempts {
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

    // ========================================================================
    // validate_search_replace_blocks Tests
    // ========================================================================

    fn make_block(file: &str, search: &str, replace: &str) -> (String, String, String) {
        (file.to_string(), search.to_string(), replace.to_string())
    }

    #[test]
    fn test_validation_passes_valid_blocks() {
        let response = "a".repeat(100);
        let blocks = vec![make_block(
            "src/main.rs",
            "fn old_function() {\n    return 1;\n}",
            "fn new_function() {\n    return 2;\n}",
        )];
        assert!(validate_search_replace_blocks(&response, &blocks).is_ok());
    }

    #[test]
    fn test_validation_passes_valid_blocks_multiple_files() {
        let response = "b".repeat(100);
        let blocks = vec![
            make_block(
                "src/main.rs",
                "fn old_function() {\n    return 1;\n}",
                "fn new_function() {\n    return 2;\n}",
            ),
            make_block(
                "src/lib.rs",
                "pub fn helper() {}\n",
                "pub fn helper_v2() {}\n",
            ),
        ];
        assert!(validate_search_replace_blocks(&response, &blocks).is_ok());
    }

    #[test]
    fn test_validation_rejects_short_response() {
        let response = "short";
        let blocks = vec![make_block(
            "src/main.rs",
            "fn old_function() {\n    return 1;\n}",
            "fn new_function() {\n    return 2;\n}",
        )];
        let err = validate_search_replace_blocks(response, &blocks).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("Response too short"),
            "Expected 'Response too short', got: {}",
            msg
        );
    }

    #[test]
    fn test_validation_rejects_trivial_search() {
        let response = "x".repeat(100);
        let blocks = vec![make_block(
            "src/main.rs",
            "short",
            "fn new_function() {\n    return 2;\n}",
        )];
        let err = validate_search_replace_blocks(&response, &blocks).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("SEARCH text too short"),
            "Expected 'SEARCH text too short', got: {}",
            msg
        );
    }

    #[test]
    fn test_validation_rejects_empty_replace() {
        let response = "x".repeat(100);
        let blocks = vec![make_block(
            "src/main.rs",
            "fn old_function() {\n    return 1;\n}",
            "",
        )];
        let err = validate_search_replace_blocks(&response, &blocks).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("REPLACE text is empty"),
            "Expected 'REPLACE text is empty', got: {}",
            msg
        );
    }

    #[test]
    fn test_validation_rejects_duplicate_blocks() {
        let response = "x".repeat(100);
        let blocks = vec![
            make_block(
                "src/main.rs",
                "fn old_function() {\n    return 1;\n}",
                "fn new_function() {\n    return 2;\n}",
            ),
            make_block(
                "src/main.rs",
                "fn old_function() {\n    return 1;\n}",
                "fn new_function() {\n    return 2;\n}",
            ),
        ];
        let err = validate_search_replace_blocks(&response, &blocks).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("Duplicate SEARCH block"),
            "Expected 'Duplicate SEARCH block', got: {}",
            msg
        );
    }

    #[test]
    fn test_validation_different_files_same_search_ok() {
        let response = "x".repeat(100);
        let blocks = vec![
            make_block(
                "src/main.rs",
                "fn old_function() {\n    return 1;\n}",
                "fn new_function() {\n    return 2;\n}",
            ),
            make_block(
                "src/lib.rs",
                "fn old_function() {\n    return 1;\n}",
                "fn new_function() {\n    return 2;\n}",
            ),
        ];
        // Same search text in different files is valid (e.g., similar code in multiple locations)
        assert!(validate_search_replace_blocks(&response, &blocks).is_ok());
    }
}
