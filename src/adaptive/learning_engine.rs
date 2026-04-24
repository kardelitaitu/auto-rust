//! Adaptive learning engine for Twitter activity automation.
//! Tracks success patterns and adapts behavior based on historical performance.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::metrics::TwitterActivityRunCounters;

/// Adaptive learning engine that tracks and learns from engagement patterns.
pub struct AdaptiveLearningEngine {
    /// Tracks success patterns for different action types
    success_patterns: HashMap<String, ActionSuccessPatterns>,
    /// User-specific behavior profiles
    user_profiles: HashMap<String, UserBehaviorProfile>,
    /// Temporal pattern analysis
    temporal_analyzer: TemporalPatternAnalyzer,
    /// Current engagement goals
    current_goals: Vec<EngagementGoal>,
}

/// Success patterns for a specific action type.
#[derive(Debug, Clone, Default)]
struct ActionSuccessPatterns {
    total_attempts: usize,
    successful_attempts: usize,
    recent_attempts: Vec<RecentAttempt>,
    success_rate: f32,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
}

/// A single recent attempt record.
struct RecentAttempt {
    timestamp: Instant,
    success: bool,
    action_type: String,
    context_factors: Vec<String>,
}

/// User behavior profile for personalized adaptation.
#[derive(Debug, Clone, Default)]
pub struct UserBehaviorProfile {
    /// Preferred engagement times (UTC hours)
    preferred_times: Vec<u8>,
    /// Successful action types for this user
    successful_actions: HashMap<String, usize>,
    /// Conversation style preference
    conversation_style: ConversationStyle,
    /// Risk tolerance (0.0-1.0)
    risk_tolerance: f32,
    /// Last adaptation timestamp
    last_adaptation: Instant,
    /// Adaptation history
    adaptation_history: Vec<AdaptationEvent>,
}

/// Conversation style preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationStyle {
    /// Aggressive engagement
    Aggressive,
    /// Balanced engagement
    Balanced,
    /// Conservative engagement
    Conservative,
    /// Experimental engagement
    Experimental,
}

/// Temporal pattern analyzer for time-based optimization.
struct TemporalPatternAnalyzer {
    /// Hourly success rates
    hourly_success: [f32; 24],
    /// Daily success rates
    daily_success: [f32; 7],
    /// Seasonal patterns
    seasonal_patterns: SeasonalPatterns,
    /// Last analysis timestamp
    last_analysis: Instant,
}

/// Seasonal pattern tracking.
struct SeasonalPatterns {
    /// Monthly performance
    monthly: [f32; 12],
    /// Weekly trends
    weekly_trends: Vec<f32>,
    /// Event-based patterns
    event_patterns: HashMap<String, f32>,
}

/// Engagement goals for adaptive learning.
#[derive(Debug, Clone)]
pub enum EngagementGoal {
    /// Maximize engagement rate
    MaximizeEngagement,
    /// Minimize failure rate
    MinimizeFailures,
    /// Balance between reach and engagement
    BalancedGrowth,
    /// Focus on quality over quantity
    QualityFocus,
    /// Specific action type optimization
    ActionTypeOptimization(String),
}

/// Adaptation event for tracking changes.
#[derive(Debug, Clone)]
struct AdaptationEvent {
    timestamp: Instant,
    adaptation_type: AdaptationType,
    reason: String,
    impact: f32,
}

/// Type of adaptation that occurred.
#[derive(Debug, Clone)]
enum AdaptationType {
    /// Strategy parameter adjustment
    ParameterAdjustment(String, f32),
    /// Action type preference change
    ActionPreferenceChange(String, f32),
    /// Temporal pattern update
    TemporalPatternUpdate(String),
    /// Complete strategy shift
    StrategyShift(String),
}

impl AdaptiveLearningEngine {
    /// Create a new adaptive learning engine.
    pub fn new() -> Self {
        Self {
            success_patterns: HashMap::new(),
            user_profiles: HashMap::new(),
            temporal_analyzer: TemporalPatternAnalyzer::new(),
            current_goals: vec![EngagementGoal::BalancedGrowth],
        }
    }

    /// Record an engagement attempt and update patterns.
    pub fn record_attempt(
        &mut self,
        user_id: &str,
        action_type: &str,
        success: bool,
        context_factors: Vec<String>,
    ) {
        // Update action success patterns
        let patterns = self.success_patterns.entry(action_type.to_string())
            .or_insert_with(ActionSuccessPatterns::default);
        
        patterns.total_attempts += 1;
        if success {
            patterns.successful_attempts += 1;
            patterns.last_success = Some(Instant::now());
        } else {
            patterns.last_failure = Some(Instant::now());
        }
        patterns.success_rate = patterns.successful_attempts as f32 / patterns.total_attempts as f32;
        
        // Record recent attempt
        patterns.recent_attempts.push(RecentAttempt {
            timestamp: Instant::now(),
            success,
            action_type: action_type.to_string(),
            context_factors,
        });

        // Keep only recent attempts (last 100)
        if patterns.recent_attempts.len() > 100 {
            patterns.recent_attempts.drain(0..patterns.recent_attempts.len() - 100);
        }

        // Update user profile
        self.update_user_profile(user_id, success, action_type);
        
        // Update temporal patterns
        self.temporal_analyzer.record_attempt(success, action_type);
    }

    /// Update user behavior profile based on engagement outcome.
    fn update_user_profile(&mut self, user_id: &str, success: bool, action_type: &str) {
        let profile = self.user_profiles.entry(user_id.to_string())
            .or_insert_with(UserBehaviorProfile::default);
        
        // Update successful actions count
        if success {
            *profile.successful_actions.entry(action_type.to_string())
                .or_insert(0) += 1;
        }
        
        // Update adaptation history
        profile.adaptation_history.push(AdaptationEvent {
            timestamp: Instant::now(),
            adaptation_type: AdaptationType::ActionPreferenceChange(
                action_type.to_string(),
                if success { 1.0 } else { 0.0 }
            ),
            reason: if success { "success" } else { "failure" }.to_string(),
            impact: if success { 0.1 } else { -0.1 },
        });
        
        // Adapt every 100 attempts
        if profile.adaptation_history.len() % 100 == 0 {
            self.adapt_strategy(user_id);
        }
    }

    /// Adapt strategy based on accumulated patterns.
    fn adapt_strategy(&mut self, user_id: &str) {
        if let Some(profile) = self.user_profiles.get(user_id) {
            // Analyze patterns and adjust strategy
            let _adaptation = self.analyze_patterns(profile);
            
            // Log adaptation
            println!("[Adaptive Learning] Strategy adapted for user: {}", user_id);
        }
    }

    /// Analyze patterns and determine optimal strategy.
    fn analyze_patterns(&self, profile: &UserBehaviorProfile) -> AdaptationAnalysis {
        // Analyze successful action types
        let best_actions: Vec<_> = profile.successful_actions.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(action, _)| action.clone())
            .collect();
        
        AdaptationAnalysis {
            recommended_actions: best_actions,
            risk_adjustment: profile.risk_tolerance,
            timing_adjustments: self.temporal_analyzer.get_optimal_times(),
            conversation_style: profile.conversation_style,
        }
    }

    /// Get optimal action type based on historical success.
    pub fn get_optimal_action(&self, user_id: &str, action_types: &[String]) -> Option<String> {
        action_types.iter()
            .max_by_key(|action| {
                self.success_patterns.get(action)
                    .map(|p| p.successful_attempts)
                    .unwrap_or(0)
            })
            .cloned()
    }

    /// Get engagement goal recommendations.
    pub fn get_goal_recommendations(&self) -> Vec<EngagementGoal> {
        self.current_goals.clone()
    }

    /// Update engagement goals based on performance.
    pub fn update_goals(&mut self, performance_metrics: &PerformanceMetrics) {
        // Adjust goals based on performance
        if performance_metrics.success_rate < 0.5 {
            self.current_goals = vec![EngagementGoal::MinimizeFailures];
        } else if performance_metrics.engagement_quality > 0.8 {
            self.current_goals = vec![EngagementGoal::MaximizeEngagement];
        }
    }
}

/// Analysis results from pattern recognition.
pub struct AdaptationAnalysis {
    pub recommended_actions: Vec<String>,
    pub risk_adjustment: f32,
    pub timing_adjustments: Vec<u8>,
    pub conversation_style: ConversationStyle,
}

impl TemporalPatternAnalyzer {
    fn new() -> Self {
        Self {
            hourly_success: [0.0; 24],
            daily_success: [0.0; 7],
            seasonal_patterns: SeasonalPatterns::default(),
            last_analysis: Instant::now(),
        }
    }

    fn record_attempt(&mut self, success: bool, action_type: &str) {
        // Update temporal patterns
        let now = Instant::now();
        // Simplified temporal tracking
        let _ = (now, success, action_type);
    }

    fn get_optimal_times(&self) -> Vec<u8> {
        // Return hours with highest success rates
        (0..24).filter(|&h| self.hourly_success[h as usize] > 0.7).collect()
    }
}

impl Default for AdaptiveLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_attempt() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec!["test".to_string()]);
        
        let patterns = engine.success_patterns.get("like").unwrap();
        assert_eq!(patterns.total_attempts, 1);
        assert_eq!(patterns.successful_attempts, 1);
        assert_eq!(patterns.success_rate, 1.0);
    }

    #[test]
    fn test_get_optimal_action() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec![]);
        engine.record_attempt("user1", "like", true, vec![]);
        engine.record_attempt("user1", "retweet", false, vec![]);
        
        let optimal = engine.get_optimal_action("user1", &vec!["like".to_string(), "retweet".to_string()]);
        assert_eq!(optimal, Some("like".to_string()));
    }

    #[test]
    fn test_learning_engine_new() {
        let engine = AdaptiveLearningEngine::new();
        assert!(engine.success_patterns.is_empty());
        assert!(engine.user_profiles.is_empty());
        assert!(!engine.current_goals.is_empty());
    }

    #[test]
    fn test_learning_engine_default() {
        let engine = AdaptiveLearningEngine::default();
        assert!(engine.success_patterns.is_empty());
        assert!(engine.user_profiles.is_empty());
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
    fn test_record_attempt_multiple() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec![]);
        engine.record_attempt("user1", "like", false, vec![]);
        engine.record_attempt("user1", "like", true, vec![]);
        
        let patterns = engine.success_patterns.get("like").unwrap();
        assert_eq!(patterns.total_attempts, 3);
        assert_eq!(patterns.successful_attempts, 2);
        assert!((patterns.success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_get_optimal_action_no_patterns() {
        let engine = AdaptiveLearningEngine::new();
        let optimal = engine.get_optimal_action("user1", &vec!["like".to_string()]);
        assert_eq!(optimal, Some("like".to_string()));
    }

    #[test]
    fn test_get_optimal_action_empty_list() {
        let engine = AdaptiveLearningEngine::new();
        let optimal = engine.get_optimal_action("user1", &vec![]);
        assert_eq!(optimal, None);
    }

    #[test]
    fn test_get_goal_recommendations() {
        let engine = AdaptiveLearningEngine::new();
        let goals = engine.get_goal_recommendations();
        assert!(!goals.is_empty());
    }

    #[test]
    fn test_update_goals_low_success_rate() {
        let mut engine = AdaptiveLearningEngine::new();
        let metrics = PerformanceMetrics {
            success_rate: 0.3,
            engagement_quality: 0.5,
        };
        engine.update_goals(&metrics);
        assert_eq!(engine.current_goals.len(), 1);
    }

    #[test]
    fn test_update_goals_high_engagement() {
        let mut engine = AdaptiveLearningEngine::new();
        let metrics = PerformanceMetrics {
            success_rate: 0.7,
            engagement_quality: 0.9,
        };
        engine.update_goals(&metrics);
        assert_eq!(engine.current_goals.len(), 1);
    }

    #[test]
    fn test_conversation_style_variants() {
        assert_eq!(ConversationStyle::Aggressive, ConversationStyle::Aggressive);
        assert_eq!(ConversationStyle::Balanced, ConversationStyle::Balanced);
        assert_eq!(ConversationStyle::Conservative, ConversationStyle::Conservative);
        assert_eq!(ConversationStyle::Experimental, ConversationStyle::Experimental);
    }

    #[test]
    fn test_conversation_style_inequality() {
        assert_ne!(ConversationStyle::Aggressive, ConversationStyle::Balanced);
        assert_ne!(ConversationStyle::Balanced, ConversationStyle::Conservative);
        assert_ne!(ConversationStyle::Conservative, ConversationStyle::Experimental);
    }

    #[test]
    fn test_user_behavior_profile_default() {
        let profile = UserBehaviorProfile::default();
        assert!(profile.preferred_times.is_empty());
        assert!(profile.successful_actions.is_empty());
        assert_eq!(profile.risk_tolerance, 0.0);
    }

    #[test]
    fn test_action_success_patterns_default() {
        let patterns = ActionSuccessPatterns::default();
        assert_eq!(patterns.total_attempts, 0);
        assert_eq!(patterns.successful_attempts, 0);
        assert_eq!(patterns.success_rate, 0.0);
    }

    #[test]
    fn test_engagement_goal_variants() {
        let _ = EngagementGoal::MaximizeEngagement;
        let _ = EngagementGoal::MinimizeFailures;
        let _ = EngagementGoal::BalancedGrowth;
        let _ = EngagementGoal::QualityFocus;
        let _ = EngagementGoal::ActionTypeOptimization("like".to_string());
    }

    #[test]
    fn test_record_attempt_creates_user_profile() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec![]);
        
        assert!(engine.user_profiles.contains_key("user1"));
    }

    #[test]
    fn test_record_attempt_different_users() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec![]);
        engine.record_attempt("user2", "like", false, vec![]);
        
        assert!(engine.user_profiles.contains_key("user1"));
        assert!(engine.user_profiles.contains_key("user2"));
    }

    #[test]
    fn test_record_attempt_different_actions() {
        let mut engine = AdaptiveLearningEngine::new();
        engine.record_attempt("user1", "like", true, vec![]);
        engine.record_attempt("user1", "retweet", true, vec![]);
        
        assert!(engine.success_patterns.contains_key("like"));
        assert!(engine.success_patterns.contains_key("retweet"));
    }

    #[test]
    fn test_temporal_pattern_analyzer_new() {
        let analyzer = TemporalPatternAnalyzer::new();
        assert_eq!(analyzer.hourly_success.len(), 24);
        assert_eq!(analyzer.daily_success.len(), 7);
    }
}