use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bin_path(name: &str) -> PathBuf {
    let var_name = format!("CARGO_BIN_EXE_{}", name);
    if let Some(path) = std::env::var_os(&var_name) {
        return PathBuf::from(path);
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    });
    path
}

/// Returns (tool_name, base_args) for each CLI worker.
fn agent_configs() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("codex", vec!["-p", "contract fixture", "--dry-run"]),
        ("gemini", vec!["-p", "contract fixture", "--dry-run"]),
        ("opencode", vec!["-p", "contract fixture", "--dry-run"]),
        // kilocode uses a "run" subcommand instead of -p
        ("kilocode", vec!["run", "contract fixture", "--dry-run"]),
        ("ollama", vec!["-p", "contract fixture", "--dry-run"]),
        ("nvidia", vec!["-p", "contract fixture", "--dry-run"]),
    ]
}

/// Run a CLI worker with the given role and apply role-specific assertions
/// against the parsed JSON output.
fn test_worker_role<'a>(
    tool: &str,
    mut base_args: Vec<&'a str>,
    role: &'a str,
    assert_fn: impl Fn(&Value),
) {
    // Append --role <role>
    base_args.push("--role");
    base_args.push(role);

    let output = Command::new(bin_path(tool))
        .args(&base_args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|err| panic!("failed to run {} --role {}: {}", tool, role, err));

    assert!(
        output.status.success(),
        "{} --role {} failed\nstdout: {}\nstderr: {}",
        tool,
        role,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|err| panic!("{} stdout is not JSON: {}\n{}", tool, err, stdout));

    // Common contract: status must be "ok" and description must be present
    assert_eq!(
        json.get("status").and_then(Value::as_str),
        Some("ok"),
        "{} --role {}: status is not 'ok': {}",
        tool,
        role,
        json,
    );
    assert!(
        json.get("description").and_then(Value::as_str).is_some(),
        "{} --role {}: missing description field in: {}",
        tool,
        role,
        json,
    );

    // Role-specific assertions
    assert_fn(&json);
}

// ---------------------------------------------------------------------------
// Contract tests: Observer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires external CLI binaries (codex, gemini, etc.) not present in CI env"]
fn all_agents_observer_contract() {
    for (tool, args) in agent_configs() {
        test_worker_role(tool, args.clone(), "observer", |json| {
            let desc = json.get("description").and_then(Value::as_str).unwrap();
            assert!(
                desc.contains("Observer") || desc.contains("observer"),
                "{} observer: description should mention role, got: {}",
                tool,
                desc,
            );
        });
    }
}

// ---------------------------------------------------------------------------
// Contract tests: Strategist
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires external CLI binaries (codex, gemini, etc.) not present in CI env"]
fn all_agents_strategist_contract() {
    for (tool, args) in agent_configs() {
        test_worker_role(tool, args.clone(), "strategist", |json| {
            let desc = json.get("description").and_then(Value::as_str).unwrap();

            // The description should mention strategic planning or the role name
            let has_strategic_keyword = desc.contains("Strateg")
                || desc.contains("strateg")
                || desc.contains("Plan")
                || desc.contains("plan")
                || desc.contains("Design")
                || desc.contains("design");
            assert!(
                has_strategic_keyword,
                "{} strategist: description should mention planning/strategy, got: {}",
                tool, desc,
            );

            // If spec_path is present (real agents), it should be non-empty
            if let Some(spec_path) = json.get("spec_path").and_then(Value::as_str) {
                assert!(
                    !spec_path.is_empty(),
                    "{} strategist: spec_path should be non-empty when present",
                    tool,
                );
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Contract tests: Coder
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires external CLI binaries (codex, gemini, etc.) not present in CI env"]
fn all_agents_coder_contract() {
    for (tool, args) in agent_configs() {
        test_worker_role(tool, args.clone(), "coder", |json| {
            let desc = json.get("description").and_then(Value::as_str).unwrap();

            // The description should mention code/implementation
            let has_code_keyword = desc.contains("Code")
                || desc.contains("code")
                || desc.contains("Implement")
                || desc.contains("implement")
                || desc.contains("Generate")
                || desc.contains("generate")
                || desc.contains("minimal")
                || desc.contains("safe");
            assert!(
                has_code_keyword,
                "{} coder: description should mention code/implementation, got: {}",
                tool, desc,
            );
        });
    }
}

// ---------------------------------------------------------------------------
// Contract tests: Auditor
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires external CLI binaries (codex, gemini, etc.) not present in CI env"]
fn all_agents_auditor_contract() {
    for (tool, args) in agent_configs() {
        test_worker_role(tool, args.clone(), "auditor", |json| {
            let desc = json.get("description").and_then(Value::as_str).unwrap();

            // The description should mention audit/quality/assessment
            let has_audit_keyword = desc.contains("Audit")
                || desc.contains("audit")
                || desc.contains("PASS")
                || desc.contains("FAIL")
                || desc.contains("Assessment")
                || desc.contains("assessment")
                || desc.contains("Quality")
                || desc.contains("quality")
                || desc.contains("Security")
                || desc.contains("security")
                || desc.contains("review")
                || desc.contains("Review")
                || desc.contains("validate")
                || desc.contains("Validate");
            assert!(
                has_audit_keyword,
                "{} auditor: description should mention audit/quality/assessment, got: {}",
                tool, desc,
            );
        });
    }
}
