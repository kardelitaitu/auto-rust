use anyhow::{Context, Result};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bacon_core::{collect_source_context, read_role_prompt, GitSnapshot};

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

fn coder_responses_dir(root: &Path) -> PathBuf {
    root.join(".bacon").join("sessions").join("coder_responses")
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
/// Returns a list of (`file_path`, `search_text`, `replace_text`) tuples.
fn parse_search_replace_blocks(response: &str) -> Vec<(String, String, String)> {
    let normalized = normalize_search_replace_fences(response);

    // Match blocks where the file path is on a line before <<<<<<< SEARCH
    let re = regex::Regex::new(
        r"(?ms)(?:^|\n)\s*([^\n]+?\.(?:rs|toml|md|json|ps1|sh|js|ts|css|html))\s*\n\s*<<<<<<< SEARCH\s*\n(.*?)\n\s*=======\s*\n(.*?)\n\s*>>>>>>> REPLACE"
    ).expect("invalid SEARCH/REPLACE regex");

    let mut blocks = Vec::new();
    for cap in re.captures_iter(&normalized) {
        let file_path = cap[1].trim().to_string();
        let search = cap[2].to_string();
        let replace = cap[3].to_string();
        if !file_path.is_empty() {
            blocks.push((file_path, search, replace));
        }
    }
    blocks
}

fn normalize_search_replace_fences(response: &str) -> String {
    response
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            match trimmed {
                "<<<<< SEARCH" | "<<<<<< SEARCH" | "<<<<<<< SEARCH" => "<<<<<<< SEARCH",
                "======" | "=======" | "========" => "=======",
                ">>>>>> REPLACE" | ">>>>>>> REPLACE" => ">>>>>>> REPLACE",
                _ => line,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_speculative_prose(response: &str) -> bool {
    let lower = response.to_lowercase();
    lower.contains("based on the assumption")
        || lower.contains("actual implementation may vary")
        || response
            .lines()
            .any(|line| line.trim_start().starts_with("Note:"))
}

fn stray_patch_marker_line(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.len() >= 4
            && trimmed
                .chars()
                .all(|ch| ch == '<' || ch == '>' || ch == '=')
        {
            Some(trimmed.to_string())
        } else {
            None
        }
    })
}

/// Validate parsed SEARCH/REPLACE blocks for quality and correctness.
///
/// Checks performed:
/// - **Response too short**: Raw response is below `MIN_RESPONSE_LENGTH`
/// - **Trivial SEARCH text**: SEARCH content is below `MIN_SEARCH_LENGTH` bytes
/// - **Trivial REPLACE text**: REPLACE content is below `MIN_REPLACE_LENGTH` bytes
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
            "Response too short ({} bytes, minimum {}) - likely not a real SEARCH/REPLACE output",
            response.trim().len(),
            MIN_RESPONSE_LENGTH
        );
    }

    if contains_speculative_prose(response) {
        anyhow::bail!(
            "Response contains speculative prose or assumptions; output concrete SEARCH/REPLACE blocks only"
        );
    }

    // 2. Block-level: reject blocks with trivial content
    for (i, (file_path, search, replace)) in blocks.iter().enumerate() {
        let search_trimmed = search.trim();
        let replace_trimmed = replace.trim();

        if let Some(marker) = stray_patch_marker_line(search_trimmed) {
            anyhow::bail!(
                "Block #{} ({}): SEARCH text contains stray patch marker `{}`",
                i + 1,
                file_path,
                marker
            );
        }

        if let Some(marker) = stray_patch_marker_line(replace_trimmed) {
            anyhow::bail!(
                "Block #{} ({}): REPLACE text contains stray patch marker `{}`",
                i + 1,
                file_path,
                marker
            );
        }

        if search_trimmed.len() < MIN_SEARCH_LENGTH {
            anyhow::bail!(
                "Block #{} ({}): SEARCH text too short ({} bytes, minimum {}) - likely hallucinated",
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
    for (file_path, search, _) in blocks {
        let key = format!("{file_path}::{search}");
        if !seen.insert(key.clone()) {
            anyhow::bail!(
                "Duplicate SEARCH block found for {file_path} — LLM likely hallucinated or repeated output"
            );
        }
    }

    Ok(())
}

/// Find a unique whitespace-insensitive match and map it back to the original content span.
fn find_unique_whitespace_insensitive_match(
    content: &str,
    search: &str,
) -> Result<Option<(usize, usize)>> {
    let search_flat: String = search.chars().filter(|c| !c.is_whitespace()).collect();
    if search_flat.is_empty() {
        anyhow::bail!("SEARCH text is empty after whitespace normalization");
    }

    let content_flat: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    let matches: Vec<_> = content_flat.match_indices(&search_flat).collect();
    match matches.len() {
        0 => Ok(None),
        1 => {
            let flat_pos = matches[0].0;

            // Map flat position back to original content position by counting chars.
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

            // Find end byte offset in original content by walking forward.
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

            Ok(Some((orig_pos, end_pos)))
        }
        count => anyhow::bail!(
            "SEARCH text matches {count} locations after whitespace normalization; use a more specific block"
        ),
    }
}

#[derive(Debug)]
struct NonBlankLine<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn nonblank_lines(text: &str) -> Vec<NonBlankLine<'_>> {
    let mut lines = Vec::new();
    let mut offset = 0usize;

    for segment in text.split_inclusive('\n') {
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let line_text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        let start = offset;
        let end = offset + line_text.len();

        if !line_text.trim().is_empty() {
            lines.push(NonBlankLine {
                text: line_text,
                start,
                end,
            });
        }

        offset += segment.len();
    }

    if offset < text.len() {
        let line_text = &text[offset..];
        if !line_text.trim().is_empty() {
            lines.push(NonBlankLine {
                text: line_text,
                start: offset,
                end: text.len(),
            });
        }
    }

    lines
}

fn find_unique_blank_line_insensitive_match(
    content: &str,
    search: &str,
) -> Result<Option<(usize, usize)>> {
    let search_lines = nonblank_lines(search);
    if search_lines.is_empty() {
        anyhow::bail!("SEARCH text has no nonblank lines");
    }

    let content_lines = nonblank_lines(content);
    let mut matches = Vec::new();
    for window in content_lines.windows(search_lines.len()) {
        if window
            .iter()
            .zip(search_lines.iter())
            .all(|(content_line, search_line)| content_line.text == search_line.text)
        {
            let start = window.first().expect("nonempty window").start;
            let end = window.last().expect("nonempty window").end;
            matches.push((start, end));
        }
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        count => anyhow::bail!(
            "SEARCH text matches {count} locations when ignoring blank lines; use a more specific block"
        ),
    }
}

fn allows_whitespace_insensitive_fallback(file_path: &str) -> bool {
    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        != Some("rs")
}

fn suggest_existing_rust_target(root: &Path, rel: &Path) -> Option<(PathBuf, String)> {
    if rel.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return None;
    }

    let parent = rel.parent().unwrap_or_else(|| Path::new(""));
    let stem = rel.file_stem()?.to_str()?;
    let candidates = [stem.strip_suffix("_test"), stem.strip_suffix("_tests")];

    for base in candidates.into_iter().flatten() {
        let candidate = parent.join(format!("{base}.rs"));
        let abs = root.join(&candidate);
        if abs.is_file() {
            let note = std::fs::read_to_string(&abs).ok().map_or_else(
                || "That looks like the closest existing Rust source file.".to_string(),
                |content| {
                    if content.contains("mod tests") || content.contains("#[cfg(test)]") {
                        "That file already contains an inline `mod tests` block.".to_string()
                    } else {
                        "That looks like the closest existing Rust source file.".to_string()
                    }
                },
            );
            return Some((candidate, note));
        }
    }

    None
}

fn normalize_scope_path(path: &str) -> String {
    path.trim_start_matches(&['/', '\\'][..]).replace('\\', "/")
}

fn yaml_sequence_strings(value: &serde_yml::Value) -> Vec<String> {
    value
        .as_sequence()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(normalize_scope_path)
                .collect()
        })
        .unwrap_or_default()
}

fn declared_spec_code_paths(spec_path: &Path) -> Vec<String> {
    let yaml_path = spec_path.join("spec.yaml");
    let value = std::fs::read_to_string(&yaml_path)
        .ok()
        .and_then(|content| serde_yml::from_str::<serde_yml::Value>(&content).ok());

    let Some(value) = value else {
        return Vec::new();
    };

    value
        .get("files")
        .and_then(|files| files.get("code"))
        .map(yaml_sequence_strings)
        .unwrap_or_default()
}

fn plan_spec_code_paths(spec_path: &Path) -> Vec<String> {
    let spec_text = format!(
        "{}\n{}",
        spec_io::read_spec_file(spec_path, "plan.md"),
        spec_io::read_spec_file(spec_path, "validation.md")
    );

    crate::bacon_core::extract_repo_file_refs(&spec_text)
        .into_iter()
        .filter(|path| {
            path.starts_with("src/")
                || path.starts_with("tests/")
                || path.starts_with("benches/")
                || path.starts_with("examples/")
                || path.starts_with("scripts/")
                || path.starts_with("config/")
                || matches!(
                    path.as_str(),
                    "Cargo.toml"
                        | "build.rs"
                        | "rust-toolchain.toml"
                        | "check-fast.ps1"
                        | "check.ps1"
                )
        })
        .collect()
}

fn response_preview(response: &str, max_chars: usize) -> String {
    let trimmed = response.trim();
    let preview: String = trimmed.chars().take(max_chars).collect();
    if trimmed.chars().count() > max_chars {
        format!("{preview}...[truncated]")
    } else {
        preview
    }
}

fn save_coder_response(
    root: &Path,
    ctx: &PipelineCtx,
    attempt: u32,
    response: &str,
) -> Result<PathBuf> {
    let responses_dir = coder_responses_dir(root);
    std::fs::create_dir_all(&responses_dir).context("failed to create coder_responses dir")?;

    let spec_id = ctx
        .spec_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or_else(
            || "unknown-spec".to_string(),
            |n| n.to_string_lossy().to_string(),
        );
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let response_path = responses_dir.join(format!(
        "{}_attempt_{}_{}.txt",
        safe_file_stem(&spec_id),
        attempt,
        timestamp_ms
    ));

    std::fs::write(&response_path, response)
        .with_context(|| format!("failed to write {}", response_path.display()))?;
    Ok(response_path)
}

fn spec_scope_paths(spec_path: &Path) -> Vec<String> {
    let plan_paths = plan_spec_code_paths(spec_path);
    if !plan_paths.is_empty() {
        return plan_paths;
    }
    declared_spec_code_paths(spec_path)
}

fn path_allowed_by_scope(path: &str, scope_paths: &[String]) -> bool {
    let path = normalize_scope_path(path);
    scope_paths.iter().any(|scope| {
        let scope = normalize_scope_path(scope);
        if scope.ends_with('/') {
            path.starts_with(&scope)
        } else {
            path == scope
        }
    })
}

fn validate_blocks_within_spec_scope(
    ctx: &PipelineCtx,
    blocks: &[(String, String, String)],
) -> Result<()> {
    let Some(spec_path) = ctx.spec_path.as_deref() else {
        return Ok(());
    };

    let scope_paths = spec_scope_paths(spec_path);
    if scope_paths.is_empty() {
        return Ok(());
    }

    let invalid: Vec<String> = blocks
        .iter()
        .map(|(file_path, _, _)| normalize_scope_path(file_path))
        .filter(|file_path| !path_allowed_by_scope(file_path, &scope_paths))
        .collect();

    if invalid.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "SEARCH/REPLACE targets are outside the active spec scope: {}. Allowed code paths: {}",
        invalid.join(", "),
        scope_paths.join(", ")
    );
}

/// Detect a Rust item boundary that was accidentally merged onto one line.
///
/// This catches invalid patterns like `}    #[test]`, which rustfmt will reject
/// and which usually indicate the LLM dropped a newline between items.
fn detect_merged_rust_attribute_boundary(content: &str) -> Option<usize> {
    content.lines().enumerate().find_map(|(line_idx, line)| {
        let trimmed = line.trim_start();
        trimmed.strip_prefix('}').and_then(|rest| {
            if rest.trim_start().starts_with("#[") {
                Some(line_idx + 1)
            } else {
                None
            }
        })
    })
}

fn validate_rust_item_boundaries(root: &Path, rel_paths: &[PathBuf]) -> Result<()> {
    for rel in rel_paths {
        if rel.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let abs_path = root.join(rel);
        let content = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("failed to read {}", abs_path.display()))?;
        if let Some(line_no) = detect_merged_rust_attribute_boundary(&content) {
            anyhow::bail!(
                "merged Rust item boundary detected in {} at line {} - keep `}}` and `#[...]` on separate lines",
                abs_path.display(),
                line_no
            );
        }
    }
    Ok(())
}

/// Apply SEARCH/REPLACE blocks directly to working tree files with `GitSnapshot` rollback.
///
/// 1. Creates a `GitSnapshot` for safe rollback.
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
            let suggestion = suggest_existing_rust_target(root, &rel);
            let suggestion_text = suggestion
                .as_ref()
                .map(|(suggested_rel, note)| {
                    let source_context =
                        collect_source_context(&suggested_rel.display().to_string(), 1, 120);
                    if source_context.is_empty() {
                        format!("\nDid you mean {}? {}", suggested_rel.display(), note)
                    } else {
                        format!(
                            "\nDid you mean {}? {}\n\n{}",
                            suggested_rel.display(),
                            note,
                            source_context
                        )
                    }
                })
                .unwrap_or_default();
            anyhow::bail!(
                "SEARCH/REPLACE target file not found: {} (resolved to {}){}",
                file_path,
                abs.display(),
                suggestion_text
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
        let allow_fuzzy = allows_whitespace_insensitive_fallback(clean);

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
            info!("SEARCH/REPLACE applied to {clean} (exact match)");
            continue;
        }

        // Fallback: whitespace-insensitive matching for non-Rust files only.
        if allow_fuzzy {
            if let Some((orig_pos, end_pos)) =
                find_unique_whitespace_insensitive_match(&content, search)?
            {
                let new_content =
                    format!("{}{}{}", &content[..orig_pos], replace, &content[end_pos..]);
                std::fs::write(&abs_path, &new_content)
                    .with_context(|| format!("failed to write {}", abs_path.display()))?;
                info!("SEARCH/REPLACE applied to {clean} (whitespace-insensitive match)");
            } else {
                // Rollback before returning error (warn if restore fails)
                if let Err(e) = snapshot.restore() {
                    warn!("Failed to restore snapshot during SEARCH/REPLACE error recovery: {e}");
                }
                anyhow::bail!(
                    "SEARCH text not found in {clean} - even after whitespace-insensitive matching"
                );
            }
        } else {
            // Rust files allow only blank-line drift; nonblank lines must match exactly.
            if let Some((orig_pos, end_pos)) =
                find_unique_blank_line_insensitive_match(&content, search)?
            {
                let new_content =
                    format!("{}{}{}", &content[..orig_pos], replace, &content[end_pos..]);
                std::fs::write(&abs_path, &new_content)
                    .with_context(|| format!("failed to write {}", abs_path.display()))?;
                info!("SEARCH/REPLACE applied to {clean} (blank-line-insensitive Rust match)");
            } else {
                // Rollback before returning error (warn if restore fails)
                if let Err(e) = snapshot.restore() {
                    warn!("Failed to restore snapshot during SEARCH/REPLACE error recovery: {e}");
                }
                anyhow::bail!(
                    "SEARCH text not found in {clean} - exact nonblank line match required for Rust files"
                );
            }
        }
    }

    if let Err(e) = validate_rust_item_boundaries(root, &rel_paths) {
        if let Err(restore_err) = snapshot.restore() {
            warn!("Failed to restore snapshot during Rust boundary validation: {restore_err}");
        }
        anyhow::bail!("SEARCH/REPLACE introduced a merged Rust item boundary; rolled back: {e}");
    }

    // Run check-fast.ps1 to verify
    match run_check_fast(root) {
        Ok(output) => info!(
            "check-fast.ps1 passed after SEARCH/REPLACE: {}",
            output.trim()
        ),
        Err(e) => {
            warn!("check-fast.ps1 failed after SEARCH/REPLACE: {e}");
            snapshot.restore()?;
            anyhow::bail!("SEARCH/REPLACE changes failed check-fast.ps1; rolled back: {e}");
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

/// Apply a queued patch to the main repository using `GitSnapshot` for rollback.
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
        anyhow::bail!("git apply --check failed; rolled back via snapshot:\n{stderr}");
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
        anyhow::bail!("git apply failed; rolled back via snapshot:\n{stderr}");
    }

    // Validate
    let check_fast_output = match run_check_fast(root) {
        Ok(output) => output,
        Err(err) => {
            snapshot
                .restore()
                .context("snapshot rollback failed after check-fast failure")?;
            anyhow::bail!("auto-apply check-fast.ps1 failed; rolled back via snapshot:\n{err}");
        }
    };

    snapshot
        .mark_applied()
        .context("failed to mark snapshot as applied")?;

    info!("Auto-apply passed for {}", queued.patch_path.display());
    Ok(AutoApplyReport {
        _applied_path: Some(archive_approved_patch(root, &queued.patch_path)),
        output: format!("auto-apply passed\nlocal check-fast:\n{check_fast_output}"),
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
        anyhow::bail!("check-fast.ps1 exited with error:\nstdout:\n{stdout}\nstderr:\n{stderr}");
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
    if let Some(spec_path) = &ctx.spec_path {
        let plan = spec_io::read_spec_file(spec_path, "plan.md");
        let validation_spec = spec_io::read_spec_file(spec_path, "validation.md");
        // Collect actual source file contents referenced throughout spec text
        let all_spec_text = format!("{plan}\n{validation_spec}");
        let source_context = collect_source_context(&all_spec_text, 8, 300);
        format!("## Plan\n{plan}\n\n## Validation Criteria\n{validation_spec}{source_context}")
    } else {
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

fn attach_live_source_context(error_text: &str) -> String {
    let source_context = collect_source_context(error_text, 2, 120);
    if source_context.is_empty() {
        error_text.to_string()
    } else {
        format!(
            "{error_text}\n\n{source_context}\n\nIMPORTANT: Use the exact source excerpt above as the basis for the next SEARCH block. Do not reconstruct Rust item boundaries from memory."
        )
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

pub async fn run(llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let system_prompt = role_prompt();

    // Mark spec as in-progress at the start of implementation
    if !ctx.dry_run {
        if let Some(ref spec_path) = ctx.spec_path {
            if let Ok(mut meta) = spec_io::read_spec_meta(spec_path) {
                if meta.status == "approved" {
                    meta.status = "in-progress".to_string();
                    if let Err(e) = spec_io::write_spec_meta(spec_path, &meta) {
                        warn!("Failed to set spec status to in-progress: {e}");
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
                "Implement the following spec using SEARCH/REPLACE blocks.\n\n{spec_context}\n\n\
                 For each file you need to change, output the file path, then:\n\
                 <<<<<<< SEARCH\n\
                 [exact lines to replace]\n\
                 =======\n\
                 [new lines]\n\
                 >>>>>>> REPLACE\n\
                 CRITICAL: Copy SEARCH lines EXACTLY from the source files - character for character.\n\
                 Do NOT output unified diff patches (diff --git). Only output SEARCH/REPLACE blocks.\n\
                 In Rust files, keep `}}` and `#[...]` on separate lines.\n\
                 For Rust files, the SEARCH text must match exactly; do not rely on whitespace-only matching.\n\
                 If a Rust change is a test addition, prefer the existing source file that already contains `mod tests` instead of inventing a new `*_test.rs` file."
            )
        } else {
            format!(
                "Implement the following spec using SEARCH/REPLACE blocks.\n\n\
                 The previous attempt had these issues:\n{last_error}\n\n\
                 Spec context:\n{spec_context}\n\n\
                 For each file you need to change, output the file path, then:\n\
                 <<<<<<< SEARCH\n\
                 [exact lines to replace]\n\
                 =======\n\
                 [new lines]\n\
                 >>>>>>> REPLACE\n\
                 CRITICAL: Copy SEARCH lines EXACTLY from the source files - character for character.\n\
                 Do NOT output unified diff patches (diff --git). Only output SEARCH/REPLACE blocks. \
                 Do not refuse or explain why you can't do it - just produce the blocks.\n\
                 In Rust files, keep `}}` and `#[...]` on separate lines.\n\
                 For Rust files, the SEARCH text must match exactly; do not rely on whitespace-only matching.\n\
                 If a Rust change is a test addition, prefer the existing source file that already contains `mod tests` instead of inventing a new `*_test.rs` file."
            )
        };

        let max_attempts = effective_max_attempts(args.max_attempts);
        info!("NVIDIA Coder calling API (attempt {attempt}/{max_attempts})...");

        let messages = vec![
            crate::llm::ChatMessage::system(system_prompt.clone()),
            crate::llm::ChatMessage::user(prompt),
        ];
        let response = match llm.chat(messages).await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("NVIDIA API call failed on attempt {attempt}: {e}");
                warn!("{err_msg}");
                errors.push(err_msg.clone());
                let max_attempts = effective_max_attempts(args.max_attempts);
                if attempt >= max_attempts {
                    warn!("Max retries exhausted — signalling scope reduction needed");
                    return Ok(signal_scope_reduction(ctx, errors));
                }
                last_error = format!("API error: {e}");
                attempt += 1;
                continue;
            }
        };

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let response_artifact = if ctx.dry_run {
            None
        } else {
            match save_coder_response(root, ctx, attempt, &response) {
                Ok(path) => Some(path),
                Err(e) => {
                    warn!("Failed to save NVIDIA Coder response for attempt {attempt}: {e}");
                    None
                }
            }
        };
        let preview = response_preview(&response, 1200);
        match response_artifact {
            Some(path) => info!(
                "NVIDIA Coder response attempt {} ({} chars; saved to {}):\n{}",
                attempt,
                response.chars().count(),
                path.display(),
                preview
            ),
            None => info!(
                "NVIDIA Coder response attempt {} ({} chars):\n{}",
                attempt,
                response.chars().count(),
                preview
            ),
        }

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
                "NVIDIA Coder refused on attempt {attempt} (consecutive refusal #{consecutive_refusals})"
            );
            errors.push(format!("Attempt {attempt} refused with: {refusal_log}"));

            // 2 consecutive refusals → abort pipeline, no scope reduction
            if consecutive_refusals >= 2 {
                let report = format!(
                    "NVIDIA Coder refused to implement after {consecutive_refusals} consecutive refusals.\n\n\
                     Attempt {attempt}:\n{refusal_log}\n\n\
                     Refusal chain ({consecutive_refusals} consecutive refusals)."
                );
                warn!("NVIDIA Coder: 2 consecutive refusals — aborting pipeline, needs human approval");
                if !ctx.dry_run {
                    if let Some(ref spec_path) = ctx.spec_path {
                        let validation_path = spec_path.join("validation.md");
                        let _ = std::fs::write(
                            &validation_path,
                            format!("# Coder Refusal Report\n\n{report}"),
                        );
                        if let Ok(mut meta) = spec_io::read_spec_meta(spec_path) {
                            meta.status = "needs-human-approval".to_string();
                            if let Err(e) = spec_io::write_spec_meta(spec_path, &meta) {
                                warn!("Failed to persist needs-human-approval to spec.yaml: {e}");
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
                "The previous attempt produced a refusal instead of a patch. Provide a concrete diff. Refusal was: {refusal_log}"
            );
            attempt += 1;
            continue;
        }
        consecutive_refusals = 0;

        // Parse SEARCH/REPLACE blocks from LLM response
        let search_replace_blocks = parse_search_replace_blocks(&response);
        if !search_replace_blocks.is_empty()
            && normalize_search_replace_fences(&response) != response
        {
            warn!(
                "NVIDIA Coder response attempt {attempt} used malformed SEARCH/REPLACE fence markers; normalized before parsing"
            );
        }

        if search_replace_blocks.is_empty() {
            warn!(
                "NVIDIA Coder returned response with no SEARCH/REPLACE blocks on attempt {attempt}"
            );
            errors.push(format!(
                "Attempt {} produced no valid SEARCH/REPLACE blocks (response length: {})",
                attempt,
                response.len()
            ));
            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted - signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            let response_preview = response.lines().take(8).collect::<Vec<_>>().join("\n");
            last_error = format!(
                "The previous attempt did not produce valid SEARCH/REPLACE blocks.\n\nLast response preview:\n{response_preview}\n\nReturn only blocks, with the file path on the first non-empty line, then:\npath/to/file.rs\n<<<<<<< SEARCH\n...\n=======\n...\n>>>>>>> REPLACE\nNo markdown, no headings, no prose, no unified diffs."
            );
            attempt += 1;
            continue;
        }

        // Validate block quality before attempting to apply
        if let Err(e) = validate_search_replace_blocks(&response, &search_replace_blocks) {
            warn!("NVIDIA Coder SEARCH/REPLACE validation failed on attempt {attempt}: {e}");
            errors.push(format!("Attempt {attempt} validation failed: {e}"));
            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted — signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = format!(
                "The previous attempt produced invalid SEARCH/REPLACE blocks: {e}\n\n\
                 Make sure each block has meaningful SEARCH content (at least {MIN_SEARCH_LENGTH} characters) \
                 and non-empty REPLACE content."
            );
            attempt += 1;
            continue;
        }

        if let Err(e) = validate_blocks_within_spec_scope(ctx, &search_replace_blocks) {
            warn!(
                "NVIDIA Coder generated out-of-scope SEARCH/REPLACE blocks on attempt {attempt}: {e}"
            );
            errors.push(format!("Attempt {attempt} scope validation failed: {e}"));
            let max_attempts = effective_max_attempts(args.max_attempts);
            if attempt >= max_attempts {
                warn!("Max retries exhausted - signalling scope reduction needed");
                return Ok(signal_scope_reduction(ctx, errors));
            }
            last_error = format!(
                "{e}\n\nOnly edit files listed by the active spec. Do not introduce unrelated files."
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
                warn!("SEARCH/REPLACE failed on attempt {attempt}: {e}");
                errors.push(format!("Attempt {attempt} SEARCH/REPLACE failed: {e}"));
                let max_attempts = effective_max_attempts(args.max_attempts);
                if attempt >= max_attempts {
                    return Ok(signal_scope_reduction(ctx, errors));
                }
                last_error = attach_live_source_context(&format!(
                    "SEARCH/REPLACE failed: {e}\n\n\
                     ---\n\n\
                     Make sure SEARCH lines match the source files EXACTLY."
                ));
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
        if ctx.dry_run {
            // Dry run — just return the patch response
            info!("Dry run: skipping patch verification and queue, returning patch directly");
            let mut result = PipelineCtx::new(response).with_dry_run(true);
            result.spec_path = ctx.spec_path.clone();
            result.confidence = extracted_confidence;
            return Ok(result);
        } else {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            match run_check_fast(root) {
                Ok(ref check_output) => {
                    info!("Patch verification passed on attempt {attempt}");

                    // Queue the patch
                    let approved_dir = approved_patches_dir(root);
                    std::fs::create_dir_all(&approved_dir)
                        .context("failed to create approved_patches dir")?;
                    let spec_id = ctx
                        .spec_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map_or_else(
                            || "unknown-spec".to_string(),
                            |n| n.to_string_lossy().to_string(),
                        );
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
                                warn!("Auto-apply failed on attempt {attempt}: {e}");
                                errors.push(format!("Attempt {attempt} auto-apply failed: {e}"));
                                let max_attempts = effective_max_attempts(args.max_attempts);
                                if attempt >= max_attempts {
                                    return Ok(signal_scope_reduction(ctx, errors));
                                }
                                last_error = format!(
                                    "Auto-apply failed: {e}\nCheck output: {check_output}\n\n\
        ---\n\nFix the patch and ensure check-fast.ps1 passes."
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
                    warn!("Patch verification failed on attempt {attempt}: {e}");
                    errors.push(format!("Attempt {attempt} verification failed: {e}"));
                    let max_attempts = effective_max_attempts(args.max_attempts);
                    if attempt >= max_attempts {
                        return Ok(signal_scope_reduction(ctx, errors));
                    }
                    let new_error = attach_live_source_context(&format!(
                        "Patch verification failed:\n{e}\n\n\
                                 ---\n\n\
                                 Fix the patch and ensure check-fast.ps1 passes."
                    ));
                    if attempt > 1 && new_error == last_error {
                        repeated_error_count += 1;
                        if repeated_error_count >= 1 {
                            warn!("Same verification error repeated - skipping remaining retries");
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

    #[test]
    fn test_response_preview_truncates_long_output() {
        let preview = response_preview("abcdef", 3);

        assert_eq!(preview, "abc...[truncated]");
    }

    #[test]
    fn test_save_coder_response_writes_raw_reply() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let ctx = PipelineCtx::new("test".to_string());

        let path = save_coder_response(temp.path(), &ctx, 2, "raw coder reply")?;

        assert!(path.exists());
        assert_eq!(
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|name| name.to_str()),
            Some("coder_responses")
        );
        assert_eq!(std::fs::read_to_string(path)?, "raw coder reply");
        Ok(())
    }

    #[test]
    fn test_parse_search_replace_blocks_normalizes_near_miss_fences() {
        let response = r#"
src/adaptive/learning_engine.rs
<<<<< SEARCH
    pub fn new() -> Self {
========
    pub fn new() -> Result<Self> {
>>>>>> REPLACE
"#;

        let blocks = parse_search_replace_blocks(response);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, "src/adaptive/learning_engine.rs");
        assert!(blocks[0].1.contains("pub fn new() -> Self"));
        assert!(blocks[0].2.contains("pub fn new() -> Result<Self>"));
    }

    #[test]
    fn test_normalize_search_replace_fences_keeps_valid_fences() {
        let response = "file.rs\n<<<<<<< SEARCH\nold\n=======\nnew\n>>>>>>> REPLACE";

        assert_eq!(normalize_search_replace_fences(response), response);
    }

    #[test]
    fn test_validation_rejects_speculative_prose_note() {
        let response = "x".repeat(100)
            + "\nNote: The above SEARCH/REPLACE blocks are based on the assumption.";
        let blocks = vec![make_block(
            "src/main.rs",
            "fn old_function() {\n    return 1;\n}",
            "fn new_function() {\n    return 2;\n}",
        )];

        let err = validate_search_replace_blocks(&response, &blocks).unwrap_err();
        let msg = format!("{}", err);

        assert!(msg.contains("speculative prose"));
    }

    #[test]
    fn test_validation_rejects_stray_patch_marker_inside_search() {
        let response = "x".repeat(100);
        let blocks = vec![make_block(
            "src/main.rs",
            "#[allow(dead_code)]\n>>>>>>",
            "fn new_function() {\n    return 2;\n}",
        )];

        let err = validate_search_replace_blocks(&response, &blocks).unwrap_err();
        let msg = format!("{}", err);

        assert!(msg.contains("stray patch marker"));
    }

    #[test]
    fn test_detect_merged_rust_attribute_boundary() {
        assert_eq!(detect_merged_rust_attribute_boundary("}\n#[test]\n"), None);
        assert_eq!(
            detect_merged_rust_attribute_boundary("}    #[test]\n"),
            Some(1)
        );
    }

    #[test]
    fn test_unique_whitespace_insensitive_match() {
        let content = "fn a() {\n    let value = 1;\n}\n";
        let search = "fn a() {\nlet value = 1;\n}";
        let span = find_unique_whitespace_insensitive_match(content, search).unwrap();
        assert!(span.is_some());
    }

    #[test]
    fn test_ambiguous_whitespace_insensitive_match_rejected() {
        let content = "fn a() {}\nfn a() {}\n";
        let search = "fn a() {}";
        let err = find_unique_whitespace_insensitive_match(content, search).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("matches 2 locations"));
    }

    #[test]
    fn test_blank_line_insensitive_match_ignores_only_blank_lines() {
        let content = "fn a() {\n    let one = 1;\n\n    let two = 2;\n}\n";
        let search = "fn a() {\n    let one = 1;\n    let two = 2;\n}";

        let span = find_unique_blank_line_insensitive_match(content, search).unwrap();

        assert_eq!(span, Some((0, content.trim_end().len())));
    }

    #[test]
    fn test_blank_line_insensitive_match_rejects_changed_nonblank_line() {
        let content = "fn a() {\n    let one = 1;\n\n    let two = 2;\n}\n";
        let search = "fn a() {\n    let one = 1;\n    let two = 3;\n}";

        let span = find_unique_blank_line_insensitive_match(content, search).unwrap();

        assert_eq!(span, None);
    }

    #[test]
    fn test_whitespace_insensitive_fallback_disabled_for_rust_files() {
        assert!(!allows_whitespace_insensitive_fallback("src/main.rs"));
        assert!(!allows_whitespace_insensitive_fallback("src/bin/tool.rs"));
        assert!(allows_whitespace_insensitive_fallback("docs/spec.md"));
        assert!(allows_whitespace_insensitive_fallback(".bacon/workflow.md"));
    }

    #[test]
    fn test_attach_live_source_context_includes_rust_file_excerpt() {
        let error = "SEARCH text not found in src/adaptive/learning_engine.rs - exact match required for Rust files";
        let context = attach_live_source_context(error);

        assert!(context.contains("SEARCH text not found"));
        assert!(context.contains("## Relevant Source Files"));
        assert!(context.contains("src/adaptive/learning_engine.rs"));
        assert!(context.contains("IMPORTANT: Use the exact source excerpt above"));
    }

    #[test]
    fn test_suggest_existing_rust_target_prefers_module_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let rel = Path::new("src/adaptive/learning_engine_test.rs");

        let suggestion = suggest_existing_rust_target(root, rel)
            .expect("expected a nearby Rust target suggestion");

        assert_eq!(
            suggestion.0,
            PathBuf::from("src/adaptive/learning_engine.rs")
        );
        assert!(suggestion.1.contains("mod tests") || suggestion.1.contains("closest existing"));
    }

    #[test]
    fn test_validate_blocks_within_spec_scope_rejects_unlisted_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("spec.yaml"),
            r#"
id: test-spec
title: Test Spec
status: approved
owner: pipeline
implementer: pipeline
priority: P1
files:
  code:
    - src/bacon_agent_nvidia/
"#,
        )?;
        std::fs::write(
            temp.path().join("plan.md"),
            "Update src/bacon_agent_nvidia/auditor.rs.",
        )?;
        std::fs::write(temp.path().join("validation.md"), "Run check-fast.ps1.")?;

        let mut ctx = PipelineCtx::new("test".to_string());
        ctx.spec_path = Some(temp.path().to_path_buf());
        let blocks = vec![(
            "src/adaptive/predictive_scorer.rs".to_string(),
            "old content".to_string(),
            "new content".to_string(),
        )];

        let err = validate_blocks_within_spec_scope(&ctx, &blocks).unwrap_err();
        let msg = format!("{}", err);

        assert!(msg.contains("outside the active spec scope"));
        assert!(msg.contains("src/bacon_agent_nvidia/auditor.rs"));
        Ok(())
    }

    #[test]
    fn test_validate_blocks_within_spec_scope_allows_plan_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("spec.yaml"),
            r#"
id: test-spec
title: Test Spec
status: approved
owner: pipeline
implementer: pipeline
priority: P1
files:
  code:
    - src/bacon_agent_nvidia/
"#,
        )?;
        std::fs::write(
            temp.path().join("plan.md"),
            "Update src/bacon_agent_nvidia/auditor.rs.",
        )?;
        std::fs::write(temp.path().join("validation.md"), "Run check-fast.ps1.")?;

        let mut ctx = PipelineCtx::new("test".to_string());
        ctx.spec_path = Some(temp.path().to_path_buf());
        let blocks = vec![(
            "src/bacon_agent_nvidia/auditor.rs".to_string(),
            "old content".to_string(),
            "new content".to_string(),
        )];

        validate_blocks_within_spec_scope(&ctx, &blocks)?;
        Ok(())
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
