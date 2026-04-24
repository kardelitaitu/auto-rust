//! Tests for adaptive learning engine.
//! Validates pattern tracking, user profiling, and strategy adaptation.

use crate::adaptive::learning_engine::{
    AdaptiveLearningEngine, EngagementGoal, UserBehaviorProfile,
};

#[test]
fn test_learning_engine_creation() {
    let engine = AdaptiveLearningEngine::new();
    assert!(engine.success_patterns.is_empty());
    assert!(engine.user_profiles.is_empty());
}

#[test]
fn test_record_attempt_success() {
    let mut engine = AdaptiveLearningEngine::new();
    engine.record_attempt("user1", "like", true, vec!["test".to_string()]);
    
    let patterns = engine.success_patterns.get("like").unwrap();
    assert_eq!(patterns.total_attempts, 1);
    assert_eq!(patterns.successful_attempts, 1);
    assert_eq!(patterns.success_rate, 1.0);
}

#[test]
fn test_record_attempt_failure() {
    let mut engine = AdaptiveLearningEngine::new();
    engine.record_attempt("user1", "like", false, vec!["test".to_string()]);
    
    let patterns = engine.success_patterns.get("like").unwrap();
    assert_eq!(patterns.total_attempts, 1);
    assert_eq!(patterns.successful_attempts, 0);
    assert_eq!(patterns.success_rate, 0.0);
}

#[test]
fn test_record_multiple_attempts() {
    let mut engine = AdaptiveLearningEngine::new();
    
    // Record 3 successes and 1 failure
    engine.record_attempt("user1", "like", true, vec![]);
    engine.record_attempt("user1", "like", true, vec![]);
    engine.record_attempt("user1", "like", false, vec![]);
    engine.record_attempt("user1", "like", true, vec![]);
    
    let patterns = engine.success_patterns.get("like").unwrap();
    assert_eq!(patterns.total_attempts, 4);
    assert_eq!(patterns.successful_attempts, 3);
    assert_eq!(patterns.success_rate, 0.75);
}

#[test]
fn test_get_optimal_action() {
    let mut engine = AdaptiveLearningEngine::new();
    
    // Record successes for different action types
    engine.record_attempt("user1", "like", true, vec![]);
    engine.record_attempt("user1", "like", true, vec![]);
    engine.record_attempt("user1", "retweet", true, vec![]);
    
    let action_types = vec!["like".to_string(), "retweet".to_string()];
    let optimal = engine.get_optimal_action("user1", &action_types);
    
    assert!(optimal.is_some());
    assert_eq!(optimal.unwrap(), "like"); // like has more successes
}

#[test]
fn test_get_goal_recommendations() {
    let engine = AdaptiveLearningEngine::new();
    let goals = engine.get_goal_recommendations();
    
    assert!(!goals.is_empty());
}

#[test]
fn test_user_profile_creation() {
    let mut engine = AdaptiveLearningEngine::new();
    engine.record_attempt("user1", "like", true, vec![]);
    
    let profile = engine.user_profiles.get("user1");
    assert!(profile.is_some());
}

#[test]
fn test_recent_attempts_limit() {
    let mut engine = AdaptiveLearningEngine::new();
    
    // Record more than 100 attempts
    for i in 0..150 {
        engine.record_attempt("user1", "like", true, vec![format!("context_{}", i)]);
    }
    
    let patterns = engine.success_patterns.get("like").unwrap();
    assert!(patterns.recent_attempts.len() <= 100);
}
