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

impl ProfileParam {
    /// Creates a new profile parameter.
    pub fn new(base: f64, deviation_pct: f64) -> Self {
        Self {
            base,
            deviation_pct,
        }
    }

    /// Returns randomized value within deviation range.
    /// Uses uniform distribution: base * (1 ± deviation_pct/100)
    #[allow(dead_code)]
    pub fn random(&self) -> f64 {
        if self.deviation_pct == 0.0 {
            return self.base;
        }
        let mut rng = rand::thread_rng();
        let deviation = (rng.gen::<f64>() * 2.0 - 1.0) * self.deviation_pct / 100.0;
        self.base * (1.0 + deviation)
    }

    /// Returns randomized value as u64.
    #[allow(dead_code)]
    pub fn random_u64(&self) -> u64 {
        self.random() as u64
    }

    /// Returns randomized value as u32.
    #[allow(dead_code)]
    pub fn random_u32(&self) -> u32 {
        self.random() as u32
    }

    /// Returns randomized value clamped to range.
    #[allow(dead_code)]
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

    // === Twitter-specific ===
    /// Probability of diving into a thread when viewing a tweet (0-100%)
    #[serde(default = "default_dive_probability")]
    pub dive_probability: ProfileParam,
}

fn default_dive_probability() -> ProfileParam {
    ProfileParam {
        base: 0.35,          // 35% base probability
        deviation_pct: 20.0, // ±20% variation
    }
}

impl BrowserProfile {
    /// Creates a profile from a preset.
    pub fn from_preset(preset: &ProfilePreset) -> Self {
        match preset {
            ProfilePreset::Average => Self::average(),
            ProfilePreset::Teen => Self::teen(),
            ProfilePreset::Senior => Self::senior(),
            ProfilePreset::Enthusiast => Self::enthusiast(),
            ProfilePreset::PowerUser => Self::power_user(),
            ProfilePreset::Cautious => Self::cautious(),
            ProfilePreset::Impatient => Self::impatient(),
            ProfilePreset::Erratic => Self::erratic(),
            ProfilePreset::Researcher => Self::researcher(),
            ProfilePreset::Casual => Self::casual(),
            ProfilePreset::Professional => Self::professional(),
            ProfilePreset::Novice => Self::novice(),
            ProfilePreset::Expert => Self::expert(),
            ProfilePreset::Distracted => Self::distracted(),
            ProfilePreset::Focused => Self::focused(),
            ProfilePreset::Analytical => Self::analytical(),
            ProfilePreset::QuickScanner => Self::quick_scanner(),
            ProfilePreset::Thorough => Self::thorough(),
            ProfilePreset::Adaptive => Self::adaptive(),
            ProfilePreset::Stressed => Self::stressed(),
            ProfilePreset::Leisure => Self::leisure(),
        }
    }

    /// Derives scroll behavior from the profile.
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
    pub fn cursor_movement_config(&self) -> CursorMovementConfig {
        let cursor = self.cursor_behavior();
        cursor.to_movement_config()
    }

    /// Derives typing behavior from the profile.
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
    pub fn action_delay_behavior(&self) -> ActionDelayBehavior {
        ActionDelayBehavior {
            min_ms: self.action_delay_min.random_clamped(0.0, 5_000.0).round() as u64,
            variance_pct: self.action_delay_variance_pct.random_clamped(0.0, 100.0),
        }
    }

    /// Derives safe edge ratio for random cursor moves.
    /// Larger values keep movement farther from viewport edges.
    pub fn random_cursor_safe_edge_ratio(&self) -> f64 {
        let precision = self.cursor_precision.base.clamp(60.0, 100.0);
        let extra = ((100.0 - precision) / 40.0) * 0.08;
        (0.10 + extra).clamp(0.10, 0.18)
    }

    /// Builds a stable runtime snapshot for a session.
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

impl BrowserProfile {
    /// Average user - typical everyday browsing
    pub fn average() -> Self {
        Self {
            name: "Average".into(),
            description: "Typical everyday user behavior".into(),
            cursor_speed: p(0.6, 10.0),
            cursor_step_delay: p(15.0, 20.0),
            cursor_curve_spread: p(50.0, 20.0),
            cursor_precision: p(95.0, 5.0),
            cursor_micro_pause_chance: p(10.0, 30.0),
            cursor_micro_pause_duration: p(100.0, 30.0),
            typing_speed_mean: p(120.0, 20.0),
            typing_speed_stddev: p(40.0, 25.0),
            typo_rate: p(2.0, 50.0),
            typing_word_pause: p(500.0, 30.0),
            typo_notice_delay: p(300.0, 30.0),
            typo_retry_delay: p(200.0, 30.0),
            typo_recovery_chance: p(96.0, 20.0),
            click_reaction_delay: p(50.0, 30.0),
            click_offset: p(5.0, 40.0),
            scroll_amount: p(750.0, 30.0),
            scroll_smoothness: p(70.0, 20.0),
            scroll_pause: p(500.0, 30.0),
            action_delay_min: p(500.0, 30.0),
            action_delay_variance_pct: p(50.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Teen - fast, less precise
    pub fn teen() -> Self {
        Self {
            name: "Teen".into(),
            description: "Young user - fast, less precise".into(),
            cursor_speed: p(0.9, 20.0),
            cursor_step_delay: p(8.0, 30.0),
            cursor_curve_spread: p(80.0, 30.0),
            cursor_precision: p(85.0, 10.0),
            cursor_micro_pause_chance: p(5.0, 40.0),
            cursor_micro_pause_duration: p(50.0, 40.0),
            typing_speed_mean: p(130.0, 30.0),
            typing_speed_stddev: p(30.0, 40.0),
            typo_rate: p(5.0, 50.0),
            typing_word_pause: p(300.0, 40.0),
            typo_notice_delay: p(200.0, 40.0),
            typo_retry_delay: p(100.0, 40.0),
            typo_recovery_chance: p(72.0, 30.0),
            click_reaction_delay: p(30.0, 40.0),
            click_offset: p(15.0, 40.0),
            scroll_amount: p(1200.0, 40.0),
            scroll_smoothness: p(40.0, 30.0),
            scroll_pause: p(200.0, 40.0),
            action_delay_min: p(300.0, 40.0),
            action_delay_variance_pct: p(60.0, 30.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Senior - slower, more deliberate
    pub fn senior() -> Self {
        Self {
            name: "Senior".into(),
            description: "Older user - slower, more deliberate".into(),
            cursor_speed: p(0.35, 10.0),
            cursor_step_delay: p(30.0, 15.0),
            cursor_curve_spread: p(30.0, 20.0),
            cursor_precision: p(98.0, 2.0),
            cursor_micro_pause_chance: p(20.0, 20.0),
            cursor_micro_pause_duration: p(200.0, 20.0),
            typing_speed_mean: p(200.0, 15.0),
            typing_speed_stddev: p(30.0, 20.0),
            typo_rate: p(1.0, 30.0),
            typing_word_pause: p(800.0, 20.0),
            typo_notice_delay: p(500.0, 20.0),
            typo_retry_delay: p(300.0, 20.0),
            typo_recovery_chance: p(98.0, 5.0),
            click_reaction_delay: p(100.0, 20.0),
            click_offset: p(2.0, 30.0),
            scroll_amount: p(450.0, 20.0),
            scroll_smoothness: p(90.0, 10.0),
            scroll_pause: p(800.0, 20.0),
            action_delay_min: p(800.0, 20.0),
            action_delay_variance_pct: p(30.0, 15.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Enthusiast - precise, researched
    pub fn enthusiast() -> Self {
        Self {
            name: "Enthusiast".into(),
            description: "Tech-savvy user - precise, researched".into(),
            cursor_speed: p(0.7, 10.0),
            cursor_step_delay: p(12.0, 20.0),
            cursor_curve_spread: p(40.0, 20.0),
            cursor_precision: p(99.0, 1.0),
            cursor_micro_pause_chance: p(8.0, 30.0),
            cursor_micro_pause_duration: p(80.0, 30.0),
            typing_speed_mean: p(125.0, 10.0),
            typing_speed_stddev: p(25.0, 20.0),
            typo_rate: p(1.0, 40.0),
            typing_word_pause: p(400.0, 25.0),
            typo_notice_delay: p(250.0, 25.0),
            typo_retry_delay: p(150.0, 25.0),
            typo_recovery_chance: p(98.0, 15.0),
            click_reaction_delay: p(40.0, 25.0),
            click_offset: p(3.0, 30.0),
            scroll_amount: p(600.0, 25.0),
            scroll_smoothness: p(80.0, 15.0),
            scroll_pause: p(600.0, 25.0),
            action_delay_min: p(600.0, 25.0),
            action_delay_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Power user - fast, efficient
    pub fn power_user() -> Self {
        Self {
            name: "PowerUser".into(),
            description: "Experienced user - fast, efficient".into(),
            cursor_speed: p(1.1, 15.0),
            cursor_step_delay: p(5.0, 25.0),
            cursor_curve_spread: p(25.0, 30.0),
            cursor_precision: p(97.0, 3.0),
            cursor_micro_pause_chance: p(3.0, 50.0),
            cursor_micro_pause_duration: p(30.0, 50.0),
            typing_speed_mean: p(120.0, 20.0),
            typing_speed_stddev: p(20.0, 30.0),
            typo_rate: p(0.5, 50.0),
            typing_word_pause: p(200.0, 30.0),
            typo_notice_delay: p(150.0, 30.0),
            typo_retry_delay: p(80.0, 30.0),
            typo_recovery_chance: p(60.0, 40.0),
            click_reaction_delay: p(20.0, 30.0),
            click_offset: p(2.0, 40.0),
            scroll_amount: p(1500.0, 30.0),
            scroll_smoothness: p(20.0, 40.0),
            scroll_pause: p(150.0, 30.0),
            action_delay_min: p(200.0, 30.0),
            action_delay_variance_pct: p(30.0, 30.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Cautious - careful, lots of pauses
    pub fn cautious() -> Self {
        Self {
            name: "Cautious".into(),
            description: "Careful user - lots of pauses, verification".into(),
            cursor_speed: p(0.4, 15.0),
            cursor_step_delay: p(27.0, 20.0),
            cursor_curve_spread: p(35.0, 25.0),
            cursor_precision: p(99.5, 0.5),
            cursor_micro_pause_chance: p(25.0, 20.0),
            cursor_micro_pause_duration: p(250.0, 20.0),
            typing_speed_mean: p(190.0, 15.0),
            typing_speed_stddev: p(35.0, 20.0),
            typo_rate: p(0.5, 40.0),
            typing_word_pause: p(700.0, 20.0),
            typo_notice_delay: p(450.0, 20.0),
            typo_retry_delay: p(280.0, 20.0),
            typo_recovery_chance: p(98.0, 10.0),
            click_reaction_delay: p(150.0, 20.0),
            click_offset: p(1.0, 30.0),
            scroll_amount: p(375.0, 25.0),
            scroll_smoothness: p(95.0, 5.0),
            scroll_pause: p(1000.0, 15.0),
            action_delay_min: p(1000.0, 20.0),
            action_delay_variance_pct: p(25.0, 15.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Impatient - quick, minimal pauses
    pub fn impatient() -> Self {
        Self {
            name: "Impatient".into(),
            description: "Quick decision maker - minimal pauses".into(),
            cursor_speed: p(1.2, 10.0),
            cursor_step_delay: p(4.0, 20.0),
            cursor_curve_spread: p(60.0, 25.0),
            cursor_precision: p(80.0, 15.0),
            cursor_micro_pause_chance: p(2.0, 50.0),
            cursor_micro_pause_duration: p(20.0, 50.0),
            typing_speed_mean: p(120.0, 25.0),
            typing_speed_stddev: p(15.0, 40.0),
            typo_rate: p(8.0, 50.0),
            typing_word_pause: p(150.0, 40.0),
            typo_notice_delay: p(120.0, 40.0),
            typo_retry_delay: p(60.0, 40.0),
            typo_recovery_chance: p(96.0, 15.0),
            click_reaction_delay: p(15.0, 40.0),
            click_offset: p(20.0, 40.0),
            scroll_amount: p(1800.0, 35.0),
            scroll_smoothness: p(10.0, 50.0),
            scroll_pause: p(100.0, 40.0),
            action_delay_min: p(100.0, 40.0),
            action_delay_variance_pct: p(20.0, 40.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Erratic - inconsistent timing
    pub fn erratic() -> Self {
        Self {
            name: "Erratic".into(),
            description: "Inconsistent timing and speed".into(),
            cursor_speed: p(0.6, 50.0),
            cursor_step_delay: p(18.0, 60.0),
            cursor_curve_spread: p(70.0, 50.0),
            cursor_precision: p(90.0, 15.0),
            cursor_micro_pause_chance: p(15.0, 60.0),
            cursor_micro_pause_duration: p(120.0, 60.0),
            typing_speed_mean: p(140.0, 50.0),
            typing_speed_stddev: p(50.0, 50.0),
            typo_rate: p(5.0, 80.0),
            typing_word_pause: p(500.0, 60.0),
            typo_notice_delay: p(350.0, 60.0),
            typo_retry_delay: p(250.0, 60.0),
            typo_recovery_chance: p(30.0, 40.0),
            click_reaction_delay: p(60.0, 60.0),
            click_offset: p(10.0, 60.0),
            scroll_amount: p(900.0, 60.0),
            scroll_smoothness: p(50.0, 50.0),
            scroll_pause: p(400.0, 60.0),
            action_delay_min: p(400.0, 60.0),
            action_delay_variance_pct: p(70.0, 40.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Researcher - slow, thorough
    pub fn researcher() -> Self {
        Self {
            name: "Researcher".into(),
            description: "Research-focused - slow, thorough".into(),
            cursor_speed: p(0.3, 15.0),
            cursor_step_delay: p(38.0, 20.0),
            cursor_curve_spread: p(25.0, 25.0),
            cursor_precision: p(99.5, 0.5),
            cursor_micro_pause_chance: p(30.0, 20.0),
            cursor_micro_pause_duration: p(300.0, 20.0),
            typing_speed_mean: p(250.0, 15.0),
            typing_speed_stddev: p(40.0, 20.0),
            typo_rate: p(0.3, 30.0),
            typing_word_pause: p(1000.0, 15.0),
            typo_notice_delay: p(700.0, 15.0),
            typo_retry_delay: p(400.0, 15.0),
            typo_recovery_chance: p(98.0, 2.0),
            click_reaction_delay: p(200.0, 15.0),
            click_offset: p(0.0, 20.0),
            scroll_amount: p(300.0, 20.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(1500.0, 15.0),
            action_delay_min: p(1500.0, 15.0),
            action_delay_variance_pct: p(20.0, 15.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Casual - relaxed browsing
    pub fn casual() -> Self {
        Self {
            name: "Casual".into(),
            description: "Relaxed browsing - slow pace".into(),
            cursor_speed: p(0.5, 15.0),
            cursor_step_delay: p(22.0, 20.0),
            cursor_curve_spread: p(55.0, 20.0),
            cursor_precision: p(92.0, 8.0),
            cursor_micro_pause_chance: p(15.0, 30.0),
            cursor_micro_pause_duration: p(150.0, 30.0),
            typing_speed_mean: p(160.0, 20.0),
            typing_speed_stddev: p(45.0, 25.0),
            typo_rate: p(3.0, 40.0),
            typing_word_pause: p(600.0, 25.0),
            typo_notice_delay: p(400.0, 25.0),
            typo_retry_delay: p(250.0, 25.0),
            typo_recovery_chance: p(90.0, 25.0),
            click_reaction_delay: p(70.0, 30.0),
            click_offset: p(8.0, 35.0),
            scroll_amount: p(600.0, 30.0),
            scroll_smoothness: p(75.0, 20.0),
            scroll_pause: p(700.0, 25.0),
            action_delay_min: p(700.0, 25.0),
            action_delay_variance_pct: p(45.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Professional - efficient, minimal waste
    pub fn professional() -> Self {
        Self {
            name: "Professional".into(),
            description: "Work-focused - efficient, minimal waste".into(),
            cursor_speed: p(1.0, 10.0),
            cursor_step_delay: p(8.0, 15.0),
            cursor_curve_spread: p(30.0, 20.0),
            cursor_precision: p(98.0, 2.0),
            cursor_micro_pause_chance: p(5.0, 30.0),
            cursor_micro_pause_duration: p(50.0, 30.0),
            typing_speed_mean: p(125.0, 15.0),
            typing_speed_stddev: p(20.0, 20.0),
            typo_rate: p(0.8, 40.0),
            typing_word_pause: p(300.0, 20.0),
            typo_notice_delay: p(200.0, 20.0),
            typo_retry_delay: p(120.0, 20.0),
            typo_recovery_chance: p(98.0, 10.0),
            click_reaction_delay: p(30.0, 20.0),
            click_offset: p(3.0, 30.0),
            scroll_amount: p(1350.0, 20.0),
            scroll_smoothness: p(30.0, 30.0),
            scroll_pause: p(300.0, 20.0),
            action_delay_min: p(400.0, 20.0),
            action_delay_variance_pct: p(30.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Novice - slow learning curve
    pub fn novice() -> Self {
        Self {
            name: "Novice".into(),
            description: "Learning user - slow, uncertain".into(),
            cursor_speed: p(0.3, 20.0),
            cursor_step_delay: p(45.0, 25.0),
            cursor_curve_spread: p(60.0, 30.0),
            cursor_precision: p(85.0, 15.0),
            cursor_micro_pause_chance: p(35.0, 25.0),
            cursor_micro_pause_duration: p(350.0, 25.0),
            typing_speed_mean: p(260.0, 20.0),
            typing_speed_stddev: p(60.0, 30.0),
            typo_rate: p(8.0, 40.0),
            typing_word_pause: p(900.0, 25.0),
            typo_notice_delay: p(600.0, 25.0),
            typo_retry_delay: p(400.0, 25.0),
            typo_recovery_chance: p(90.0, 20.0),
            click_reaction_delay: p(250.0, 25.0),
            click_offset: p(25.0, 35.0),
            scroll_amount: p(300.0, 35.0),
            scroll_smoothness: p(85.0, 15.0),
            scroll_pause: p(1200.0, 20.0),
            action_delay_min: p(1200.0, 20.0),
            action_delay_variance_pct: p(30.0, 25.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Expert - fast, precise
    pub fn expert() -> Self {
        Self {
            name: "Expert".into(),
            description: "Skilled user - fast, precise".into(),
            cursor_speed: p(1.1, 8.0),
            cursor_step_delay: p(5.0, 15.0),
            cursor_curve_spread: p(20.0, 20.0),
            cursor_precision: p(99.5, 0.5),
            cursor_micro_pause_chance: p(2.0, 40.0),
            cursor_micro_pause_duration: p(25.0, 40.0),
            typing_speed_mean: p(120.0, 12.0),
            typing_speed_stddev: p(15.0, 20.0),
            typo_rate: p(0.2, 50.0),
            typing_word_pause: p(180.0, 20.0),
            typo_notice_delay: p(100.0, 25.0),
            typo_retry_delay: p(60.0, 25.0),
            typo_recovery_chance: p(98.0, 1.0),
            click_reaction_delay: p(15.0, 25.0),
            click_offset: p(1.0, 30.0),
            scroll_amount: p(1800.0, 15.0),
            scroll_smoothness: p(15.0, 30.0),
            scroll_pause: p(100.0, 25.0),
            action_delay_min: p(150.0, 25.0),
            action_delay_variance_pct: p(25.0, 25.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Distracted - frequent random pauses
    pub fn distracted() -> Self {
        Self {
            name: "Distracted".into(),
            description: "Frequently interrupted - random pauses".into(),
            cursor_speed: p(0.5, 25.0),
            cursor_step_delay: p(18.0, 40.0),
            cursor_curve_spread: p(55.0, 35.0),
            cursor_precision: p(88.0, 12.0),
            cursor_micro_pause_chance: p(40.0, 30.0),
            cursor_micro_pause_duration: p(400.0, 40.0),
            typing_speed_mean: p(140.0, 30.0),
            typing_speed_stddev: p(55.0, 35.0),
            typo_rate: p(5.0, 50.0),
            typing_word_pause: p(600.0, 50.0),
            typo_notice_delay: p(400.0, 50.0),
            typo_retry_delay: p(280.0, 50.0),
            typo_recovery_chance: p(36.0, 30.0),
            click_reaction_delay: p(80.0, 50.0),
            click_offset: p(12.0, 45.0),
            scroll_amount: p(675.0, 45.0),
            scroll_smoothness: p(55.0, 40.0),
            scroll_pause: p(600.0, 50.0),
            action_delay_min: p(600.0, 50.0),
            action_delay_variance_pct: p(80.0, 30.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Focused - consistent, few pauses
    pub fn focused() -> Self {
        Self {
            name: "Focused".into(),
            description: "Concentrated work - consistent, few pauses".into(),
            cursor_speed: p(0.8, 8.0),
            cursor_step_delay: p(10.0, 12.0),
            cursor_curve_spread: p(35.0, 15.0),
            cursor_precision: p(97.0, 3.0),
            cursor_micro_pause_chance: p(3.0, 40.0),
            cursor_micro_pause_duration: p(40.0, 40.0),
            typing_speed_mean: p(120.0, 10.0),
            typing_speed_stddev: p(20.0, 15.0),
            typo_rate: p(0.5, 30.0),
            typing_word_pause: p(250.0, 15.0),
            typo_notice_delay: p(150.0, 15.0),
            typo_retry_delay: p(80.0, 15.0),
            typo_recovery_chance: p(98.0, 8.0),
            click_reaction_delay: p(25.0, 15.0),
            click_offset: p(2.0, 25.0),
            scroll_amount: p(1275.0, 15.0),
            scroll_smoothness: p(35.0, 20.0),
            scroll_pause: p(250.0, 15.0),
            action_delay_min: p(300.0, 15.0),
            action_delay_variance_pct: p(20.0, 15.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Analytical - methodical scrolling
    pub fn analytical() -> Self {
        Self {
            name: "Analytical".into(),
            description: "Data gathering - methodical, even scrolling".into(),
            cursor_speed: p(0.35, 10.0),
            cursor_step_delay: p(33.0, 15.0),
            cursor_curve_spread: p(20.0, 20.0),
            cursor_precision: p(99.0, 1.0),
            cursor_micro_pause_chance: p(22.0, 20.0),
            cursor_micro_pause_duration: p(280.0, 20.0),
            typing_speed_mean: p(220.0, 12.0),
            typing_speed_stddev: p(35.0, 18.0),
            typo_rate: p(0.4, 35.0),
            typing_word_pause: p(900.0, 15.0),
            typo_notice_delay: p(600.0, 15.0),
            typo_retry_delay: p(400.0, 15.0),
            typo_recovery_chance: p(98.0, 3.0),
            click_reaction_delay: p(180.0, 15.0),
            click_offset: p(1.0, 25.0),
            scroll_amount: p(375.0, 10.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(1800.0, 10.0),
            action_delay_min: p(1800.0, 10.0),
            action_delay_variance_pct: p(15.0, 12.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Quick scanner - fast scroll, quick clicks
    pub fn quick_scanner() -> Self {
        Self {
            name: "QuickScanner".into(),
            description: "Speed-focused - fast scrolls, quick decisions".into(),
            cursor_speed: p(1.3, 15.0),
            cursor_step_delay: p(4.0, 25.0),
            cursor_curve_spread: p(70.0, 30.0),
            cursor_precision: p(75.0, 20.0),
            cursor_micro_pause_chance: p(1.0, 60.0),
            cursor_micro_pause_duration: p(15.0, 60.0),
            typing_speed_mean: p(120.0, 30.0),
            typing_speed_stddev: p(12.0, 45.0),
            typo_rate: p(5.0, 40.0),
            typing_word_pause: p(100.0, 50.0),
            typo_notice_delay: p(80.0, 50.0),
            typo_retry_delay: p(40.0, 50.0),
            typo_recovery_chance: p(78.0, 20.0),
            click_reaction_delay: p(10.0, 50.0),
            click_offset: p(30.0, 45.0),
            scroll_amount: p(2250.0, 25.0),
            scroll_smoothness: p(5.0, 60.0),
            scroll_pause: p(80.0, 50.0),
            action_delay_min: p(80.0, 50.0),
            action_delay_variance_pct: p(15.0, 50.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Thorough - slow, complete coverage
    pub fn thorough() -> Self {
        Self {
            name: "Thorough".into(),
            description: "Complete coverage - slow, comprehensive".into(),
            cursor_speed: p(0.25, 12.0),
            cursor_step_delay: p(45.0, 18.0),
            cursor_curve_spread: p(22.0, 22.0),
            cursor_precision: p(99.8, 0.2),
            cursor_micro_pause_chance: p(35.0, 18.0),
            cursor_micro_pause_duration: p(400.0, 18.0),
            typing_speed_mean: p(280.0, 12.0),
            typing_speed_stddev: p(45.0, 18.0),
            typo_rate: p(0.2, 30.0),
            typing_word_pause: p(1200.0, 12.0),
            typo_notice_delay: p(800.0, 12.0),
            typo_retry_delay: p(500.0, 12.0),
            typo_recovery_chance: p(98.0, 0.5),
            click_reaction_delay: p(300.0, 12.0),
            click_offset: p(0.0, 20.0),
            scroll_amount: p(225.0, 15.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(2000.0, 10.0),
            action_delay_min: p(2000.0, 10.0),
            action_delay_variance_pct: p(12.0, 12.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Adaptive - adjusts based on content
    pub fn adaptive() -> Self {
        Self {
            name: "Adaptive".into(),
            description: "Adjusts based on content type".into(),
            cursor_speed: p(0.6, 40.0),
            cursor_step_delay: p(18.0, 50.0),
            cursor_curve_spread: p(50.0, 45.0),
            cursor_precision: p(93.0, 12.0),
            cursor_micro_pause_chance: p(15.0, 50.0),
            cursor_micro_pause_duration: p(150.0, 50.0),
            typing_speed_mean: p(130.0, 40.0),
            typing_speed_stddev: p(45.0, 45.0),
            typo_rate: p(3.0, 70.0),
            typing_word_pause: p(500.0, 50.0),
            typo_notice_delay: p(350.0, 50.0),
            typo_retry_delay: p(220.0, 50.0),
            typo_recovery_chance: p(84.0, 40.0),
            click_reaction_delay: p(60.0, 50.0),
            click_offset: p(8.0, 50.0),
            scroll_amount: p(825.0, 50.0),
            scroll_smoothness: p(60.0, 45.0),
            scroll_pause: p(550.0, 50.0),
            action_delay_min: p(550.0, 50.0),
            action_delay_variance_pct: p(55.0, 40.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Stressed - fast, less accurate
    pub fn stressed() -> Self {
        Self {
            name: "Stressed".into(),
            description: "Time pressure - fast, less accurate".into(),
            cursor_speed: p(1.1, 20.0),
            cursor_step_delay: p(5.0, 30.0),
            cursor_curve_spread: p(65.0, 35.0),
            cursor_precision: p(78.0, 18.0),
            cursor_micro_pause_chance: p(8.0, 50.0),
            cursor_micro_pause_duration: p(35.0, 50.0),
            typing_speed_mean: p(120.0, 28.0),
            typing_speed_stddev: p(18.0, 45.0),
            typo_rate: p(9.0, 55.0),
            typing_word_pause: p(180.0, 45.0),
            typo_notice_delay: p(100.0, 45.0),
            typo_retry_delay: p(50.0, 45.0),
            typo_recovery_chance: p(93.0, 15.0),
            click_reaction_delay: p(18.0, 45.0),
            click_offset: p(22.0, 45.0),
            scroll_amount: p(1650.0, 35.0),
            scroll_smoothness: p(12.0, 55.0),
            scroll_pause: p(130.0, 45.0),
            action_delay_min: p(130.0, 45.0),
            action_delay_variance_pct: p(25.0, 45.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Leisure - slow, exploratory
    pub fn leisure() -> Self {
        Self {
            name: "Leisure".into(),
            description: "Enjoyment-focused - slow, exploratory".into(),
            cursor_speed: p(0.3, 12.0),
            cursor_step_delay: p(33.0, 18.0),
            cursor_curve_spread: p(65.0, 20.0),
            cursor_precision: p(90.0, 10.0),
            cursor_micro_pause_chance: p(25.0, 25.0),
            cursor_micro_pause_duration: p(300.0, 25.0),
            typing_speed_mean: p(210.0, 18.0),
            typing_speed_stddev: p(55.0, 22.0),
            typo_rate: p(2.5, 45.0),
            typing_word_pause: p(800.0, 20.0),
            typo_notice_delay: p(500.0, 20.0),
            typo_retry_delay: p(320.0, 20.0),
            typo_recovery_chance: p(93.0, 22.0),
            click_reaction_delay: p(120.0, 22.0),
            click_offset: p(10.0, 35.0),
            scroll_amount: p(420.0, 28.0),
            scroll_smoothness: p(90.0, 10.0),
            scroll_pause: p(1000.0, 18.0),
            action_delay_min: p(1000.0, 18.0),
            action_delay_variance_pct: p(35.0, 18.0),
            dive_probability: p(0.35, 20.0),
        }
    }
}

/// Helper to create ProfileParam with base and deviation.
fn p(base: f64, deviation_pct: f64) -> ProfileParam {
    ProfileParam::new(base, deviation_pct)
}

/// Creates a randomized profile from a preset for this session.
/// Applies random variation to all parameters based on their deviation percentages.
pub fn randomize_profile(preset: &ProfilePreset) -> BrowserProfile {
    // Note: The ProfileParam::random() is called when using the profile,
    // so the profile itself stores the base values and deviation.
    // This function can be extended if we want to pre-randomize all values.

    BrowserProfile::from_preset(preset)
}

/// Returns a random profile preset.
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
        // Should be within ±10%
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
        assert!(ratio >= 0.0 && ratio <= 1.0);
    }

    #[test]
    fn test_randomize_profile() {
        let profile = randomize_profile(&ProfilePreset::Average);
        assert!(!profile.name.is_empty());
    }

    #[test]
    fn test_random_preset() {
        let preset = random_preset();
        // Just verify it returns a valid preset
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
    fn test_profile_preset_average() {
        let preset = ProfilePreset::Average;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Average");
    }

    #[test]
    fn test_profile_preset_teen() {
        let preset = ProfilePreset::Teen;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Teen");
    }

    #[test]
    fn test_profile_preset_senior() {
        let preset = ProfilePreset::Senior;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Senior");
    }

    #[test]
    fn test_profile_preset_enthusiast() {
        let preset = ProfilePreset::Enthusiast;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Enthusiast");
    }

    #[test]
    fn test_profile_preset_power_user() {
        let preset = ProfilePreset::PowerUser;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "PowerUser");
    }

    #[test]
    fn test_profile_preset_cautious() {
        let preset = ProfilePreset::Cautious;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Cautious");
    }

    #[test]
    fn test_profile_preset_impatient() {
        let preset = ProfilePreset::Impatient;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Impatient");
    }

    #[test]
    fn test_profile_preset_erratic() {
        let preset = ProfilePreset::Erratic;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Erratic");
    }

    #[test]
    fn test_profile_preset_researcher() {
        let preset = ProfilePreset::Researcher;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Researcher");
    }

    #[test]
    fn test_profile_preset_casual() {
        let preset = ProfilePreset::Casual;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Casual");
    }

    #[test]
    fn test_profile_preset_professional() {
        let preset = ProfilePreset::Professional;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Professional");
    }

    #[test]
    fn test_profile_preset_novice() {
        let preset = ProfilePreset::Novice;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Novice");
    }

    #[test]
    fn test_profile_preset_expert() {
        let preset = ProfilePreset::Expert;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Expert");
    }

    #[test]
    fn test_profile_preset_distracted() {
        let preset = ProfilePreset::Distracted;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Distracted");
    }

    #[test]
    fn test_profile_preset_focused() {
        let preset = ProfilePreset::Focused;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Focused");
    }

    #[test]
    fn test_profile_preset_analytical() {
        let preset = ProfilePreset::Analytical;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Analytical");
    }

    #[test]
    fn test_profile_preset_quick_scanner() {
        let preset = ProfilePreset::QuickScanner;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "QuickScanner");
    }

    #[test]
    fn test_profile_preset_thorough() {
        let preset = ProfilePreset::Thorough;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Thorough");
    }

    #[test]
    fn test_profile_preset_adaptive() {
        let preset = ProfilePreset::Adaptive;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Adaptive");
    }

    #[test]
    fn test_profile_preset_stressed() {
        let preset = ProfilePreset::Stressed;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Stressed");
    }

    #[test]
    fn test_profile_preset_leisure() {
        let preset = ProfilePreset::Leisure;
        let profile = BrowserProfile::from_preset(&preset);
        assert_eq!(profile.name, "Leisure");
    }
}
