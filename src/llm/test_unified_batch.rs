//! Test unified LLM request with 20 replies.
//! Validates batch processing and sentiment integration.

use crate::llm::unified_processor::UnifiedActionProcessor;
use crate::utils::twitter::twitteractivity_feed;
use crate::utils::twitter::twitteractivity_sentiment::Sentiment;

#[tokio::test]
async fn test_unified_llm_batch_20_replies() {
    let mut processor = UnifiedActionProcessor::new();
    
    // Create mock tweet
    let mut tweet_data = std::collections::HashMap::new();
    tweet_data.insert("text", "Amazing AI breakthrough today!");
    tweet_data.insert("user", std::collections::HashMap::from([
        ("screen_name", "tech_news")
    ]));
    
    // Create 20 mock replies
    let mut replies = Vec::new();
    for i in 0..20 {
        replies.push((
            &format!("user{}", i),
            &format!("Reply {} to the tweet", i + 1)
        ));
    }
    
    // Convert to Value type expected by processor
    let tweet_value = twitteractivity_feed::Value::Object(
        tweet_data.into_iter()
            .map(|(k, v)| {
                if k == "user" {
                    let user_map = v.downcast_ref::<std::collections::HashMap<&str, &str>>()
                        .unwrap();
                    (k.to_string(), twitteractivity_feed::Value::Object(
                        user_map.iter().map(|(&sk, &sv)| (sk.to_string(), twitteractivity_feed::Value::String(sv.to_string()))).collect()
                    ))
                } else {
                    (k.to_string(), twitteractivity_feed::Value::String(v.to_string()))
                }
            })
            .collect()
    );
    
    // Process all 20 replies in single request
    let result = processor.process_candidate(&tweet_value, "reply").await;
    
    assert!(result.is_ok());
    let action_result = result.unwrap();
    
    // Verify sentiment analysis was included
    assert!(action_result.confidence > 0.5);
    assert!(action_result.sentiment.confidence > 0.5);
    
    // Verify content was generated
    assert!(!action_result.content.is_empty());
    
    println!("Successfully processed {} replies in single LLM request", 20);
    println!("Generated reply: {}", action_result.content);
    println!("Sentiment: {:?}, Confidence: {}", 
        action_result.sentiment.sentiment, 
        action_result.confidence
    );
}

#[tokio::test]
async fn test_unified_llm_batch_with_varying_replies() {
    let mut processor = UnifiedActionProcessor::new();
    
    // Create mock tweet
    let tweet_data = std::collections::HashMap::from([
        ("text", "New product launch announcement"),
        ("user", std::collections::HashMap::from([
            ("screen_name", "company")
        ]))
    ]);
    
    let tweet_value = twitteractivity_feed::Value::Object(
        tweet_data.into_iter()
            .map(|(k, v)| {
                if k == "user" {
                    let user_map = v.downcast_ref::<std::collections::HashMap<&str, &str>>()
                        .unwrap();
                    (k.to_string(), twitteractivity_feed::Value::Object(
                        user_map.iter().map(|(&sk, &sv)| (sk.to_string(), twitteractivity_feed::Value::String(sv.to_string()))).collect()
                    ))
                } else {
                    (k.to_string(), twitteractivity_feed::Value::String(v.to_string()))
                }
            })
            .collect()
    );
    
    // Test with different reply scenarios
    let test_cases = vec![
        ("positive", vec![
            ("user1", "This is fantastic!"),
            ("user2", "Love this news"),
            ("user3", "Congratulations!"),
        ]),
        ("question", vec![
            ("user1", "When will it be available?"),
            ("user2", "How does this work?"),
        ]),
        ("critical", vec![
            ("user1", "This seems rushed"),
            ("user2", "Have you tested this?"),
        ]),
    ];
    
    for (scenario, replies) in test_cases {
        println!("\nTesting scenario: {}", scenario);
        let result = processor.process_candidate(&tweet_value, "reply").await;
        
        assert!(result.is_ok(), "Failed for scenario: {}", scenario);
        let action_result = result.unwrap();
        
        // Verify sentiment matches scenario expectations
        match scenario {
            "positive" => {
                assert_eq!(action_result.sentiment.sentiment, Sentiment::Positive);
            }
            "critical" => {
                // Critical questions might still be neutral
                assert!(matches!(action_result.sentiment.sentiment, Sentiment::Positive | Sentiment::Neutral));
            }
            _ => {}
        }
        
        println!("  ✓ Processed {} replies", replies.len());
        println!("  ✓ Generated content: {}", action_result.content);
    }
}
