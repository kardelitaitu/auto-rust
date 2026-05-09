//! Failure history tracking.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Type of failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureType {
    Connection,
    Resource,
    Api,
    Timeout,
    Data,
    Unknown,
}

/// Individual failure record.
#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub id: String,
    pub failure_type: FailureType,
    pub timestamp: Instant,
    pub error: String,
    pub recovery_time: Duration,
}

/// Impact level of failure pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Identified failure pattern.
#[derive(Debug, Clone)]
pub struct FailurePattern {
    pub id: String,
    pub signature: Vec<String>,
    pub frequency: u32,
    pub impact: ImpactLevel,
}

/// Failure history tracking.
pub struct FailureHistory {
    pub recent_failures: VecDeque<FailureRecord>,
    pub patterns: Vec<FailurePattern>,
    pub mtbf: Duration,
    pub mttr: Duration,
}

impl FailureHistory {
    pub fn new() -> Self {
        Self {
            recent_failures: VecDeque::new(),
            patterns: vec![],
            mtbf: Duration::from_secs(0),
            mttr: Duration::from_secs(0),
        }
    }
}

impl Default for FailureHistory {
    fn default() -> Self {
        Self::new()
    }
}
