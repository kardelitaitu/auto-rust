# Implementation Notes

## Session Date: 2026-05-07

### Approach Change (After Review)

**Original plan** (REJECTED): Create `engagement/` subdirectory with multiple files.

**New plan** (APPROVED): Refactor within `twitteractivity_engagement.rs` by extracting helper functions.

### Rationale

1. **Twitter module already well-modularized**: 27 files in `src/utils/twitter/`
2. **Adding subdirectories adds complexity**: No benefit to more files
3. **process_candidate is orchestration**: Not a "God Object" - it's a flow controller
4. **Helper functions sufficient**: Extract 3-4 helpers to reduce from 762 to ~200-300 lines

### Current Code Structure (VERIFIED)

**`twitteractivity_engagement.rs`** - 1,325 lines total:

| Lines | Component | Description |
|-------|-----------|-------------|
| 1-90 | Imports + helpers | `handle_engagement_decision`, `select_candidate_action`, etc. |
| 95-857 | `process_candidate` | Main function (762 lines) |
| 858-996 | Helper functions | `like_at_position`, `extract_tweet_text`, `generate_reply_text`, etc. |
| 997-1325 | Tests | 4 test modules (322 lines) |

**`process_candidate` structure (762 lines):**

| Lines | Section | Description |
|-------|---------|-------------|
| 95-127 | Setup | Destructure context, check per-scan budget |
| 128-178 | Sentiment | Analyze tweet sentiment, modulate persona |
| 180-198 | Smart decision | Check if engagement allowed |
| 200-258 | Action selection | Determine which actions to perform |
| 260-399 | Thread dive | Dive into tweet detail if needed |
| 362-710 | Action execution | Execute like/retweet/quote/reply/follow/bookmark |
| 737-857 | Depth-first engagement | Engage with replies after root success |

### Extraction Plan

#### 1. Extract `engage_replies` (Lines 737-857, ~120 lines)
**Purpose**: Handle depth-first engagement with tweet replies.

**New function signature:**
```rust
async fn engage_replies(
    api: &TaskContext,
    persona: &PersonaWeights,
    task_config: &TaskConfig,
    counters: &mut EngagementCounters,
    actions_this_scan: &mut u32,
) -> Result<()>
```

**Benefits**:
- Removes 120 lines from `process_candidate`
- Testable independently
- Clear single responsibility

#### 2. Extract `execute_engagement_action` (Lines 362-710, ~350 lines)
**Purpose**: Execute a single engagement action with retry logic.

**New function signature:**
```rust
async fn execute_engagement_action(
    api: &TaskContext,
    action: &str,
    tweet_id: &str,
    did_dive: bool,
    counters: &mut EngagementCounters,
    task_config: &TaskConfig,
    sentiment: Sentiment,
) -> Result<bool>
```

**Benefits**:
- Removes ~350 lines from `process_candidate`
- Eliminates repetitive action handling code
- Makes each action type testable

#### 3. Extract `modulate_persona_by_sentiment` (Lines 128-178, ~50 lines)
**Purpose**: Analyze sentiment and modulate persona weights.

**New function signature:**
```rust
fn modulate_persona_by_sentiment(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> (Sentiment, PersonaWeights)
```

**Benefits**:
- Removes ~50 lines from `process_candidate`
- Separates sentiment logic
- Easier to test sentiment modulation

#### 4. Simplify Action Selection (Lines 200-258)
**Purpose**: Clean up the action selection if/else chain.

**Approach**: Possibly use a helper or restructure, but keep simple.

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-------------|
| Breaking process_candidate flow | High | Run `cargo test` after EACH extraction |
| Incorrect state passing | High | Carefully design function signatures |
| Losing retry logic | Medium | Verify retry works for each action type |
| Breaking thread dive | Medium | Test depth-first engagement separately |

### Progress Tracking

- [ ] Read full `process_candidate` function
- [ ] Extract `modulate_persona_by_sentiment` (simplest, ~50 lines)
- [ ] Extract `engage_replies` (~120 lines)
- [ ] Extract `execute_engagement_action` (~350 lines)
- [ ] Simplify action selection
- [ ] Run `cargo test` - all tests pass
- [ ] Run `.\check.ps1` - full CI passes
- [ ] Update line counts in spec

### Key Reminders

1. **Keep all code in same file** - no new files
2. **Run `cargo check` after each change**
3. **Don't change behavior** - only restructure
4. **Add rustdoc** to new helper functions
5. **Verify tests pass** - especially engagement tests


