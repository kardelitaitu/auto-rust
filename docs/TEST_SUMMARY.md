# Test Summary

> Coverage reports for codebase modules. Run `cargo tarpaulin --out Html --output-dir ./coverage` to update.

---

## Core Files

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/main.rs | 0 | 84 | 0.0% | Entry point - needs integration tests |
| src/browser.rs | 32 | 260 | 12.3% | Browser discovery, session management |
| src/orchestrator.rs | 45 | 331 | 13.6% | Task orchestration, group execution |
| src/config.rs | 332 | 569 | 58.3% | Config loading and validation |
| src/metrics.rs | 159 | 211 | 75.4% | Metrics collection and reporting |
| src/tracing.rs | 9 | 28 | 32.1% | Tracing initialization |
| src/runtime/mod.rs | - | - | - | Module exports only |
| src/runtime/task_context.rs | 30 | 1391 | 2.2% | Main task context - many async paths |
| src/runtime/execution.rs | 12 | 16 | 75.0% | Execution coordination |
| src/session/mod.rs | 0 | 222 | 0.0% | Session management - needs tests |
| src/health_monitor.rs | 106 | 137 | 77.4% | Health monitoring |
| src/health_logger.rs | 18 | 49 | 36.7% | Health logging |
| src/error.rs | 6 | 8 | 75.0% | Error handling |

---

## Utility Files

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/utils/navigation.rs | 5 | 201 | 2.5% | Navigation helpers - needs browser tests |
| src/utils/accessibility_locator.rs | - | - | - | See TODO accessibility program |
| src/utils/timing.rs | 48 | 50 | 96.0% | Timing utilities |
| src/utils/mouse.rs | 66 | 990 | 6.7% | Mouse movement, trajectories |
| src/utils/scroll.rs | 0 | 118 | 0.0% | Scroll helpers - needs tests |
| src/utils/zoom.rs | 0 | 96 | 0.0% | Zoom helpers - needs tests |
| src/utils/keyboard.rs | 29 | 145 | 20.0% | Keyboard helpers |
| src/utils/clipboard.rs | 22 | 98 | 22.4% | Clipboard operations |
| src/utils/native_input.rs | 23 | 100 | 23.0% | Native input handling |
| src/utils/text.rs | 12 | 12 | 100.0% | Text utilities |
| src/utils/math.rs | 15 | 15 | 100.0% | Math utilities |
| src/utils/geometry.rs | 4 | 4 | 100.0% | Geometry utilities |
| src/utils/page_size.rs | 21 | 58 | 36.2% | Page size utilities |
| src/utils/blockmedia.rs | 54 | 74 | 73.0% | Block media detection |
| src/utils/profile.rs | 624 | 626 | 99.7% | Profile management |
| src/internal/circuit_breaker.rs | 0 | 1 | 0.0% | Circuit breaker - minimal code |

---

## Task Files

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/task/mod.rs | 7 | 46 | 15.2% | Task registry |
| src/task/pageview.rs | 80 | 188 | 42.6% | Pageview task |
| src/task/twitterfollow.rs | 78 | 345 | 22.6% | Follow task |
| src/task/twitteractivity.rs | 94 | 631 | 14.9% | Activity task |
| src/task/twitterintent.rs | 53 | 121 | 43.8% | Intent task |
| src/task/twitterlike.rs | 14 | 105 | 13.3% | Like task |
| src/task/twitterretweet.rs | 14 | 164 | 8.5% | Retweet task |
| src/task/twitterquote.rs | 18 | 148 | 12.2% | Quote task |
| src/task/twitterreply.rs | 9 | 159 | 5.7% | Reply task |
| src/task/twitterdive.rs | 14 | 94 | 14.9% | Dive task |
| src/task/twittertest.rs | 21 | 205 | 10.2% | Test task |
| src/task/cookiebot.rs | 11 | 63 | 17.5% | Cookie bot task |
| src/task/demo_mouse.rs | 13 | 95 | 13.7% | Demo mouse task |
| src/task/demo_keyboard.rs | 17 | 142 | 12.0% | Demo keyboard task |
| src/task/demoqa.rs | 21 | 126 | 16.7% | DemoQA task |
| src/task/task_example.rs | 21 | 100 | 21.0% | Example task |
| src/task/policy.rs | 85 | 85 | 100.0% | Policy handling |
| src/task/security.rs | 34 | 56 | 60.7% | Security utilities |
| src/task/cdp_utils.rs | 11 | 12 | 91.7% | CDP utilities |

---

## Twitter Activity Utils

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/utils/twitter/twitteractivity_selectors.rs | 201 | 201 | 100.0% | Tweet selectors |
| src/utils/twitter/twitteractivity_limits.rs | 130 | 130 | 100.0% | Rate limits |
| src/utils/twitter/twitteractivity_persona.rs | 104 | 110 | 94.5% | Persona weights |
| src/utils/twitter/twitteractivity_decision.rs | 87 | 91 | 95.6% | Engagement decisions |
| src/utils/twitter/twitteractivity_sentiment_domains.rs | 180 | 213 | 84.5% | Domain sentiment |
| src/utils/twitter/twitteractivity_sentiment_emoji.rs | 65 | 67 | 97.0% | Emoji sentiment |
| src/utils/twitter/twitteractivity_sentiment_context.rs | 48 | 48 | 100.0% | Context modifiers |
| src/utils/twitter/twitteractivity_sentiment.rs | 67 | 74 | 90.5% | Base sentiment |
| src/utils/twitter/twitteractivity_sentiment_enhanced.rs | 102 | 317 | 32.2% | Enhanced sentiment |
| src/utils/twitter/twitteractivity_llm.rs | 40 | 106 | 37.7% | LLM integration |
| src/utils/twitter/twitteractivity_feed.rs | 0 | 24 | 0.0% | Feed handling - needs tests |
| src/utils/twitter/twitteractivity_navigation.rs | 0 | 73 | 0.0% | Navigation utils - needs tests |
| src/utils/twitter/twitteractivity_popup.rs | 0 | 11 | 0.0% | Popup handling - needs tests |
| src/utils/twitter/twitteractivity_humanized.rs | 7 | 66 | 10.6% | Humanized timing |
| src/utils/twitter/twitteractivity_dive.rs | 9 | 29 | 31.0% | Thread diving |
| src/utils/twitter/twitteractivity_interact.rs | 32 | 163 | 19.6% | Engagement actions |
| src/utils/twitter/twitteractivity_sentiment_llm.rs | 22 | 56 | 39.3% | LLM sentiment |

---

## LLM Modules

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/llm/reply_engine.rs | 111 | 111 | 100.0% | Reply generation |
| src/llm/reply_strategies.rs | 55 | 56 | 98.2% | Reply strategies |
| src/llm/client.rs | 93 | 171 | 54.4% | LLM client |
| src/llm/models.rs | 19 | 19 | 100.0% | Model definitions |
| src/llm/mod.rs | 2 | 19 | 10.5% | Module re-exports |
| src/llm/unified_processor.rs | 90 | 133 | 67.7% | Unified processing |

---

## Runtime Submodules

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/runtime/task_context/click_learning.rs | 150 | 167 | 89.8% | Click adaptation |
| src/runtime/task_context/types.rs | 15 | 16 | 93.8% | Shared types |
| src/runtime/task_context/query.rs | 1 | 25 | 4.0% | DOM query - needs browser tests |
| src/runtime/task_context/interaction.rs | 0 | 2 | 0.0% | Interaction - minimal code |

---

## Validation Modules

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/validation/task.rs | 127 | 127 | 100.0% | Task validation |
| src/validation/task_registry.rs | 46 | 46 | 100.0% | Task registry |

---

## Mouse Submodules

*Last checked: 2026-05-01*

| Filename | Lines Covered | Total Lines | Coverage % | Notes |
|----------|-------------|------------|------------|-------|
| src/utils/mouse/trajectory.rs | 103 | 104 | 99.0% | Path generation |
| src/utils/mouse/native.rs | 38 | 151 | 25.2% | Native click |
| src/utils/mouse/types.rs | 11 | 11 | 100.0% | Type definitions |

---

## Overall Statistics

- **Total Lines:** 12,181
- **Covered Lines:** 4,653
- **Coverage:** 38.2%

---

## How to Update Coverage

1. Run: `cargo tarpaulin --out Html --output-dir ./coverage`
2. Open `./coverage/tarpaulin-report.html` in browser
3. Copy "Tested/Total" values for each file
4. Update the tables in this document
5. Update "Last checked" date for modified sections