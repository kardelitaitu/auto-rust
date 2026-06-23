//! Browser behavioral profile system.
//!
//! Provides configurable profiles for human-like browser automation with
//! randomized per-session variations. Profiles control cursor movement,
//! typing speed, clicking behavior, scrolling, and timing delays.
//!
//! # Usage
//! ```no_run
//! use auto::utils::{randomize_profile, ProfilePreset};
//!
//! // Get a preset and randomize it for this session
//! let profile = randomize_profile(&ProfilePreset::Teen);
//!
//! // Pass `profile` into mouse and typing helpers that accept custom configs.
//! ```

mod presets;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::utils::mouse::{CursorMovementConfig, PathStyle, Precision, Speed};

const CURSOR_SPEED_BOOST_FACTOR: f64 = 6.0;
const CURSOR_INTERVAL_MIN_FLOOR_MS: u64 = 80;

/// A profile parameter with base value and deviation percentage.
/// Allows randomized variation per session while maintaining profile characteristics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProfileParam {
    /// Base value for this parameter
    pub base: f64,
    /// Deviation percentage (e.g., 10.0 = ±10% variation)
    pub deviation_pct: f64,
}

impl ProfileParam {
    /// Creates a new profile parameter.
    #[must_use]
    pub fn new(base: f64, deviation_pct: f64) -> Self {
        Self {
            base,
            deviation_pct,
        }
    }

    /// Returns randomized value within deviation range.
    /// Uses uniform distribution: base * (1 ± `deviation_pct/100`)
    #[must_use]
    pub fn random(&self) -> f64 {
        if self.deviation_pct == 0.0 {
            return self.base;
        }
        let mut rng = rand::thread_rng();
        let deviation = (rng.gen::<f64>() * 2.0 - 1.0) * self.deviation_pct / 100.0;
        self.base * (1.0 + deviation)
    }

    /// Returns randomized value as u64.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn random_u64(&self) -> u64 {
        self.random() as u64
    }

    /// Returns randomized value as u32.
    #[must_use]
    pub fn random_u32(&self) -> u32 {
        self.random() as u32
    }

    /// Returns randomized value clamped to range.
    #[must_use]
    pub fn random_clamped(&self, min: f64, max: f64) -> f64 {
        self.random().clamp(min, max)
    }
}

/// Creates a profile parameter from a single value (no deviation).
impl From<f64> for ProfileParam {
    fn from(base: f64) -> Self {
        Self {
            base,
            deviation_pct: 0.0,
        }
    }
}

/// Scroll behavior derived from a browser profile.
/// Tasks can use this to tune scroll amount, pause, and smoothness consistently.
#[derive(Debug, Clone, Copy)]
pub struct ScrollBehavior {
    /// Typical scroll amount in pixels.
    pub amount: i32,
    /// Typical pause after a scroll action in milliseconds.
    pub pause_ms: u64,
    /// Whether to favor smoother, more variable scrolling.
    pub smooth: bool,
    /// Whether to occasionally backtrack a little.
    pub back_scroll: bool,
}

/// Cursor behavior derived from a browser profile.
/// Tasks can use this to tune cursor movement cadence consistently.
#[derive(Debug, Clone, Copy)]
pub struct CursorBehavior {
    /// Minimum delay between cursor moves in milliseconds.
    pub interval_min_ms: u64,
    /// Maximum delay between cursor moves in milliseconds.
    pub interval_max_ms: u64,
}

impl CursorBehavior {
    /// Converts cursor cadence into a concrete movement config.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_movement_config(&self) -> CursorMovementConfig {
        let interval_min_ms = self.interval_min_ms.max(1);
        let interval_max_ms = self.interval_max_ms.max(interval_min_ms);
        CursorMovementConfig {
            speed_multiplier: (120.0 / interval_min_ms as f64).clamp(0.35, 3.0),
            min_step_delay_ms: interval_min_ms,
            max_step_delay_variance_ms: interval_max_ms.saturating_sub(interval_min_ms).max(1),
            curve_spread: interval_max_ms.saturating_sub(interval_min_ms).max(20) as f64,
            steps: None,
            add_micro_pauses: true,
            path_style: PathStyle::Bezier,
            precision: Precision::Safe,
            speed: if interval_min_ms <= 8 {
                Speed::Fast
            } else if interval_min_ms >= 20 {
                Speed::Slow
            } else {
                Speed::Normal
            },
        }
    }
}

/// Typing behavior derived from a browser profile.
#[derive(Debug, Clone, Copy)]
pub struct TypingBehavior {
    /// Mean delay between keystrokes in milliseconds.
    pub keystroke_mean_ms: u64,
    /// Keystroke jitter in milliseconds.
    pub keystroke_stddev_ms: u64,
    /// Pause between words in milliseconds.
    pub word_pause_ms: u64,
    /// Typo probability per character, percentage.
    pub typo_rate_pct: f64,
    /// Delay before noticing a typo in milliseconds.
    pub typo_notice_delay_ms: u64,
    /// Delay before correcting a typo in milliseconds.
    pub typo_retry_delay_ms: u64,
    /// Chance of correcting a typo, percentage.
    pub typo_recovery_chance_pct: f64,
}

/// Click behavior derived from a browser profile.
#[derive(Debug, Clone, Copy)]
pub struct ClickBehavior {
    /// Delay after reaching target before clicking.
    pub reaction_delay_ms: u64,
    /// Variance allowed around the reaction delay.
    pub reaction_delay_variance_pct: f64,
    /// Click offset around the target center in pixels.
    pub offset_px: i32,
}

/// General action delay behavior derived from a browser profile.
#[derive(Debug, Clone, Copy)]
pub struct ActionDelayBehavior {
    /// Minimum delay between actions in milliseconds.
    pub min_ms: u64,
    /// Allowed variance percentage.
    pub variance_pct: f64,
}

/// Session-stable behavior snapshot derived from a browser profile.
#[derive(Debug, Clone, Copy)]
pub struct ProfileRuntime {
    pub cursor: CursorBehavior,
    pub typing: TypingBehavior,
    pub click: ClickBehavior,
    pub scroll: ScrollBehavior,
    pub action_delay: ActionDelayBehavior,
    pub random_cursor_safe_edge_ratio: f64,
}

/// Complete browser behavior profile.
/// Controls all aspects of human-like browser interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserProfile {
    /// Profile name
    pub name: String,
    /// Profile description
    pub description: String,

    // === Cursor Movement ===
    /// Movement speed multiplier (1.0 = normal, 0.5 = slow, 2.0 = fast)
    pub cursor_speed: ProfileParam,
    /// Delay between movement steps in milliseconds
    pub cursor_step_delay: ProfileParam,
    /// Bezier curve control point spread (higher = more curved path)
    pub cursor_curve_spread: ProfileParam,
    /// How close cursor gets to target (0-100%, 100% = exact center)
    pub cursor_precision: ProfileParam,
    /// Probability of random micro-pause during movement (0-100%)
    pub cursor_micro_pause_chance: ProfileParam,
    /// Duration of micro-pause in milliseconds
    pub cursor_micro_pause_duration: ProfileParam,

    // === Typing ===
    /// Average keystroke delay in milliseconds
    pub typing_speed_mean: ProfileParam,
    /// Keystroke delay standard deviation
    pub typing_speed_stddev: ProfileParam,
    /// Typo probability per character (0-100%)
    pub typo_rate: ProfileParam,
    /// Pause between words in milliseconds
    pub typing_word_pause: ProfileParam,
    /// Typo notice delay: time after making typo before noticing (ms)
    pub typo_notice_delay: ProfileParam,
    /// Typo retry delay: time after backspace before ret typing (ms)
    pub typo_retry_delay: ProfileParam,
    /// Probability of correcting typo vs leaving it (0-100%)
    pub typo_recovery_chance: ProfileParam,

    // === Clicking ===
    /// Delay after arriving at target before clicking (ms)
    pub click_reaction_delay: ProfileParam,
    /// Click offset from element center in pixels
    pub click_offset: ProfileParam,

    // === Scrolling ===
    /// Scroll amount in pixels per action
    pub scroll_amount: ProfileParam,
    /// Scroll behavior: 0 = instant, 100 = smooth
    pub scroll_smoothness: ProfileParam,
    /// Pause after scrolling in milliseconds
    pub scroll_pause: ProfileParam,

    // === General Timing ===
    /// Minimum delay between actions in milliseconds
    pub action_delay_min: ProfileParam,
    /// Maximum delay variance as percentage of min
    pub action_delay_variance_pct: ProfileParam,

    // === Behavioral Variance ===
    /// Variance applied to engagement probability weights (e.g., 40.0 = ±40%).
    /// Separate from action_delay_variance_pct which controls timing jitter.
    /// Higher values = more unpredictable engagement behavior.
    #[serde(default = "default_behavior_variance")]
    pub behavior_variance_pct: ProfileParam,

    // === Twitter-specific ===
    /// Probability of diving into a thread when viewing a tweet (0-100%)
    #[serde(default = "default_dive_probability")]
    pub dive_probability: ProfileParam,
}

fn default_behavior_variance() -> ProfileParam {
    ProfileParam {
        base: 40.0,
        deviation_pct: 20.0,
    }
}

fn default_dive_probability() -> ProfileParam {
    ProfileParam {
        base: 0.35,
        deviation_pct: 20.0,
    }
}

// ============================================================================
// Derived Behaviors
// ============================================================================

impl BrowserProfile {
    /// Derives scroll behavior from the profile.
    #[must_use]
    pub fn scroll_behavior(&self) -> ScrollBehavior {
        let amount = self.scroll_amount.random_clamped(120.0, 2_000.0).round() as i32;
        let pause_ms = self.scroll_pause.random_clamped(80.0, 3_000.0).round() as u64;
        let smoothness = self.scroll_smoothness.random_clamped(0.0, 100.0);

        ScrollBehavior {
            amount: amount.max(1),
            pause_ms: pause_ms.max(1),
            smooth: smoothness >= 50.0,
            back_scroll: smoothness < 20.0 && rand::thread_rng().gen_bool(0.2),
        }
    }

    /// Derives cursor behavior from the profile.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn cursor_behavior(&self) -> CursorBehavior {
        let speed = self.cursor_speed.random_clamped(0.25, 3.0);
        let step_delay = self.cursor_step_delay.random_clamped(1.0, 60.0);

        let mut interval_min_ms = (step_delay * (2.5 / speed.max(0.25))).round() as u64;
        let mut interval_max_ms = (step_delay * (3.5 / speed.max(0.25))).round() as u64;

        interval_min_ms = interval_min_ms.clamp(200, 5_000);
        interval_max_ms = interval_max_ms.clamp(interval_min_ms, 8_000);
        interval_min_ms = ((interval_min_ms as f64) / CURSOR_SPEED_BOOST_FACTOR).round() as u64;
        interval_max_ms = ((interval_max_ms as f64) / CURSOR_SPEED_BOOST_FACTOR).round() as u64;
        interval_min_ms = interval_min_ms.clamp(CURSOR_INTERVAL_MIN_FLOOR_MS, 5_000);
        interval_max_ms = interval_max_ms.clamp(interval_min_ms, 8_000);

        CursorBehavior {
            interval_min_ms,
            interval_max_ms,
        }
    }

    /// Converts cursor behavior into a concrete movement config.
    #[must_use]
    pub fn cursor_movement_config(&self) -> CursorMovementConfig {
        let cursor = self.cursor_behavior();
        cursor.to_movement_config()
    }

    /// Derives typing behavior from the profile.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn typing_behavior(&self) -> TypingBehavior {
        TypingBehavior {
            keystroke_mean_ms: self.typing_speed_mean.random_clamped(20.0, 500.0).round() as u64,
            keystroke_stddev_ms: self.typing_speed_stddev.random_clamped(5.0, 150.0).round() as u64,
            word_pause_ms: self.typing_word_pause.random_clamped(50.0, 2_000.0).round() as u64,
            typo_rate_pct: self.typo_rate.random_clamped(0.0, 20.0),
            typo_notice_delay_ms: self.typo_notice_delay.random_clamped(50.0, 2_000.0).round()
                as u64,
            typo_retry_delay_ms: self.typo_retry_delay.random_clamped(20.0, 1_000.0).round() as u64,
            typo_recovery_chance_pct: self.typo_recovery_chance.random_clamped(0.0, 100.0),
        }
    }

    /// Derives click behavior from the profile.
    #[must_use]
    pub fn click_behavior(&self) -> ClickBehavior {
        ClickBehavior {
            reaction_delay_ms: self
                .click_reaction_delay
                .random_clamped(0.0, 2_000.0)
                .round() as u64,
            reaction_delay_variance_pct: self.action_delay_variance_pct.random_clamped(0.0, 100.0),
            offset_px: self.click_offset.random_clamped(0.0, 50.0).round() as i32,
        }
    }

    /// Derives general action delay behavior from the profile.
    #[must_use]
    pub fn action_delay_behavior(&self) -> ActionDelayBehavior {
        ActionDelayBehavior {
            min_ms: self.action_delay_min.random_clamped(0.0, 5_000.0).round() as u64,
            variance_pct: self.action_delay_variance_pct.random_clamped(0.0, 100.0),
        }
    }

    /// Derives safe edge ratio for random cursor moves.
    /// Larger values keep movement farther from viewport edges.
    #[must_use]
    pub fn random_cursor_safe_edge_ratio(&self) -> f64 {
        let precision = self.cursor_precision.base.clamp(60.0, 100.0);
        let extra = ((100.0 - precision) / 40.0) * 0.08;
        (0.10 + extra).clamp(0.10, 0.18)
    }

    /// Builds a stable runtime snapshot for a session.
    #[must_use]
    pub fn runtime(&self) -> ProfileRuntime {
        ProfileRuntime {
            cursor: self.cursor_behavior(),
            typing: self.typing_behavior(),
            click: self.click_behavior(),
            scroll: self.scroll_behavior(),
            action_delay: self.action_delay_behavior(),
            random_cursor_safe_edge_ratio: self.random_cursor_safe_edge_ratio(),
        }
    }
}

// ============================================================================
// Profile Presets
// ============================================================================

/// Available profile presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProfilePreset {
    /// Typical everyday user behavior
    Average,
    /// Young user - fast, less precise
    Teen,
    /// Older user - slower, more deliberate
    Senior,
    /// Tech-savvy user - precise, researched
    Enthusiast,
    /// Experienced user - fast, efficient
    PowerUser,
    /// Careful user - lots of pauses, verification
    Cautious,
    /// Quick decision maker - minimal pauses
    Impatient,
    /// Inconsistent timing and speed
    Erratic,
    /// Research-focused - slow, thorough
    Researcher,
    /// Relaxed browsing - slow pace
    Casual,
    /// Work-focused - efficient, minimal waste
    Professional,
    /// Learning user - slow, uncertain
    Novice,
    /// Skilled user - fast, precise
    Expert,
    /// Frequently interrupted - random pauses
    Distracted,
    /// Concentrated work - consistent, few pauses
    Focused,
    /// Data gathering - methodical, even scrolling
    Analytical,
    /// Speed-focused - fast scrolls, quick decisions
    QuickScanner,
    /// Complete coverage - slow, comprehensive
    Thorough,
    /// Adjusts based on content type
    Adaptive,
    /// Time pressure - fast, less accurate
    Stressed,
    /// Enjoyment-focused - slow, exploratory
    Leisure,
}

/// Creates a randomized profile from a preset for this session.
#[must_use]
pub fn randomize_profile(preset: &ProfilePreset) -> BrowserProfile {
    BrowserProfile::from_preset(preset)
}

/// Returns a random profile preset.
#[must_use]
pub fn random_preset() -> ProfilePreset {
    let presets = [
        ProfilePreset::Average,
        ProfilePreset::Teen,
        ProfilePreset::Senior,
        ProfilePreset::Enthusiast,
        ProfilePreset::PowerUser,
        ProfilePreset::Cautious,
        ProfilePreset::Impatient,
        ProfilePreset::Erratic,
        ProfilePreset::Researcher,
        ProfilePreset::Casual,
        ProfilePreset::Professional,
        ProfilePreset::Novice,
        ProfilePreset::Expert,
        ProfilePreset::Distracted,
        ProfilePreset::Focused,
        ProfilePreset::Analytical,
        ProfilePreset::QuickScanner,
        ProfilePreset::Thorough,
        ProfilePreset::Adaptive,
        ProfilePreset::Stressed,
        ProfilePreset::Leisure,
    ];

    let idx = rand::random::<usize>() % presets.len();
    presets[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_param_random() {
        let param = ProfileParam::new(100.0, 10.0);
        for _ in 0..100 {
            let val = param.random();
            assert!((90.0..=110.0).contains(&val), "Value {} out of range", val);
        }
    }

    #[test]
    fn test_profile_param_zero_deviation() {
        let param = ProfileParam::new(100.0, 0.0);
        assert_eq!(param.random(), 100.0);
    }

    #[test]
    fn test_all_presets() {
        let presets = [
            ProfilePreset::Average,
            ProfilePreset::Teen,
            ProfilePreset::Senior,
            ProfilePreset::Enthusiast,
            ProfilePreset::PowerUser,
            ProfilePreset::Cautious,
            ProfilePreset::Impatient,
            ProfilePreset::Erratic,
            ProfilePreset::Researcher,
            ProfilePreset::Casual,
            ProfilePreset::Professional,
            ProfilePreset::Novice,
            ProfilePreset::Expert,
            ProfilePreset::Distracted,
            ProfilePreset::Focused,
            ProfilePreset::Analytical,
            ProfilePreset::QuickScanner,
            ProfilePreset::Thorough,
            ProfilePreset::Adaptive,
            ProfilePreset::Stressed,
            ProfilePreset::Leisure,
        ];

        for preset in presets {
            let profile = BrowserProfile::from_preset(&preset);
            assert!(!profile.name.is_empty());
            assert!(!profile.description.is_empty());
        }
    }

    #[test]
    fn test_runtime_snapshot_is_stable_shape() {
        let profile = BrowserProfile::average();
        let runtime = profile.runtime();

        assert!(runtime.cursor.interval_min_ms >= CURSOR_INTERVAL_MIN_FLOOR_MS);
        assert!(runtime.cursor.interval_max_ms >= runtime.cursor.interval_min_ms);
        assert!(runtime.scroll.amount > 0);
        assert!(runtime.scroll.pause_ms > 0);
        assert!(runtime.typing.keystroke_mean_ms > 0);
        assert!(runtime.click.reaction_delay_ms <= 2_000);
        assert!(runtime.action_delay.min_ms <= 5_000);
    }

    #[test]
    fn test_derived_behaviors_are_within_bounds() {
        let profile = BrowserProfile::thorough();
        let cursor = profile.cursor_behavior();
        let scroll = profile.scroll_behavior();
        let typing = profile.typing_behavior();
        let click = profile.click_behavior();
        let action_delay = profile.action_delay_behavior();

        assert!(cursor.interval_min_ms <= cursor.interval_max_ms);
        assert!(scroll.amount > 0);
        assert!(scroll.pause_ms > 0);
        assert!(typing.keystroke_mean_ms > 0);
        assert!(typing.word_pause_ms > 0);
        assert!(typing.typo_rate_pct >= 0.0);
        assert!(click.reaction_delay_ms <= 2_000);
        assert!(action_delay.min_ms <= 5_000);
    }

    #[test]
    fn test_typing_speed_table_is_slowed_down() {
        let presets = [
            ProfilePreset::Average,
            ProfilePreset::Teen,
            ProfilePreset::Senior,
            ProfilePreset::Enthusiast,
            ProfilePreset::PowerUser,
            ProfilePreset::Cautious,
            ProfilePreset::Impatient,
            ProfilePreset::Erratic,
            ProfilePreset::Researcher,
            ProfilePreset::Casual,
            ProfilePreset::Professional,
            ProfilePreset::Novice,
            ProfilePreset::Expert,
            ProfilePreset::Distracted,
            ProfilePreset::Focused,
            ProfilePreset::Analytical,
            ProfilePreset::QuickScanner,
            ProfilePreset::Thorough,
            ProfilePreset::Adaptive,
            ProfilePreset::Stressed,
            ProfilePreset::Leisure,
        ];

        for preset in presets {
            let profile = BrowserProfile::from_preset(&preset);
            assert!(
                profile.typing_speed_mean.base >= 120.0,
                "preset {:?} is too fast: {}",
                preset,
                profile.typing_speed_mean.base
            );
        }
    }

    #[test]
    fn test_typo_recovery_table_stays_under_100() {
        let presets = [
            ProfilePreset::Average,
            ProfilePreset::Teen,
            ProfilePreset::Senior,
            ProfilePreset::Enthusiast,
            ProfilePreset::PowerUser,
            ProfilePreset::Cautious,
            ProfilePreset::Impatient,
            ProfilePreset::Erratic,
            ProfilePreset::Researcher,
            ProfilePreset::Casual,
            ProfilePreset::Professional,
            ProfilePreset::Novice,
            ProfilePreset::Expert,
            ProfilePreset::Distracted,
            ProfilePreset::Focused,
            ProfilePreset::Analytical,
            ProfilePreset::QuickScanner,
            ProfilePreset::Thorough,
            ProfilePreset::Adaptive,
            ProfilePreset::Stressed,
            ProfilePreset::Leisure,
        ];

        for preset in presets {
            let profile = BrowserProfile::from_preset(&preset);
            assert!(
                profile.typo_recovery_chance.base <= 98.0,
                "preset {:?} is too high: {}",
                preset,
                profile.typo_recovery_chance.base
            );
        }
    }

    #[test]
    fn test_profile_param_new() {
        let param = ProfileParam::new(50.0, 20.0);
        assert_eq!(param.base, 50.0);
        assert_eq!(param.deviation_pct, 20.0);
    }

    #[test]
    fn test_profile_param_random_u64() {
        let param = ProfileParam::new(100.0, 10.0);
        for _ in 0..10 {
            let val = param.random_u64();
            assert!((90..=110).contains(&val));
        }
    }

    #[test]
    fn test_profile_param_random_u32() {
        let param = ProfileParam::new(50.0, 10.0);
        for _ in 0..10 {
            let val = param.random_u32();
            assert!((45..=55).contains(&val));
        }
    }

    #[test]
    fn test_profile_param_random_clamped() {
        let param = ProfileParam::new(100.0, 50.0);
        for _ in 0..10 {
            let val = param.random_clamped(80.0, 120.0);
            assert!((80.0..=120.0).contains(&val));
        }
    }

    #[test]
    fn test_scroll_behavior_creation() {
        let scroll = ScrollBehavior {
            amount: 300,
            pause_ms: 100,
            smooth: true,
            back_scroll: false,
        };
        assert_eq!(scroll.amount, 300);
        assert_eq!(scroll.pause_ms, 100);
    }

    #[test]
    fn test_cursor_behavior_creation() {
        let cursor = CursorBehavior {
            interval_min_ms: 10,
            interval_max_ms: 20,
        };
        assert_eq!(cursor.interval_min_ms, 10);
        assert_eq!(cursor.interval_max_ms, 20);
    }

    #[test]
    fn test_cursor_behavior_to_movement_config() {
        let cursor = CursorBehavior {
            interval_min_ms: 10,
            interval_max_ms: 20,
        };
        let config = cursor.to_movement_config();
        assert_eq!(config.min_step_delay_ms, 10);
    }

    #[test]
    fn test_typing_behavior_creation() {
        let typing = TypingBehavior {
            keystroke_mean_ms: 100,
            keystroke_stddev_ms: 20,
            word_pause_ms: 300,
            typo_rate_pct: 2.0,
            typo_notice_delay_ms: 500,
            typo_retry_delay_ms: 200,
            typo_recovery_chance_pct: 90.0,
        };
        assert_eq!(typing.keystroke_mean_ms, 100);
        assert_eq!(typing.typo_rate_pct, 2.0);
    }

    #[test]
    fn test_click_behavior_creation() {
        let click = ClickBehavior {
            reaction_delay_ms: 200,
            reaction_delay_variance_pct: 20.0,
            offset_px: 5,
        };
        assert_eq!(click.reaction_delay_ms, 200);
        assert_eq!(click.reaction_delay_variance_pct, 20.0);
        assert_eq!(click.offset_px, 5);
    }

    #[test]
    fn test_action_delay_behavior_creation() {
        let delay = ActionDelayBehavior {
            min_ms: 100,
            variance_pct: 20.0,
        };
        assert_eq!(delay.min_ms, 100);
        assert_eq!(delay.variance_pct, 20.0);
    }

    #[test]
    fn test_profile_runtime_creation() {
        let profile = BrowserProfile::average();
        let runtime = profile.runtime();
        assert!(runtime.cursor.interval_min_ms > 0);
    }

    #[test]
    fn test_browser_profile_scroll_behavior() {
        let profile = BrowserProfile::average();
        let scroll = profile.scroll_behavior();
        assert!(scroll.amount > 0);
    }

    #[test]
    fn test_browser_profile_cursor_behavior() {
        let profile = BrowserProfile::average();
        let cursor = profile.cursor_behavior();
        assert!(cursor.interval_min_ms > 0);
    }

    #[test]
    fn test_browser_profile_cursor_movement_config() {
        let profile = BrowserProfile::average();
        let config = profile.cursor_movement_config();
        assert!(config.min_step_delay_ms > 0);
    }

    #[test]
    fn test_browser_profile_typing_behavior() {
        let profile = BrowserProfile::average();
        let typing = profile.typing_behavior();
        assert!(typing.keystroke_mean_ms > 0);
    }

    #[test]
    fn test_browser_profile_click_behavior() {
        let profile = BrowserProfile::average();
        let click = profile.click_behavior();
        assert!(click.reaction_delay_ms > 0);
    }

    #[test]
    fn test_browser_profile_action_delay_behavior() {
        let profile = BrowserProfile::average();
        let delay = profile.action_delay_behavior();
        assert!(delay.min_ms > 0);
    }

    #[test]
    fn test_browser_profile_random_cursor_safe_edge_ratio() {
        let profile = BrowserProfile::average();
        let ratio = profile.random_cursor_safe_edge_ratio();
        assert!((0.0..=1.0).contains(&ratio));
    }

    #[test]
    fn test_randomize_profile() {
        let profile = randomize_profile(&ProfilePreset::Average);
        assert!(!profile.name.is_empty());
    }

    #[test]
    fn test_random_preset() {
        let preset = random_preset();
        let _ = BrowserProfile::from_preset(&preset);
    }

    #[test]
    fn test_cursor_behavior_fast_interval() {
        let cursor = CursorBehavior {
            interval_min_ms: 5,
            interval_max_ms: 10,
        };
        let config = cursor.to_movement_config();
        assert_eq!(config.speed, Speed::Fast);
    }

    #[test]
    fn test_cursor_behavior_slow_interval() {
        let cursor = CursorBehavior {
            interval_min_ms: 25,
            interval_max_ms: 30,
        };
        let config = cursor.to_movement_config();
        assert_eq!(config.speed, Speed::Slow);
    }

    #[test]
    fn test_cursor_behavior_normal_interval() {
        let cursor = CursorBehavior {
            interval_min_ms: 15,
            interval_max_ms: 20,
        };
        let config = cursor.to_movement_config();
        assert_eq!(config.speed, Speed::Normal);
    }

    #[test]
    fn test_browser_profile_from_preset_average() {
        let profile = BrowserProfile::from_preset(&ProfilePreset::Average);
        assert_eq!(profile.name, "Average");
    }

    #[test]
    fn test_browser_profile_from_preset_teen() {
        let profile = BrowserProfile::from_preset(&ProfilePreset::Teen);
        assert_eq!(profile.name, "Teen");
    }

    #[test]
    fn test_randomize_profile_produces_variation() {
        let param1 = ProfileParam::new(100.0, 10.0);
        let mut found_variation = false;
        let first = param1.random();
        for _ in 0..20 {
            let val = param1.random();
            if val != first {
                found_variation = true;
                break;
            }
        }
        assert!(
            found_variation,
            "ProfileParam::random() should produce variation across calls"
        );
    }

    #[test]
    fn test_randomize_profile_preserves_preset_name() {
        let preset = ProfilePreset::Teen;
        let profile = randomize_profile(&preset);
        assert_eq!(profile.name, "Teen");
    }

    #[test]
    fn test_random_preset_returns_valid_preset() {
        let preset = random_preset();
        match preset {
            ProfilePreset::Average
            | ProfilePreset::Teen
            | ProfilePreset::Senior
            | ProfilePreset::Enthusiast
            | ProfilePreset::PowerUser
            | ProfilePreset::Cautious
            | ProfilePreset::Impatient
            | ProfilePreset::Erratic
            | ProfilePreset::Researcher
            | ProfilePreset::Casual
            | ProfilePreset::Professional
            | ProfilePreset::Novice
            | ProfilePreset::Expert
            | ProfilePreset::Distracted
            | ProfilePreset::Focused
            | ProfilePreset::Analytical
            | ProfilePreset::QuickScanner
            | ProfilePreset::Thorough
            | ProfilePreset::Adaptive
            | ProfilePreset::Stressed
            | ProfilePreset::Leisure => {}
        }
    }

    #[test]
    fn test_random_preset_distribution() {
        let mut presets = std::collections::HashSet::new();
        for _ in 0..100 {
            presets.insert(random_preset());
        }
        assert!(presets.len() >= 5);
    }
}
