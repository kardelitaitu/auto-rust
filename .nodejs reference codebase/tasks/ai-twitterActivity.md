# ai-twitterActivity.js - Flow Chart

## Main Entry Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    aiTwitterActivityTask()                       │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. LOAD SETTINGS                                              │
│     from config/settings.json                                    │
│     ├─ reply.probability (default: 0.10)                        │
│     └─ quote.probability (default: 0.03)                         │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. GET PROFILE                                                 │
│     from profileManager                                          │
│     ├─ id: "05-Balanced"                                        │
│     ├─ probabilities: { dive, like, follow, etc }               │
│     └─ inputMethod: { mouse, keyboard, wheel }                   │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. INITIALIZE AGENT                                            │
│     new AITwitterAgent(page, profile, logger, options)          │
│     ├─ replyEngine: AIReplyEngine                                │
│     ├─ quoteEngine: AIQuoteEngine                                 │
│     ├─ contextEngine: AIContextEngine                            │
│     └─ engagementTracker: limits per session                     │
│         ├─ maxReplies: 3                                         │
│         ├─ maxQuotes: 1                                         │
│         ├─ maxLikes: 5                                          │
│         ├─ maxFollows: 2                                        │
│         └─ maxRetweets: 1                                       │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. APPLY HUMANIZATION PATCH                                    │
│     └─ Inject consistency & entropy scripts                       │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. WARMUP DELAY (2-15s random)                                 │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  6. NAVIGATE TO https://x.com                                   │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  7. CHECK LOGIN STATE                                           │
│     └─ Retry up to 3 times (3s delay each)                        │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  8. RUN SESSION (10 cycles, 300-540s)                            │
│     └─ agent.runSession(cycles, minDuration, maxDuration)        │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────┴─────────────┐
                    │   PER-CYCLE EXECUTION    │
                    └─────────────┬─────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  A. DETERMINE INPUT METHOD (from profile)                        │
│     ├─ mouse (72%)      → ghostCursor.move()                     │
│     ├─ keyboard (18%)   → page.keyboard.press()                  │
│     └─ wheel (10%)     → page.mouse.wheel()                      │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  B. PHASE CALCULATION (based on cycle progress)                   │
│     ├─ WARMUP   (0-10%)  → reduced engagement                    │
│     ├─ ACTIVE   (10-80%) → normal engagement                     │
│     └─ COOLDOWN (80-100%)→ winding down                          │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  C. DIVE DECISION (profile.diveProbability)                       │
│     └─ If dive: → handleDiveSequence(tweet)                      │
└─────────────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │   DIVE SEQUENCE FLOW     │
                    └─────────────┬─────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  D. LOAD TWEET CONTEXT                                          │
│     ├─ Extract tweet text, author, URL                           │
│     ├─ Load replies (scroll if needed)                           │
│     └─ AIContextEngine.extractEnhancedContext()                   │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  E. AI ENGAGEMENT DECISION                                       │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  roll = Math.random()                                       ││
│  │                                                             ││
│  │  ├─ IF roll < replyProb (10%) → REPLY                       ││
│  │  │                                                             ││
│  │  ├─ ELSE IF roll < (replyProb + quoteProb) (13%) → QUOTE     ││
│  │  │                                                             ││
│  │  └─ ELSE → SKIP                                              ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│   REPLY FLOW     │  │   QUOTE FLOW     │  │    SKIP FLOW    │
└────────┬─────────┘  └────────┬─────────┘  └──────────────────┘
         │                    │
         ▼                    ▼
┌──────────────────┐  ┌──────────────────┐
│ handleAIReply()  │  │ handleAIQuote()  │
└────────┬─────────┘  └────────┬─────────┘
         │                    │
         ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  F. SENTIMENT GUARD                                              │
│     ├─ Analyze tweet sentiment                                    │
│     ├─ IF negative → BLOCK (like, retweet, reply, quote)         │
│     └─ IF positive/neutral → CONTINUE                            │
└─────────────────────────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  G. ENGAGEMENT LIMITS CHECK                                       │
│     ├─ IF replies >= maxReplies → SKIP                           │
│     ├─ IF quotes >= maxQuotes → SKIP                             │
│     ├─ IF likes >= maxLikes → SKIP                               │
│     └─ IF follows >= maxFollows → SKIP                            │
└─────────────────────────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  H. LLM REQUEST ROUTING (agent-connector.js)                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  Priority Chain:                                           ││
│  │                                                             ││
│  │  1. vLLM (if enabled in settings)                          ││
│  │     └─ Success? → Return response                           ││
│  │                                                             ││
│  │  2. Local Ollama (if enabled)                              ││
│  │     └─ Success? → Return response                           ││
│  │                                                             ││
│  │  3. OpenRouter Cloud (ALWAYS as final fallback)             ││
│  │     └─ Return response                                      ││
│  │                                                             ││
│  │  IF ALL FAIL → Log error, skip engagement                   ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  I. POST-PROCESSING (both reply & quote)                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                                                             ││
│  │  1. Humanize Output:                                       ││
│  │     ├─ 30% chance: convert to lowercase                    ││
│  │     └─ 80% chance: remove trailing period                  ││
│  │                                                             ││
│  │  2. Safety Validation:                                     ││
│  │     ├─ Check length (10-280 chars)                         ││
│  │     ├─ Check for excluded keywords                          ││
│  │     ├─ Check for generic responses ("interesting", etc)     ││
│  │     └─ IF validation fails → retry or skip                  ││
│  │                                                             ││
│  │  3. Post to Twitter:                                       ││
│  │     ├─ Reply: Click reply → Type → Post                      ││
│  │     └─ Quote: Click RT → Quote → Type → Post                 ││
│  │                                                             ││
│  │  4. Update Engagement Tracker                               ││
│  │     └─ Increment respective counter                          ││
│  │                                                             ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
         │                    │
         └─────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  J. READ EXPANDED TWEET                                         │
│     ├─ Scroll through replies                                    │
│     ├─ Micro-interactions (3-8% chance):                       │
│     │   ├─ Text highlighting                                    │
│     │   ├─ Right-click                                          │
│     │   ├─ Logo click                                           │
│     │   ├─ Whitespace click                                     │
│     │   └─ Random fidget                                        │
│     └─ Idle ghosting (if idle > 2s)                              │
└─────────────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │  END OF CYCLE - LOOP     │
                    │   (up to 10 cycles)      │
                    └─────────────┬─────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  9. SESSION CLEANUP                                              │
│     ├─ Record metrics: follows, likes, retweets, tweets          │
│     ├─ AI Stats: attempts, successes, skips, failures            │
│     ├─ Log engagement status                                      │
│     └─ Close page                                                │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  10. TASK COMPLETE                                               │
└─────────────────────────────────────────────────────────────────┘
```

---

## Probability Configuration (config/settings.json)

```json
{
    "twitter": {
        "reply": {
            "probability": 0.1 // 10% chance to reply
        },
        "quote": {
            "probability": 0.03 // 3% chance to quote (if reply skipped)
        }
    },
    "engagement": {
        "maxReplies": 3, // Max replies per session
        "maxQuotes": 1, // Max quotes per session
        "maxLikes": 5, // Max likes per session
        "maxFollows": 2, // Max follows per session
        "maxRetweets": 1 // Max retweets per session
    }
}
```

---

## AI Decision Branching (Per Tweet)

```
                    TWEET DETECTED
                          │
                          ▼
              ┌───────────────────────┐
              │  Roll Random (0-100)  │
              └───────────┬───────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
     [0-10%]         [10-13%]        [13-100%]
          │               │               │
          ▼               ▼               ▼
       REPLY            QUOTE            SKIP
          │               │               │
          ▼               ▼               ▼
    handleAIReply    handleAIQuote       done
          │               │
          ▼               ▼
    Sentiment Guard   Sentiment Guard
          │               │
          ▼               ▼
    Limits Check     Limits Check
          │               │
          ▼               ▼
       LLM Request    LLM Request
   (vLLM→Ollama→Cloud)
          │               │
          ▼               ▼
   Humanize + Post   Humanize + Post
          │               │
          └───────────────┘
                    │
                    ▼
           Update Tracker
                    │
                    ▼
                 done
```

---

## LLM Routing Chain (agent-connector.js)

```
┌─────────────────────────────────────────────────────────────┐
│                   generate_reply Request                    │
└──────────────────────────┬────────────────────────────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │  vLLM Enabled?         │
              │  (settings.llm.vllm)   │
              └───────────┬────────────┘
                          │
           ┌──────────────┴──────────────┐
           │ YES                            │ NO
           ▼                                ▼
   ┌────────────────┐         ┌────────────────────────┐
   │ Send to vLLM   │         │ Ollama Enabled?         │
   │ (OpenAI compat)│         │ (settings.llm.local)   │
   └───────┬────────┘         └───────────┬────────────┘
           │                              │
           │              ┌──────────────┴──────────────┐
           │              │ YES                            │ NO
           │              ▼                                ▼
           │    ┌────────────────┐         ┌────────────────────────┐
           │    │ Send to Ollama │         │ Send to OpenRouter     │
           │    │ (localhost:11434)        │ (cloud fallback)       │
           │    └───────┬────────┘         └───────────┬────────────┘
           │            │                              │
           │            │              SUCCESS          │
           ▼            ▼                              ▼
   ┌───────────────┐   │              ┌────────────────────────┐
   │ SUCCESS       │   │              │ Return Response        │
   │ Return Result │   │              │ (metadata: routedTo:   │
   └───────────────┘   │              │  "cloud", "vllm", etc)│
           │            │              └────────────────────────┘
           │            │                              │
           │            ▼                              │
           │    ┌───────────────┐                       │
           │    │ SUCCESS       │                       │
           │    │ Return Result │                       │
           │    └───────────────┘                       │
           │            │                               │
           │            ▼                               │
           │    ┌───────────────┐                       │
           │    │ FAILED        │                       │
           │    │ Try Ollama    │───────────────────────┘
           │    └───────────────┘
           │
           ▼
   ┌───────────────┐
   │ FAILED        │
   │ Try Ollama    │
   └───────┬───────┘
           │
           │ SUCCESS
           ▼
   ┌───────────────┐
   │ Return Result │
   │ (routedTo:    │
   │  "ollama")    │
   └───────────────┘
```

---

## Engagement Limits (per session)

```
┌────────────────────────────────────────┐
│         ENGAGEMENT TRACKER             │
├────────────────────────────────────────┤
│  Counter          │  Used  │  Max     │
├───────────────────┼────────┼──────────┤
│  replies          │    2   │     3    │
│  quotes           │    0   │     1    │
│  likes            │    5   │     5    │
│  follows          │    1   │     2    │
│  retweets         │    0   │     1    │
│  bookmarks        │    0   │     2    │
└───────────────────┴────────┴──────────┘

When any counter reaches max:
  → Skip that engagement type for rest of session
```

---

## Session Phases

```
SESSION PROGRESS (0% ──────────────────────────────────────────── 100%)
      │
      ├────────────────┬─────────────────────────┬────────────────┤
      │                │                         │                │
   WARMUP           ACTIVE                   COOLDOWN
   (0-10%)          (10-80%)                 (80-100%)
      │                │                         │                │
      ├─ Reduced      ├─ Normal engagement      ├─ Reduced       │
      │  engagement   │   rates                 │  engagement    │
      ├─ More idle    ├─ Full AI capability     ├─ Wrap up       │
      │  behavior     │                         │  activities     │
      └───────────────┴─────────────────────────┴────────────────┘
```

---

## Key Files

| File                          | Purpose                    |
| ----------------------------- | -------------------------- |
| `tasks/ai-twitterActivity.js` | Main task entry point      |
| `utils/ai-twitterAgent.js`    | Agent with AI capabilities |
| `utils/ai-reply-engine.js`    | AI reply generation        |
| `utils/ai-quote-engine.js`    | AI quote generation        |
| `utils/ai-context-engine.js`  | Extract tweet context      |
| `utils/sentiment-guard.js`    | Skip negative content      |
| `utils/engagement-limits.js`  | Track per-session limits   |
| `core/agent-connector.js`     | Route LLM requests         |
| `core/vllm-client.js`         | vLLM provider              |
| `core/ollama-client.js`       | Ollama provider            |
| `core/cloud-client.js`        | OpenRouter provider        |
