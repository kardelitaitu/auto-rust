# Twitter Agent — State Machine & Cycle Flow

This document describes the `TwitterAgent` orchestrator: its state, per-cycle execution flow, and helper methods.

---

## 6. Per-Cycle Execution Flow

### 6.1 High-Level State Machine

```rust
// TwitterAgent orchestrates N cycles (5–10) within a total duration budget (540–840s)
pub struct TwitterAgent {
    page: Page,
    profile: BrowserProfile,
    config: TwitterConfig,
    state: AgentState,
    persona: TwitterPersona, // derived from profile at startup
}

struct AgentState {
    cycle: u32,
    start_time: Instant,
    counters: EngagementCounters,
    current_phase: Phase,
}

struct EngagementCounters {
    likes: u32,
    retweets: u32,
    follows: u32,
    replies: u32,
    quotes: u32,
    bookmarks: u32,
    skipped_sentiment: u32,
    skipped_limits: u32,
    skipped_dive_decision: u32,
}
```

### 6.2 Full Cycle Pseudocode

```rust
/// Possible engagement actions the agent can take on a tweet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engagement {
    Like,
    Reply,
    Retweet,
    Quote,
    Follow,
    Bookmark,
}

/// Outcome of a single cycle execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    SessionComplete,
    DurationBudgetExceeded,
    NavigationFailed,
    NavigationTimeout,
    SkippedDive,
    NoTweetsFound,
    FeedScanError,
    ClickFailed,
    ContextLoadFailed,
    BlockedBySentiment,
    AllLimitsReached,
    ActionFailed(Engagement),
    ActionDisabled,
    Engaged(Engagement),
}

async fn run_cycle(&mut self) -> Result<CycleOutcome> {
    // ═══════════════════════════════════════════════════════════════
    // PHASE 0: Session budget check
    // ═══════════════════════════════════════════════════════════════
    if self.state.cycle >= self.config.max_cycles {
        return Ok(CycleOutcome::SessionComplete);
    }
    let elapsed = self.state.start_time.elapsed().as_secs();
    if elapsed >= self.config.max_duration_sec {
        return Ok(CycleOutcome::DurationBudgetExceeded);
    }
    self.state.cycle += 1;

    info!(
        "[twitter][cycle {}/{}] Starting (elapsed: {}s)",
        self.state.cycle,
        self.config.max_cycles,
        elapsed
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 1: Select entry point (weighted random)
    // ═══════════════════════════════════════════════════════════════
    let url = self.select_entry_point().await?;
    info!("[twitter][cycle {}] Entry: {}", self.state.cycle, url);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 2: Navigate with timeout
    // ═══════════════════════════════════════════════════════════════
    let nav_result = timeout(
        Duration::from_secs(20),
        navigation::goto(&self.page, &url, 15000)
    ).await;

    match nav_result {
        Ok(Ok(())) => { /* proceed */ }
        Ok(Err(e)) => {
            warn!("[twitter][cycle {}] Navigation failed: {}", self.state.cycle, e);
            return Ok(CycleOutcome::NavigationFailed);
        }
        Err(_) => {
            warn!("[twitter][cycle {}] Navigation timeout", self.state.cycle);
            return Ok(CycleOutcome::NavigationTimeout);
        }
    }

    // Wait for network idle (or timeout)
    let _ = navigation::wait_for_load(&self.page, 5000).await;

    // ═══════════════════════════════════════════════════════════════
    // PHASE 3: Close popups/modals (defensive)
    // ═══════════════════════════════════════════════════════════════
    if let Err(e) = twitter_popup::close_all_modals(&self.page).await {
        warn!("[twitter][cycle {}] Modal close error: {}", self.state.cycle, e);
        // Non-fatal — continue
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 4: Determine persona phase (warmup/active/cooldown)
    // ═══════════════════════════════════════════════════════════════
    let progress = self.state.cycle as f64 / self.config.max_cycles as f64;
    self.state.current_phase = if progress < 0.10 {
        Phase::Warmup
    } else if progress < 0.80 {
        Phase::Active
    } else {
        Phase::Cooldown
    };

    // ═══════════════════════════════════════════════════════════════
    // PHASE 5: Dive decision — roll against profile's dive_probability × persona multiplier
    // ═══════════════════════════════════════════════════════════════
    let base_dive_p = self.profile.dive_probability.random();
    let adjusted_dive_p = base_dive_p * self.persona.dive_probability_multiplier();
    let should_dive = rand::random::<f64>() < adjusted_dive_p;

    if !should_dive {
        info!("[twitter][cycle {}] Skipped (dive roll: {:.3} < {:.3})",
            self.state.cycle, adjusted_dive_p, adjusted_dive_p);
        self.state.counters.skipped_dive_decision += 1;
        return Ok(CycleOutcome::SkippedDive);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 6: Locate a tweet in feed (surface level)
    // ═══════════════════════════════════════════════════════════════
    let tweet = match twitter_feed::find_random_tweet(&self.page).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            warn!("[twitter][cycle {}] No tweets found in feed", self.state.cycle);
            return Ok(CycleOutcome::NoTweetsFound);
        }
        Err(e) => {
            error!("[twitter][cycle {}] Feed scan error: {}", self.state.cycle, e);
            return Ok(CycleOutcome::FeedScanError);
        }
    };

    info!("[twitter][cycle {}] Found tweet by @{} (id: {})",
        self.state.cycle, tweet.author_handle, tweet.id);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 7: Click tweet to expand (dive into context)
    // ═══════════════════════════════════════════════════════════════
    if let Err(e) = twitter_dive::click_tweet(&self.page, &tweet).await {
        warn!("[twitter][cycle {}] Click failed: {}", self.state.cycle, e);
        return Ok(CycleOutcome::ClickFailed);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 8: Simulate reading — scroll within tweet context, maybe open replies
    // ═══════════════════════════════════════════════════════════════
    self.simulate_reading(&tweet).await?;

    // ═══════════════════════════════════════════════════════════════
    // PHASE 9: Load tweet context (post-click, post-reading)
    // ═══════════════════════════════════════════════════════════════
    let context = match twitter_dive::load_tweet_context(&self.page, &tweet).await {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!("[twitter][cycle {}] Context load failed: {}", self.state.cycle, e);
            return Ok(CycleOutcome::ContextLoadFailed);
        }
    };

    // ═══════════════════════════════════════════════════════════════
    // PHASE 10: Sentiment guard (keyword blocklist)
    // ═══════════════════════════════════════════════════════════════
    if self.config.sentiment.block_negative_engagement {
        if twitter_sentiment::contains_negative_sentiment(&context.full_text, &self.config.sentiment.negative_keywords) {
            info!("[twitter][cycle {}] Blocked by sentiment guard (negative content)", self.state.cycle);
            self.state.counters.skipped_sentiment += 1;
            return Ok(CycleOutcome::BlockedBySentiment);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 11: Limit guard — build list of actions still available
    // ═══════════════════════════════════════════════════════════════
    let available_actions = self.available_engagement_actions();
    if available_actions.is_empty() {
        info!("[twitter][cycle {}] All engagement limits reached", self.state.cycle);
        return Ok(CycleOutcome::AllLimitsReached);
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 12: Roll engagement action (weighted by config probabilities)
    // ═══════════════════════════════════════════════════════════════
    let action = self.roll_engagement(&available_actions).await;
    debug!("[twitter][cycle {}] Rolled action: {:?}", self.state.cycle, action);

    // ═══════════════════════════════════════════════════════════════
    // PHASE 13: Execute engagement action via UI automation
    // ═══════════════════════════════════════════════════════════════
    let action_start = Instant::now();
    match action {
        Engagement::Like => {
            if let Err(e) = twitter_interact::like_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Like failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Retweet => {
            if let Err(e) = twitter_interact::retweet_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Retweet failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Follow => {
            if let Err(e) = twitter_interact::follow_user(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Follow failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
        }
        Engagement::Reply => {
            if !self.config.engagement.reply_with_ai {
                info!("[twitter][cycle {}] Reply skipped (REPLY_WITH_AI=false)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            bail!("Reply action with AI not implemented in V1");
        }
        Engagement::Quote => {
            if !self.config.engagement.quote_with_ai {
                info!("[twitter][cycle {}] Quote skipped (QUOTE_WITH_AI=false)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            bail!("Quote action with AI not implemented in V1");
        }
        Engagement::Bookmark => {
            if self.config.limits.max_bookmarks_per_session == 0 {
                info!("[twitter][cycle {}] Bookmark skipped (limit=0)", self.state.cycle);
                return Ok(CycleOutcome::ActionDisabled);
            }
            if let Err(e) = twitter_interact::bookmark_tweet(&self.page, &tweet).await {
                warn!("[twitter][cycle {}] Bookmark failed: {}", self.state.cycle, e);
                return Ok(CycleOutcome::ActionFailed(action));
            }
         }
     }
    let action_duration = action_start.elapsed();

    // ═══════════════════════════════════════════════════════════════
    // PHASE 11: Update counters and log
    // ═══════════════════════════════════════════════════════════════
    self.increment_counter(&action);
    info!(
        "[twitter][cycle {}] ✓ {:?} @{} ({}ms)",
        self.state.cycle,
        action,
        tweet.author_handle,
        action_duration.as_millis()
    );

    // ═══════════════════════════════════════════════════════════════
    // PHASE 12: Human pause before next cycle
    // ═══════════════════════════════════════════════════════════════
    timing::human_pause(3000, 50).await;

    Ok(CycleOutcome::Engaged(action))
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional TwitterAgent helper methods
// ─────────────────────────────────────────────────────────────────────────────

    /// Increment the counter for the given engagement action.
    fn increment_counter(&mut self, action: &Engagement) {
        self.state.counters.increment(*action);
    }

    /// Build the list of engagement actions still allowed by limits and probability > 0.
    /// Each action is checked against its per-session counter and engagement config.
    fn available_engagement_actions(&self) -> Vec<Engagement> {
        let mut actions = Vec::new();
        if self.state.counters.likes < self.config.limits.max_likes_per_session
            && self.config.engagement.like_probability > 0.0
        {
            actions.push(Engagement::Like);
        }
        if self.state.counters.retweets < self.config.limits.max_retweets_per_session
            && self.config.engagement.retweet_probability > 0.0
        {
            actions.push(Engagement::Retweet);
        }
        if self.state.counters.follows < self.config.limits.max_follows_per_session
            && self.config.engagement.follow_probability > 0.0
        {
            actions.push(Engagement::Follow);
        }
        if self.config.limits.max_replies_per_session > 0
            && self.state.counters.replies < self.config.limits.max_replies_per_session
            && self.config.engagement.reply_with_ai
            && self.config.engagement.reply_probability > 0.0
        {
            actions.push(Engagement::Reply);
        }
        if self.config.limits.max_quotes_per_session > 0
            && self.state.counters.quotes < self.config.limits.max_quotes_per_session
            && self.config.engagement.quote_with_ai
            && self.config.engagement.quote_probability > 0.0
        {
            actions.push(Engagement::Quote);
        }
        if self.state.counters.bookmarks < self.config.limits.max_bookmarks_per_session
            && self.config.engagement.bookmark_probability > 0.0
        {
            actions.push(Engagement::Bookmark);
        }
        actions
    }

    /// Roll an engagement action from the available actions list using
    /// weighted probabilities from config (normalized to sum=1 over available set).
    async fn roll_engagement(&self, available: &[Engagement]) -> Engagement {
        use rand::distributions::{Distribution, WeightedIndex};
        use std::collections::HashMap;

        // Build weight map: action → configured probability
        let mut action_weights: HashMap<Engagement, f64> = HashMap::new();
        action_weights.insert(Engagement::Like, self.config.engagement.like_probability);
        action_weights.insert(Engagement::Retweet, self.config.engagement.retweet_probability);
        action_weights.insert(Engagement::Follow, self.config.engagement.follow_probability);
        action_weights.insert(Engagement::Reply, self.config.engagement.reply_probability);
        action_weights.insert(Engagement::Quote, self.config.engagement.quote_probability);
        action_weights.insert(Engagement::Bookmark, self.config.engagement.bookmark_probability);

        // Extract weights for available actions only
        let mut weights_vec = Vec::new();
        for action in available {
            let w = action_weights.get(action).cloned().unwrap_or(0.0);
            weights_vec.push(w.max(0.0)); // no negative weights
        }

        let sum: f64 = weights_vec.iter().sum();
        if sum <= 0.0 || weights_vec.is_empty() {
            // Fallback: uniform random among available
            let idx = rand::random::<usize>() % available.len();
            return available[idx];
        }

        // Normalize to positive integers for WeightedIndex
        let int_weights: Vec<u32> = weights_vec
            .iter()
            .map(|&w| (w / sum * 1000.0) as u32)
            .map(|w| if w == 0 { 1 } else { w }) // avoid zero weight collapse
            .collect();

        match WeightedIndex::new(&int_weights) {
            Ok(dist) => {
                let idx = dist.sample(&mut rand::thread_rng());
                available[idx]
            }
            Err(_) => {
                // Fallback uniform
                let idx = rand::random::<usize>() % available.len();
                available[idx]
            }
        }
    }

    /// Simulate human reading behavior after tweet expansion.
    /// Includes short pauses, minor scroll within tweet, maybe open replies.
    async fn simulate_reading(&self, tweet: &TweetMetadata) -> Result<()> {
        // Pause to "read" the tweet text (3–6 seconds)
        let read_time = rand::thread_rng().gen_range(3000..6000);
        timing::human_pause(read_time, 30).await;

        // Maybe scroll a bit within tweet if it's long (20% chance)
        if rand::random::<f64>() < 0.2 {
            let small_scroll = rand::thread_rng().gen_range(50..200);
            scroll::scroll_down(&self.page, small_scroll).await?;
            timing::human_pause(500, 30).await;
        }

        // Small chance (10%) to glance at replies — scroll down 1–2 screenfuls
        if rand::random::<f64>() < 0.1 {
            let vp = page_size::get_viewport(&self.page).await?;
            let scroll_amount = (vp.height * (1.0 + rand::random::<f64>())) as i32;
            scroll::scroll_down(&self.page, scroll_amount).await?;
            timing::human_pause(2000, 40).await;
            // Scroll back to tweet
            scroll::scroll_up(&self.page, scroll_amount).await?;
            timing::human_pause(500, 30).await;
        }

        Ok(())
    }
```

### 6.3 Cycle Termination Conditions

The task ends when **any** of these become true:

1. `cycle >= config.max_cycles` (upper bound)
2. `elapsed_sec >= config.max_duration_sec` (time budget exhausted)
3. Critical error (browser crash, navigation failure → abort task with error)
4. ALL engagement counters hit their limits (early exit)

Return `TaskResult::success()` with metrics including final counter state.

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
