# Twitter Activity — Metrics & Observability

This document covers logging, metrics collection, and `run-summary.json` integration for the Twitter activity task.

---

## 8. Monitoring & Metrics

### 8.1 Task-Level Metrics (TwitterActivity-specific)

`TwitterAgent` maintains in-memory counters exported at task completion:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct TwitterMetrics {
    pub session_id: String,
    pub profile_name: String,
    pub twitter_persona: String,
    pub total_cycles: u32,
    pub total_duration_ms: u64,
    pub engagement: EngagementCounters,
    pub skipped_by_phase: SkippedCounters,
    pub entry_point_breakdown: HashMap<String, u32>,
    pub errors: Vec<TwitterError>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngagementCounters {
    pub likes: u32,
    pub retweets: u32,
    pub follows: u32,
    pub replies: u32,
    pub quotes: u32,
    pub bookmarks: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkippedCounters {
    pub dive_decision: u32,
    pub sentiment_block: u32,
    pub limit_exceeded: u32,
    pub navigation_failed: u32,
    pub context_load_failed: u32,
    pub no_tweets_found: u32,
    pub action_failed: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TwitterError {
    pub cycle: u32,
    pub phase: &'static str,
    pub message: String,
}
```

### 8.2 Structured Log Format

All logs use structured logging via `log::info!()`, `warn!()`, `error!()`, `debug!()`.

**Log format**:
```
HH:MM:SS [session=roxy-0001][profile=Casual][task=twitterActivity][cycle=3/8] INFO Engagement: like @user123 (duration=234ms)
```

**Log event types**:
- `INFO`: cycle start/complete, engagement successful, entry point selected
- `WARN`: navigation failure, modal close fallback, selector miss, action verification failed
- `ERROR`: critical failure (browser crash, config load failure)
- `DEBUG`: persona parameters, dive roll values, sentiment check result

**Per-cycle log sequence**:
```
[twi] cycle X/Y starting: entry=<url>
[twi] Found tweet by @handle (id: ...)
[twi] ✓ Like @handle (123ms)
[twi] Skipped (dive roll: 0.XX < 0.YY)
[twi] Blocked by sentiment guard (keyword: "hate")
[twi] All engagement limits reached — ending session
```

### 8.3 Run Summary Integration

The existing `MetricsCollector` (`src/metrics.rs`) exports `run-summary.json`. For V1, Twitter-specific metrics are logged but not embedded in the JSON. The JSON includes top-level task success counts.

**Future V2**: Extend `TaskResult` with `metadata: Option<serde_json::Value>` to embed per-task breakdown:

```json
{
  "summary": { "total_tasks": 10, "succeeded": 9, ... },
  "tasks": [
    {
      "name": "twitterActivity",
      "session_id": "roxy-0001",
      "duration_ms": 723450,
      "status": "success",
      "twitter": {
        "profile": "Casual",
        "persona": "casual",
        "cycles": 8,
        "engagement": { "likes": 4, "retweets": 2, "follows": 1, "bookmarks": 0 },
        "skipped": { "dive": 2, "sentiment": 0, "limits": 0 },
        "entry_points": { "home": 5, "explore": 2, "notifications": 1 }
      }
    }
  ]
}
```

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
