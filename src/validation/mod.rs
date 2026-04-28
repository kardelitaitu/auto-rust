pub mod task;
pub mod task_registry;

// Payload validation (from task.rs)
pub use task::validate_task;

// Task name/presence validation (from task_registry.rs)
pub use task_registry::{
    is_known_task, task_file_exists, validate_task as validate_task_name, validate_task_groups,
    validate_task_groups_strict, TaskValidationResult,
};
