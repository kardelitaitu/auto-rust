use crate::runtime::task_context::click_learning::{
    sanitize_path_component, ClickAdaptation, ClickElementPriority, ClickFatigueLevel,
    ClickLearningState, ClickPageContext, ClickTimingContext, SelectorLearningStats,
};
use crate::runtime::task_context::types::{
    ClickAndWaitOutcome, FileMetadata, FocusOutcome, FocusStatus, HttpResponse,
    RandomCursorOutcome, Rect, WaitForVisibleStatus,
};
use crate::runtime::task_context::{
    click_learning_path, deserialize_evaluated_json, load_click_learning,
    nativeclick_public_log_line, save_click_learning, validate_session_data_for_tests,
    wrapper_timeout_context,
};
use crate::utils::mouse::CursorMovementConfig;
use crate::utils::ClickOutcome;
use std::collections::HashMap;

#[test]
fn test_wrapper_timeout_context_format() {
    assert_eq!(
        wrapper_timeout_context("wait_for_load", "timeout_ms=3000"),
        "wrapper_timeout | stage=wait_for_load timeout_ms=3000"
    );
}

#[test]
fn test_focus_summary_format() {
    let outcome = FocusOutcome {
        focus: FocusStatus::Success,
        x: 12.3,
        y: 45.6,
    };
    assert_eq!(outcome.summary(), "focus:success (12.3,45.6)");
}

#[test]
fn test_randomcursor_summary_format() {
    let outcome = RandomCursorOutcome {
        x: 10.0,
        y: 20.0,
        movement: CursorMovementConfig {
            speed_multiplier: 1.0,
            min_step_delay_ms: 10,
            max_step_delay_variance_ms: 5,
            curve_spread: 20.0,
            steps: None,
            add_micro_pauses: true,
            path_style: crate::utils::mouse::PathStyle::Bezier,
            precision: crate::utils::mouse::Precision::Safe,
            speed: crate::utils::mouse::Speed::Normal,
        },
    };
    assert_eq!(outcome.summary(), "randomcursor (10.0,20.0) delay:10..15");
}

#[test]
fn test_click_and_wait_summary_format() {
    let outcome = ClickAndWaitOutcome {
        click: ClickOutcome {
            click: crate::utils::mouse::ClickStatus::Success,
            x: 1.0,
            y: 2.0,
            screen_x: None,
            screen_y: None,
        },
        next_selector: ".next".into(),
        next_visible: WaitForVisibleStatus::Visible,
        timeout_ms: 500,
    };
    assert_eq!(
        outcome.summary(),
        "Clicked (1.0,2.0) wait_for:.next visible:visible timeout:500ms"
    );
}

#[test]
fn test_click_and_wait_timeout_summary_format() {
    let outcome = ClickAndWaitOutcome {
        click: ClickOutcome {
            click: crate::utils::mouse::ClickStatus::Success,
            x: 1.0,
            y: 2.0,
            screen_x: None,
            screen_y: None,
        },
        next_selector: ".next".into(),
        next_visible: WaitForVisibleStatus::Timeout,
        timeout_ms: 500,
    };
    assert_eq!(
        outcome.summary(),
        "Clicked (1.0,2.0) wait_for:.next visible:timeout timeout:500ms"
    );
}

#[test]
fn test_nativeclick_public_log_format() {
    let line = nativeclick_public_log_line("#submit", 708.04, 335.19);
    assert_eq!(line, "[task-api] clicked (#submit) at 708.0,335.2");
}

#[test]
fn test_click_context_classification() {
    assert_eq!(
        ClickTimingContext::classify_page("https://x.com/home"),
        ClickPageContext::Social
    );
    assert_eq!(
        ClickTimingContext::classify_priority("button[data-testid='submit']"),
        ClickElementPriority::Critical
    );
    assert_eq!(
        ClickTimingContext::classify_fatigue(80),
        ClickFatigueLevel::Tired
    );
}

#[test]
fn test_learning_adaptation_increases_after_failures() {
    let mut learning = ClickLearningState::default();
    for _ in 0..4 {
        learning.record("button[data-testid='retweet']", false);
    }
    let context = ClickTimingContext::from_observation(
        "https://x.com/home",
        "button[data-testid='retweet']",
        learning.interaction_count,
        learning.recent_success_rate(),
    );
    let adaptation = learning.adaptation_for("button[data-testid='retweet']", &context);
    assert!(adaptation.reaction_delay_multiplier > 1.0);
    assert!(adaptation.extra_stability_wait_ms >= 250);
    assert!(adaptation.require_strict_verification);
}

#[test]
fn test_timing_profile_scales_with_low_success_rate() {
    let context = ClickTimingContext::from_observation(
        "https://x.com/home",
        "button[data-testid='like']",
        60,
        0.55,
    );
    let profile = context.timing_profile(250, 20, 8, &ClickAdaptation::default());
    assert!(profile.reaction_delay_ms >= 250);
    assert!(profile.attention_pause_ms >= 200);
}

#[test]
fn test_learning_window_caps_recent_results() {
    let mut learning = ClickLearningState::default();
    for i in 0..80 {
        learning.record("a[href='/x']", i % 2 == 0);
    }
    assert_eq!(
        learning.recent_results.len(),
        ClickLearningState::RECENT_WINDOW
    );
}

#[test]
fn test_click_learning_persistence_roundtrip() {
    let mut learning = ClickLearningState::default();
    learning.record("button[data-testid='like']", true);
    learning.record("button[data-testid='like']", false);
    learning.record("button[data-testid='retweet']", false);

    let unique = format!(
        "click-learning-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);

    save_click_learning(&path, &learning).expect("save click learning");
    let loaded = load_click_learning(&path).expect("load click learning");

    assert_eq!(loaded.interaction_count, learning.interaction_count);
    assert_eq!(loaded.total_attempts, learning.total_attempts);
    assert_eq!(loaded.total_successes, learning.total_successes);
    assert_eq!(loaded.recent_results.len(), learning.recent_results.len());
    assert_eq!(
        loaded.selector_stats("button[data-testid='like']").attempts,
        2
    );
    assert_eq!(
        loaded
            .selector_stats("button[data-testid='like']")
            .successes,
        1
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_deserialize_evaluated_json_from_string() {
    let value = serde_json::Value::String(r#"{"theme":"dark","fontSize":"14"}"#.to_string());
    let parsed: std::collections::HashMap<String, String> =
        deserialize_evaluated_json(value).expect("deserialize stringified json");

    assert_eq!(parsed.get("theme"), Some(&"dark".to_string()));
    assert_eq!(parsed.get("fontSize"), Some(&"14".to_string()));
}

#[test]
fn test_deserialize_evaluated_json_from_object() {
    let value = serde_json::json!({
        "theme": "dark",
        "fontSize": "14"
    });
    let parsed: std::collections::HashMap<String, String> =
        deserialize_evaluated_json(value).expect("deserialize object json");

    assert_eq!(parsed.get("theme"), Some(&"dark".to_string()));
    assert_eq!(parsed.get("fontSize"), Some(&"14".to_string()));
}

#[test]
fn test_validate_session_data_flags_invalid_payloads() {
    let data = crate::task::policy::SessionData {
        cookies: vec![serde_json::json!({"value": "abc"})],
        local_storage: std::collections::HashMap::new(),
        exported_at: chrono::Utc::now(),
        url: String::new(),
    };

    let warnings = validate_session_data_for_tests(&data);

    assert!(warnings
        .iter()
        .any(|warning| warning == "Cookie[0] missing 'name' field"));
    assert!(warnings
        .iter()
        .any(|warning| warning == "SessionData url is empty"));
    assert!(!warnings
        .iter()
        .any(|warning| warning == "SessionData has no cookies and no localStorage"));
}

#[test]
fn test_click_page_context_variants() {
    assert_eq!(ClickPageContext::Home, ClickPageContext::Home);
    assert_eq!(ClickPageContext::Form, ClickPageContext::Form);
    assert_eq!(ClickPageContext::Social, ClickPageContext::Social);
}

#[test]
fn test_click_element_priority_variants() {
    assert_eq!(
        ClickElementPriority::Critical,
        ClickElementPriority::Critical
    );
    assert_eq!(ClickElementPriority::Normal, ClickElementPriority::Normal);
    assert_eq!(
        ClickElementPriority::Optional,
        ClickElementPriority::Optional
    );
}

#[test]
fn test_click_fatigue_level_variants() {
    assert_eq!(ClickFatigueLevel::Rested, ClickFatigueLevel::Rested);
    assert_eq!(ClickFatigueLevel::Normal, ClickFatigueLevel::Normal);
    assert_eq!(ClickFatigueLevel::Tired, ClickFatigueLevel::Tired);
}

#[test]
fn test_click_adaptation_default() {
    let adaptation = ClickAdaptation::default();
    assert_eq!(adaptation.extra_stability_wait_ms, 0);
    assert_eq!(adaptation.reaction_delay_multiplier, 1.0);
    assert!(!adaptation.require_strict_verification);
}

#[test]
fn test_selector_learning_stats_default() {
    let stats = SelectorLearningStats::default();
    assert_eq!(stats.attempts, 0);
    assert_eq!(stats.successes, 0);
    assert_eq!(stats.consecutive_failures, 0);
}

#[test]
fn test_click_learning_state_default() {
    let state = ClickLearningState::default();
    assert_eq!(state.interaction_count, 0);
    assert_eq!(state.total_attempts, 0);
    assert!(state.recent_results.is_empty());
}

#[test]
fn test_click_learning_state_recent_success_rate_empty() {
    let state = ClickLearningState::default();
    assert_eq!(state.recent_success_rate(), 1.0);
}

#[test]
fn test_click_learning_state_record_success() {
    let mut state = ClickLearningState::default();
    state.record("#button", true);
    assert_eq!(state.total_attempts, 1);
    assert_eq!(state.total_successes, 1);
}

#[test]
fn test_click_learning_state_record_failure() {
    let mut state = ClickLearningState::default();
    state.record("#button", false);
    assert_eq!(state.total_attempts, 1);
    assert_eq!(state.total_successes, 0);
}

#[test]
fn test_click_learning_state_selector_stats() {
    let mut state = ClickLearningState::default();
    state.record("#button", true);
    state.record("#button", false);
    let stats = state.selector_stats("#button");
    assert_eq!(stats.attempts, 2);
    assert_eq!(stats.successes, 1);
}

#[test]
fn test_focus_status_variants() {
    assert_eq!(FocusStatus::Success, FocusStatus::Success);
    assert_eq!(FocusStatus::Failed, FocusStatus::Failed);
}

#[test]
fn test_wait_for_visible_status_variants() {
    assert_eq!(WaitForVisibleStatus::Visible, WaitForVisibleStatus::Visible);
    assert_eq!(WaitForVisibleStatus::Timeout, WaitForVisibleStatus::Timeout);
}

#[test]
fn test_sanitize_path_component_alphanumeric() {
    assert_eq!(sanitize_path_component("test123"), "test123");
}

#[test]
fn test_sanitize_path_component_special_chars() {
    assert_eq!(sanitize_path_component("test@#$"), "test");
}

#[test]
fn test_sanitize_path_component_empty() {
    assert_eq!(sanitize_path_component("@#$"), "default");
}

#[test]
fn test_sanitize_path_component_spaces() {
    assert_eq!(sanitize_path_component("test name"), "test_name");
}

#[test]
fn test_click_timing_context_classify_page_home() {
    assert_eq!(
        ClickTimingContext::classify_page("https://example.com/"),
        ClickPageContext::Home
    );
}

#[test]
fn test_click_timing_context_classify_page_form() {
    assert_eq!(
        ClickTimingContext::classify_page("https://example.com/login"),
        ClickPageContext::Form
    );
}

#[test]
fn test_click_timing_context_classify_priority_normal() {
    assert_eq!(
        ClickTimingContext::classify_priority("button"),
        ClickElementPriority::Normal
    );
}

#[test]
fn test_click_timing_context_classify_priority_optional() {
    assert_eq!(
        ClickTimingContext::classify_priority("button.ad"),
        ClickElementPriority::Optional
    );
}

#[test]
fn test_click_timing_context_classify_fatigue_rested() {
    assert_eq!(
        ClickTimingContext::classify_fatigue(10),
        ClickFatigueLevel::Rested
    );
}

#[test]
fn test_click_timing_context_classify_fatigue_normal() {
    assert_eq!(
        ClickTimingContext::classify_fatigue(30),
        ClickFatigueLevel::Normal
    );
}

#[test]
fn test_click_learning_state_consecutive_failures_tracking() {
    let mut state = ClickLearningState::default();
    state.record("#button", false);
    state.record("#button", false);
    let stats = state.selector_stats("#button");
    assert_eq!(stats.consecutive_failures, 2);
}

#[test]
fn test_click_learning_state_consecutive_failures_reset_on_success() {
    let mut state = ClickLearningState::default();
    state.record("#button", false);
    state.record("#button", false);
    state.record("#button", true);
    let stats = state.selector_stats("#button");
    assert_eq!(stats.consecutive_failures, 0);
}

#[test]
fn test_click_learning_state_multiple_selectors() {
    let mut state = ClickLearningState::default();
    state.record("#button1", true);
    state.record("#button2", false);
    let stats1 = state.selector_stats("#button1");
    let stats2 = state.selector_stats("#button2");
    assert_eq!(stats1.attempts, 1);
    assert_eq!(stats1.successes, 1);
    assert_eq!(stats2.attempts, 1);
    assert_eq!(stats2.successes, 0);
}

#[test]
fn test_click_learning_state_interaction_count() {
    let mut state = ClickLearningState::default();
    state.record("#button", true);
    state.record("#button", false);
    state.record("#link", true);
    assert_eq!(state.interaction_count, 3);
}

#[test]
fn test_click_learning_state_recent_success_rate_mixed() {
    let mut state = ClickLearningState::default();
    for i in 0..10 {
        state.record("#button", i % 2 == 0);
    }
    let rate = state.recent_success_rate();
    assert!((0.0..=1.0).contains(&rate));
}

#[test]
fn test_click_learning_state_recent_success_rate_all_success() {
    let mut state = ClickLearningState::default();
    for _ in 0..10 {
        state.record("#button", true);
    }
    assert_eq!(state.recent_success_rate(), 1.0);
}

#[test]
fn test_click_learning_state_recent_success_rate_all_failure() {
    let mut state = ClickLearningState::default();
    for _ in 0..10 {
        state.record("#button", false);
    }
    assert_eq!(state.recent_success_rate(), 0.0);
}

#[test]
fn test_click_adaptation_with_high_multiplier() {
    let adaptation = ClickAdaptation {
        extra_stability_wait_ms: 500,
        reaction_delay_multiplier: 2.5,
        require_strict_verification: true,
        click_offset_adjustment_px: 0,
        prefer_coordinate_fallback: false,
        reaction_variance_boost_pct: 0,
    };
    assert_eq!(adaptation.extra_stability_wait_ms, 500);
    assert_eq!(adaptation.reaction_delay_multiplier, 2.5);
    assert!(adaptation.require_strict_verification);
}

#[test]
fn test_click_timing_profile_from_observation() {
    let context = ClickTimingContext::from_observation("https://example.com", "#button", 10, 0.8);
    assert_eq!(context.page, ClickPageContext::Home);
    assert_eq!(context.priority, ClickElementPriority::Normal);
    assert_eq!(context.fatigue, ClickFatigueLevel::Rested);
    assert_eq!(context.recent_success_rate, 0.8);
}

#[test]
fn test_click_timing_context_classify_page_commerce() {
    assert_eq!(
        ClickTimingContext::classify_page("https://shop.example.com"),
        ClickPageContext::Commerce
    );
}

#[test]
fn test_click_timing_context_classify_page_content() {
    assert_eq!(
        ClickTimingContext::classify_page("https://blog.example.com/article"),
        ClickPageContext::Content
    );
}

#[test]
fn test_click_timing_context_classify_page_other() {
    // Need a URL with more than 2 path segments to trigger Other
    assert_eq!(
        ClickTimingContext::classify_page("https://unknown.example.com/path/to/page"),
        ClickPageContext::Other
    );
}

#[test]
fn test_click_timing_context_classify_priority_critical_data_testid() {
    assert_eq!(
        ClickTimingContext::classify_priority("[data-testid='submit']"),
        ClickElementPriority::Critical
    );
}

#[test]
fn test_click_timing_context_classify_priority_critical_type_submit() {
    assert_eq!(
        ClickTimingContext::classify_priority("button[type='submit']"),
        ClickElementPriority::Critical
    );
}

#[test]
fn test_click_timing_context_classify_fatigue_boundary_normal() {
    // Boundary is < 50 for Normal, so 49 should be Normal
    assert_eq!(
        ClickTimingContext::classify_fatigue(49),
        ClickFatigueLevel::Normal
    );
}

#[test]
fn test_click_timing_context_classify_fatigue_boundary_tired() {
    assert_eq!(
        ClickTimingContext::classify_fatigue(70),
        ClickFatigueLevel::Tired
    );
}

#[test]
fn test_click_timing_context_classify_fatigue_boundary_rested() {
    // Boundary is < 15 for Rested, so 14 should be Rested
    assert_eq!(
        ClickTimingContext::classify_fatigue(14),
        ClickFatigueLevel::Rested
    );
}

#[test]
fn test_sanitize_path_component_unicode() {
    assert_eq!(sanitize_path_component("test🎉"), "test");
}

#[test]
fn test_sanitize_path_component_underscores() {
    assert_eq!(sanitize_path_component("test_name"), "test_name");
}

#[test]
fn test_sanitize_path_component_dashes() {
    assert_eq!(sanitize_path_component("test-name"), "test-name");
}

#[test]
fn test_click_learning_state_selector_stats_nonexistent() {
    let state = ClickLearningState::default();
    let stats = state.selector_stats("#nonexistent");
    assert_eq!(stats.attempts, 0);
    assert_eq!(stats.successes, 0);
}

// ============================================================================
// Click Retry Behavior Tests
// ============================================================================

#[test]
fn test_click_retry_backoff_calculation() {
    // Verify backoff_ms formula: (150 + (attempt * 180)) clamped 100-1000
    let calculate_backoff = |attempt: u64| -> u64 {
        let adaptation_wait = 0u64; // No extra stability wait for test
        (150 + (attempt * 180))
            .saturating_add(adaptation_wait / 2)
            .clamp(100, 1_000)
    };

    // Attempt 1: 150 + 180 = 330ms
    assert_eq!(calculate_backoff(1), 330);

    // Attempt 2: 150 + 360 = 510ms
    assert_eq!(calculate_backoff(2), 510);

    // Attempt 3: 150 + 540 = 690ms
    assert_eq!(calculate_backoff(3), 690);

    // Attempt 4: 150 + 720 = 870ms
    assert_eq!(calculate_backoff(4), 870);

    // Attempt 5: 150 + 900 = 1050ms → clamped to 1000ms
    assert_eq!(calculate_backoff(5), 1_000);

    // Attempt 10: 150 + 1800 = 1950ms → clamped to 1000ms
    assert_eq!(calculate_backoff(10), 1_000);
}

#[test]
fn test_click_retry_attempt_delay_progression() {
    // Verify attempt_delay increases 18% per attempt
    // Formula: base_ms * (1.0 + ((attempt - 1) * 0.18))
    let base_ms = 1000u64;

    let calculate_delay = |attempt: u32| -> u64 {
        (base_ms as f64 * (1.0 + ((attempt.saturating_sub(1)) as f64 * 0.18))).round() as u64
    };

    // Attempt 1: 1000 * 1.0 = 1000ms
    assert_eq!(calculate_delay(1), 1000);

    // Attempt 2: 1000 * 1.18 = 1180ms
    assert_eq!(calculate_delay(2), 1180);

    // Attempt 3: 1000 * 1.36 = 1360ms
    assert_eq!(calculate_delay(3), 1360);

    // Verify progression is multiplicative not additive
    let delay_1 = calculate_delay(1);
    let delay_2 = calculate_delay(2);
    let delay_3 = calculate_delay(3);

    // Each attempt adds 18% of base, not 18% of previous
    assert_eq!(delay_2 - delay_1, 180); // 18% of 1000
    assert_eq!(delay_3 - delay_2, 180); // 18% of 1000 (not 18% of 1180)
}

#[test]
fn test_screenshot_filename_format() {
    // Test filename generation matches expected format
    let session_id = "test-session-123";
    let now = chrono::Utc::now();
    let filename = format!(
        "{}-{}-{}.jpg",
        now.format("%Y-%m-%d"),
        now.format("%H-%M"),
        session_id
    );

    // Verify format: yyyy-mm-dd-hh-mm-sessionid.jpg
    assert!(filename.ends_with(".jpg"));
    assert!(filename.contains("test-session-123"));
    assert!(filename.len() > 20); // Reasonable length for timestamp + session
}

#[test]
fn test_screenshot_directory_path() {
    let screenshot_dir = std::path::Path::new("data/screenshot");
    assert_eq!(
        screenshot_dir.to_str().expect("Invalid path"),
        "data/screenshot"
    );
}

// ============================================================================
// Browser Management Tests
// ============================================================================

#[test]
fn test_browser_data_default() {
    let data = crate::task::policy::BrowserData::default();
    assert!(data.cookies.is_empty());
    assert!(data.local_storage.is_empty());
    assert!(data.session_storage.is_empty());
    assert!(data.indexeddb_names.is_empty());
    assert!(data.source.is_empty());
    assert!(data.browser_version.is_none());
}

#[test]
fn test_browser_data_serialization_roundtrip() {
    use chrono::Utc;
    use std::collections::HashMap;

    let mut local_storage = HashMap::new();
    let mut origin_data = HashMap::new();
    origin_data.insert("key1".to_string(), "value1".to_string());
    origin_data.insert("key2".to_string(), "value2".to_string());
    local_storage.insert("example.com".to_string(), origin_data);

    let mut indexeddb = HashMap::new();
    indexeddb.insert(
        "example.com".to_string(),
        vec!["db1".to_string(), "db2".to_string()],
    );

    let data = crate::task::policy::BrowserData {
        cookies: vec![serde_json::json!({"name": "test", "value": "cookie"})],
        local_storage,
        session_storage: HashMap::new(),
        indexeddb_names: indexeddb,
        exported_at: Utc::now(),
        source: "https://example.com".to_string(),
        browser_version: Some("Chrome 120".to_string()),
    };

    // Serialize
    let json = serde_json::to_string(&data).expect("Should serialize");

    // Deserialize
    let restored: crate::task::policy::BrowserData =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(restored.cookies.len(), 1);
    assert_eq!(restored.source, "https://example.com");
    assert_eq!(restored.browser_version, Some("Chrome 120".to_string()));
    assert_eq!(restored.local_storage.len(), 1);
    assert!(restored.local_storage.contains_key("example.com"));
}

#[test]
fn test_permissions_include_browser_export_import() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_browser_export);
    assert!(!perms.allow_browser_import);

    // Test with custom permissions
    let custom_policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_browser_export: true,
            allow_browser_import: true,
            ..Default::default()
        },
    };

    assert!(custom_policy.permissions.allow_browser_export);
    assert!(custom_policy.permissions.allow_browser_import);
}

#[test]
fn test_file_metadata_struct() {
    let metadata = FileMetadata {
        size: 1024,
        modified: std::time::SystemTime::UNIX_EPOCH,
        created: std::time::SystemTime::UNIX_EPOCH,
    };

    assert_eq!(metadata.size, 1024);

    // Test serialization
    let json = serde_json::to_string(&metadata).expect("Should serialize");
    assert!(json.contains("1024"));
}

#[test]
fn test_http_response_struct() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let response = HttpResponse {
        status: 200,
        body: "{\"success\": true}".to_string(),
        headers,
    };

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "{\"success\": true}");
    assert_eq!(
        response.headers.get("Content-Type"),
        Some(&"application/json".to_string())
    );

    // Test serialization
    let json = serde_json::to_string(&response).expect("Should serialize");
    assert!(json.contains("200"));
    assert!(json.contains("success"));
}

#[test]
fn test_rect_struct() {
    let rect = Rect {
        x: 10.5,
        y: 20.5,
        width: 100.0,
        height: 50.0,
    };

    assert_eq!(rect.x, 10.5);
    assert_eq!(rect.y, 20.5);
    assert_eq!(rect.width, 100.0);
    assert_eq!(rect.height, 50.0);

    // Test serialization roundtrip
    let json = serde_json::to_string(&rect).expect("Should serialize");
    let restored: Rect = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(restored.x, 10.5);
    assert_eq!(restored.width, 100.0);
}

#[test]
fn test_click_learning_persistence_with_real_file() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("click_learning.json");

    // Create state and save
    let mut state = ClickLearningState::default();
    state.record("#button1", true);
    state.record("#button1", true);
    state.record("#button2", false);

    save_click_learning(&path, &state).expect("Should save");
    assert!(path.exists());

    // Load and verify
    let loaded = load_click_learning(&path).expect("Should load");
    assert_eq!(loaded.total_attempts, 3);
    assert_eq!(loaded.total_successes, 2);

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn test_sanitize_path_component_various_inputs() {
    assert_eq!(sanitize_path_component("normal"), "normal");
    assert_eq!(sanitize_path_component("with-dash"), "with-dash");
    assert_eq!(
        sanitize_path_component("with_underscore"),
        "with_underscore"
    );
    assert_eq!(sanitize_path_component("UPPERCASE"), "UPPERCASE");
    assert_eq!(sanitize_path_component("123"), "123");
    assert_eq!(sanitize_path_component(""), "default");
    assert_eq!(sanitize_path_component("   "), "default");
    assert_eq!(sanitize_path_component("a"), "a");
}

#[test]
fn test_click_timing_profile_edge_cases() {
    let context = ClickTimingContext {
        page: ClickPageContext::Other,
        priority: ClickElementPriority::Critical,
        fatigue: ClickFatigueLevel::Tired,
        recent_success_rate: 0.0,
    };

    let profile = context.timing_profile(200, 15, 5, &ClickAdaptation::default());
    assert!(profile.reaction_delay_ms >= 150); // Increased due to fatigue
    assert!(profile.primary_timeout_ms >= 4_000);
}

#[test]
fn test_click_adaptation_with_extreme_failures() {
    let mut learning = ClickLearningState::default();

    // Simulate many failures
    for _ in 0..20 {
        learning.record("#button", false);
    }

    let context = ClickTimingContext::from_observation(
        "https://example.com",
        "#button",
        20,  // interaction_count
        0.0, // recent_success_rate (all failures)
    );
    let adaptation = learning.adaptation_for("#button", &context);

    // Should require strict verification after many failures
    assert!(adaptation.require_strict_verification);
    assert!(adaptation.prefer_coordinate_fallback);
}

// ============================================================================
// API v0.0.3 Permission Denial Tests
// ============================================================================

#[test]
fn test_cookie_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_export_cookies);
    assert!(!perms.allow_import_cookies);
}

#[test]
fn test_session_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_export_session);
    assert!(!perms.allow_import_session);
}

#[test]
fn test_clipboard_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_session_clipboard);
}

#[test]
fn test_data_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_read_data);
    assert!(!perms.allow_write_data);
}

#[test]
fn test_http_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_http_requests);
}

#[test]
fn test_dom_inspection_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_dom_inspection);
}

#[test]
fn test_browser_permissions_default_false() {
    let perms = crate::task::policy::TaskPermissions::default();
    assert!(!perms.allow_browser_export);
    assert!(!perms.allow_browser_import);
}

// ============================================================================
// API v0.0.3 Check Permission Tests
// ============================================================================

#[test]
fn test_check_permission_cookie_export() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_export_cookies: true,
            ..Default::default()
        },
    };
    let static_policy = Box::leak(Box::new(policy));

    // We can't easily test check_permission without a TaskContext,
    // but we can verify the permission struct works
    assert!(static_policy.permissions.allow_export_cookies);
    assert!(!static_policy.permissions.allow_import_cookies);
}

#[test]
fn test_check_permission_session_import() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_import_session: true,
            ..Default::default()
        },
    };
    let static_policy = Box::leak(Box::new(policy));

    assert!(static_policy.permissions.allow_import_session);
    assert!(!static_policy.permissions.allow_export_session);
}

#[test]
fn test_check_permission_data_read_write() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_read_data: true,
            allow_write_data: true,
            ..Default::default()
        },
    };
    let static_policy = Box::leak(Box::new(policy));

    assert!(static_policy.permissions.allow_read_data);
    assert!(static_policy.permissions.allow_write_data);
}

// ============================================================================
// API v0.0.3 Data Structures Tests
// ============================================================================

#[test]
fn test_session_data_empty_initialization() {
    use std::collections::HashMap;
    let data = crate::task::policy::SessionData {
        cookies: vec![],
        local_storage: HashMap::new(),
        exported_at: chrono::Utc::now(),
        url: String::new(),
    };
    assert!(data.cookies.is_empty());
    assert!(data.local_storage.is_empty());
    assert!(data.url.is_empty());
}

#[test]
fn test_session_data_serialization() {
    use std::collections::HashMap;

    let mut local_storage = HashMap::new();
    local_storage.insert("key".to_string(), "value".to_string());

    let data = crate::task::policy::SessionData {
        cookies: vec![serde_json::json!({"name": "test"})],
        local_storage,
        exported_at: chrono::Utc::now(),
        url: "https://example.com".to_string(),
    };

    let json = serde_json::to_string(&data).expect("Should serialize");
    assert!(json.contains("example.com"));
    assert!(json.contains("test"));
}

#[test]
fn test_http_response_error_display() {
    let response = HttpResponse {
        status: 404,
        body: "Not Found".to_string(),
        headers: std::collections::HashMap::new(),
    };

    assert_eq!(response.status, 404);
    assert_eq!(response.body, "Not Found");
}

#[test]
fn test_http_response_success_status() {
    let response = HttpResponse {
        status: 200,
        body: "OK".to_string(),
        headers: std::collections::HashMap::new(),
    };

    assert!(response.status >= 200 && response.status < 300);
}

#[test]
fn test_rect_zero_values() {
    let rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    assert_eq!(rect.x, 0.0);
    assert_eq!(rect.y, 0.0);
    assert_eq!(rect.width, 0.0);
    assert_eq!(rect.height, 0.0);
}

#[test]
fn test_rect_negative_values() {
    // Rect can theoretically have negative x/y (off-screen elements)
    let rect = Rect {
        x: -100.0,
        y: -50.0,
        width: 200.0,
        height: 100.0,
    };

    assert_eq!(rect.x, -100.0);
    assert_eq!(rect.y, -50.0);
    assert_eq!(rect.width, 200.0);
    assert_eq!(rect.height, 100.0);
}

#[test]
fn test_file_metadata_large_file() {
    let metadata = FileMetadata {
        size: 1024 * 1024 * 100, // 100 MB
        modified: std::time::SystemTime::UNIX_EPOCH,
        created: std::time::SystemTime::UNIX_EPOCH,
    };

    assert_eq!(metadata.size, 104_857_600);
}

#[test]
fn test_file_metadata_empty_file() {
    let metadata = FileMetadata {
        size: 0,
        modified: std::time::SystemTime::UNIX_EPOCH,
        created: std::time::SystemTime::UNIX_EPOCH,
    };

    assert_eq!(metadata.size, 0);
}

// ============================================================================
// API v0.0.3 Policy Integration Tests
// ============================================================================

#[test]
fn test_default_task_policy_all_permissions_false() {
    let policy = crate::task::policy::DEFAULT_TASK_POLICY;

    assert!(!policy.permissions.allow_screenshot);
    assert!(!policy.permissions.allow_export_cookies);
    assert!(!policy.permissions.allow_import_cookies);
    assert!(!policy.permissions.allow_export_session);
    assert!(!policy.permissions.allow_import_session);
    assert!(!policy.permissions.allow_session_clipboard);
    assert!(!policy.permissions.allow_read_data);
    assert!(!policy.permissions.allow_write_data);
    assert!(!policy.permissions.allow_http_requests);
    assert!(!policy.permissions.allow_dom_inspection);
    assert!(!policy.permissions.allow_browser_export);
    assert!(!policy.permissions.allow_browser_import);
}

#[test]
fn test_twitter_policy_has_required_permissions() {
    use crate::task::policy::TWITTERACTIVITY_POLICY;

    assert!(TWITTERACTIVITY_POLICY.permissions.allow_export_cookies);
    assert!(TWITTERACTIVITY_POLICY.permissions.allow_session_clipboard);
    assert!(TWITTERACTIVITY_POLICY.permissions.allow_read_data);
    assert!(TWITTERACTIVITY_POLICY.permissions.allow_screenshot);
    // allow_write_data is implied by allow_screenshot
}

#[test]
fn test_cookiebot_policy_has_required_permissions() {
    use crate::task::policy::COOKIEBOT_POLICY;

    assert!(COOKIEBOT_POLICY.permissions.allow_export_cookies);
    assert!(COOKIEBOT_POLICY.permissions.allow_screenshot);
}

#[test]
fn test_pageview_policy_has_default_permissions() {
    use crate::task::policy::PAGEVIEW_POLICY;

    assert!(!PAGEVIEW_POLICY.permissions.allow_screenshot);
    assert!(!PAGEVIEW_POLICY.permissions.allow_export_cookies);
    assert!(!PAGEVIEW_POLICY.permissions.allow_http_requests);
}

// ============================================================================
// API v0.0.3 BrowserData Advanced Tests
// ============================================================================

#[test]
fn test_browser_data_with_multiple_origins() {
    use chrono::Utc;
    use std::collections::HashMap;

    let mut local_storage = HashMap::new();
    let mut origin1 = HashMap::new();
    origin1.insert("key1".to_string(), "value1".to_string());
    let mut origin2 = HashMap::new();
    origin2.insert("key2".to_string(), "value2".to_string());

    local_storage.insert("example.com".to_string(), origin1);
    local_storage.insert("api.example.com".to_string(), origin2);

    let data = crate::task::policy::BrowserData {
        cookies: vec![],
        local_storage,
        session_storage: HashMap::new(),
        indexeddb_names: HashMap::new(),
        exported_at: Utc::now(),
        source: "test".to_string(),
        browser_version: None,
    };

    assert_eq!(data.local_storage.len(), 2);
    assert!(data.local_storage.contains_key("example.com"));
    assert!(data.local_storage.contains_key("api.example.com"));
}

#[test]
fn test_browser_data_with_indexeddb() {
    use chrono::Utc;
    use std::collections::HashMap;

    let mut indexeddb = HashMap::new();
    indexeddb.insert(
        "example.com".to_string(),
        vec!["my-database".to_string(), "cache-store".to_string()],
    );

    let data = crate::task::policy::BrowserData {
        cookies: vec![],
        local_storage: HashMap::new(),
        session_storage: HashMap::new(),
        indexeddb_names: indexeddb,
        exported_at: Utc::now(),
        source: "test".to_string(),
        browser_version: Some("Chrome 120".to_string()),
    };

    assert_eq!(data.indexeddb_names.len(), 1);
    let dbs = data
        .indexeddb_names
        .get("example.com")
        .expect("example.com should exist");
    assert_eq!(dbs.len(), 2);
    assert!(dbs.contains(&"my-database".to_string()));
}

#[test]
fn test_browser_data_empty_is_valid() {
    let data = crate::task::policy::BrowserData::default();

    // Empty browser data should be valid for import/export
    assert!(data.cookies.is_empty());
    assert!(data.local_storage.is_empty());
    assert!(data.session_storage.is_empty());
    assert!(data.indexeddb_names.is_empty());
}

// ============================================================================
// API v0.0.3 Helper Function Tests
// ============================================================================

#[test]
fn test_sanitize_path_component_with_special_chars() {
    assert_eq!(sanitize_path_component("test/file"), "test_file");
    assert_eq!(sanitize_path_component("test..file"), "test__file");
    assert_eq!(sanitize_path_component("test\\file"), "test_file"); // Single \ becomes single _
}

#[test]
fn test_sanitize_path_component_unicode_extended() {
    // Unicode chars become underscores, then trimmed, empty becomes "default"
    assert_eq!(sanitize_path_component("测试"), "default"); // All unicode -> "__" -> trim -> "default"
                                                            // Mixed content: ascii parts preserved, unicode becomes underscores
    assert_eq!(sanitize_path_component("test日本語file"), "test___file"); // 3 Japanese chars = 3 underscores
    assert_eq!(sanitize_path_component("日本語test"), "test"); // Leading underscores trimmed
    assert_eq!(sanitize_path_component("test日本語"), "test"); // Trailing underscores trimmed
}

#[test]
fn test_sanitize_path_component_long_name() {
    let long_name = "a".repeat(300);
    let result = sanitize_path_component(&long_name);
    // Should not panic and should preserve the name (or truncate)
    assert!(!result.is_empty());
}

#[test]
fn test_click_learning_path_generation() {
    use crate::utils::profile::ProfilePreset;
    use crate::utils::randomize_profile;

    let profile = randomize_profile(&ProfilePreset::Average);
    let path = click_learning_path("session-123", &profile);
    assert!(path.is_some());

    let path = path.expect("Path should exist");
    let path_str = path.to_str().expect("Invalid path");
    assert!(path_str.contains("click-learning"));
    assert!(path_str.contains("session-123"));
    assert!(path_str.contains(&profile.name));
}

#[test]
fn test_click_learning_save_and_load_roundtrip() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("test_learning.json");

    // Create and save
    let mut state = ClickLearningState::default();
    for i in 0..5 {
        state.record(&format!("#button{}", i), i % 2 == 0);
    }

    save_click_learning(&path, &state).expect("Should save");

    // Load
    let loaded = load_click_learning(&path).expect("Should load");
    assert_eq!(loaded.total_attempts, 5);
    assert_eq!(loaded.total_successes, 3); // 0, 2, 4 are even (success)

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn test_click_learning_empty_save() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("empty_learning.json");

    let state = ClickLearningState::default();
    save_click_learning(&path, &state).expect("Should save empty state");

    let loaded = load_click_learning(&path).expect("Should load empty state");
    assert_eq!(loaded.total_attempts, 0);
    assert_eq!(loaded.total_successes, 0);

    let _ = fs::remove_file(&path);
}

// ============================================================================
// API v0.0.3 Error Handling Tests
// ============================================================================

#[test]
fn test_error_permission_denied_format() {
    let err = crate::error::TaskError::PermissionDenied {
        permission: "allow_test",           // &'static str
        task_name: "test-task".to_string(), // String
    };

    let msg = format!("{}", err);
    assert!(msg.contains("allow_test"));
    assert!(msg.contains("test-task"));
}

#[test]
fn test_error_invalid_path_format() {
    let err = crate::error::TaskError::InvalidPath("Invalid chars: ../test".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid chars"));
}

// ============================================================================
// API v0.0.3 Data Validation Tests
// ============================================================================

#[test]
fn test_browser_data_version_compatibility() {
    use chrono::Utc;
    use std::collections::HashMap;

    // Simulate older version without browser_version field
    let data = crate::task::policy::BrowserData {
        cookies: vec![],
        local_storage: HashMap::new(),
        session_storage: HashMap::new(),
        indexeddb_names: HashMap::new(),
        exported_at: Utc::now(),
        source: "legacy".to_string(),
        browser_version: None, // Older export may not have this
    };

    // Should serialize with null browser_version
    let json = serde_json::to_string(&data).expect("Should serialize");
    assert!(json.contains("null") || json.contains("browser_version"));
}

#[test]
fn test_session_data_url_validation() {
    use std::collections::HashMap;

    // Valid URLs
    let data1 = crate::task::policy::SessionData {
        url: "https://example.com/path?query=1".to_string(),
        cookies: vec![],
        local_storage: HashMap::new(),
        exported_at: chrono::Utc::now(),
    };
    assert!(!data1.url.is_empty());

    // Empty URL should be allowed (for validation testing)
    let data2 = crate::task::policy::SessionData {
        url: "".to_string(),
        cookies: vec![],
        local_storage: HashMap::new(),
        exported_at: chrono::Utc::now(),
    };
    assert!(data2.url.is_empty());
}

#[test]
fn test_http_response_with_headers() {
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let response = HttpResponse {
        status: 200,
        body: "{}".to_string(),
        headers,
    };

    assert_eq!(response.headers.len(), 3);
    assert!(response.headers.contains_key("Content-Type"));
    assert!(response.headers.contains_key("Authorization"));
    assert!(response.headers.contains_key("X-Custom-Header"));
}

// ============================================================================
// API v0.0.3 Permission Combination Tests
// ============================================================================

#[test]
fn test_permission_combinations_full_access() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(60_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_screenshot: true,
            allow_export_cookies: true,
            allow_import_cookies: true,
            allow_export_session: true,
            allow_import_session: true,
            allow_session_clipboard: true,
            allow_read_data: true,
            allow_write_data: true,
            allow_http_requests: true,
            allow_dom_inspection: true,
            allow_browser_export: true,
            allow_browser_import: true,
        },
    };

    assert!(policy.permissions.allow_screenshot);
    assert!(policy.permissions.allow_browser_export);
    assert!(policy.permissions.allow_browser_import);
}

#[test]
fn test_permission_combinations_read_only() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_read_data: true,
            allow_export_cookies: true,
            allow_export_session: true,
            allow_browser_export: true,
            allow_dom_inspection: true,
            allow_screenshot: true,
            ..Default::default()
        },
    };

    // Read operations allowed
    assert!(policy.permissions.allow_read_data);
    assert!(policy.permissions.allow_export_cookies);
    assert!(policy.permissions.allow_browser_export);
    assert!(policy.permissions.allow_dom_inspection);

    // Write operations denied
    assert!(!policy.permissions.allow_write_data);
    assert!(!policy.permissions.allow_import_cookies);
    assert!(!policy.permissions.allow_browser_import);
}

#[test]
fn test_permission_combinations_network_only() {
    let policy = crate::task::policy::TaskPolicy {
        max_duration_ms: crate::session::DurationMs::new_const(30_000),
        permissions: crate::task::policy::TaskPermissions {
            allow_http_requests: true,
            allow_read_data: true, // For response caching
            ..Default::default()
        },
    };

    assert!(policy.permissions.allow_http_requests);
    assert!(policy.permissions.allow_read_data);
    assert!(!policy.permissions.allow_write_data);
    assert!(!policy.permissions.allow_export_cookies);
}
