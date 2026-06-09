#[cfg(test)]
mod task_doc_routing {
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read(path: &str) -> String {
        fs::read_to_string(repo_root().join(path))
            .unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
    }

    #[test]
    fn twitter_task_pointer_docs_match_canonical_routing() {
        let cases = [
            ("twitteractivity", "Twitter Activity Task"),
            ("twitterfollow", "Twitter Follow Task"),
            ("twitterintent", "Twitter Intent Task"),
            ("twitterreply", "Twitter Reply Task"),
        ];

        for (task, title) in cases {
            let pointer = read(&format!("docs/TASKS/{task}.md"));
            let expected = format!(
                "# {title}\n\nCanonical task documentation now lives in [`src/task/{task}.md`](../../src/task/{task}.md).\n"
            );
            assert_eq!(
                pointer.replace("\r\n", "\n"),
                expected,
                "pointer doc drifted for {task}"
            );
        }
    }

    #[test]
    fn overview_routes_twitter_task_docs_to_src_task() {
        let overview = read("docs/TASKS/overview.md").replace("\r\n", "\n");

        for task in [
            "twitteractivity",
            "twitterfollow",
            "twitterintent",
            "twitterreply",
        ] {
            let link = format!("../../src/task/{task}.md");
            assert!(
                overview.contains(&link),
                "overview should link {task} to {link}"
            );
        }

        assert!(
            overview.contains("Task-specific docs are canonical in `src/task/*.md`"),
            "overview should state the canonical task-doc location"
        );
    }
}
