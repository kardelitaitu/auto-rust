//! Browser profile preset constructors.
//!
//! Contains all 21 preset constructors for `BrowserProfile` and the `from_preset()`
//! dispatcher. Extracted from `mod.rs` to split the large profile file.

use super::BrowserProfile;
use super::ProfileParam;

/// Helper to create `ProfileParam` with base and deviation.
fn p(base: f64, deviation_pct: f64) -> ProfileParam {
    ProfileParam::new(base, deviation_pct)
}

impl BrowserProfile {
    /// Creates a profile from a preset.
    #[must_use]
    pub fn from_preset(preset: &super::ProfilePreset) -> Self {
        match preset {
            super::ProfilePreset::Average => Self::average(),
            super::ProfilePreset::Teen => Self::teen(),
            super::ProfilePreset::Senior => Self::senior(),
            super::ProfilePreset::Enthusiast => Self::enthusiast(),
            super::ProfilePreset::PowerUser => Self::power_user(),
            super::ProfilePreset::Cautious => Self::cautious(),
            super::ProfilePreset::Impatient => Self::impatient(),
            super::ProfilePreset::Erratic => Self::erratic(),
            super::ProfilePreset::Researcher => Self::researcher(),
            super::ProfilePreset::Casual => Self::casual(),
            super::ProfilePreset::Professional => Self::professional(),
            super::ProfilePreset::Novice => Self::novice(),
            super::ProfilePreset::Expert => Self::expert(),
            super::ProfilePreset::Distracted => Self::distracted(),
            super::ProfilePreset::Focused => Self::focused(),
            super::ProfilePreset::Analytical => Self::analytical(),
            super::ProfilePreset::QuickScanner => Self::quick_scanner(),
            super::ProfilePreset::Thorough => Self::thorough(),
            super::ProfilePreset::Adaptive => Self::adaptive(),
            super::ProfilePreset::Stressed => Self::stressed(),
            super::ProfilePreset::Leisure => Self::leisure(),
        }
    }

    /// Average user - typical everyday browsing
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Teen - fast, less precise
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Senior - slower, more deliberate
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Enthusiast - precise, researched
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Power user - fast, efficient
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Cautious - careful, lots of pauses
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Impatient - quick, minimal pauses
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Erratic - inconsistent timing
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Researcher - slow, thorough
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Casual - relaxed browsing
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Professional - efficient, minimal waste
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Novice - slow learning curve
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Expert - fast, precise
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Distracted - frequent random pauses
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Focused - consistent, few pauses
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Analytical - methodical scrolling
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Quick scanner - fast scroll, quick clicks
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Thorough - slow, complete coverage
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Adaptive - adjusts based on content
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Stressed - fast, less accurate
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }

    /// Leisure - slow, exploratory
    #[must_use]
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
            behavior_variance_pct: p(40.0, 20.0),
            dive_probability: p(0.35, 20.0),
        }
    }
}
