# Twitter Activity V2 Roadmap

## Overview

V2 enhances the Twitter activity automation with **LLM-powered engagement** for authentic, context-aware interactions.

---

## ✅ V1 Features (Complete)

| Feature | Status | Notes |
|---------|--------|-------|
| Like | ✅ Production | With rate limits |
| Retweet | ✅ Production | Native RT only |
| Follow | ✅ Production | With limit checks |
| Reply | ⚠️ Stub | Basic text generation |
| Thread Dive | ✅ Production | Persona-based probability |
| Bookmark | ⚠️ Disabled | `max_bookmarks=0` |
| Quote Tweet | ❌ V2 | Requires LLM |

**V1 Limits:** 5 likes, 3 RTs, 2 follows, 10 total actions per session

---

## 🎯 V2 Goals

### **1. LLM-Powered Replies** 
Generate contextual, human-like replies based on tweet content and community sentiment.

**Implementation:**
```rust
// src/utils/twitter/twitteractivity_llm.rs
pub async fn generate_reply(
    api: &TaskContext,
    tweet_author: &str,
    tweet_text: &str,
    top_replies: Vec<(String, String)>,
) -> Result<String> {
    let llm = Llm::new()?;
    let messages = build_reply_messages(tweet_author, tweet_text, &top_replies);
    llm.chat_with_fallback(messages).await
}
```

**Prompt Strategy:**
- Read top 5-10 replies for context
- Match community sentiment/tone
- Generate unique, specific response
- Enforce Twitter formatting (no emojis, mentions, hashtags)

**Safety Guards:**
- Blocklist for controversial topics
- Sentiment analysis (skip negative tweets)
- Character limit enforcement (<280 chars)
- Language matching (reply in same language as tweet)

---

### **2. Quote Tweets with Commentary**
Add original commentary when quote tweeting, building on community conversation.

**Implementation:**
```rust
pub async fn quote_tweet_with_commentary(
    api: &TaskContext,
    tweet_url: &str,
) -> Result<bool> {
    // 1. Extract tweet text + replies
    let (author, text, replies) = extract_tweet_context(api, tweet_url).await?;
    
    // 2. Generate commentary via LLM
    let messages = build_quote_messages(&author, &text, &replies);
    let commentary = llm.chat_with_fallback(messages).await?;
    
    // 3. Click quote tweet button
    click_quote_tweet_button(api).await?;
    
    // 4. Type commentary
    api.keyboard("[data-testid='tweetTextarea']", &commentary).await?;
    
    // 5. Submit
    api.press("Enter").await?;
    
    Ok(true)
}
```

**Configuration:**
```toml
[twitter_activity.llm]
enabled = true
provider = "ollama"  # or "openrouter"
model = "llama3.2:latest"
temperature = 0.7
max_tokens = 100
quote_tweet_probability = 0.15  # 15% of RTs become quote tweets
```

---

### **3. Smart Engagement Decisions**
Use LLM to decide WHETHER to engage, not just HOW to engage.

**Implementation:**
```rust
pub struct EngagementDecision {
    pub should_engage: bool,
    pub engagement_type: Option<EngagementType>,
    pub confidence: f64,
    pub reasoning: String,
}

pub async fn decide_engagement(
    tweet_text: &str,
    context: &TweetContext,
) -> Result<EngagementDecision> {
    let prompt = format!(
        "Should I engage with this tweet? Consider:\n\
         - Content quality\n\
         - Controversy level\n\
         - Community sentiment\n\
         \nTweet: {}",
        tweet_text
    );
    
    let response = llm.generate(&prompt).await?;
    parse_engagement_decision(&response)
}
```

**Decision Factors:**
- Tweet quality (spam vs. valuable)
- Controversy level (avoid drama)
- Community sentiment (positive/negative consensus)
- Account trustworthiness
- Topic relevance to persona

---

### **4. Multi-Turn Conversations**
Track and continue conversations over multiple replies.

**Implementation:**
```rust
pub struct ConversationTracker {
    pub tweet_id: String,
    pub our_reply_id: Option<String>,
    pub context: Vec<ChatMessage>,
}

// Enable follow-up replies when someone responds to us
pub async fn check_for_replies_to_us(api: &TaskContext) -> Result<Vec<ConversationTracker>> {
    // Query notifications for replies to our recent replies
    // Generate contextual follow-ups
}
```

---

### **5. Persona-Enhanced Behavior**
Deeper persona integration with LLM prompts.

**Example Prompts by Persona:**

**Teen Persona:**
```
You're a Gen Z Twitter user. Reply casually with:
- Internet slang (but no emojis)
- Lowercase, abbreviated words
- Sarcastic or humorous tone
```

**Professional Persona:**
```
You're a industry professional. Reply with:
- Thoughtful analysis
- Professional tone (but not stiff)
- Specific insights from experience
```

**Researcher Persona:**
```
You're an academic/researcher. Reply with:
- Evidence-based reasoning
- Nuanced perspectives
- Citations when relevant (URLs)
```

---

## 📋 V2 Implementation Plan

### **Phase 1: Infrastructure (Week 1)**
- [ ] Create `src/utils/twitter/twitteractivity_llm.rs`
- [ ] Add LLM config to `TwitterActivityConfig`
- [ ] Add `llm_enabled` flag to engagement limits
- [ ] Integration tests for LLM client

### **Phase 2: Reply Generation (Week 1-2)**
- [ ] Implement `generate_reply()` with fallback
- [ ] Add reply quality validation (length, banned words)
- [ ] Integrate with existing `reply_to_tweet()` flow
- [ ] Add `reply_probability` config (default: 0.02 → 0.05 with LLM)

### **Phase 3: Quote Tweet (Week 2)**
- [ ] Implement `quote_tweet_with_commentary()`
- [ ] Add quote tweet button detection
- [ ] Type commentary with human-like delays
- [ ] Add `quote_tweet_probability` config (default: 0.15 of RTs)

### **Phase 4: Smart Decisions (Week 3)**
- [ ] Implement `decide_engagement()` 
- [ ] Add content quality scoring
- [ ] Controversy detection (skip political drama)
- [ ] Sentiment-aware engagement

### **Phase 5: Testing & Tuning (Week 3-4)**
- [ ] Integration tests with mock LLM
- [ ] Live testing with Ollama
- [ ] Tune probabilities based on success rate
- [ ] Add metrics for LLM engagement quality

---

## 🔧 Configuration Example

```toml
[twitter_activity]
feed_scan_duration_ms = 120000
feed_scroll_count = 12
engagement_candidate_count = 5

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 3          # Increased from 1 (LLM-powered)
max_quote_tweets = 2     # NEW: V2 feature
max_thread_dives = 3
max_total_actions = 15   # Increased from 10

[twitter_activity.llm]
enabled = true
provider = "ollama"      # or "openrouter"
model = "llama3.2:latest"
temperature = 0.7
max_tokens = 100
timeout_ms = 60000

# Engagement probabilities with LLM
reply_probability = 0.05         # 5% of eligible tweets
quote_tweet_probability = 0.15   # 15% of RTs become quote tweets
smart_decision_enabled = true    # Use LLM to decide engagement

# Safety settings
block_controversial = true       # Skip political/drama tweets
sentiment_guard_enabled = true   # Skip negative tweets
language_matching = true         # Reply in tweet's language
```

---

## 📊 Success Metrics

### **V1 Metrics (Current)**
- Actions per session: 8-10
- Success rate: 95%+ (mechanical actions)
- Time per session: 2-3 minutes

### **V2 Targets**
- Actions per session: 12-15 (more replies/quote tweets)
- Reply quality score: 4/5 (human-like, contextual)
- Engagement rate: 2-3x increase (more meaningful interactions)
- Time per session: 3-5 minutes (longer for LLM generation)

---

## ⚠️ Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLM rate limits | Medium | Fallback provider, caching |
| Slow response times | Low | Async generation, timeouts |
| Poor quality replies | High | Quality validation, human review mode |
| Account flags | Medium | Conservative limits, gradual rollout |
| Cost (OpenRouter) | Low | Ollama primary, OpenRouter fallback |

---

## 🚀 Getting Started

### **Prerequisites**
1. Ollama installed locally OR OpenRouter API key
2. LLM model: `llama3.2:latest` (or similar 7B+ model)
3. V1 Twitter activity working successfully

### **Quick Start**
```bash
# 1. Install Ollama
curl https://ollama.ai/install.sh | sh

# 2. Pull model
ollama pull llama3.2

# 3. Enable in config
# Edit config/default.toml:
[twitter_activity.llm]
enabled = true
provider = "ollama"
model = "llama3.2:latest"

# 4. Test reply generation
cargo run twitteractivity
```

---

## 📚 Related Files

- `src/llm/` - LLM client infrastructure
- `src/utils/twitter/twitteractivity_interact.rs` - Engagement actions
- `config/llm.toml` - LLM provider configuration
- `twitterActivity.md` - Original design document

---

## 🎯 V2 Timeline

| Week | Milestone | Deliverables |
|------|-----------|--------------|
| 1 | Infrastructure | LLM integration, config, tests |
| 2 | Reply + Quote | Working reply/quote generation |
| 3 | Smart Decisions | Engagement quality scoring |
| 4 | Polish | Testing, tuning, documentation |

**Estimated Effort:** 2-3 weeks (part-time)

---

## ✅ V2 Acceptance Criteria

- [ ] LLM-powered replies indistinguishable from human
- [ ] Quote tweets add meaningful commentary
- [ ] Smart decisions skip low-quality/controversial content
- [ ] Zero account flags or rate limit issues
- [ ] 95%+ LLM availability (with fallback)
- [ ] All V2 features behind config flags (opt-in)

---

**Status:** Ready for implementation
**Priority:** Medium (after V1 stabilization)
**Owner:** TBD
