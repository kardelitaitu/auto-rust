use anyhow::Result;
use log::{info, warn};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::bacon_core::{read_role_prompt, run_powershell_with_args};
use crate::llm::{ChatMessage, Llm};

use super::cli::RunArgs;
use super::spec_io;
use super::types::PipelineCtx;

pub async fn run(llm: &Llm, _args: &RunArgs, ctx: &PipelineCtx) -> Result<PipelineCtx> {
    let system_prompt = read_role_prompt("strategist");

    let user_prompt = if ctx.scope_reduction_needed {
        // Scope reduction mode: the Coder failed, and we need a reduced-scope plan
        let errors = ctx.coder_errors.join("\n---\n");
        format!(
            "Review this finding and design a REDUCED-SCOPE implementation plan.\n\n\
             The previous implementation attempt failed with these errors:\n{}\n\n\
             Original finding:\n{}\n\n\
             IMPORTANT: Produce a simpler plan with fewer changes. \
             Reduce the number of files touched, simplify the approach, or \
             limit the scope to the most essential part. \
             If the original plan touched 3 files, make this plan touch 1-2.\n\n\
             If the approach is sound and risks acceptable, output a plan with:\n\
             1. A clear title (one line)\n\
             2. Step-by-step implementation steps\n\
             3. Current state description (baseline)\n\
             4. Any API changes needed\n\
             5. How to validate\n\
             6. Design decisions and risks\n\n\
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

    let messages = vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ];

    info!("Calling Strategist LLM...");
    let response = llm
        .chat(messages)
        .await
        .map_err(|e| anyhow::anyhow!("Strategist LLM call failed: {}", e))?;

    if response.trim().starts_with("REJECTED:") {
        warn!("Strategist rejected the proposal: {}", response);
        return Err(anyhow::anyhow!("Strategist rejected: {}", response));
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
        info!("Strategist confidence: {}", conf.as_str());
    }

    println!("=== Strategist Output ===");
    println!("{}", response);
    println!("=========================");

    // Write spec package
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

    let mut output = PipelineCtx::new(ctx.description.clone());
    output.spec_path = spec_path;
    output.dry_run = ctx.dry_run;
    output.confidence = confidence;
    Ok(output)
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

    // ...then write spec.yaml LAST so the package is only discovered once fully written.
    write_generated_spec_yaml(spec_dir, dir_name, title)?;

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
            docs: ["README.md", "plan.md", "validation.md", "notes.md"]
                .into_iter()
                .map(|file| format!("{}{}", active_prefix, file))
                .collect(),
        },
        acceptance: vec![
            "Generated spec package passes spec-lint.ps1 before implementation starts.".to_string(),
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
    crate::bacon_core::extract_title(text)
}

fn slugify(text: &str) -> String {
    crate::bacon_core::slugify(text)
}

fn extract_section(plan: &str, headers: &[&str], fallback: &str) -> String {
    crate::bacon_core::extract_section(plan, headers, fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn generated_spec_package_passes_spec_lint() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let active = temp.path().join("_active");
        std::fs::create_dir_all(&active)?;

        let spec_dir = active.join("0001-tiny-safe-improvement");
        std::fs::create_dir(&spec_dir)?;
        let plan = "# Tiny Safe Improvement\n\n## Baseline\nCurrent state.\n\n## Validation\nRun check.ps1.\n\n## Design Decisions and Risks\nKeep scope small.";
        let spec_path = write_spec_package_in(
            &spec_dir,
            "0001-tiny-safe-improvement",
            "Tiny Safe Improvement",
            plan,
        )?;

        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec-lint.ps1");
        let output = std::process::Command::new(if cfg!(windows) { "powershell" } else { "pwsh" })
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script.to_string_lossy().as_ref(),
                "-Directory",
                spec_path.to_string_lossy().as_ref(),
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()?;

        assert!(
            output.status.success(),
            "generated spec failed spec-lint\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(())
    }
}
