use anyhow::Result;
use log::{info, warn};
use serde::Serialize;
use std::path::{Path, PathBuf};

use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;
use crate::bacon_core::cli_types::RunArgs;
use crate::bacon_core::run_powershell_with_args;

fn role_prompt() -> String {
    crate::bacon_core::read_role_prompt("strategist")
}

fn build_source_context(seed_text: &str) -> String {
    let source_context = crate::bacon_core::collect_source_context(seed_text, 8, 300);
    if source_context.is_empty() {
        crate::bacon_core::gather_project_context()
    } else {
        source_context
    }
}

fn build_prompt(ctx: &PipelineCtx) -> String {
    let seed_text = if ctx.scope_reduction_needed {
        format!(
            "{}\n\n{}",
            ctx.coder_errors.join("\n---\n"),
            ctx.description
        )
    } else {
        ctx.description.clone()
    };
    let source_context = build_source_context(&seed_text);
    let evidence_guard =
        "IMPORTANT: Treat the source excerpts below as the source of truth. Do not describe a symbol as unused if the excerpts show it being used.";

    if ctx.scope_reduction_needed {
        let errors = ctx.coder_errors.join("\n---\n");
        format!(
            "Review this finding and design a REDUCED-SCOPE implementation plan.\n\n\
             The previous implementation attempt failed with these errors:\n{}\n\n\
             Original finding:\n{}\n\n\
             {}\n\n\
             {}\n\n\
             IMPORTANT: Produce a simpler plan with fewer changes. \
             Reduce the number of files touched, simplify the approach, or \
             limit the scope to the most essential part.\n\n\
             If you believe the original scope is already minimal, output:\n\
             SCOPE_AT_MINIMUM: <explanation>",
            errors, ctx.description, source_context, evidence_guard
        )
    } else {
        format!(
            "Review this finding and design an implementation plan:\n\n\
             {}\n\n\
             {}\n\n\
             {}\n\n\
             If the approach is sound and risks acceptable, output a plan with:\n\
             1. A clear title (one line)\n\
             2. Step-by-step implementation steps\n\
             3. Current state description (baseline)\n\
             4. Any API changes needed\n\
             5. How to validate\n\
             6. Design decisions and risks\n\n\
             If risks are too high, start your response with 'REJECTED:' and explain why.",
            ctx.description, source_context, evidence_guard
        )
    }
}

pub async fn run(_llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let config = crate::bacon_agent_nvidia::cli::nvidia_config_from_args(args);
    let system_prompt = role_prompt();
    let prompt = build_prompt(ctx);

    info!("NVIDIA Strategist calling API with model: {}", config.model);
    let response = nvidia_api::chat(&config, &system_prompt, &prompt).await?;

    if response.trim().starts_with("REJECTED:") {
        anyhow::bail!("Strategist rejected: {}", response);
    }

    // Scope gate: count unique file paths referenced in the plan
    let file_count = crate::bacon_core::count_spec_file_refs(&response);
    if file_count > 3 {
        warn!(
            "Strategist plan references {} files (> 3 recommended). Consider reducing scope.",
            file_count
        );
    }

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Strategist confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Strategist Output ===");
    println!("{}", response);
    println!("================================");

    // Write spec package and validate with spec-lint
    let spec_path = if ctx.dry_run {
        info!("DRY RUN: would write spec package");
        None
    } else {
        let spec_path = write_spec_package(&response)?;
        info!("Spec package created at: {}", spec_path.display());

        // Gate: run spec-lint on the new package
        info!("Running spec-lint validation...");
        let spec_path_arg = spec_path.to_string_lossy().to_string();
        let (passed, output) =
            run_powershell_with_args("spec-lint.ps1", &["-Directory", spec_path_arg.as_str()])?;
        if !passed {
            warn!("spec-lint failed — stopping before Coder");
            anyhow::bail!("generated spec failed spec-lint:\n{}", output);
        } else {
            info!("spec-lint passed");
        }

        Some(spec_path)
    };

    let mut result = PipelineCtx::new(response).with_dry_run(ctx.dry_run);
    result.spec_path = spec_path;
    result.confidence = confidence;
    Ok(result)
}

pub fn write_spec_package(plan: &str) -> Result<PathBuf> {
    let title = extract_title(plan);
    let slug = slugify(&title);
    let active = spec_io::active_dir();
    let (spec_dir, number) = spec_io::allocate_spec_dir(&active, &slug)?;
    let dir_name = format!("{:04}-{}", number, &slug);
    write_spec_package_in(&spec_dir, &dir_name, &title, plan)
}

fn write_spec_package_in(
    spec_dir: &Path,
    dir_name: &str,
    title: &str,
    plan: &str,
) -> Result<PathBuf> {
    // Write all content files first...
    std::fs::write(spec_dir.join("plan.md"), plan)?;

    // Baseline content is embedded within plan.md — no separate file needed.

    let validation = extract_section(
        plan,
        &["Validation", "Validation Criteria"],
        "Run check.ps1.",
    );
    let validation = validation.replace("check.ps1", "check-fast.ps1");
    std::fs::write(spec_dir.join("validation.md"), validation)?;

    // Design decisions and risks are embedded within plan.md — no separate notes.md needed.
    // Status, owner, implementer metadata lives in spec.yaml — no README.md needed.

    // ...then write spec.yaml LAST so the package is only discovered once fully written.
    write_generated_spec_yaml(spec_dir, dir_name, title, plan)?;

    Ok(spec_dir.to_path_buf())
}

#[derive(Serialize)]
struct GeneratedSpecYaml {
    version: u32,
    id: String,
    title: String,
    status: String,
    owner: String,
    implementer: String,
    priority: String,
    area: Vec<String>,
    files: GeneratedSpecFiles,
    acceptance: Vec<String>,
    non_goals: Vec<String>,
    risks: Vec<String>,
}

#[derive(Serialize)]
struct GeneratedSpecFiles {
    code: Vec<String>,
    docs: Vec<String>,
}

/// Write the generated spec.yaml from a GeneratedSpecYaml struct.
///
/// # Metadata defaults
///
/// - `owner` and `implementer` are hardcoded to `"pipeline"` because the
///   pipeline is the sole generator of automated specs. This is intentional.
/// - `priority` is extracted dynamically from the plan text.
/// - `area` is extracted dynamically from the plan text.
fn write_generated_spec_yaml(
    spec_dir: &Path,
    dir_name: &str,
    title: &str,
    plan: &str,
) -> Result<()> {
    let active_prefix = format!("docs/specs/_active/{}/", dir_name);
    let code_files = extract_code_file_refs(plan);
    let spec = GeneratedSpecYaml {
        version: 1,
        id: dir_name.to_string(),
        title: title.to_string(),
        status: "approved".to_string(),
        owner: "pipeline".to_string(),
        implementer: "pipeline".to_string(),
        priority: extract_priority(plan),
        area: extract_area(plan),
        files: GeneratedSpecFiles {
            code: code_files,
            docs: ["plan.md", "validation.md"]
                .into_iter()
                .map(|file| format!("{}{}", active_prefix, file))
                .collect(),
        },
        acceptance: {
            let criteria = crate::bacon_core::extract_section(plan, &["Acceptance Criteria"], "");
            if criteria.is_empty() {
                vec![
                    "Generated spec package is complete and validated.".to_string(),
                    "Implementation validates with check-fast.ps1 before completion.".to_string(),
                ]
            } else {
                criteria
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        !t.is_empty() && !t.starts_with('#')
                    })
                    .map(|l| {
                        l.trim_matches(|c: char| c == '-' || c == '*' || c == ' ')
                            .to_string()
                    })
                    .filter(|l| !l.is_empty())
                    .collect()
            }
        },
        non_goals: vec!["No unchecked auto-apply of generated patches.".to_string()],
        risks: vec!["LLM-generated plans may still need human review for scope.".to_string()],
    };

    let content = serde_yml::to_string(&spec)?;
    std::fs::write(spec_dir.join("spec.yaml"), content)?;
    Ok(())
}

fn extract_code_file_refs(plan: &str) -> Vec<String> {
    let refs: Vec<String> = crate::bacon_core::extract_repo_file_refs(plan)
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
        .collect();

    if refs.is_empty() {
        vec!["src/".to_string()]
    } else {
        refs
    }
}

fn extract_title(text: &str) -> String {
    crate::bacon_core::extract_title(text)
}

fn slugify(text: &str) -> String {
    crate::bacon_core::slugify(text)
}
fn extract_section(plan: &str, headers: &[&str], fallback: &str) -> String {
    crate::bacon_core::extract_section(plan, headers, fallback)
}

/// Extract priority level from a spec plan, defaulting to "P2" if not found.
fn extract_priority(plan: &str) -> String {
    for line in plan.lines() {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("priority:")
            || lower.starts_with("**priority:**")
            || lower.starts_with("- priority:")
        {
            if let Some(val) = line.split(':').nth(1) {
                let trimmed = val.trim().trim_matches(&[' ', '*', '`', '"', '\''][..]);
                match trimmed.to_lowercase().as_str() {
                    "p0" | "critical" => return "P0".to_string(),
                    "p1" | "high" => return "P1".to_string(),
                    "p3" | "low" => return "P3".to_string(),
                    _ => {}
                }
            }
        }
    }
    "P2".to_string() // default medium priority
}

/// Extract area tags from a spec plan, defaulting to ["bacon"] if not found.
fn extract_area(plan: &str) -> Vec<String> {
    for line in plan.lines() {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("area:")
            || lower.starts_with("**area:**")
            || lower.starts_with("- area:")
            || lower.starts_with("tags:")
        {
            if let Some(val) = line.split(':').nth(1) {
                let cleaned = val
                    .trim()
                    .trim_matches(&[' ', '[', ']', '*', '`', '"', '\''][..]);
                let areas: Vec<String> = cleaned
                    .split([',', ' '].as_slice())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !areas.is_empty() {
                    return areas;
                }
            }
        }
    }
    vec!["bacon".to_string()] // default area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_relevant_source_files() {
        let ctx = PipelineCtx::new(
            "Please review src/adaptive/predictive_scorer.rs for an unused field.".to_string(),
        );
        let prompt = build_prompt(&ctx);

        assert!(prompt.contains("Please review src/adaptive/predictive_scorer.rs"));
        assert!(prompt.contains("## Relevant Source Files"));
        assert!(prompt.contains("src/adaptive/predictive_scorer.rs"));
        assert!(prompt.contains("source of truth"));
    }

    #[test]
    fn build_prompt_uses_coder_errors_for_scope_reduction_context() {
        let mut ctx = PipelineCtx::new("Original finding".to_string());
        ctx.scope_reduction_needed = true;
        ctx.coder_errors =
            vec!["SEARCH text not found in src/adaptive/predictive_scorer.rs".to_string()];

        let prompt = build_prompt(&ctx);

        assert!(prompt.contains("REDUCED-SCOPE implementation plan"));
        assert!(prompt.contains("src/adaptive/predictive_scorer.rs"));
        assert!(prompt.contains("SEARCH text not found"));
    }

    #[test]
    fn extract_code_file_refs_prefers_specific_plan_files() {
        let plan = "Update src/bacon_agent_nvidia/auditor.rs and docs/specs/README.md.";
        let refs = extract_code_file_refs(plan);

        assert_eq!(refs, vec!["src/bacon_agent_nvidia/auditor.rs"]);
    }

    #[test]
    fn test_extract_title_from_h1() {
        assert_eq!(extract_title("# My Title\ncontent"), "My Title");
    }

    #[test]
    fn test_extract_title_from_h2() {
        assert_eq!(extract_title("## My Title\ncontent"), "My Title");
    }

    #[test]
    fn test_extract_title_fallback_to_first_line() {
        assert_eq!(
            extract_title("First non-empty line\nsecond"),
            "First non-empty line"
        );
    }

    #[test]
    fn test_extract_title_empty_fallback() {
        assert_eq!(extract_title(""), "Untitled");
    }

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(
            slugify("Fix: error handling & retry!"),
            "fix--error-handling---retry"
        );
    }

    #[test]
    fn test_slugify_truncates() {
        let long = "a".repeat(100);
        assert!(slugify(&long).len() <= 40);
    }

    #[test]
    fn test_extract_section_found() {
        let plan = "## Baseline\nCurrent state.\n\n## Validation\nRun check-fast.ps1.";
        let result = extract_section(plan, &["Baseline"], "fallback");
        assert!(result.contains("## Baseline"));
        assert!(result.contains("Current state."));
        assert!(!result.contains("## Validation"));
    }

    #[test]
    fn test_extract_section_fallback() {
        let plan = "## Title\ncontent";
        let result = extract_section(plan, &["Missing"], "fallback text");
        assert_eq!(result, "fallback text");
    }

    #[test]
    fn test_extract_section_exact_header_match() {
        let plan = "## Validation\ncontent\n\n## Validate More\nother";
        let result = extract_section(plan, &["Validation"], "fallback");
        assert!(result.contains("Validation"));
        assert!(result.contains("content"));
        assert!(!result.contains("Validate More"));
    }

    #[test]
    fn test_write_spec_package_in_creates_streamlined_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let active = temp.path().join("_active");
        std::fs::create_dir_all(&active)?;
        let spec_dir = active.join("0001-test-spec");
        std::fs::create_dir(&spec_dir)?;

        let plan = "# Test Spec\n\n## Baseline\nCurrent state.\n\n## Validation\nRun check-fast.ps1.\n\n## Design Decisions and Risks\nKeep scope small.";
        let spec_path = write_spec_package_in(&spec_dir, "0001-test-spec", "Test Spec", plan)?;

        // Verify streamlined file set (3 files)
        assert!(spec_path.join("spec.yaml").exists(), "spec.yaml missing");
        assert!(spec_path.join("plan.md").exists(), "plan.md missing");
        assert!(
            spec_path.join("validation.md").exists(),
            "validation.md missing"
        );

        // Verify redundant boilerplate files are NOT created
        assert!(
            !spec_path.join("ci-commands.md").exists(),
            "ci-commands.md should not be created"
        );
        assert!(
            !spec_path.join("quality-rules.md").exists(),
            "quality-rules.md should not be created"
        );
        assert!(
            !spec_path.join("validation-checklist.md").exists(),
            "validation-checklist.md should not be created"
        );

        Ok(())
    }
}
