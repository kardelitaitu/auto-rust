use anyhow::Result;
use log::{info, warn};
use serde::Serialize;
use std::path::{Path, PathBuf};

use super::spec_io;
use super::types::PipelineCtx;
use crate::core::cli_types::RunArgs;

fn role_prompt() -> String {
    crate::core::read_role_prompt("strategist")
}

fn build_source_context(seed_text: &str) -> String {
    let source_context = crate::core::collect_source_context(seed_text, 8, 300);
    if source_context.is_empty() {
        crate::core::gather_project_context()
    } else {
        source_context
    }
}

fn build_prompt(ctx: &PipelineCtx) -> String {
    let source_context = build_source_context(&ctx.description);
    let evidence_guard =
        "IMPORTANT: Treat the source excerpts below as the source of truth. Do not describe a symbol as unused if the excerpts show it being used.";
    format!(
        "Review this finding and design an implementation plan:


             {}


             {}


             {}


             If the approach is sound and risks acceptable, output a plan with:

             1. A clear title (one line)

             2. Step-by-step implementation steps

             3. Current state description (baseline)

             4. Any API changes needed

             5. How to validate

             6. Design decisions and risks


             Hard constraints for autonomous Bacon runs:

             - Do not add dependencies or edit Cargo.toml.

             - Keep scope to 1-3 existing repo files.

             - Include check-fast.ps1 in the Validation section.

             - Reject performance tuning, dependency changes, rewrites, and speculative efficiency work.

             - Prefer concrete correctness, cleanup, or test-gap fixes over speculative performance work.


             If risks are too high, start your response with 'REJECTED:' and explain why.",
        ctx.description, source_context, evidence_guard
    )
}

pub async fn run(llm: &crate::llm::Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let system_prompt = role_prompt();
    let prompt = build_prompt(ctx);

    info!("NVIDIA Strategist calling API...");
    let messages = vec![
        crate::llm::ChatMessage::system(system_prompt),
        crate::llm::ChatMessage::user(prompt),
    ];
    let response = llm.chat(messages).await?;

    if response.trim().starts_with("REJECTED:") {
        anyhow::bail!("Strategist rejected: {response}");
    }

    if let Err(e) = validate_autonomous_plan(&response) {
        warn!("Strategist produced out-of-scope plan: {e}");
        return Ok(PipelineCtx::new(
            format!("No autonomous-safe plan produced: {e}"),
            ctx.fs.clone(),
            ctx.runner.clone(),
            ctx.llm.clone(),
        )
        .with_dry_run(ctx.dry_run)
        .with_confidence(crate::core::extract_confidence(&response)));
    }

    // Scope gate: count unique file paths referenced in the plan
    let file_count = crate::core::count_spec_file_refs(&response);
    if file_count > 3 {
        warn!(
            "Strategist plan references {file_count} files (> 3 recommended). Consider reducing scope."
        );
    }

    // Extract and log confidence
    let confidence = crate::core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Strategist confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Strategist Output ===");
    println!("{response}");
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
        let (passed, output) = crate::core::run_spec_lint(&spec_path)?;
        if !passed {
            warn!("spec-lint failed — stopping before Coder");
            anyhow::bail!(
                "generated spec failed spec-lint:
{output}"
            );
        }
        info!("spec-lint passed");

        // Gate: Existence Guard — verify all file paths in the plan actually exist
        let missing = crate::core::validate_spec_file_refs(&spec_path);
        if !missing.is_empty() {
            let msg = format!(
                "Strategist hallucination detected! The plan references non-existent files:
- {}
Stopping before Coder.",
                missing.join(
                    "
- "
                )
            );
            warn!("{msg}");
            anyhow::bail!("{msg}");
        }

        Some(spec_path)
    };

    let mut result = PipelineCtx::new(
        response,
        ctx.fs.clone(),
        ctx.runner.clone(),
        ctx.llm.clone(),
    )
    .with_dry_run(ctx.dry_run);
    result.spec_path = spec_path;
    result.confidence = confidence;
    Ok(result)
}

fn validate_autonomous_plan(plan: &str) -> Result<()> {
    let lower = plan.to_lowercase();
    let banned = [
        "add a new dependency",
        "add new dependency",
        "new crate",
        "dependency",
        "dependencies",
        "as a dependency",
        "assume that",
        "assume ",
        "cargo add",
        "cargo.toml",
        "ndarray",
        "nalgebra",
        "linear algebra library",
        "array or matrix",
        "performance",
        "performant",
        "optimize",
        "optimizing",
        "optimization",
        "efficient",
        "efficiency",
        "large models",
        "potentially",
        "may not be",
        "without actual",
        "uncertain",
        "benchmark",
        "profiling",
        "performance benefits",
        "performance overhead",
        "performance regressions",
    ];

    if let Some(hit) = banned.iter().find(|needle| lower.contains(**needle)) {
        anyhow::bail!(
            "Strategist plan is outside autonomous scope: mentions '{hit}'. 
             Bacon auto plans must be grounded, low-risk maintenance work."
        );
    }

    let file_count = crate::core::count_spec_file_refs(plan);
    if file_count > 3 {
        anyhow::bail!(
            "Strategist plan is outside autonomous scope: references {file_count} repo files (> 3)"
        );
    }

    Ok(())
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

/// Write the generated spec.yaml from a `GeneratedSpecYaml` struct.
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
    let active_prefix = format!("docs/specs/_active/{dir_name}/");
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
                .map(|file| format!("{active_prefix}{file}"))
                .collect(),
        },
        acceptance: {
            let criteria = crate::core::extract_section(plan, &["Acceptance Criteria"], "");
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
    let refs: Vec<String> = crate::core::extract_repo_file_refs(plan)
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
    crate::core::extract_title(text)
}

fn slugify(text: &str) -> String {
    crate::core::slugify(text)
}
fn extract_section(plan: &str, headers: &[&str], fallback: &str) -> String {
    crate::core::extract_section(plan, headers, fallback)
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

    fn new_mock_ctx(desc: &str) -> PipelineCtx {
        PipelineCtx::new(desc.to_string(), None, None, None)
    }

    #[test]
    #[ignore = "requires host project filesystem — build_prompt calls collect_source_context which reads files"]
    fn build_prompt_includes_relevant_source_files() {
        let ctx =
            new_mock_ctx("Please review src/adaptive/predictive_scorer.rs for an unused field.");
        let prompt = build_prompt(&ctx);

        assert!(prompt.contains("Please review src/adaptive/predictive_scorer.rs"));
        assert!(prompt.contains("## Relevant Source Files"));
        assert!(prompt.contains("src/adaptive/predictive_scorer.rs"));
        assert!(prompt.contains("source of truth"));
    }

    #[test]
    fn extract_code_file_refs_prefers_specific_plan_files() {
        let plan = "Update src/agent/auditor.rs and docs/specs/README.md.";
        let refs = extract_code_file_refs(plan);

        assert_eq!(refs, vec!["src/agent/auditor.rs"]);
    }

    #[test]
    fn validate_autonomous_plan_rejects_speculative_performance_work() {
        let plan = "# Improve Predictive Scorer Performance


            ## Baseline

            The Vec<f32> storage may not be efficient for large models.


            ## Implementation Steps

            Replace coefficients with an array or matrix from a linear algebra library.


            ## Validation

            Run check-fast.ps1.


            Confidence: Medium";

        let err = validate_autonomous_plan(plan).unwrap_err().to_string();
        assert!(err.contains("outside autonomous scope"));
    }

    #[test]
    fn validate_autonomous_plan_accepts_small_grounded_maintenance_work() -> Result<()> {
        let plan = "# Add Spec Numbering Regression Test


            ## Baseline

            src/core/spec_io.rs allocates spec directories by scanning active specs.


            ## Implementation Steps

            Add one test case in src/core/spec_io.rs for active and done numbering.


            ## Validation

            Run check-fast.ps1.


            Confidence: High";

        validate_autonomous_plan(plan)
    }

    #[test]
    fn test_extract_title_from_h1() {
        assert_eq!(
            extract_title(
                "# My Title
content"
            ),
            "My Title"
        );
    }

    #[test]
    fn test_extract_title_from_h2() {
        assert_eq!(
            extract_title(
                "## My Title
content"
            ),
            "My Title"
        );
    }

    #[test]
    fn test_extract_title_fallback_to_first_line() {
        assert_eq!(
            extract_title(
                "First non-empty line
second"
            ),
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
        let plan = "## Baseline
Current state.

## Validation
Run check-fast.ps1.";
        let result = extract_section(plan, &["Baseline"], "fallback");
        assert!(result.contains("## Baseline"));
        assert!(result.contains("Current state."));
        assert!(!result.contains("## Validation"));
    }

    #[test]
    fn test_extract_section_fallback() {
        let plan = "## Title
content";
        let result = extract_section(plan, &["Missing"], "fallback text");
        assert_eq!(result, "fallback text");
    }

    #[test]
    fn test_extract_section_exact_header_match() {
        let plan = "## Validation
content

## Validate More
other";
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

        let plan = "# Test Spec

## Baseline
Current state.

## Validation
Run check-fast.ps1.

## Design Decisions and Risks
Keep scope small.";
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
