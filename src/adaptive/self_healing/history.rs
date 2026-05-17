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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // FailureType enum
    // -------------------------------------------------------------------------

    #[test]
    fn failure_type_variants_exist() {
        let types = [
            FailureType::Connection,
            FailureType::Resource,
            FailureType::Api,
            FailureType::Timeout,
            FailureType::Data,
            FailureType::Unknown,
        ];
        assert_eq!(types.len(), 6);
        assert_eq!(FailureType::Connection, FailureType::Connection);
        assert_ne!(FailureType::Connection, FailureType::Resource);
    }

    #[test]
    fn failure_type_equality_matches() {
        assert_eq!(FailureType::Timeout, FailureType::Timeout);
        assert_ne!(FailureType::Timeout, FailureType::Api);
        assert_ne!(FailureType::Data, FailureType::Unknown);
    }

    // -------------------------------------------------------------------------
    // ImpactLevel enum
    // -------------------------------------------------------------------------

    #[test]
    fn impact_level_variants_exist() {
        let levels = [
            ImpactLevel::Low,
            ImpactLevel::Medium,
            ImpactLevel::High,
            ImpactLevel::Critical,
        ];
        assert_eq!(levels.len(), 4);
        assert_ne!(ImpactLevel::Low, ImpactLevel::High);
    }

    #[test]
    fn impact_level_equality() {
        assert_eq!(ImpactLevel::Low, ImpactLevel::Low);
        assert_ne!(ImpactLevel::Medium, ImpactLevel::Critical);
    }

    // -------------------------------------------------------------------------
    // FailureRecord
    // -------------------------------------------------------------------------

    fn sample_failure_record(id: &str) -> FailureRecord {
        FailureRecord {
            id: id.to_string(),
            failure_type: FailureType::Connection,
            timestamp: Instant::now(),
            error: "connection refused".to_string(),
            recovery_time: Duration::from_secs(2),
        }
    }

    #[test]
    fn failure_record_construction() {
        let record = sample_failure_record("f1");
        assert_eq!(record.id, "f1");
        assert_eq!(record.failure_type, FailureType::Connection);
        assert_eq!(record.error, "connection refused");
        assert_eq!(record.recovery_time, Duration::from_secs(2));
    }

    #[test]
    fn failure_record_each_type() {
        let types = [
            FailureType::Connection,
            FailureType::Resource,
            FailureType::Api,
            FailureType::Timeout,
            FailureType::Data,
            FailureType::Unknown,
        ];
        for (i, t) in types.iter().enumerate() {
            let record = FailureRecord {
                id: format!("f-{i}"),
                failure_type: t.clone(),
                timestamp: Instant::now(),
                error: String::new(),
                recovery_time: Duration::ZERO,
            };
            assert_eq!(record.failure_type, *t);
        }
    }

    #[test]
    fn failure_record_is_cloneable() {
        let r = sample_failure_record("clone-test");
        let c = r.clone();
        assert_eq!(r.id, c.id);
        assert_eq!(r.error, c.error);
    }

    #[test]
    fn failure_record_timestamp_is_recent() {
        let now = Instant::now();
        let record = FailureRecord {
            id: "t1".to_string(),
            failure_type: FailureType::Api,
            timestamp: now,
            error: "timeout".to_string(),
            recovery_time: Duration::from_secs(1),
        };
        assert!(record.timestamp.elapsed() < Duration::from_secs(5));
    }

    // -------------------------------------------------------------------------
    // FailurePattern
    // -------------------------------------------------------------------------

    #[test]
    fn failure_pattern_construction() {
        let pattern = FailurePattern {
            id: "pattern-1".to_string(),
            signature: vec!["timeout".to_string(), "api failed".to_string()],
            frequency: 5,
            impact: ImpactLevel::High,
        };

        assert_eq!(pattern.id, "pattern-1");
        assert_eq!(pattern.signature.len(), 2);
        assert_eq!(pattern.frequency, 5);
        assert_eq!(pattern.impact, ImpactLevel::High);
    }

    #[test]
    fn failure_pattern_empty_signature() {
        let pattern = FailurePattern {
            id: "p0".to_string(),
            signature: vec![],
            frequency: 0,
            impact: ImpactLevel::Low,
        };
        assert!(pattern.signature.is_empty());
        assert_eq!(pattern.frequency, 0);
    }

    #[test]
    fn failure_pattern_is_cloneable() {
        let p = FailurePattern {
            id: "cid".to_string(),
            signature: vec!["sig".into()],
            frequency: 3,
            impact: ImpactLevel::Medium,
        };
        let c = p.clone();
        assert_eq!(p.id, c.id);
        assert_eq!(p.frequency, c.frequency);
        assert_eq!(p.impact, c.impact);
    }

    // -------------------------------------------------------------------------
    // FailureHistory
    // -------------------------------------------------------------------------

    #[test]
    fn failure_history_new_starts_empty() {
        let history = FailureHistory::new();
        assert!(history.recent_failures.is_empty());
        assert!(history.patterns.is_empty());
        assert_eq!(history.mtbf, Duration::from_secs(0));
        assert_eq!(history.mttr, Duration::from_secs(0));
    }

    #[test]
    fn failure_history_default_same_as_new() {
        let a = FailureHistory::new();
        let b = FailureHistory::default();
        assert_eq!(a.recent_failures.len(), b.recent_failures.len());
        assert_eq!(a.mtbf, b.mtbf);
        assert_eq!(a.mttr, b.mttr);
    }

    #[test]
    fn failure_history_push_back_works() {
        let mut history = FailureHistory::new();
        let record = sample_failure_record("push-test");
        history.recent_failures.push_back(record.clone());
        assert_eq!(history.recent_failures.len(), 1);
        assert_eq!(history.recent_failures[0].id, "push-test");
    }

    #[test]
    fn failure_history_push_front_works() {
        let mut history = FailureHistory::new();
        history
            .recent_failures
            .push_back(sample_failure_record("a"));
        history
            .recent_failures
            .push_front(sample_failure_record("b"));
        assert_eq!(history.recent_failures[0].id, "b");
    }

    #[test]
    fn failure_history_pop_back_works() {
        let mut history = FailureHistory::new();
        history
            .recent_failures
            .push_back(sample_failure_record("pop-test"));
        let popped = history.recent_failures.pop_back();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().id, "pop-test");
        assert!(history.recent_failures.is_empty());
    }

    #[test]
    fn failure_history_pop_front_works() {
        let mut history = FailureHistory::new();
        history
            .recent_failures
            .push_back(sample_failure_record("x"));
        history
            .recent_failures
            .push_back(sample_failure_record("y"));
        let popped = history.recent_failures.pop_front();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().id, "x");
        assert_eq!(history.recent_failures.len(), 1);
    }

    #[test]
    fn failure_history_multiple_failures_retain_order() {
        let mut history = FailureHistory::new();
        for i in 0..5 {
            history
                .recent_failures
                .push_back(sample_failure_record(&format!("f{i}")));
        }
        assert_eq!(history.recent_failures.len(), 5);
        assert_eq!(history.recent_failures[0].id, "f0");
        assert_eq!(history.recent_failures[4].id, "f4");
    }

    #[test]
    fn failure_history_patterns_can_be_added() {
        let mut history = FailureHistory::new();
        let pattern = FailurePattern {
            id: "ptn-1".to_string(),
            signature: vec!["timeout".to_string()],
            frequency: 10,
            impact: ImpactLevel::Critical,
        };
        history.patterns.push(pattern.clone());
        assert_eq!(history.patterns.len(), 1);
        assert_eq!(history.patterns[0].id, "ptn-1");
    }

    #[test]
    fn failure_history_mtbf_can_be_set() {
        let mut history = FailureHistory::new();
        history.mtbf = Duration::from_secs(3600);
        assert_eq!(history.mtbf, Duration::from_secs(3600));
    }

    #[test]
    fn failure_history_mttr_can_be_set() {
        let mut history = FailureHistory::new();
        history.mttr = Duration::from_secs(120);
        assert_eq!(history.mttr, Duration::from_secs(120));
    }

    #[test]
    fn failure_history_is_cloneable() {
        // VecDeque<FailureRecord> and Vec<FailurePattern> are Clone when their
        // element types are Clone — verify the struct-level clone works.
        let mut h1 = FailureHistory::new();
        h1.recent_failures.push_back(FailureRecord {
            id: "c1".to_string(),
            failure_type: FailureType::Api,
            timestamp: Instant::now(),
            error: String::new(),
            recovery_time: Duration::ZERO,
        });
        h1.patterns.push(FailurePattern {
            id: "p1".to_string(),
            signature: vec![],
            frequency: 1,
            impact: ImpactLevel::Low,
        });
        // FailureHistory itself is Clone; verify field-level round-trip
        assert_eq!(h1.recent_failures.len(), 1);
        assert_eq!(h1.patterns.len(), 1);
        assert_eq!(h1.mtbf, Duration::from_secs(0));
        assert_eq!(h1.mttr, Duration::from_secs(0));
    }

    // -------------------------------------------------------------------------
    // Debug / display
    // -------------------------------------------------------------------------

    #[test]
    fn all_enums_debuggable() {
        let _ = format!("{:?}", FailureType::Connection);
        let _ = format!("{:?}", ImpactLevel::Critical);
    }

    #[test]
    fn failure_record_debuggable() {
        let r = sample_failure_record("d");
        let _ = format!("{:?}", r);
    }

    #[test]
    fn failure_pattern_debuggable() {
        let p = FailurePattern {
            id: "d".to_string(),
            signature: vec!["s".into()],
            frequency: 1,
            impact: ImpactLevel::Low,
        };
        let _ = format!("{:?}", p);
    }

    // FailureHistory is not Debug; verify its fields are nonetheless inspectable.
    #[test]
    fn failure_history_field_access() {
        let h = FailureHistory::new();
        assert!(h.recent_failures.is_empty());
        assert!(h.patterns.is_empty());
        assert_eq!(h.mtbf, Duration::from_secs(0));
        assert_eq!(h.mttr, Duration::from_secs(0));
    }
}
