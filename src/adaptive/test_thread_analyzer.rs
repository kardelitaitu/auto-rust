//! Tests for thread analyzer.
//! Validates conversation context analysis and participant tracking.

use crate::adaptive::thread_analyzer::{ThreadAnalyzer, ThreadContext, Participant};

#[test]
fn test_thread_analyzer_creation() {
    let analyzer = ThreadAnalyzer::new();
    assert!(analyzer.thread_cache.is_empty());
}

#[test]
fn test_analyze_thread() {
    let mut analyzer = ThreadAnalyzer::new();
    
    let tweets = vec![
        ("user1", "Original tweet"),
        ("user2", "Reply to original"),
        ("user1", "Response to reply"),
    ];
    
    let context = analyzer.analyze_thread(&tweets);
    assert_eq!(context.participant_count, 2);
    assert!(context.depth >= 1);
}

#[test]
fn test_track_participant() {
    let mut analyzer = ThreadAnalyzer::new();
    
    analyzer.track_participant("user1", "tweet");
    analyzer.track_participant("user2", "reply");
    
    assert!(analyzer.participants.contains_key("user1"));
    assert!(analyzer.participants.contains_key("user2"));
}

#[test]
fn test_extract_topic() {
    let analyzer = ThreadAnalyzer::new();
    
    let topic = analyzer.extract_topic("AI breakthrough today");
    assert!(!topic.is_empty());
}

#[test]
fn test_calculate_engagement_potential() {
    let analyzer = ThreadAnalyzer::new();
    
    let context = ThreadContext {
        participant_count: 5,
        depth: 3,
        topic: "AI".to_string(),
        sentiment_score: 0.8,
    };
    
    let potential = analyzer.calculate_engagement_potential(&context);
    assert!(potential >= 0.0 && potential <= 1.0);
}

#[test]
fn test_identify_key_participants() {
    let mut analyzer = ThreadAnalyzer::new();
    
    analyzer.track_participant("user1", "tweet");
    analyzer.track_participant("user2", "reply");
    analyzer.track_participant("user2", "reply");
    
    let key_participants = analyzer.identify_key_participants(1);
    assert!(!key_participants.is_empty());
}
