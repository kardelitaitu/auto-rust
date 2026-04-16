# Browser Profile Documentation

This document describes the behavioral profiles for human-like browser automation.

## ProfileParam

Each parameter is defined with a base value and deviation percentage:

```rust
pub struct ProfileParam {
    pub base: f64,        // base value
    pub deviation_pct: f64, // e.g., 10.0 = ±10% variation
}
```

When used, `param.random()` returns a value within the deviation range.

## BrowserProfile Parameters

### Cursor Movement
| Parameter | Type | Description |
|-----------|------|-------------|
| `cursor_speed` | f64 | Movement speed multiplier (1.0 = normal, 0.5 = slow, 2.0 = fast) |
| `cursor_step_delay` | u64 | Delay between movement steps (ms) |
| `cursor_curve_spread` | f64 | Bezier curve randomness (higher = more curved) |
| `cursor_precision` | % | How close cursor gets to target (100% = exact center) |
| `cursor_micro_pause_chance` | % | Probability of random micro-pause during movement |
| `cursor_micro_pause_duration` | u64 | Duration of micro-pause (ms) |

### Typing
| Parameter | Type | Description |
|-----------|------|-------------|
| `typing_speed_mean` | u64 | Average keystroke delay (ms) |
| `typing_speed_stddev` | u64 | Keystroke delay standard deviation |
| `typo_rate` | % | Typo probability per character |
| `typing_word_pause` | u64 | Pause between words (ms) |
| `typo_notice_delay` | u64 | Time after making typo before noticing (ms) |
| `typo_retry_delay` | u64 | Time after backspace before retyping (ms) |
| `typo_recovery_chance` | % | Probability of correcting typo vs leaving it |

### Clicking
| Parameter | Type | Description |
|-----------|------|-------------|
| `click_reaction_delay` | u64 | Delay after arriving at target before clicking (ms) |
| `click_offset` | f64 | Click offset from element center (pixels) |

### Scrolling
| Parameter | Type | Description |
|-----------|------|-------------|
| `scroll_amount` | f64 | Scroll amount in pixels per action |
| `scroll_smoothness` | % | Scroll behavior (0 = instant, 100 = smooth) |
| `scroll_pause` | u64 | Pause after scrolling (ms) |

### General Timing
| Parameter | Type | Description |
|-----------|------|-------------|
| `action_delay_min` | u64 | Minimum delay between actions (ms) |
| `action_delay_variance_pct` | % | Maximum delay variance as percentage of min |

## Profile Presets

### Typo Parameters Table

| Profile | Typo Rate (%) | Notice Delay (ms) | Retry Delay (ms) | Recovery Chance (%) | **Effective Typo (%)** |
|---------|--------------|-------------------|------------------|---------------------|------------------------|
| Expert | 0.2 | 100 | 60 | 99 | 0.002 |
| Thorough | 0.2 | 800 | 500 | 99.5 | 0.001 |
| Researcher | 0.3 | 700 | 400 | 98 | 0.006 |
| Analytical | 0.4 | 600 | 400 | 97 | 0.012 |
| Senior | 1.0 | 500 | 300 | 95 | 0.05 |
| Cautious | 0.5 | 450 | 280 | 90 | 0.05 |
| Professional | 0.8 | 200 | 120 | 90 | 0.08 |
| Enthusiast | 1.0 | 250 | 150 | 85 | 0.15 |
| PowerUser | 0.5 | 150 | 80 | 90 | 0.05 |
| Focused | 0.5 | 150 | 80 | 92 | 0.04 |
| Average | 2.0 | 300 | 200 | 80 | 0.4 |
| Leisure | 2.5 | 500 | 320 | 78 | 0.55 |
| Casual | 3.0 | 400 | 250 | 75 | 0.75 |
| Novice | 8.0 | 600 | 400 | 75 | 2.0 |
| Teen | 5.0 | 200 | 100 | 60 | 2.0 |
| Impatient | 8.0 | 120 | 60 | 80 | 1.6 |
| Stressed | 9.0 | 100 | 50 | 78 | 1.98 |
| Erratic | 5.0 | 350 | 250 | 25 | 3.75 |
| Adaptive | 3.0 | 350 | 220 | 25 | 2.25 |
| Distracted | 4.0 | 400 | 280 | 40 | 2.4 |
| QuickScanner | 5.0 | 80 | 40 | 65 | 1.75 |

**Note:** Maximum effective typo rate is capped at 4%.

### All Parameters by Profile

#### Average
Typical everyday user behavior
- Cursor: speed 1.0±10%, step delay 10ms, curve spread 50, precision 95%
- Typing: 120ms±20%, typo 2%, word pause 500ms
- Click: reaction 50ms, offset 5px
- Scroll: amount 500px, smooth 70%, pause 500ms
- Actions: delay 500ms±50%

#### Teen
Young user - fast, less precise
- Cursor: speed 1.5±20%, step delay 5ms, curve spread 80, precision 85%
- Typing: 80ms±30%, typo 5%, word pause 300ms
- Click: reaction 30ms, offset 15px
- Scroll: amount 800px, smooth 40%, pause 200ms
- Actions: delay 300ms±60%

#### Senior
Older user - slower, more deliberate
- Cursor: speed 0.6±10%, step delay 20ms, curve spread 30, precision 98%
- Typing: 200ms±15%, typo 1%, word pause 800ms
- Click: reaction 100ms, offset 2px
- Scroll: amount 300px, smooth 90%, pause 800ms
- Actions: delay 800ms±30%

#### Enthusiast
Tech-savvy user - precise, researched
- Cursor: speed 1.2±10%, step delay 8ms, curve spread 40, precision 99%
- Typing: 100ms±10%, typo 1%, word pause 400ms
- Click: reaction 40ms, offset 3px
- Scroll: amount 400px, smooth 80%, pause 600ms
- Actions: delay 600ms±40%

#### PowerUser
Experienced user - fast, efficient
- Cursor: speed 1.8±15%, step delay 3ms, curve spread 25, precision 97%
- Typing: 70ms±20%, typo 0.5%, word pause 200ms
- Click: reaction 20ms, offset 2px
- Scroll: amount 1000px, smooth 20%, pause 150ms
- Actions: delay 200ms±30%

#### Cautious
Careful user - lots of pauses, verification
- Cursor: speed 0.7±15%, step delay 18ms, curve spread 35, precision 99.5%
- Typing: 180ms±15%, typo 0.5%, word pause 700ms
- Click: reaction 150ms, offset 1px
- Scroll: amount 250px, smooth 95%, pause 1000ms
- Actions: delay 1000ms±25%

#### Impatient
Quick decision maker - minimal pauses
- Cursor: speed 2.0±10%, step delay 2ms, curve spread 60, precision 80%
- Typing: 60ms±25%, typo 8%, word pause 150ms
- Click: reaction 15ms, offset 20px
- Scroll: amount 1200px, smooth 10%, pause 100ms
- Actions: delay 100ms±20%

#### Erratic
Inconsistent timing and speed
- Cursor: speed 1.0±50%, step delay 12ms±60%, curve spread 70, precision 90%
- Typing: 120ms±50%, typo 5%, word pause 500ms
- Click: reaction 60ms, offset 10px
- Scroll: amount 600px, smooth 50%, pause 400ms
- Actions: delay 400ms±70%

#### Researcher
Research-focused - slow, thorough
- Cursor: speed 0.5±15%, step delay 25ms, curve spread 25, precision 99.5%
- Typing: 250ms±15%, typo 0.3%, word pause 1000ms
- Click: reaction 200ms, offset 0px
- Scroll: amount 200px, smooth 100%, pause 1500ms
- Actions: delay 1500ms±20%

#### Casual
Relaxed browsing - slow pace
- Cursor: speed 0.8±15%, step delay 15ms, curve spread 55, precision 92%
- Typing: 150ms±20%, typo 3%, word pause 600ms
- Click: reaction 70ms, offset 8px
- Scroll: amount 400px, smooth 75%, pause 700ms
- Actions: delay 700ms±45%

#### Professional
Work-focused - efficient, minimal waste
- Cursor: speed 1.6±10%, step delay 5ms, curve spread 30, precision 98%
- Typing: 80ms±15%, typo 0.8%, word pause 300ms
- Click: reaction 30ms, offset 3px
- Scroll: amount 900px, smooth 30%, pause 300ms
- Actions: delay 400ms±30%

#### Novice
Learning user - slow, uncertain
- Cursor: speed 0.5±20%, step delay 30ms, curve spread 60, precision 85%
- Typing: 250ms±20%, typo 8%, word pause 900ms
- Click: reaction 250ms, offset 25px
- Scroll: amount 200px, smooth 85%, pause 1200ms
- Actions: delay 1200ms±30%

#### Expert
Skilled user - fast, precise
- Cursor: speed 1.9±8%, step delay 3ms, curve spread 20, precision 99.5%
- Typing: 60ms±12%, typo 0.2%, word pause 180ms
- Click: reaction 15ms, offset 1px
- Scroll: amount 1200px, smooth 15%, pause 100ms
- Actions: delay 150ms±25%

#### Distracted
Frequently interrupted - random pauses
- Cursor: speed 0.9±25%, step delay 12ms, curve spread 55, precision 88%
- Typing: 130ms±30%, typo 5%, word pause 600ms
- Click: reaction 80ms, offset 12px
- Scroll: amount 450px, smooth 55%, pause 600ms
- Actions: delay 600ms±80%

#### Focused
Concentrated work - consistent, few pauses
- Cursor: speed 1.4±8%, step delay 7ms, curve spread 35, precision 97%
- Typing: 90ms±10%, typo 0.5%, word pause 250ms
- Click: reaction 25ms, offset 2px
- Scroll: amount 850px, smooth 35%, pause 250ms
- Actions: delay 300ms±20%

#### Analytical
Data gathering - methodical, even scrolling
- Cursor: speed 0.6±10%, step delay 22ms, curve spread 20, precision 99%
- Typing: 220ms±12%, typo 0.4%, word pause 900ms
- Click: reaction 180ms, offset 1px
- Scroll: amount 250px, smooth 100%, pause 1800ms
- Actions: delay 1800ms±15%

#### QuickScanner
Speed-focused - fast scrolls, quick decisions
- Cursor: speed 2.2±15%, step delay 2ms, curve spread 70, precision 75%
- Typing: 50ms±30%, typo 5%, word pause 100ms
- Click: reaction 10ms, offset 30px
- Scroll: amount 1500px, smooth 5%, pause 80ms
- Actions: delay 80ms±15%

#### Thorough
Complete coverage - slow, comprehensive
- Cursor: speed 0.4±12%, step delay 30ms, curve spread 22, precision 99.8%
- Typing: 280ms±12%, typo 0.2%, word pause 1200ms
- Click: reaction 300ms, offset 0px
- Scroll: amount 150px, smooth 100%, pause 2000ms
- Actions: delay 2000ms±12%

#### Adaptive
Adjusts based on content type
- Cursor: speed 1.0±40%, step delay 12ms±50%, curve spread 50, precision 93%
- Typing: 120ms±40%, typo 3%, word pause 500ms
- Click: reaction 60ms, offset 8px
- Scroll: amount 550px, smooth 60%, pause 550ms
- Actions: delay 550ms±55%

#### Stressed
Time pressure - fast, less accurate
- Cursor: speed 1.8±20%, step delay 3ms, curve spread 65, precision 78%
- Typing: 65ms±28%, typo 9%, word pause 180ms
- Click: reaction 18ms, offset 22px
- Scroll: amount 1100px, smooth 12%, pause 130ms
- Actions: delay 130ms±25%

#### Leisure
Enjoyment-focused - slow, exploratory
- Cursor: speed 0.55±12%, step delay 22ms, curve spread 65, precision 90%
- Typing: 200ms±18%, typo 2.5%, word pause 800ms
- Click: reaction 120ms, offset 10px
- Scroll: amount 280px, smooth 90%, pause 1000ms
- Actions: delay 1000ms±35%

## Usage Example

```rust
use crate::utils::profile::{BrowserProfile, ProfilePreset, randomize_profile};

// Get a preset and randomize it for this session
let profile = randomize_profile(&ProfilePreset::Teen);

// Use random values from profile
let cursor_speed = profile.cursor_speed.random();
let typing_delay = profile.typing_speed_mean.random_u64();
let should_fix_typo = rand::random::<f64>() * 100.0 < profile.typo_recovery_chance.random();
```
