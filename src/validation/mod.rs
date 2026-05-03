pub mod task;
pub mod task_registry;

// Payload validation (from task.rs)
pub use task::validate_task;

// Task name/presence validation (from task_registry.rs)
pub use task_registry::{
    get_task_descriptor, is_known_task, validate_task as validate_task_name, validate_task_groups,
    validate_task_groups_strict, TaskValidationResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test to verify validation re-exports are accessible.
    #[test]
    fn test_validation_re_exports_exist() {
        // These just need to compile - verifies module structure
        let _: Option<TaskValidationResult> = None;

        // Verify functions are accessible
        let _: fn(&str) -> bool = is_known_task;
        let _: fn(
            &str,
        ) -> std::result::Result<
            crate::task::registry::TaskDescriptor,
            crate::task::registry::RegistryError,
        > = get_task_descriptor;
    }
}
