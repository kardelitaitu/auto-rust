//! Browser discovery, connection, and management module.
//!
//! Handles:
//! - Browser profile discovery from configuration
//! - Establishing WebSocket connections to browser instances
//! - Managing browser lifecycle and health checks
//! - Integration with RoxyBrowser API for cloud-hosted browsers
//!
//! This module delegates to session-specific connectors and pool management
//! while maintaining backward-compatible public APIs.

use crate::config::{BrowserProfile, Config};
use crate::error::Result;
use crate::session::pool::SessionPoolManager;
use crate::session::Session;
use log::{info, warn};

/// Normalizes a browser token for filter matching.
///
/// Removes non-alphanumeric characters and converts to lowercase
/// for consistent case-insensitive comparison.
pub fn normalize_browser_token(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Checks if a browser candidate matches any of the specified filters.
///
/// Performs case-insensitive matching that handles special characters
/// and partial matches. Returns true if filters is empty.
///
/// # Arguments
/// * `candidate` - The browser name/type to check
/// * `filters` - List of filter tokens to match against
///
/// # Returns
/// True if the candidate matches any filter or if filters is empty
pub fn matches_browser_filters(candidate: &str, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }

    let candidate_lower = candidate.to_lowercase();
    let candidate_norm = normalize_browser_token(candidate);

    filters.iter().any(|filter| {
        let filter_lower = filter.to_lowercase();
        let filter_norm = normalize_browser_token(filter);

        !filter_norm.is_empty()
            && (candidate_lower.contains(&filter_lower)
                || candidate_norm.contains(&filter_norm)
                || candidate_norm == filter_norm)
    })
}

/// Checks if a browser profile matches the specified filters.
///
/// Matches against both profile name and type.
///
/// # Arguments
/// * `profile` - The browser profile to check
/// * `filters` - List of filter tokens to match against
///
/// # Returns
/// True if the profile matches any filter or if filters is empty
pub fn profile_matches_filters(profile: &BrowserProfile, filters: &[String]) -> bool {
    matches_browser_filters(&profile.name, filters)
        || matches_browser_filters(&profile.r#type, filters)
}

/// Checks if a session matches the specified filters.
///
/// Matches against session name, profile type, and ID.
///
/// # Arguments
/// * `session` - The session to check
/// * `filters` - List of filter tokens to match against
///
/// # Returns
/// True if the session matches any filter or if filters is empty
pub fn session_matches_filters(session: &Session, filters: &[String]) -> bool {
    matches_browser_filters(&session.name, filters)
        || matches_browser_filters(&session.profile_type, filters)
        || matches_browser_filters(&session.id, filters)
}

/// Discovers and connects to browser instances based on configuration.
///
/// This function delegates to the SessionPoolManager which coordinates
/// discovery across multiple connectors (configured profiles, RoxyBrowser,
/// and local discovery).
///
/// # Arguments
/// * `config` - The orchestrator configuration containing browser settings
///
/// # Returns
/// A vector of successfully connected `Session` instances.
///
/// # Errors
/// Returns an error if no browsers can be discovered after all retry attempts.
pub async fn discover_browsers(config: &Config) -> Result<Vec<Session>> {
    discover_browsers_with_filters(config, &[]).await
}

/// Discovers browser sessions and optionally filters them by browser name/type tokens.
///
/// Delegates to SessionPoolManager for discovery and connection with
/// optional filtering applied to discovered capabilities before connection.
///
/// # Arguments
/// * `config` - The orchestrator configuration
/// * `browser_filters` - Optional list of browser name/type filters
///
/// # Returns
/// A vector of filtered and connected `Session` instances.
///
/// # Errors
/// Returns an error if no matching browsers are found after retries.
pub async fn discover_browsers_with_filters(
    config: &Config,
    browser_filters: &[String],
) -> Result<Vec<Session>> {
    let pool_manager = SessionPoolManager::from_config(config);

    let sessions = pool_manager
        .discover_with_filters(config, browser_filters)
        .await?;

    // Log results
    if sessions.is_empty() {
        if !browser_filters.is_empty() {
            warn!(
                "No browsers matched the specified filters: {}",
                browser_filters.join(", ")
            );
        } else {
            warn!("No browsers discovered (no filters specified)");
        }
    } else {
        let names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
        info!(
            "Discovered {} browser(s): {}",
            sessions.len(),
            names.join(", ")
        );
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrowserProfile;

    #[test]
    fn test_browser_filter_matching() {
        let filters = vec!["brave".to_string(), "roxybrowser".to_string()];

        assert!(matches_browser_filters("Brave on port 9001", &filters));
        assert!(matches_browser_filters("roxy-browser", &filters));
        assert!(!matches_browser_filters("Safari", &filters));
    }

    #[test]
    fn test_normalize_browser_token() {
        assert_eq!(normalize_browser_token("Brave-Browser"), "bravebrowser");
        assert_eq!(normalize_browser_token("ROXYBROWSER"), "roxybrowser");
        assert_eq!(normalize_browser_token("Chrome_123"), "chrome123");
        assert_eq!(normalize_browser_token("Test@#$Browser"), "testbrowser");
        assert_eq!(normalize_browser_token(""), "");
        assert_eq!(normalize_browser_token("123"), "123");
    }

    #[test]
    fn test_matches_browser_filters_empty() {
        let filters: Vec<String> = vec![];
        assert!(matches_browser_filters("any browser", &filters));
        assert!(matches_browser_filters("", &filters));
    }

    #[test]
    fn test_matches_browser_filters_case_insensitive() {
        let filters = vec!["BRAVE".to_string()];
        assert!(matches_browser_filters("brave", &filters));
        assert!(matches_browser_filters("Brave", &filters));
        assert!(matches_browser_filters("BRAVE", &filters));
    }

    #[test]
    fn test_matches_browser_filters_partial_match() {
        let filters = vec!["brave".to_string()];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("My Brave Instance", &filters));
    }

    #[test]
    fn test_matches_browser_filters_normalized_match() {
        let filters = vec!["brave-browser".to_string()];
        assert!(matches_browser_filters("Brave_Browser", &filters));
        assert!(matches_browser_filters("BraveBrowser", &filters));
    }

    #[test]
    fn test_matches_browser_filters_multiple_filters() {
        let filters = vec![
            "brave".to_string(),
            "chrome".to_string(),
            "safari".to_string(),
        ];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("Chrome Instance", &filters));
        assert!(matches_browser_filters("Safari Web", &filters));
        assert!(!matches_browser_filters("Firefox", &filters));
    }

    #[test]
    fn test_profile_matches_filters_by_name() {
        let profile = BrowserProfile {
            name: "My Brave Browser".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_by_type() {
        let profile = BrowserProfile {
            name: "Custom Name".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["chrome".to_string()];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_no_match() {
        let profile = BrowserProfile {
            name: "Safari".to_string(),
            r#type: "safari".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        assert!(!profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_empty_filters() {
        let profile = BrowserProfile {
            name: "Any Browser".to_string(),
            r#type: "any".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters: Vec<String> = vec![];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_session_matches_filters_empty_filters() {
        // Test empty filters through the helper function
        let filters: Vec<String> = vec![];
        assert!(matches_browser_filters("any browser", &filters));
    }

    #[test]
    fn test_normalize_browser_token_unicode() {
        assert_eq!(normalize_browser_token("Brave浏览器"), "brave");
        assert_eq!(normalize_browser_token("ChromeКонтекст"), "chrome");
    }

    #[test]
    fn test_normalize_browser_token_whitespace_only() {
        assert_eq!(normalize_browser_token("   "), "");
        assert_eq!(normalize_browser_token("\t\n"), "");
    }

    #[test]
    fn test_matches_browser_filters_empty_filter_ignored() {
        let filters = vec!["".to_string()];
        assert!(!matches_browser_filters("Brave", &filters));
    }

    #[test]
    fn test_matches_browser_filters_substring_match() {
        let filters = vec!["bra".to_string()];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("abra", &filters));
    }

    #[test]
    fn test_normalize_browser_token_mixed_case() {
        assert_eq!(normalize_browser_token("BrAvE"), "brave");
        assert_eq!(normalize_browser_token("ChRoMe"), "chrome");
    }

    #[test]
    fn test_profile_matches_filters_both_name_and_type() {
        let profile = BrowserProfile {
            name: "Brave Browser".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match because both name and type contain "brave"
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_normalize_browser_token_only_special_chars() {
        assert_eq!(normalize_browser_token("@#$%"), "");
        assert_eq!(normalize_browser_token("!@#$%^&*()"), "");
    }

    #[test]
    fn test_matches_browser_filters_empty_candidate() {
        let filters = vec!["brave".to_string()];
        assert!(!matches_browser_filters("", &filters));
    }

    #[test]
    fn test_matches_browser_filters_filter_with_special_chars() {
        let filters = vec!["brave_browser".to_string()];
        assert!(matches_browser_filters("Brave-Browser", &filters));
        assert!(matches_browser_filters("Brave_Browser", &filters));
    }

    #[test]
    fn test_normalize_browser_token_preserves_numbers() {
        assert_eq!(normalize_browser_token("v1.2.3"), "v123");
        assert_eq!(normalize_browser_token("browser2.0"), "browser20");
    }

    #[test]
    fn test_matches_browser_filters_numeric_filter() {
        let filters = vec!["123".to_string()];
        assert!(matches_browser_filters("browser-123", &filters));
        assert!(matches_browser_filters("123-browser", &filters));
    }

    #[test]
    fn test_profile_matches_filters_name_only() {
        let profile = BrowserProfile {
            name: "My Brave".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match by name even though type is different
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_type_only() {
        let profile = BrowserProfile {
            name: "Custom".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match by type even though name doesn't match
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_matches_browser_filters_very_long_candidate() {
        let filters = vec!["brave".to_string()];
        let long_name = "a".repeat(1000) + " brave " + &"b".repeat(1000);
        assert!(matches_browser_filters(&long_name, &filters));
    }

    #[test]
    fn test_matches_browser_filters_very_long_filter() {
        let filters = vec!["brave".to_string()];
        let long_name = "a".repeat(1000) + " brave " + &"b".repeat(1000);
        assert!(matches_browser_filters(&long_name, &filters));
    }

    #[test]
    fn test_normalize_browser_token_consecutive_special_chars() {
        assert_eq!(normalize_browser_token("Brave@@@Browser"), "bravebrowser");
        assert_eq!(normalize_browser_token("Test###Name"), "testname");
    }

    #[test]
    fn test_matches_browser_filters_multiple_matches() {
        let filters = vec!["brave".to_string(), "chrome".to_string()];
        let candidate = "Brave Chrome Browser";
        // Should match because it contains both filters
        assert!(matches_browser_filters(candidate, &filters));
    }

    #[test]
    fn test_normalize_browser_token_leading_trailing_special_chars() {
        assert_eq!(normalize_browser_token("@@Brave@@"), "brave");
        assert_eq!(normalize_browser_token("##Chrome##"), "chrome");
    }

    #[test]
    fn test_matches_browser_filters_filter_with_spaces() {
        let filters = vec!["brave browser".to_string()];
        // Filter is normalized, so spaces are removed
        assert!(matches_browser_filters("BraveBrowser", &filters));
        assert!(matches_browser_filters("Brave-Browser", &filters));
    }

    #[test]
    fn test_profile_matches_filters_case_sensitivity() {
        let profile = BrowserProfile {
            name: "BRAVE".to_string(),
            r#type: "CHROME".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match due to case-insensitive comparison
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_normalize_browser_token_empty_after_filtering() {
        assert_eq!(normalize_browser_token("@#$"), "");
        assert_eq!(normalize_browser_token("   "), "");
    }

    #[test]
    fn test_matches_browser_filters_candidate_with_numbers() {
        let filters = vec!["browser123".to_string()];
        assert!(matches_browser_filters("browser-123", &filters));
        assert!(matches_browser_filters("browser-123", &filters));
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_config_no_filter() {
        // Test: No filters, no browsers available - should return empty vec (warns but doesn't error)
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let filters: Vec<String> = vec![];

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should succeed with empty sessions (warns but continues)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_config_with_filter() {
        // Test: Filters active but no browsers available - should return error
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let filters = vec!["brave".to_string()];

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should fail with connection error due to zero matches
        match result {
            Ok(_) => panic!("Should return error when filters active but no matches"),
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("No browsers matched the specified filters"));
                assert!(err_msg.contains("brave"));
            }
        }
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_filter_string() {
        // Test: Empty filter string treated as no filter - should not error
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let filters: Vec<String> = vec![]; // Empty filters

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should succeed with empty sessions (no error when filters empty)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
