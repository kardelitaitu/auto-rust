use anyhow::{Context, Result};
use log::{info, warn};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bacon_core::{collect_source_context, read_role_prompt, GitSnapshot};
use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

fn role_prompt() -> String {
    read_role_prompt("coder")
}

const MAX_ATTEMPTS: u32 = 4;

fn read_spec_file(spec_path: &Path, name: &str) -> String {
    let path = spec_path.join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        warn!("Failed to read spec file {}: {}", path.display(), e);
        String::new()
    })
}

pub async fn run(llm: &Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec_path = match &ctx.spec_path {
        Some(p) => p.clone(),
        None if ctx.dry_run => {
            info!(
                "DRY RUN: no spec path available; would run Coder after Strategist writes a spec"
            );
            return Ok(PipelineCtx::new(ctx.description.clone()).with_dry_run(true));
        }
        None => anyhow::bail!("No spec path provided to Coder"),
    };

    let plan = std::fs::read_to_string(spec_path.join("plan.md"))?;
    let spec_name = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Mark in-progress
    if !ctx.dry_run {
        mark_in_progress(&spec_path)?;
    }

    let system_prompt = role_prompt();

    let mut attempt = 1u32;
    let mut last_error = String::new();
    let mut approved_patch_path: Option<PathBuf> = None;
    let mut consecutive_refusals = 0u32;
    let mut needs_human_approval = false;
    let mut repeated_error_count = 0u32;

    let system_message = ChatMessage::system(system_prompt);

    // Read supplementary spec files for context
    let baseline = read_spec_file(&spec_path, "baseline.md");
    let api_outline = read_spec_file(&spec_path, "internal-api-outline.md");
    let validation_spec = read_spec_file(&spec_path, "validation.md");

    // Collect actual source file contents referenced in spec text
    let all_spec_text = format!(
        "{}\n{}\n{}\n{}",
        plan, baseline, api_outline, validation_spec
    );
    let source_context = collect_source_context(&all_spec_text, 8, 100);
    let mut extracted_confidence: Option<crate::bacon_core::Confidence>;

    loop {
        let user_prompt = if attempt == 1 {
            format!(
                "Implement the following spec ({}):\n\n\
                 Spec path: {}\n\n\
                 ## Plan\n{}\n\n\
                 ## Baseline\n{}\n\n\
                 ## API Changes\n{}\n\n\
                 ## Validation Criteria\n{}\n{}\n\n\
                 Produce one unified diff patch only. Do not edit files \
                 directly. The patch must apply with git apply and pass .\\check-fast.ps1 plus full .\\check.ps1. \
                 Output either raw unified diff starting with diff --git, or one ```diff fenced block.",
                spec_name,
                spec_path.display(),
                plan,
                baseline,
                api_outline,
                validation_spec,
                source_context
            )
        } else {
            format!(
                "The previous implementation attempt failed. Fix these errors:\n\n\
                 {}\n\n\
                 Spec path: {}\n\n\
                 ## Plan\n{}\n\n\
                 ## Baseline\n{}\n\n\
                 ## API Changes\n{}\n\n\
                 ## Validation Criteria\n{}\n{}\n\n\
                 Produce one corrected unified diff patch only. Do not include prose outside the patch.",
                last_error,
                spec_path.display(),
                plan,
                baseline,
                api_outline,
                validation_spec,
                source_context
            )
        };

        let messages = vec![system_message.clone(), ChatMessage::user(&user_prompt)];

        info!(
            "Calling Coder LLM (attempt {}/{})...",
            attempt, MAX_ATTEMPTS
        );
        let response = llm
            .chat(messages)
            .await
            .map_err(|e| anyhow::anyhow!("Coder LLM call failed: {}", e))?;

        // Extract and log confidence
        extracted_confidence = crate::bacon_core::extract_confidence(&response);
        if let Some(ref conf) = extracted_confidence {
            info!("Coder confidence (attempt {}): {}", attempt, conf.as_str());
        }

        println!("=== Coder Output (attempt {}) ===", attempt);
        println!("{}", response);
        println!("================================");

        // Check for LLM refusal BEFORE dry-run check so refusals are always logged
        let is_refusal = is_refusal(&response);

        if is_refusal {
            consecutive_refusals += 1;
            warn!(
                "Coder refused to implement (attempt {}, consecutive refusal #{}): refusal detected",
                attempt, consecutive_refusals
            );
        } else {
            consecutive_refusals = 0;
        }

        if is_refusal && consecutive_refusals >= 2 {
            let report = format!(
                "Coder refused to implement after {} consecutive refusals.\n\n\
                 Attempt {}:\n{}\n\n\
                 Refusal chain ({} consecutive refusals).",
                consecutive_refusals, attempt, response, consecutive_refusals
            );
            warn!("Coder: 2 consecutive refusals — aborting pipeline, needs human approval");
            mark_needs_human_approval(&spec_path, &report)?;
            let mut output = PipelineCtx::new(ctx.description.clone()).with_dry_run(ctx.dry_run);
            output.spec_path = Some(spec_path);
            output.coder_refused = true;
            output.confidence = extracted_confidence;
            return Ok(output);
        }

        if ctx.dry_run {
            if is_refusal {
                info!("DRY RUN: Coder refused implementation — would handle refusal in real run");
            } else {
                info!("DRY RUN: would validate a Coder patch and queue approved artifact");
            }
            break;
        }

        if is_refusal {
            if attempt >= MAX_ATTEMPTS {
                let report = format!(
                    "Coder refused to implement after {} attempts.\nRefusal:\n{}",
                    attempt, response
                );
                info!("Coder exhausted retries — signalling scope reduction needed");
                let output = signal_scope_reduction(ctx, vec![report]);
                return Ok(output);
            }
            // Feed refusal back as the error for retry — the LLM might change its mind
            last_error = format!(
                "The previous attempt produced a refusal instead of a patch. \
                 Provide a concrete diff. The refusal was: {}",
                response.chars().take(200).collect::<String>()
            );
            attempt += 1;
            continue;
        }

        info!(
            "Validating Coder patch with full gate (attempt {})...",
            attempt
        );
        match verify_and_queue_patch(&root, &spec_path, attempt, &response) {
            Ok(mut queued) => {
                info!(
                    "Coder patch queued for approval: {}",
                    queued.patch_path.display()
                );

                approved_patch_path = Some(queued.patch_path.clone());

                if args.auto_apply {
                    info!("Auto-apply requested; checking main worktree gates...");
                    match auto_apply_queued_patch(&root, &queued) {
                        Ok(report) => {
                            if let Some(applied_path) = report.applied_path.clone() {
                                queued.patch_path = applied_path;
                            }
                            queued.auto_apply = Some(report);
                        }
                        Err(err) => {
                            let report = format!("{}", err);
                            warn!("Auto-apply gate rejected the patch: {}", report);
                            needs_human_approval = true;
                            break;
                        }
                    }
                }

                write_implementation_notes(&spec_path, &queued)?;
                mark_implemented(&spec_path)?;
                break;
            }
            Err(err) if attempt >= MAX_ATTEMPTS => {
                let report = format!("{}", err);
                warn!("Max retries exhausted — signalling scope reduction needed");
                let output = signal_scope_reduction(ctx, vec![report]);
                return Ok(output);
            }
            Err(err) => {
                let report = format!("{}", err);
                warn!("Coder patch validation failed on attempt {}", attempt);
                // Repeated error detection: if same error occurs twice, short-circuit
                if attempt > 1 && report == last_error {
                    repeated_error_count += 1;
                    if repeated_error_count >= 1 {
                        warn!(
                            "Same error repeated on attempt {} — skipping remaining retries",
                            attempt
                        );
                        let output = signal_scope_reduction(ctx, vec![report]);
                        return Ok(output);
                    }
                } else {
                    repeated_error_count = 0;
                }
                last_error = report;
                attempt += 1;
            }
        }
    }

    let mut output = PipelineCtx::new(ctx.description.clone());
    output.spec_path = Some(spec_path);
    output.dry_run = ctx.dry_run;
    output.confidence = extracted_confidence;
    output.patch_path = approved_patch_path;
    output.needs_human_approval = needs_human_approval;
    Ok(output)
}

#[derive(Debug)]
struct QueuedPatch {
    patch_path: PathBuf,
    changed_paths: Vec<String>,
    gate: PatchGateReport,
    auto_apply: Option<AutoApplyReport>,
}

#[derive(Debug)]
struct PatchGateReport {
    base_commit: String,
    check_fast_output: String,
    check_output: String,
}

#[derive(Debug, Clone)]
struct AutoApplyReport {
    applied_path: Option<PathBuf>,
    output: String,
}

fn verify_and_queue_patch(
    root: &Path,
    spec_path: &Path,
    attempt: u32,
    response: &str,
) -> Result<QueuedPatch> {
    let patch =
        extract_unified_diff(response).context("Coder output did not include a unified diff")?;
    let changed_paths = changed_paths_from_patch(&patch);
    if changed_paths.is_empty() {
        anyhow::bail!("Coder patch did not contain changed file paths");
    }

    let temp_patch = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_patch.path(), &patch)?;
    let gate = validate_patch_with_full_gate(root, temp_patch.path())?;

    let approved_dir = approved_patches_dir(root);
    std::fs::create_dir_all(&approved_dir)?;
    let spec_id = spec_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown-spec".to_string());
    let patch_name = format!("{}_attempt_{}.diff", safe_file_stem(&spec_id), attempt);
    let patch_path = approved_dir.join(patch_name);
    std::fs::write(&patch_path, patch)?;

    Ok(QueuedPatch {
        patch_path,
        changed_paths,
        gate,
        auto_apply: None,
    })
}

fn extract_unified_diff(text: &str) -> Option<String> {
    if let Some(fenced) = extract_fenced_diff(text) {
        return Some(fenced);
    }

    let start = text.find("diff --git ").or_else(|| text.find("--- a/"))?;
    let diff = text[start..].trim();
    if diff.is_empty() {
        None
    } else {
        Some(format_patch(diff))
    }
}

fn extract_fenced_diff(text: &str) -> Option<String> {
    let mut in_diff = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if !in_diff && (trimmed.starts_with("```diff") || trimmed.starts_with("```patch")) {
            in_diff = true;
            continue;
        }
        if in_diff && trimmed.starts_with("```") {
            let patch = lines.join("\n");
            return (!patch.trim().is_empty()).then(|| format_patch(patch.trim()));
        }
        if in_diff {
            lines.push(line);
        }
    }

    None
}

fn format_patch(patch: &str) -> String {
    let normalized = patch.replace("\r\n", "\n");
    if normalized.ends_with('\n') {
        normalized
    } else {
        format!("{}\n", normalized)
    }
}

fn changed_paths_from_patch(patch: &str) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("--- a/") {
            if path != "/dev/null" {
                paths.insert(path.replace('\\', "/"));
            }
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            if path != "/dev/null" {
                paths.insert(path.replace('\\', "/"));
            }
        }
    }
    paths.into_iter().collect()
}

fn approved_patches_dir(root: &Path) -> PathBuf {
    root.join(".bacon")
        .join("sessions")
        .join("approved_patches")
}

fn safe_file_stem(input: &str) -> String {
    let stem: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem: String = stem.trim_matches('_').chars().take(80).collect();
    if stem.is_empty() {
        "patch".to_string()
    } else {
        stem
    }
}

fn validate_patch_with_full_gate(root: &Path, patch_path: &Path) -> Result<PatchGateReport> {
    let base_commit = current_head(root)?;
    let temp = tempfile::tempdir()?;
    let worktree = temp.path().join("repo");

    let clone_output = Command::new("git")
        .args(["clone", "--quiet", "--shared"])
        .arg(root)
        .arg(&worktree)
        .output()
        .context("failed to clone repository for patch validation")?;
    ensure_success("git clone", clone_output)?;

    let check_output = Command::new("git")
        .arg("-C")
        .arg(&worktree)
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
    ensure_success("git apply --check", check_output)?;

    let apply_output = Command::new("git")
        .arg("-C")
        .arg(&worktree)
        .args([
            "apply",
            "--recount",
            "--allow-overlap",
            "--ignore-whitespace",
            "--ignore-space-change",
        ])
        .arg(patch_path)
        .output()
        .context("failed to run git apply")?;
    ensure_success("git apply", apply_output)?;

    let check_fast_output = run_check_fast(&worktree)?;
    let check_output = run_full_check(&worktree)?;

    Ok(PatchGateReport {
        base_commit,
        check_fast_output,
        check_output,
    })
}

fn run_check_fast(worktree: &Path) -> Result<String> {
    run_powershell_script(worktree, "check-fast.ps1")
}

fn run_full_check(worktree: &Path) -> Result<String> {
    run_powershell_script(worktree, "check.ps1")
}

fn run_powershell_script(worktree: &Path, script_name: &str) -> Result<String> {
    let script = worktree.join(script_name);
    if !script.is_file() {
        anyhow::bail!("{} is missing from validation workspace", script_name);
    }

    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };
    let args = vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-File".to_string(),
        script.to_string_lossy().to_string(),
    ];

    let output = Command::new(shell)
        .args(args)
        .current_dir(worktree)
        .output()
        .with_context(|| format!("failed to run {}", script_name))?;
    let combined = combined_output(&output);
    if !output.status.success() {
        anyhow::bail!("{} failed:\n{}", script_name, combined);
    }

    Ok(combined)
}

fn auto_apply_queued_patch(root: &Path, queued: &QueuedPatch) -> Result<AutoApplyReport> {
    let current = current_head(root)?;
    if current != queued.gate.base_commit {
        anyhow::bail!(
            "auto-apply rejected: repository HEAD changed from {} to {}",
            queued.gate.base_commit,
            current
        );
    }

    ensure_patch_targets_clean(root, &queued.changed_paths)?;

    // Capture pre-apply state via GitSnapshot, then apply and verify
    let changed_pathsbuf: Vec<PathBuf> = queued.changed_paths.iter().map(PathBuf::from).collect();
    let snapshot = GitSnapshot::create(root, &changed_pathsbuf)
        .context("failed to create pre-apply snapshot")?;

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
        .context("failed to run main git apply --check")?;
    ensure_success("main git apply --check", check_output)?;

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
        .context("failed to run main git apply")?;
    ensure_success("main git apply", apply_output)?;

    let local_check = match run_check_fast(root) {
        Ok(output) => output,
        Err(err) => {
            snapshot
                .restore()
                .context("snapshot rollback failed after check-fast failure")?;
            anyhow::bail!(
                "auto-apply local check-fast failed; patch rolled back via snapshot:\n{}",
                err
            );
        }
    };

    snapshot
        .mark_applied()
        .context("failed to mark snapshot as applied")?;

    let applied_path = archive_applied_patch(root, &queued.patch_path);
    let output = format!(
        "auto-apply passed\nbase: {}\nlocal check-fast:\n{}",
        queued.gate.base_commit, local_check
    );

    Ok(AutoApplyReport {
        applied_path,
        output,
    })
}

fn current_head(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to read git HEAD")?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse HEAD failed:\n{}", combined_output(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_patch_targets_clean(root: &Path, changed_paths: &[String]) -> Result<()> {
    if changed_paths.is_empty() {
        anyhow::bail!("auto-apply rejected: patch has no changed paths");
    }

    ensure_git_quiet(root, &["diff", "--quiet", "--"], changed_paths)
        .context("auto-apply rejected: patch target files have unstaged changes")?;
    ensure_git_quiet(root, &["diff", "--cached", "--quiet", "--"], changed_paths)
        .context("auto-apply rejected: patch target files have staged changes")?;

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "--"]);
    for path in changed_paths {
        command.arg(path);
    }
    let output = command
        .output()
        .context("failed to check untracked patch targets")?;
    if !output.status.success() {
        anyhow::bail!(
            "git ls-files --others failed:\n{}",
            combined_output(&output)
        );
    }
    let untracked = String::from_utf8_lossy(&output.stdout);
    if !untracked.trim().is_empty() {
        anyhow::bail!(
            "auto-apply rejected: patch target files have untracked conflicts:\n{}",
            untracked
        );
    }

    Ok(())
}

fn ensure_git_quiet(root: &Path, args: &[&str], paths: &[String]) -> Result<()> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    for path in paths {
        command.arg(path);
    }
    let output = command
        .output()
        .context("failed to run git cleanliness check")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", combined_output(&output))
    }
}

fn archive_applied_patch(root: &Path, patch_path: &Path) -> Option<PathBuf> {
    let applied_dir = approved_patches_dir(root).join("applied");
    if let Err(err) = std::fs::create_dir_all(&applied_dir) {
        warn!("failed to create applied patch archive: {}", err);
        return None;
    }

    let file_name = patch_path.file_name()?;
    let applied_path = applied_dir.join(file_name);
    if let Err(err) = std::fs::rename(patch_path, &applied_path) {
        warn!("failed to move applied patch into archive: {}", err);
        return None;
    }
    Some(applied_path)
}

fn ensure_success(label: &str, output: std::process::Output) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{} failed:\n{}", label, combined_output(&output))
    }
}

fn combined_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        stdout.to_string()
    } else {
        format!("{}\n{}", stdout, stderr)
    }
}

fn write_implementation_notes(spec_path: &Path, queued: &QueuedPatch) -> Result<()> {
    let changed = queued
        .changed_paths
        .iter()
        .map(|path| format!("- `{}`", path))
        .collect::<Vec<_>>()
        .join("\n");
    let auto_apply = queued
        .auto_apply
        .as_ref()
        .map(|report| {
            format!(
                "\n## Auto Apply\n\n- Applied patch archive: `{}`\n\n```text\n{}\n```\n",
                report
                    .applied_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(not archived)".to_string()),
                report.output
            )
        })
        .unwrap_or_default();
    let notes = format!(
        "# Implementation Notes\n\n\
         Coder produced a verified patch candidate.\n\n\
         - Patch: `{}`\n\
         - Base commit: `{}`\n\
         - Validation: `.\\check-fast.ps1` and `.\\check.ps1`\n\n\
         ## Changed Paths\n\n\
         {}\n\n\
         ## check-fast.ps1 Output\n\n\
         ```text\n{}\
         ```\n\n\
         ## check.ps1 Output\n\n\
         ```text\n{}\
         ```\n\
         {}\n",
        queued.patch_path.display(),
        queued.gate.base_commit,
        changed,
        queued.gate.check_fast_output,
        queued.gate.check_output,
        auto_apply
    );
    std::fs::write(spec_path.join("implementation-notes.md"), notes)?;
    Ok(())
}

fn mark_in_progress(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "in-progress".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status set to: in-progress");
    Ok(())
}

fn mark_implemented(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "implemented".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status set to: implemented");
    Ok(())
}

fn mark_needs_human_approval(path: &Path, report: &str) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "needs-human-approval".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    std::fs::write(
        path.join("validation.md"),
        format!("# Coder Failure Report\n\n{}", report),
    )?;
    info!("Spec status set to: needs-human-approval (retries exhausted)");
    Ok(())
}

fn mark_approved(path: &Path) -> Result<()> {
    let mut meta = spec_io::read_spec_meta(path)?;
    meta.status = "approved".to_string();
    spec_io::write_spec_meta(path, &meta)?;
    info!("Spec status reset to: approved (ready for scope reduction retry)");
    Ok(())
}

fn is_refusal(response: &str) -> bool {
    crate::bacon_core::is_refusal(response)
}

fn signal_scope_reduction(ctx: &PipelineCtx, errors: Vec<String>) -> PipelineCtx {
    let mut output = PipelineCtx::new(ctx.description.clone());
    output.scope_reduction_needed = true;
    output.coder_errors = errors;
    output.scope_reduction_count = ctx.scope_reduction_count + 1;
    output.spec_path = ctx.spec_path.clone();
    output.dry_run = ctx.dry_run;
    output.confidence = ctx.confidence;
    if let Some(ref spec_path) = ctx.spec_path {
        if !ctx.dry_run {
            let _ = mark_approved(spec_path);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_command(cwd: &Path, program: &str, args: &[&str]) -> Result<()> {
        let output = Command::new(program).args(args).current_dir(cwd).output()?;
        assert!(
            output.status.success(),
            "{} {:?} failed\nstdout: {}\nstderr: {}",
            program,
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    fn write_tiny_rust_repo(root: &Path, check_script: &str) -> Result<()> {
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"bacon-coder-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )?;
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn answer() -> i32 {\n    41\n}\n",
        )?;
        std::fs::copy(
            PathBuf::from(".").join("check-fast.ps1"),
            root.join("check-fast.ps1"),
        )?;
        std::fs::write(root.join("check.ps1"), check_script)?;

        run_command(root, "git", &["init"])?;
        run_command(
            root,
            "git",
            &["config", "user.email", "bacon@example.invalid"],
        )?;
        run_command(root, "git", &["config", "user.name", "Bacon Test"])?;
        run_command(root, "git", &["add", "."])?;
        run_command(root, "git", &["commit", "-m", "initial"])?;
        Ok(())
    }

    fn tiny_patch() -> &'static str {
        concat!(
            "diff --git a/src/lib.rs b/src/lib.rs\n",
            "--- a/src/lib.rs\n",
            "+++ b/src/lib.rs\n",
            "@@ -1,3 +1,3 @@\n",
            " pub fn answer() -> i32 {\n",
            "-    41\n",
            "+    42\n",
            " }\n",
        )
    }

    fn passing_check_script() -> &'static str {
        "Write-Host 'check.ps1 pass'\nexit 0\n"
    }

    fn failing_check_script() -> &'static str {
        "Write-Error 'check.ps1 fail'\nexit 1\n"
    }

    #[test]
    fn extracts_fenced_unified_diff() {
        let response = format!("Here is the patch:\n```diff\n{}```", tiny_patch());
        let patch = extract_unified_diff(&response).expect("patch should be extracted");
        assert!(patch.starts_with("diff --git a/src/lib.rs b/src/lib.rs"));
        assert_eq!(changed_paths_from_patch(&patch), vec!["src/lib.rs"]);
    }

    #[test]
    fn coder_fixture_queues_tiny_patch_after_check_fast_passes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("fixture");
        std::fs::create_dir_all(&root)?;
        write_tiny_rust_repo(&root, passing_check_script())?;

        let spec_path = root.join("docs/specs/_active/0001-tiny-coder-fixture");
        std::fs::create_dir_all(&spec_path)?;
        let queued = verify_and_queue_patch(&root, &spec_path, 1, tiny_patch())?;

        assert!(queued.patch_path.is_file());
        assert!(queued.patch_path.ends_with(Path::new(
            ".bacon/sessions/approved_patches/0001-tiny-coder-fixture_attempt_1.diff"
        )));
        assert_eq!(queued.changed_paths, vec!["src/lib.rs"]);
        assert!(
            queued.gate.check_fast_output.contains("check-fast: pass"),
            "{}",
            queued.gate.check_fast_output
        );
        assert!(
            queued.gate.check_output.contains("check.ps1 pass"),
            "{}",
            queued.gate.check_output
        );

        Ok(())
    }

    #[test]
    fn coder_fixture_rejects_patch_when_full_check_fails() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("fixture");
        std::fs::create_dir_all(&root)?;
        write_tiny_rust_repo(&root, failing_check_script())?;

        let spec_path = root.join("docs/specs/_active/0001-full-check-failure");
        std::fs::create_dir_all(&spec_path)?;
        let err = verify_and_queue_patch(&root, &spec_path, 1, tiny_patch()).unwrap_err();

        assert!(err.to_string().contains("check.ps1 failed"), "{err}");
        assert!(!approved_patches_dir(&root).exists());
        Ok(())
    }

    #[test]
    fn auto_apply_rejects_dirty_patch_targets() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("fixture");
        std::fs::create_dir_all(&root)?;
        write_tiny_rust_repo(&root, passing_check_script())?;

        let spec_path = root.join("docs/specs/_active/0001-dirty-target");
        std::fs::create_dir_all(&spec_path)?;
        let queued = verify_and_queue_patch(&root, &spec_path, 1, tiny_patch())?;
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn answer() -> i32 {\n    40\n}\n",
        )?;

        let err = auto_apply_queued_patch(&root, &queued).unwrap_err();
        assert!(err.to_string().contains("unstaged changes"), "{}", err);
        Ok(())
    }

    #[test]
    fn auto_apply_rejects_stale_head() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("fixture");
        std::fs::create_dir_all(&root)?;
        write_tiny_rust_repo(&root, passing_check_script())?;

        let spec_path = root.join("docs/specs/_active/0001-stale-head");
        std::fs::create_dir_all(&spec_path)?;
        let queued = verify_and_queue_patch(&root, &spec_path, 1, tiny_patch())?;

        std::fs::write(root.join("README.md"), "new commit\n")?;
        run_command(&root, "git", &["add", "README.md"])?;
        run_command(&root, "git", &["commit", "-m", "move head"])?;

        let err = auto_apply_queued_patch(&root, &queued).unwrap_err();
        assert!(err.to_string().contains("HEAD changed"), "{}", err);
        Ok(())
    }

    #[test]
    fn auto_apply_applies_and_archives_verified_patch() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("fixture");
        std::fs::create_dir_all(&root)?;
        write_tiny_rust_repo(&root, passing_check_script())?;

        let spec_path = root.join("docs/specs/_active/0001-auto-apply");
        std::fs::create_dir_all(&spec_path)?;
        let queued = verify_and_queue_patch(&root, &spec_path, 1, tiny_patch())?;
        let report = auto_apply_queued_patch(&root, &queued)?;

        let lib = std::fs::read_to_string(root.join("src/lib.rs"))?;
        assert!(lib.contains("42"));
        assert!(report
            .applied_path
            .as_ref()
            .is_some_and(|p| p.ends_with(Path::new(
                ".bacon/sessions/approved_patches/applied/0001-auto-apply_attempt_1.diff"
            ))));
        assert!(!queued.patch_path.exists());
        Ok(())
    }

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
        let mut ctx = PipelineCtx::new("test".to_string());
        ctx.spec_path = Some(PathBuf::from("/tmp/spec"));
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
