use anyhow::Result;
use log::info;
use regex::Regex;
use serde::Serialize;
use std::path::Path;

use super::cli::RunArgs;
use super::nvidia_api;
use super::spec_io;
use super::types::PipelineCtx;

fn role_prompt() -> String {
    crate::bacon_core::read_role_prompt("strategist")
}

pub async fn run(_llm: &crate::llm::Llm, args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let config = args.nvidia_config();
    let system_prompt = role_prompt();

    let prompt = if ctx.scope_reduction_needed {
        let errors = ctx.coder_errors.join("\n---\n");
        format!(
            "Review this finding and design a REDUCED-SCOPE implementation plan.\n\n\
             The previous implementation attempt failed with these errors:\n{}\n\n\
             Original finding:\n{}\n\n\
             IMPORTANT: Produce a simpler plan with fewer changes. \
             Reduce the number of files touched, simplify the approach, or \
             limit the scope to the most essential part.\n\n\
             If you believe the original scope is already minimal, output:\n\
             SCOPE_AT_MINIMUM: <explanation>",
            errors, ctx.description
        )
    } else {
        format!(
            "Review this finding and design an implementation plan:\n\n\
             {}\n\n\
             If the approach is sound and risks acceptable, output a plan with:\n\
             1. A clear title (one line)\n\
             2. Step-by-step implementation steps\n\
             3. Current state description (baseline)\n\
             4. Any API changes needed\n\
             5. How to validate\n\
             6. Design decisions and risks\n\n\
             If risks are too high, start your response with 'REJECTED:' and explain why.",
            ctx.description
        )
    };

    info!("NVIDIA Strategist calling API with model: {}", config.model);
    let response = nvidia_api::chat(&config, &system_prompt, &prompt).await?;

    if response.trim().starts_with("REJECTED:") {
        anyhow::bail!("Strategist rejected: {}", response);
    }

    // Extract and log confidence
    let confidence = crate::bacon_core::extract_confidence(&response);
    if let Some(ref conf) = confidence {
        info!("NVIDIA Strategist confidence: {}", conf.as_str());
    }

    println!("=== NVIDIA Strategist Output ===");
    println!("{}", response);
    println!("================================");

    // Write spec package (streamlined: 8 files, no redundant boilerplate)
    let spec_path = if ctx.dry_run {
        info!("DRY RUN: would write spec package");
        None
    } else {
        let spec_path = write_spec_package(&response)?;
        info!("Spec package created at: {}", spec_path.display());
        Some(spec_path)
    };

    let mut result = PipelineCtx::new(response).with_dry_run(ctx.dry_run);
    result.spec_path = spec_path;
    result.confidence = confidence;
    Ok(result)
}

pub fn write_spec_package(plan: &str) -> Result<std::path::PathBuf> {
    let number = spec_io::next_spec_number()?;
    write_spec_package_in(&spec_io::active_dir(), number, plan)
}

fn write_spec_package_in(active: &Path, number: u32, plan: &str) -> Result<std::path::PathBuf> {
    let title = extract_title(plan);
    let dir_name = format!("{:04}-{}", number, slugify(&title));
    let spec_dir = active.join(&dir_name);
    std::fs::create_dir_all(&spec_dir)?;

    write_generated_spec_yaml(&spec_dir, &dir_name, &title)?;
    std::fs::write(spec_dir.join("plan.md"), plan)?;

    let baseline = extract_section(plan, &["Baseline"], "Current state description.");
    std::fs::write(spec_dir.join("baseline.md"), baseline)?;

    let validation = extract_section(
        plan,
        &["Validation", "Validation Criteria"],
        "Run check.ps1.",
    );
    std::fs::write(spec_dir.join("validation.md"), validation)?;

    let notes = extract_section(
        plan,
        &["Design Decisions and Risks", "Risks", "Design Decisions"],
        "See plan.md.",
    );
    std::fs::write(spec_dir.join("notes.md"), notes)?;

    std::fs::write(
        spec_dir.join("README.md"),
        format!(
            "# {}\n\nStatus: `approved`\n\nOwner: `pipeline`\nImplementer: `pipeline`\n",
            title
        ),
    )?;

    Ok(spec_dir)
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

fn write_generated_spec_yaml(spec_dir: &Path, dir_name: &str, title: &str) -> Result<()> {
    let active_prefix = format!("docs/specs/_active/{}/", dir_name);
    let spec = GeneratedSpecYaml {
        version: 1,
        id: dir_name.to_string(),
        title: title.to_string(),
        status: "approved".to_string(),
        owner: "pipeline".to_string(),
        implementer: "pipeline".to_string(),
        priority: "P2".to_string(),
        area: vec!["bacon".to_string()],
        files: GeneratedSpecFiles {
            code: vec!["src/".to_string()],
            docs: [
                "README.md",
                "plan.md",
                "validation.md",
                "notes.md",
                "baseline.md",
                "internal-api-outline.md",
            ]
            .into_iter()
            .map(|file| format!("{}{}", active_prefix, file))
            .collect(),
        },
        acceptance: vec![
            "Generated spec package is complete and validated.".to_string(),
            "Implementation validates with check.ps1 before completion.".to_string(),
        ],
        non_goals: vec!["No unchecked auto-apply of generated patches.".to_string()],
        risks: vec!["LLM-generated plans may still need human review for scope.".to_string()],
    };

    let content = serde_yml::to_string(&spec)?;
    std::fs::write(spec_dir.join("spec.yaml"), content)?;
    Ok(())
}

fn extract_title(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return stripped.to_string();
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            return stripped.to_string();
        }
    }
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .chars()
        .take(40)
        .collect()
}

fn extract_section(plan: &str, headers: &[&str], fallback: &str) -> String {
    // Build regex for exact ## HeaderName matching (case-insensitive)
    let pattern = headers
        .iter()
        .map(|h| regex::escape(h))
        .collect::<Vec<_>>()
        .join("|");
    let re = match Regex::new(&format!(r"(?m)^##\s+({pattern})\s*$")) {
        Ok(r) => r,
        Err(_) => return fallback.to_string(),
    };

    if let Some(m) = re.find(plan) {
        let start = m.start();
        let rest = &plan[m.end()..];
        let end = if let Some(next) = rest.find("\n## ") {
            m.end() + next
        } else {
            plan.len()
        };
        let result = plan[start..end].to_string();
        if result.trim().is_empty() {
            fallback.to_string()
        } else {
            result
        }
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let plan = "## Baseline\nCurrent state.\n\n## Validation\nRun check.ps1.";
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

        let plan = "# Test Spec\n\n## Baseline\nCurrent state.\n\n## Validation\nRun check.ps1.\n\n## Design Decisions and Risks\nKeep scope small.";
        let spec_path = write_spec_package_in(&active, 1, plan)?;

        // Verify streamlined file set (6 files matching _template)
        assert!(spec_path.join("spec.yaml").exists(), "spec.yaml missing");
        assert!(spec_path.join("plan.md").exists(), "plan.md missing");
        assert!(
            spec_path.join("baseline.md").exists(),
            "baseline.md missing"
        );
        assert!(
            spec_path.join("validation.md").exists(),
            "validation.md missing"
        );
        assert!(spec_path.join("notes.md").exists(), "notes.md missing");
        assert!(spec_path.join("README.md").exists(), "README.md missing");

        // Verify redundant boilerplate files are NOT created
        assert!(
            !spec_path.join("internal-api-outline.md").exists(),
            "internal-api-outline.md should not be created"
        );
        assert!(
            !spec_path.join("implementation-notes.md").exists(),
            "implementation-notes.md should not be created"
        );
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
