//! Browser behavioral profile system.
//!
//! Provides configurable profiles for human-like browser automation with
//! randomized per-session variations. Profiles control cursor movement,
//! typing speed, clicking behavior, scrolling, and timing delays.
//!
//! # Usage
//! ```
//! use crate::utils::profile::{BrowserProfile, ProfilePreset, randomize_profile};
//!
//! // Get a preset and randomize it for this session
//! let profile = randomize_profile(&ProfilePreset::Teen);
//!
//! // Use with cursor movement
//! use crate::utils::mouse::cursor_move_to_with_profile(page, x, y, &profile).await;
//! ```

use rand::Rng;
use serde::{Deserialize, Serialize};

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
            cursor_speed: p(1.0, 10.0),
            cursor_step_delay: p(10.0, 20.0),
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
            typo_recovery_chance: p(80.0, 20.0),
            click_reaction_delay: p(50.0, 30.0),
            click_offset: p(5.0, 40.0),
            scroll_amount: p(500.0, 30.0),
            scroll_smoothness: p(70.0, 20.0),
            scroll_pause: p(500.0, 30.0),
            action_delay_min: p(500.0, 30.0),
            action_delay_variance_pct: p(50.0, 20.0),
        }
    }

    /// Teen - fast, less precise
    pub fn teen() -> Self {
        Self {
            name: "Teen".into(),
            description: "Young user - fast, less precise".into(),
            cursor_speed: p(1.5, 20.0),
            cursor_step_delay: p(5.0, 30.0),
            cursor_curve_spread: p(80.0, 30.0),
            cursor_precision: p(85.0, 10.0),
            cursor_micro_pause_chance: p(5.0, 40.0),
            cursor_micro_pause_duration: p(50.0, 40.0),
            typing_speed_mean: p(80.0, 30.0),
            typing_speed_stddev: p(30.0, 40.0),
            typo_rate: p(5.0, 50.0),
            typing_word_pause: p(300.0, 40.0),
            typo_notice_delay: p(200.0, 40.0),
            typo_retry_delay: p(100.0, 40.0),
            typo_recovery_chance: p(60.0, 30.0),
            click_reaction_delay: p(30.0, 40.0),
            click_offset: p(15.0, 40.0),
            scroll_amount: p(800.0, 40.0),
            scroll_smoothness: p(40.0, 30.0),
            scroll_pause: p(200.0, 40.0),
            action_delay_min: p(300.0, 40.0),
            action_delay_variance_pct: p(60.0, 30.0),
        }
    }

    /// Senior - slower, more deliberate
    pub fn senior() -> Self {
        Self {
            name: "Senior".into(),
            description: "Older user - slower, more deliberate".into(),
            cursor_speed: p(0.6, 10.0),
            cursor_step_delay: p(20.0, 15.0),
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
            typo_recovery_chance: p(95.0, 5.0),
            click_reaction_delay: p(100.0, 20.0),
            click_offset: p(2.0, 30.0),
            scroll_amount: p(300.0, 20.0),
            scroll_smoothness: p(90.0, 10.0),
            scroll_pause: p(800.0, 20.0),
            action_delay_min: p(800.0, 20.0),
            action_delay_variance_pct: p(30.0, 15.0),
        }
    }

    /// Enthusiast - precise, researched
    pub fn enthusiast() -> Self {
        Self {
            name: "Enthusiast".into(),
            description: "Tech-savvy user - precise, researched".into(),
            cursor_speed: p(1.2, 10.0),
            cursor_step_delay: p(8.0, 20.0),
            cursor_curve_spread: p(40.0, 20.0),
            cursor_precision: p(99.0, 1.0),
            cursor_micro_pause_chance: p(8.0, 30.0),
            cursor_micro_pause_duration: p(80.0, 30.0),
            typing_speed_mean: p(100.0, 10.0),
            typing_speed_stddev: p(25.0, 20.0),
            typo_rate: p(1.0, 40.0),
            typing_word_pause: p(400.0, 25.0),
            typo_notice_delay: p(250.0, 25.0),
            typo_retry_delay: p(150.0, 25.0),
            typo_recovery_chance: p(85.0, 15.0),
            click_reaction_delay: p(40.0, 25.0),
            click_offset: p(3.0, 30.0),
            scroll_amount: p(400.0, 25.0),
            scroll_smoothness: p(80.0, 15.0),
            scroll_pause: p(600.0, 25.0),
            action_delay_min: p(600.0, 25.0),
            action_delay_variance_pct: p(40.0, 20.0),
        }
    }

    /// Power user - fast, efficient
    pub fn power_user() -> Self {
        Self {
            name: "PowerUser".into(),
            description: "Experienced user - fast, efficient".into(),
            cursor_speed: p(1.8, 15.0),
            cursor_step_delay: p(3.0, 25.0),
            cursor_curve_spread: p(25.0, 30.0),
            cursor_precision: p(97.0, 3.0),
            cursor_micro_pause_chance: p(3.0, 50.0),
            cursor_micro_pause_duration: p(30.0, 50.0),
            typing_speed_mean: p(70.0, 20.0),
            typing_speed_stddev: p(20.0, 30.0),
            typo_rate: p(0.5, 50.0),
            typing_word_pause: p(200.0, 30.0),
            typo_notice_delay: p(150.0, 30.0),
            typo_retry_delay: p(80.0, 30.0),
            typo_recovery_chance: p(50.0, 40.0),
            click_reaction_delay: p(20.0, 30.0),
            click_offset: p(2.0, 40.0),
            scroll_amount: p(1000.0, 30.0),
            scroll_smoothness: p(20.0, 40.0),
            scroll_pause: p(150.0, 30.0),
            action_delay_min: p(200.0, 30.0),
            action_delay_variance_pct: p(30.0, 30.0),
        }
    }

    /// Cautious - careful, lots of pauses
    pub fn cautious() -> Self {
        Self {
            name: "Cautious".into(),
            description: "Careful user - lots of pauses, verification".into(),
            cursor_speed: p(0.7, 15.0),
            cursor_step_delay: p(18.0, 20.0),
            cursor_curve_spread: p(35.0, 25.0),
            cursor_precision: p(99.5, 0.5),
            cursor_micro_pause_chance: p(25.0, 20.0),
            cursor_micro_pause_duration: p(250.0, 20.0),
            typing_speed_mean: p(180.0, 15.0),
            typing_speed_stddev: p(35.0, 20.0),
            typo_rate: p(0.5, 40.0),
            typing_word_pause: p(700.0, 20.0),
            typo_notice_delay: p(450.0, 20.0),
            typo_retry_delay: p(280.0, 20.0),
            typo_recovery_chance: p(90.0, 10.0),
            click_reaction_delay: p(150.0, 20.0),
            click_offset: p(1.0, 30.0),
            scroll_amount: p(250.0, 25.0),
            scroll_smoothness: p(95.0, 5.0),
            scroll_pause: p(1000.0, 15.0),
            action_delay_min: p(1000.0, 20.0),
            action_delay_variance_pct: p(25.0, 15.0),
        }
    }

    /// Impatient - quick, minimal pauses
    pub fn impatient() -> Self {
        Self {
            name: "Impatient".into(),
            description: "Quick decision maker - minimal pauses".into(),
            cursor_speed: p(2.0, 10.0),
            cursor_step_delay: p(2.0, 20.0),
            cursor_curve_spread: p(60.0, 25.0),
            cursor_precision: p(80.0, 15.0),
            cursor_micro_pause_chance: p(2.0, 50.0),
            cursor_micro_pause_duration: p(20.0, 50.0),
            typing_speed_mean: p(60.0, 25.0),
            typing_speed_stddev: p(15.0, 40.0),
            typo_rate: p(8.0, 50.0),
            typing_word_pause: p(150.0, 40.0),
            typo_notice_delay: p(120.0, 40.0),
            typo_retry_delay: p(60.0, 40.0),
            typo_recovery_chance: p(80.0, 15.0),
            click_reaction_delay: p(15.0, 40.0),
            click_offset: p(20.0, 40.0),
            scroll_amount: p(1200.0, 35.0),
            scroll_smoothness: p(10.0, 50.0),
            scroll_pause: p(100.0, 40.0),
            action_delay_min: p(100.0, 40.0),
            action_delay_variance_pct: p(20.0, 40.0),
        }
    }

    /// Erratic - inconsistent timing
    pub fn erratic() -> Self {
        Self {
            name: "Erratic".into(),
            description: "Inconsistent timing and speed".into(),
            cursor_speed: p(1.0, 50.0),
            cursor_step_delay: p(12.0, 60.0),
            cursor_curve_spread: p(70.0, 50.0),
            cursor_precision: p(90.0, 15.0),
            cursor_micro_pause_chance: p(15.0, 60.0),
            cursor_micro_pause_duration: p(120.0, 60.0),
            typing_speed_mean: p(120.0, 50.0),
            typing_speed_stddev: p(50.0, 50.0),
            typo_rate: p(5.0, 80.0),
            typing_word_pause: p(500.0, 60.0),
            typo_notice_delay: p(350.0, 60.0),
            typo_retry_delay: p(250.0, 60.0),
            typo_recovery_chance: p(25.0, 40.0),
            click_reaction_delay: p(60.0, 60.0),
            click_offset: p(10.0, 60.0),
            scroll_amount: p(600.0, 60.0),
            scroll_smoothness: p(50.0, 50.0),
            scroll_pause: p(400.0, 60.0),
            action_delay_min: p(400.0, 60.0),
            action_delay_variance_pct: p(70.0, 40.0),
        }
    }

    /// Researcher - slow, thorough
    pub fn researcher() -> Self {
        Self {
            name: "Researcher".into(),
            description: "Research-focused - slow, thorough".into(),
            cursor_speed: p(0.5, 15.0),
            cursor_step_delay: p(25.0, 20.0),
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
            scroll_amount: p(200.0, 20.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(1500.0, 15.0),
            action_delay_min: p(1500.0, 15.0),
            action_delay_variance_pct: p(20.0, 15.0),
        }
    }

    /// Casual - relaxed browsing
    pub fn casual() -> Self {
        Self {
            name: "Casual".into(),
            description: "Relaxed browsing - slow pace".into(),
            cursor_speed: p(0.8, 15.0),
            cursor_step_delay: p(15.0, 20.0),
            cursor_curve_spread: p(55.0, 20.0),
            cursor_precision: p(92.0, 8.0),
            cursor_micro_pause_chance: p(15.0, 30.0),
            cursor_micro_pause_duration: p(150.0, 30.0),
            typing_speed_mean: p(150.0, 20.0),
            typing_speed_stddev: p(45.0, 25.0),
            typo_rate: p(3.0, 40.0),
            typing_word_pause: p(600.0, 25.0),
            typo_notice_delay: p(400.0, 25.0),
            typo_retry_delay: p(250.0, 25.0),
            typo_recovery_chance: p(75.0, 25.0),
            click_reaction_delay: p(70.0, 30.0),
            click_offset: p(8.0, 35.0),
            scroll_amount: p(400.0, 30.0),
            scroll_smoothness: p(75.0, 20.0),
            scroll_pause: p(700.0, 25.0),
            action_delay_min: p(700.0, 25.0),
            action_delay_variance_pct: p(45.0, 20.0),
        }
    }

    /// Professional - efficient, minimal waste
    pub fn professional() -> Self {
        Self {
            name: "Professional".into(),
            description: "Work-focused - efficient, minimal waste".into(),
            cursor_speed: p(1.6, 10.0),
            cursor_step_delay: p(5.0, 15.0),
            cursor_curve_spread: p(30.0, 20.0),
            cursor_precision: p(98.0, 2.0),
            cursor_micro_pause_chance: p(5.0, 30.0),
            cursor_micro_pause_duration: p(50.0, 30.0),
            typing_speed_mean: p(80.0, 15.0),
            typing_speed_stddev: p(20.0, 20.0),
            typo_rate: p(0.8, 40.0),
            typing_word_pause: p(300.0, 20.0),
            typo_notice_delay: p(200.0, 20.0),
            typo_retry_delay: p(120.0, 20.0),
            typo_recovery_chance: p(90.0, 10.0),
            click_reaction_delay: p(30.0, 20.0),
            click_offset: p(3.0, 30.0),
            scroll_amount: p(900.0, 20.0),
            scroll_smoothness: p(30.0, 30.0),
            scroll_pause: p(300.0, 20.0),
            action_delay_min: p(400.0, 20.0),
            action_delay_variance_pct: p(30.0, 20.0),
        }
    }

    /// Novice - slow learning curve
    pub fn novice() -> Self {
        Self {
            name: "Novice".into(),
            description: "Learning user - slow, uncertain".into(),
            cursor_speed: p(0.5, 20.0),
            cursor_step_delay: p(30.0, 25.0),
            cursor_curve_spread: p(60.0, 30.0),
            cursor_precision: p(85.0, 15.0),
            cursor_micro_pause_chance: p(35.0, 25.0),
            cursor_micro_pause_duration: p(350.0, 25.0),
            typing_speed_mean: p(250.0, 20.0),
            typing_speed_stddev: p(60.0, 30.0),
            typo_rate: p(8.0, 40.0),
            typing_word_pause: p(900.0, 25.0),
            typo_notice_delay: p(600.0, 25.0),
            typo_retry_delay: p(400.0, 25.0),
            typo_recovery_chance: p(75.0, 20.0),
            click_reaction_delay: p(250.0, 25.0),
            click_offset: p(25.0, 35.0),
            scroll_amount: p(200.0, 35.0),
            scroll_smoothness: p(85.0, 15.0),
            scroll_pause: p(1200.0, 20.0),
            action_delay_min: p(1200.0, 20.0),
            action_delay_variance_pct: p(30.0, 25.0),
        }
    }

    /// Expert - fast, precise
    pub fn expert() -> Self {
        Self {
            name: "Expert".into(),
            description: "Skilled user - fast, precise".into(),
            cursor_speed: p(1.9, 8.0),
            cursor_step_delay: p(3.0, 15.0),
            cursor_curve_spread: p(20.0, 20.0),
            cursor_precision: p(99.5, 0.5),
            cursor_micro_pause_chance: p(2.0, 40.0),
            cursor_micro_pause_duration: p(25.0, 40.0),
            typing_speed_mean: p(60.0, 12.0),
            typing_speed_stddev: p(15.0, 20.0),
            typo_rate: p(0.2, 50.0),
            typing_word_pause: p(180.0, 20.0),
            typo_notice_delay: p(100.0, 25.0),
            typo_retry_delay: p(60.0, 25.0),
            typo_recovery_chance: p(99.0, 1.0),
            click_reaction_delay: p(15.0, 25.0),
            click_offset: p(1.0, 30.0),
            scroll_amount: p(1200.0, 15.0),
            scroll_smoothness: p(15.0, 30.0),
            scroll_pause: p(100.0, 25.0),
            action_delay_min: p(150.0, 25.0),
            action_delay_variance_pct: p(25.0, 25.0),
        }
    }

    /// Distracted - frequent random pauses
    pub fn distracted() -> Self {
        Self {
            name: "Distracted".into(),
            description: "Frequently interrupted - random pauses".into(),
            cursor_speed: p(0.9, 25.0),
            cursor_step_delay: p(12.0, 40.0),
            cursor_curve_spread: p(55.0, 35.0),
            cursor_precision: p(88.0, 12.0),
            cursor_micro_pause_chance: p(40.0, 30.0),
            cursor_micro_pause_duration: p(400.0, 40.0),
            typing_speed_mean: p(130.0, 30.0),
            typing_speed_stddev: p(55.0, 35.0),
            typo_rate: p(5.0, 50.0),
            typing_word_pause: p(600.0, 50.0),
            typo_notice_delay: p(400.0, 50.0),
            typo_retry_delay: p(280.0, 50.0),
            typo_recovery_chance: p(30.0, 30.0),
            click_reaction_delay: p(80.0, 50.0),
            click_offset: p(12.0, 45.0),
            scroll_amount: p(450.0, 45.0),
            scroll_smoothness: p(55.0, 40.0),
            scroll_pause: p(600.0, 50.0),
            action_delay_min: p(600.0, 50.0),
            action_delay_variance_pct: p(80.0, 30.0),
        }
    }

    /// Focused - consistent, few pauses
    pub fn focused() -> Self {
        Self {
            name: "Focused".into(),
            description: "Concentrated work - consistent, few pauses".into(),
            cursor_speed: p(1.4, 8.0),
            cursor_step_delay: p(7.0, 12.0),
            cursor_curve_spread: p(35.0, 15.0),
            cursor_precision: p(97.0, 3.0),
            cursor_micro_pause_chance: p(3.0, 40.0),
            cursor_micro_pause_duration: p(40.0, 40.0),
            typing_speed_mean: p(90.0, 10.0),
            typing_speed_stddev: p(20.0, 15.0),
            typo_rate: p(0.5, 30.0),
            typing_word_pause: p(250.0, 15.0),
            typo_notice_delay: p(150.0, 15.0),
            typo_retry_delay: p(80.0, 15.0),
            typo_recovery_chance: p(92.0, 8.0),
            click_reaction_delay: p(25.0, 15.0),
            click_offset: p(2.0, 25.0),
            scroll_amount: p(850.0, 15.0),
            scroll_smoothness: p(35.0, 20.0),
            scroll_pause: p(250.0, 15.0),
            action_delay_min: p(300.0, 15.0),
            action_delay_variance_pct: p(20.0, 15.0),
        }
    }

    /// Analytical - methodical scrolling
    pub fn analytical() -> Self {
        Self {
            name: "Analytical".into(),
            description: "Data gathering - methodical, even scrolling".into(),
            cursor_speed: p(0.6, 10.0),
            cursor_step_delay: p(22.0, 15.0),
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
            typo_recovery_chance: p(97.0, 3.0),
            click_reaction_delay: p(180.0, 15.0),
            click_offset: p(1.0, 25.0),
            scroll_amount: p(250.0, 10.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(1800.0, 10.0),
            action_delay_min: p(1800.0, 10.0),
            action_delay_variance_pct: p(15.0, 12.0),
        }
    }

    /// Quick scanner - fast scroll, quick clicks
    pub fn quick_scanner() -> Self {
        Self {
            name: "QuickScanner".into(),
            description: "Speed-focused - fast scrolls, quick decisions".into(),
            cursor_speed: p(2.2, 15.0),
            cursor_step_delay: p(2.0, 25.0),
            cursor_curve_spread: p(70.0, 30.0),
            cursor_precision: p(75.0, 20.0),
            cursor_micro_pause_chance: p(1.0, 60.0),
            cursor_micro_pause_duration: p(15.0, 60.0),
            typing_speed_mean: p(50.0, 30.0),
            typing_speed_stddev: p(12.0, 45.0),
            typo_rate: p(5.0, 40.0),
            typing_word_pause: p(100.0, 50.0),
            typo_notice_delay: p(80.0, 50.0),
            typo_retry_delay: p(40.0, 50.0),
            typo_recovery_chance: p(65.0, 20.0),
            click_reaction_delay: p(10.0, 50.0),
            click_offset: p(30.0, 45.0),
            scroll_amount: p(1500.0, 25.0),
            scroll_smoothness: p(5.0, 60.0),
            scroll_pause: p(80.0, 50.0),
            action_delay_min: p(80.0, 50.0),
            action_delay_variance_pct: p(15.0, 50.0),
        }
    }

    /// Thorough - slow, complete coverage
    pub fn thorough() -> Self {
        Self {
            name: "Thorough".into(),
            description: "Complete coverage - slow, comprehensive".into(),
            cursor_speed: p(0.4, 12.0),
            cursor_step_delay: p(30.0, 18.0),
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
            typo_recovery_chance: p(99.5, 0.5),
            click_reaction_delay: p(300.0, 12.0),
            click_offset: p(0.0, 20.0),
            scroll_amount: p(150.0, 15.0),
            scroll_smoothness: p(100.0, 0.0),
            scroll_pause: p(2000.0, 10.0),
            action_delay_min: p(2000.0, 10.0),
            action_delay_variance_pct: p(12.0, 12.0),
        }
    }

    /// Adaptive - adjusts based on content
    pub fn adaptive() -> Self {
        Self {
            name: "Adaptive".into(),
            description: "Adjusts based on content type".into(),
            cursor_speed: p(1.0, 40.0),
            cursor_step_delay: p(12.0, 50.0),
            cursor_curve_spread: p(50.0, 45.0),
            cursor_precision: p(93.0, 12.0),
            cursor_micro_pause_chance: p(15.0, 50.0),
            cursor_micro_pause_duration: p(150.0, 50.0),
            typing_speed_mean: p(120.0, 40.0),
            typing_speed_stddev: p(45.0, 45.0),
            typo_rate: p(3.0, 70.0),
            typing_word_pause: p(500.0, 50.0),
            typo_notice_delay: p(350.0, 50.0),
            typo_retry_delay: p(220.0, 50.0),
            typo_recovery_chance: p(70.0, 40.0),
            click_reaction_delay: p(60.0, 50.0),
            click_offset: p(8.0, 50.0),
            scroll_amount: p(550.0, 50.0),
            scroll_smoothness: p(60.0, 45.0),
            scroll_pause: p(550.0, 50.0),
            action_delay_min: p(550.0, 50.0),
            action_delay_variance_pct: p(55.0, 40.0),
        }
    }

    /// Stressed - fast, less accurate
    pub fn stressed() -> Self {
        Self {
            name: "Stressed".into(),
            description: "Time pressure - fast, less accurate".into(),
            cursor_speed: p(1.8, 20.0),
            cursor_step_delay: p(3.0, 30.0),
            cursor_curve_spread: p(65.0, 35.0),
            cursor_precision: p(78.0, 18.0),
            cursor_micro_pause_chance: p(8.0, 50.0),
            cursor_micro_pause_duration: p(35.0, 50.0),
            typing_speed_mean: p(65.0, 28.0),
            typing_speed_stddev: p(18.0, 45.0),
            typo_rate: p(9.0, 55.0),
            typing_word_pause: p(180.0, 45.0),
            typo_notice_delay: p(100.0, 45.0),
            typo_retry_delay: p(50.0, 45.0),
            typo_recovery_chance: p(78.0, 15.0),
            click_reaction_delay: p(18.0, 45.0),
            click_offset: p(22.0, 45.0),
            scroll_amount: p(1100.0, 35.0),
            scroll_smoothness: p(12.0, 55.0),
            scroll_pause: p(130.0, 45.0),
            action_delay_min: p(130.0, 45.0),
            action_delay_variance_pct: p(25.0, 45.0),
        }
    }

    /// Leisure - slow, exploratory
    pub fn leisure() -> Self {
        Self {
            name: "Leisure".into(),
            description: "Enjoyment-focused - slow, exploratory".into(),
            cursor_speed: p(0.55, 12.0),
            cursor_step_delay: p(22.0, 18.0),
            cursor_curve_spread: p(65.0, 20.0),
            cursor_precision: p(90.0, 10.0),
            cursor_micro_pause_chance: p(25.0, 25.0),
            cursor_micro_pause_duration: p(300.0, 25.0),
            typing_speed_mean: p(200.0, 18.0),
            typing_speed_stddev: p(55.0, 22.0),
            typo_rate: p(2.5, 45.0),
            typing_word_pause: p(800.0, 20.0),
            typo_notice_delay: p(500.0, 20.0),
            typo_retry_delay: p(320.0, 20.0),
            typo_recovery_chance: p(78.0, 22.0),
            click_reaction_delay: p(120.0, 22.0),
            click_offset: p(10.0, 35.0),
            scroll_amount: p(280.0, 28.0),
            scroll_smoothness: p(90.0, 10.0),
            scroll_pause: p(1000.0, 18.0),
            action_delay_min: p(1000.0, 18.0),
            action_delay_variance_pct: p(35.0, 18.0),
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
    let profile = BrowserProfile::from_preset(preset);

    // Note: The ProfileParam::random() is called when using the profile,
    // so the profile itself stores the base values and deviation.
    // This function can be extended if we want to pre-randomize all values.

    profile
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
            assert!(val >= 90.0 && val <= 110.0, "Value {} out of range", val);
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
}
