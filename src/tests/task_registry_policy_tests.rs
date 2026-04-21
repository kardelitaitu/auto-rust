#[cfg(test)]
mod tests {
    use crate::{cli, task};

    #[test]
    fn canonical_task_registry_includes_twitterreply_and_demoqa() {
        assert!(cli::is_known_task("twitterreply"));
        assert!(cli::is_known_task("demoqa"));
        assert!(task::TASK_NAMES.contains(&"twitterreply"));
        assert!(task::TASK_NAMES.contains(&"demoqa"));
    }
}
