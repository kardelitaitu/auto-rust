#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn canonical_task_registry_includes_twitterreply_and_demoqa() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cli = fs::read_to_string(root.join("src/cli.rs")).expect("read cli.rs");
        let task_mod = fs::read_to_string(root.join("task/mod.rs")).expect("read task/mod.rs");

        assert!(cli.contains("\"twitterreply\""));
        assert!(task_mod.contains("\"twitterreply\" => twitterreply::run"));
        assert!(cli.contains("\"demoqa\""));
        assert!(task_mod.contains("\"demoqa\" => demoqa::run"));
    }
}
