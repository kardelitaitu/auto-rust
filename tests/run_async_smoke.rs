use std::process::Command;

#[test]
fn test_binary_list_tasks_smoke() {
    let binary = env!("CARGO_BIN_EXE_auto");

    let output = Command::new(binary)
        .arg("--list-tasks")
        .output()
        .expect("failed to run auto binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Available Tasks:"));
    // Since test-llmreply was added, there are 16 tasks now instead of 15
    assert!(stdout.contains("Total: 16 tasks"));
}
