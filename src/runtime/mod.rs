//! Runtime lifecycle and task execution context.
//!
//! `TaskContext` is the task-api surface. It exposes the short
//! `api.*` verbs used by task code and keeps browser/session lifecycle
//! hidden behind the runtime layer.

pub mod browser {
    #[allow(unused_imports)]
    pub use crate::browser::discover_browsers;
}

pub mod session {
    #[allow(unused_imports)]
    pub use crate::session::{Session, SessionState};
}

pub mod health_monitor {
    #[allow(unused_imports)]
    pub use crate::health_monitor::{HealthMonitor, HealthState};
}

pub mod orchestrator {
    #[allow(unused_imports)]
    pub use crate::orchestrator::Orchestrator;
}

pub mod execution;
pub mod task_context;

#[cfg(test)]
mod tests {
    /// Smoke test to verify runtime module structure compiles.
    #[test]
    fn test_runtime_module_compiles() {
        // All re-exports are just aliases - verify structure
        assert!(true);
    }
}
