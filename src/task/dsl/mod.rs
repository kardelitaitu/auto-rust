//! DSL module.
//!
//! Contains the DSL types and executor implementation.
//! - `types`: DSL type definitions (TaskDefinition, Action, Condition, etc.)
//! - `cache`: SelectorCache with LRU eviction
//! - `debug`: DebugEvent, Breakpoint, debug infrastructure
//! - `profiling`: ActionProfiler, ActionMetrics, ExecutionReport
//! - `evaluator`: Variable substitution, condition evaluation
//! - `control_flow`: Control flow action handlers (If, Loop, Foreach, etc.)
//! - `executor`: DslExecutor struct and main execution methods
//! - `parser`: Parser functions for DSL task files

// DSL types (moved from dsl.rs)
pub mod types;

// Parser functions
pub mod parser;

// Executor submodules
pub mod cache;
pub mod control_flow;
pub mod debug;
pub mod evaluator;
pub mod executor;
pub mod profiling;

// Compatibility module for code that expects `dsl_executor` as separate module
pub mod dsl_executor;

// Re-exports for backward compatibility
pub use cache::{CacheStats, SelectorCache, SelectorCacheEntry};
pub use debug::{Breakpoint, DebugEvent, DebugEventType};
pub use executor::{DslExecutionStats, DslExecutor, DEFAULT_CACHE_ENABLED, MAX_CALL_DEPTH};
pub use parser::{
    format_task_definition, get_task_definition, parse_task_file, parse_task_toml, parse_task_yaml,
    validate_task_definition,
};
pub use profiling::{ActionMetrics, ActionProfiler, ExecutionReport};
pub use types::{
    Action, Condition, ForeachCollection, IncludeSpec, LogLevel, ParameterDef, ParameterType,
    TaskDefinition,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify all modules are accessible
        let _cache = cache::SelectorCache::new();
        let _entry = cache::SelectorCacheEntry::new(true, false, None, 0);
        let _stats = cache::CacheStats {
            size: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            hit_rate: 0.0,
        };
    }

    #[test]
    fn test_re_exports() {
        // Verify DSL types are accessible
        let _def = TaskDefinition {
            name: "test".to_string(),
            description: "Test".to_string(),
            policy: "default".to_string(),
            parameters: std::collections::HashMap::new(),
            include: vec![],
            actions: vec![],
        };
    }
}
