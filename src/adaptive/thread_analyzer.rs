//! Thread analyzer for conversation context and relationship mapping.
//! Provides deep analysis of Twitter conversation threads and user interactions.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::utils::twitter::twitteractivity_sentiment::{Sentiment, analyze_tweet_sentiment};

/// Complete conversation context for a thread.
#[derive(Debug, Clone, Default)]
pub struct ConversationContext {
    /// Full thread structure with parent-child relationships
    pub thread_structure: ThreadGraph,
    /// Sentiment flow through the conversation
    pub sentiment_flow: SentimentFlowAnalysis,
    /// Key participants and their influence levels
    pub influential_participants: Vec<ParticipantInfluence>,
    /// Conversation dynamics and patterns
    pub conversation_dynamics: ConversationDynamics,
    /// Topic clusters within the conversation
    pub topic_clusters: Vec<TopicCluster>,
}

/// Thread graph structure representing conversation hierarchy.
#[derive(Debug, Clone, Default)]
pub struct ThreadGraph {
    /// Root tweet (original post)
    pub root_tweet: Option<TweetNode>,
    /// All tweets in the thread
    pub tweets: HashMap<String, TweetNode>,
    /// Reply relationships (parent_id -> Vec<child_id>)
    pub relationships: HashMap<String, Vec<String>>,
    /// Quoted tweets
    pub quotes: HashMap<String, Vec<String>>,
}

/// Single tweet node in the thread graph.
#[derive(Debug, Clone)]
pub struct TweetNode {
    /// Tweet identifier
    pub id: String,
    /// Author ID
    pub author: String,
    /// Tweet text
    pub text: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Sentiment analysis result
    pub sentiment: SentimentAnalysis,
    /// Engagement metrics
    pub engagement: EngagementMetrics,
    /// Position in thread (depth from root)
    pub depth: u32,
    /// Whether this is a reply
    pub is_reply: bool,
    /// Whether this is a quote tweet
    pub is_quote: bool,
}

/// Sentiment analysis for a single tweet.
#[derive(Debug, Clone)]
pub struct SentimentAnalysis {
    /// Overall sentiment
    pub sentiment: Sentiment,
    /// Sentiment score (-1.0 to 1.0)
    pub score: f32,
    /// Confidence level
    pub confidence: f32,
    /// Key sentiment factors
    pub factors: Vec<String>,
}

/// Engagement metrics for a tweet.
#[derive(Debug, Clone, Default)]
pub struct EngagementMetrics {
    /// Like count
    pub likes: u32,
    /// Retweet count
    pub retweets: u32,
    /// Reply count
    pub replies: u32,
    /// Quote count
    pub quotes: u32,
    /// View count
    pub views: u32,
    /// Engagement rate
    pub rate: f32,
}

/// Sentiment flow analysis through the conversation.
#[derive(Debug, Clone, Default)]
pub struct SentimentFlowAnalysis {
    /// Overall thread sentiment
    pub overall_sentiment: Sentiment,
    /// Average sentiment score
    pub avg_score: f32,
    /// Sentiment variance
    pub variance: f32,
    /// Sentiment by depth level
    pub by_depth: HashMap<u32, f32>,
    /// Sentiment trajectory (how sentiment changes)
    pub trajectory: Vec<(Instant, f32)>,
    /// Sentiment volatility
    pub volatility: f32,
}

/// Influence metrics for a participant.
#[derive(Debug, Clone)]
pub struct ParticipantInfluence {
    /// Participant identifier
    pub participant_id: String,
    /// Influence score (0.0 to 1.0)
    pub influence_score: f32,
    /// Number of interactions
    pub interaction_count: u32,
    /// Average sentiment of their contributions
    pub avg_sentiment: f32,
    /// Whether they are a topic initiator
    pub is_initiator: bool,
    /// Topics they contribute to
    pub topics: HashSet<String>,
    /// Response time metrics
    pub avg_response_time: f32,
    /// Network position metrics
    pub network_centrality: f32,
}

/// Conversation dynamics and patterns.
#[derive(Debug, Clone, Default)]
pub struct ConversationDynamics {
    /// Conversation style (debate, discussion, announcement, etc.)
    pub style: ConversationStyle,
    /// Engagement level (low, medium, high)
    pub engagement_level: EngagementLevel,
    /// Conflict indicators
    pub conflict_level: ConflictLevel,
    /// Topic coherence
    pub coherence_score: f32,
    /// Response patterns
    pub response_patterns: ResponsePatterns,
}

/// Conversation style classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationStyle {
    /// Balanced discussion
    Discussion,
    /// Debating/argumentative
    Debate,
    /// Information sharing
    Announcement,
    /// Question and answer
    QandA,
    /// Storytelling/narrative
    Storytelling,
    /// Mixed style
    Mixed,
}

/// Engagement level classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngagementLevel {
    /// Low engagement
    Low,
    /// Medium engagement
    Medium,
    /// High engagement
    High,
    /// Viral engagement
    Viral,
}

/// Conflict level indicators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictLevel {
    /// No conflict
    None,
    /// Minor disagreements
    Minor,
    /// Moderate conflict
    Moderate,
    /// High conflict/debate
    High,
}

/// Response pattern analysis.
#[derive(Debug, Clone, Default)]
pub struct ResponsePatterns {
    /// Average response time in seconds
    pub avg_response_time: f32,
    /// Response time standard deviation
    pub response_time_std: f32,
    /// Sequential response rate
    pub sequential_rate: f32,
    /// Thread continuation rate
    pub continuation_rate: f32,
}

/// Topic cluster analysis.
#[derive(Debug, Clone)]
pub struct TopicCluster {
    /// Topic identifier
    pub topic_id: String,
    /// Topic keywords
    pub keywords: Vec<String>,
    /// Tweets in this topic
    pub tweet_ids: Vec<String>,
    /// Topic sentiment
    pub sentiment: f32,
    /// Topic coherence score
    pub coherence: f32,
    /// Topic size (number of tweets)
    pub size: usize,
    /// Topic evolution over time
    pub evolution: TopicEvolution,
}

/// Topic evolution tracking.
#[derive(Debug, Clone, Default)]
pub struct TopicEvolution {
    /// Topic emergence time
    pub emergence_time: Instant,
    /// Topic peak time
    pub peak_time: Option<Instant>,
    /// Topic decline time
    pub decline_time: Option<Instant>,
    /// Topic lifespan
    pub lifespan: Option<Instant>,
    /// Sentiment changes over time
    pub sentiment_timeline: Vec<(Instant, f32)>,
}

impl ThreadGraph {
    /// Create a new empty thread graph.
    pub fn new() -> Self {
        Self {
            root_tweet: None,
            tweets: HashMap::new(),
            relationships: HashMap::new(),
            quotes: HashMap::new(),
        }
    }

    /// Add a tweet to the thread graph.
    pub fn add_tweet(&mut self, node: TweetNode) {
        self.tweets.insert(node.id.clone(), node);
    }

    /// Add a relationship between tweets.
    pub fn add_relationship(&mut self, parent_id: String, child_id: String) {
        self.relationships
            .entry(parent_id)
            .or_insert_with(Vec::new)
            .push(child_id);
    }

    /// Add a quote relationship.
    pub fn add_quote(&mut self, source_id: String, quoted_id: String) {
        self.quotes
            .entry(source_id)
            .or_insert_with(Vec::new)
            .push(quoted_id);
    }

    /// Get all replies to a specific tweet.
    pub fn get_replies(&self, tweet_id: &str) -> Vec<&TweetNode> {
        self.relationships
            .get(tweet_id)
            .map(|ids| ids.iter().filter_map(|id| self.tweets.get(id)).collect())
            .unwrap_or_default()
    }

    /// Calculate thread depth statistics.
    pub fn depth_statistics(&self) -> (u32, u32, f32) {
        if self.tweets.is_empty() {
            return (0, 0, 0.0);
        }

        let depths: Vec<u32> = self.tweets.values().map(|t| t.depth).collect();
        let max_depth = *depths.iter().max().unwrap();
        let min_depth = *depths.iter().min().unwrap();
        let avg_depth = depths.iter().sum::<u32>() as f32 / depths.len() as f32;

        (min_depth, max_depth, avg_depth)
    }
}

impl Default for ThreadGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationContext {
    /// Create a new conversation context from thread data.
    pub fn new(thread_graph: ThreadGraph) -> Self {
        let sentiment_flow = Self::analyze_sentiment_flow(&thread_graph);
        let influential_participants = Self::identify_influential_participants(&thread_graph);
        let conversation_dynamics = Self::analyze_conversation_dynamics(&thread_graph);
        let topic_clusters = Self::identify_topic_clusters(&thread_graph);

        Self {
            thread_structure: thread_graph,
            sentiment_flow,
            influential_participants,
            conversation_dynamics,
            topic_clusters,
        }
    }

    /// Analyze sentiment flow through the conversation.
    fn analyze_sentiment_flow(graph: &ThreadGraph) -> SentimentFlowAnalysis {
        if graph.tweets.is_empty() {
            return SentimentFlowAnalysis::default();
        }

        let sentiments: Vec<f32> = graph
            .tweets
            .values()
            .map(|t| t.sentiment.score)
            .collect();

        let avg_score = sentiments.iter().sum::<f32>() / sentiments.len() as f32;
        let variance = sentiments
            .iter()
            .map(|s| (s - avg_score).powi(2))
            .sum::<f32>()
            / sentiments.len() as f32;

        let mut sorted_by_time: Vec<_> = graph
            .tweets
            .values()
            .map(|t| (t.timestamp, t.sentiment.score))
            .collect();
        sorted_by_time.sort_by_key(|(time, _)| *time);

        SentimentFlowAnalysis {
            overall_sentiment: if avg_score > 0.1 {
                Sentiment::Positive
            } else if avg_score < -0.1 {
                Sentiment::Negative
            } else {
                Sentiment::Neutral
            },
            avg_score,
            variance,
            by_depth: HashMap::new(), // Would be populated with depth analysis
            trajectory: sorted_by_time,
            volatility: variance.sqrt(),
        }
    }

    /// Identify influential participants in the conversation.
    fn identify_influential_participants(graph: &ThreadGraph) -> Vec<ParticipantInfluence> {
        let mut participant_stats: HashMap<String, (u32, f32, HashSet<String>)> = HashMap::new();

        for tweet in graph.tweets.values() {
            let entry = participant_stats
                .entry(tweet.author.clone())
                .or_insert((0, 0.0, HashSet::new()));
            entry.0 += 1;
            entry.1 += tweet.sentiment.score;
            entry.2.insert(topic_keywords(&tweet.text));
        }

        let total_tweets = graph.tweets.len() as f32;
        let mut participants: Vec<ParticipantInfluence> = participant_stats
            .into_iter()
            .map(|(id, (count, total_sentiment, topics))| {
                let avg_sentiment = if count > 0 {
                    total_sentiment / count as f32
                } else {
                    0.0
                };
                let influence_score = (count as f32 / total_tweets).min(1.0);

                ParticipantInfluence {
                    participant_id: id,
                    influence_score,
                    interaction_count: count,
                    avg_sentiment,
                    is_initiator: graph.root_tweet.as_ref().map_or(false, |r| r.author == id),
                    topics: topics,
                    avg_response_time: 0.0, // Would be calculated from timestamps
                    network_centrality: influence_score * (1.0 + avg_sentiment.abs()),
                }
            })
            .collect();

        participants.sort_by(|a, b| b.influence_score.partial_cmp(&a.influence_score).unwrap());
        participants
    }

    /// Analyze conversation dynamics and patterns.
    fn analyze_conversation_dynamics(graph: &ThreadGraph) -> ConversationDynamics {
        let total_tweets = graph.tweets.len();
        if total_tweets == 0 {
            return ConversationDynamics::default();
        }

        let engagement_levels: Vec<u32> = graph.tweets.values().map(|t| t.engagement.total()).collect();
        let avg_engagement = engagement_levels.iter().sum::<u32>() as f32 / total_tweets as f32;

        let style = if avg_engagement > 10.0 {
            ConversationStyle::Debate
        } else if avg_engagement > 5.0 {
            ConversationStyle::Discussion
        } else {
            ConversationStyle::Announcement
        };

        let conflict_level = if graph.tweets.values().any(|t| t.sentiment.score < -0.5) {
            ConflictLevel::Moderate
        } else {
            ConflictLevel::None
        };

        ConversationDynamics {
            style,
            engagement_level: if avg_engagement > 15.0 {
                EngagementLevel::High
            } else if avg_engagement > 5.0 {
                EngagementLevel::Medium
            } else {
                EngagementLevel::Low
            },
            conflict_level,
            coherence_score: 0.7, // Would be calculated from topic coherence
            response_patterns: ResponsePatterns::default(),
        }
    }

    /// Identify topic clusters within the conversation.
    fn identify_topic_clusters(graph: &ThreadGraph) -> Vec<TopicCluster> {
        // Simplified topic clustering
        // In production, would use NLP techniques like LDA or BERT embeddings
        vec![TopicCluster {
            topic_id: "main_topic".to_string(),
            keywords: vec!["content".to_string(), "discussion".to_string()],
            tweet_ids: graph.tweets.keys().cloned().collect(),
            sentiment: graph.tweets.values().map(|t| t.sentiment.score).sum::<f32>() / graph.tweets.len() as f32,
            coherence: 0.8,
            size: graph.tweets.len(),
            evolution: TopicEvolution::default(),
        }]
    }

    /// Get conversation summary statistics.
    pub fn summary(&self) -> ConversationSummary {
        let (min_depth, max_depth, avg_depth) = self.thread_structure.depth_statistics();
        let total_engagement: EngagementMetrics = self
            .thread_structure
            .tweets
            .values()
            .map(|t| t.engagement.clone())
            .sum();

        ConversationSummary {
            total_tweets: self.thread_structure.tweets.len(),
            total_participants: self.influential_participants.len(),
            min_depth,
            max_depth,
            avg_depth,
            total_engagement,
            sentiment_flow: self.sentiment_flow.clone(),
            conversation_style: self.conversation_dynamics.style,
        }
    }
}

/// Helper function to extract topic keywords from text.
fn topic_keywords(text: &str) -> String {
    // Simplified keyword extraction
    // In production, would use NLP techniques
    text.split_whitespace()
        .filter(|word| word.len() > 4)
        .take(5)
        .collect::<Vec<_>>()
        .join("_")
}

/// Summary of conversation analysis.
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub total_tweets: usize,
    pub total_participants: usize,
    pub min_depth: u32,
    pub max_depth: u32,
    pub avg_depth: f32,
    pub total_engagement: EngagementMetrics,
    pub sentiment_flow: SentimentFlowAnalysis,
    pub conversation_style: ConversationStyle,
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self::new(ThreadGraph::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_graph_creation() {
        let mut graph = ThreadGraph::new();
        let tweet = TweetNode {
            id: "1".to_string(),
            author: "user1".to_string(),
            text: "Hello world".to_string(),
            timestamp: Instant::now(),
            sentiment: SentimentAnalysis {
                sentiment: Sentiment::Neutral,
                score: 0.0,
                confidence: 1.0,
                factors: vec![],
            },
            engagement: EngagementMetrics::default(),
            depth: 0,
            is_reply: false,
            is_quote: false,
        };
        
        graph.add_tweet(tweet);
        assert_eq!(graph.tweets.len(), 1);
    }

    #[test]
    fn test_conversation_context_creation() {
        let mut graph = ThreadGraph::new();
        let tweet = TweetNode {
            id: "1".to_string(),
            author: "user1".to_string(),
            text: "Hello world".to_string(),
            timestamp: Instant::now(),
            sentiment: SentimentAnalysis {
                sentiment: Sentiment::Neutral,
                score: 0.0,
                confidence: 1.0,
                factors: vec![],
            },
            engagement: EngagementMetrics::default(),
            depth: 0,
            is_reply: false,
            is_quote: false,
        };
        
        graph.add_tweet(tweet);
        let context = ConversationContext::new(graph);
        
        assert_eq!(context.influential_participants.len(), 1);
        assert!(context.sentiment_flow.avg_score >= -1.0);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_conversation_analysis() {
        // Create a thread with multiple tweets
        let mut graph = ThreadGraph::new();
        
        let root_tweet = TweetNode {
            id: "root".to_string(),
            author: "user1".to_string(),
            text: "What do you think about AI?".to_string(),
            timestamp: Instant::now(),
            sentiment: SentimentAnalysis {
                sentiment: Sentiment::Neutral,
                score: 0.0,
                confidence: 1.0,
                factors: vec![],
            },
            engagement: EngagementMetrics::default(),
            depth: 0,
            is_reply: false,
            is_quote: false,
        };
        
        let reply_tweet = TweetNode {
            id: "reply1".to_string(),
            author: "user2".to_string(),
            text: "AI is amazing!".to_string(),
            timestamp: Instant::now(),
            sentiment: SentimentAnalysis {
                sentiment: Sentiment::Positive,
                score: 0.8,
                confidence: 0.9,
                factors: vec!["amazing".to_string()],
            },
            engagement: EngagementMetrics {
                likes: 5,
                retweets: 2,
                ..Default::default()
            },
            depth: 1,
            is_reply: true,
            is_quote: false,
        };
        
        graph.add_tweet(root_tweet);
        graph.add_tweet(reply_tweet);
        graph.add_relationship("root".to_string(), "reply1".to_string());
        
        let context = ConversationContext::new(graph);
        let summary = context.summary();
        
        assert_eq!(summary.total_tweets, 2);
        assert!(summary.sentiment_flow.avg_score > 0.0);
    }
}
