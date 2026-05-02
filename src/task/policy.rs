//! Task policy definitions and registry.
//!
//! Provides the `TaskPolicy` and `TaskPermissions` structs for controlling
//! what capabilities a task may use, plus a registry mapping task names to policies.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

/// Task execution policy - one hard limit + 8 permissions.
///
/// The only hard requirement is `max_duration_ms` - tasks cannot hang forever.
/// Eight boolean permissions control what the task is allowed to do.
#[derive(Debug, Clone)]
pub struct TaskPolicy {
    /// Maximum execution time in milliseconds (MANDATORY).
    /// Task will be killed after this duration.
    pub max_duration_ms: u64,

    /// Permission flags - what this task is allowed to do.
    pub permissions: TaskPermissions,
}

impl TaskPolicy {
    /// Get effective permissions, including implied permissions.
    pub fn effective_permissions(&self) -> TaskPermissions {
        let mut perms = self.permissions.clone();

        // allow_screenshot implies allow_write_data (must save the image)
        if perms.allow_screenshot {
            perms.allow_write_data = true;
        }

        // allow_export_session implies allow_export_cookies (uses same CDP call)
        if perms.allow_export_session {
            perms.allow_export_cookies = true;
        }

        // allow_import_session implies allow_import_cookies
        if perms.allow_import_session {
            perms.allow_import_cookies = true;
        }

        perms
    }

    /// Validate that the policy is valid for use.
    ///
    /// Checks:
    /// - `max_duration_ms` must be > 0
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(String)` with error message if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_duration_ms == 0 {
            return Err(format!(
                "max_duration_ms must be > 0, got {}",
                self.max_duration_ms
            ));
        }

        // Future validations can be added here:
        // - Permissions conflicts
        // - Timeout bounds (e.g., max 1 hour)

        Ok(())
    }
}

impl Default for TaskPolicy {
    fn default() -> Self {
        DEFAULT_TASK_POLICY
    }
}

/// Simple boolean permissions that control task capabilities.
#[derive(Debug, Clone, Default)]
pub struct TaskPermissions {
    /// Allow capturing screenshots.
    /// NOTE: Implies `allow_write_data` capability (screenshots must be saved).
    pub allow_screenshot: bool,

    /// Allow exporting cookies from browser.
    pub allow_export_cookies: bool,

    /// Allow importing cookies into browser.
    pub allow_import_cookies: bool,

    /// Allow exporting full session data (cookies + localStorage).
    /// NOTE: Intentionally separate from `allow_export_session` for granular control.
    pub allow_export_session: bool,

    /// Allow importing full session data (cookies + localStorage).
    pub allow_import_session: bool,

    /// Allow reading/writing clipboard.
    /// NOTE: Combined read+write permission.
    pub allow_session_clipboard: bool,

    /// Allow reading data files from `config/` or `data/` folders.
    pub allow_read_data: bool,

    /// Allow writing data files to `config/` or `data/` folders.
    pub allow_write_data: bool,

    /// Allow making HTTP requests (GET, POST, download).
    pub allow_http_requests: bool,

    /// Allow DOM inspection operations (get styles, positions, etc).
    pub allow_dom_inspection: bool,

    /// Allow exporting complete browser data (cookies + storage + more).
    pub allow_browser_export: bool,

    /// Allow importing complete browser data (cookies + storage + more).
    pub allow_browser_import: bool,
}

/// Default task policy – safe defaults (all permissions off, 60 s timeout).
pub const DEFAULT_TASK_POLICY: TaskPolicy = TaskPolicy {
    max_duration_ms: 60_000, // 1 minute – safe default
    permissions: TaskPermissions {
        allow_screenshot: false,
        allow_export_cookies: false,
        allow_import_cookies: false,
        allow_export_session: false,
        allow_import_session: false,
        allow_session_clipboard: false,
        allow_read_data: false,
        allow_write_data: false,
        allow_http_requests: false,
        allow_dom_inspection: false,
        allow_browser_export: false,
        allow_browser_import: false,
    },
};

/// Session data structure for export/import operations.
///
/// Used by `allow_export_session` / `allow_import_session` permission checks.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionData {
    /// Browser cookies from the session (stored as JSON values for portability).
    pub cookies: Vec<serde_json::Value>,

    /// localStorage key-value pairs.
    pub local_storage: HashMap<String, String>,

    /// Timestamp when session was exported (for audit trail).
    pub exported_at: DateTime<Utc>,

    /// Source URL for the session.
    pub url: String,
}

/// Complete browser data for export/import operations.
///
/// Includes all persistent and session storage data from the browser.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserData {
    /// All browser cookies.
    pub cookies: Vec<serde_json::Value>,

    /// localStorage data keyed by origin (hostname -> key/value pairs).
    pub local_storage: HashMap<String, HashMap<String, String>>,

    /// sessionStorage data keyed by origin (hostname -> key/value pairs).
    pub session_storage: HashMap<String, HashMap<String, String>>,

    /// IndexedDB database names by origin (simplified - just names for now).
    pub indexeddb_names: HashMap<String, Vec<String>>,

    /// Export timestamp for versioning.
    pub exported_at: DateTime<Utc>,

    /// Source URL or identifier.
    pub source: String,

    /// Browser version info for compatibility checks.
    pub browser_version: Option<String>,
}

impl Default for BrowserData {
    fn default() -> Self {
        Self {
            cookies: Vec::new(),
            local_storage: HashMap::new(),
            session_storage: HashMap::new(),
            indexeddb_names: HashMap::new(),
            exported_at: Utc::now(),
            source: String::new(),
            browser_version: None,
        }
    }
}

// ============================================================================
// Task-specific Policies
// ============================================================================

/// CookieBot policy - handles cookie consent dialogs.
pub static COOKIEBOT_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::cookiebot::DEFAULT_COOKIEBOT_TASK_DURATION_MS,
    permissions: TaskPermissions {
        allow_export_cookies: true, // Export to verify consent state
        allow_screenshot: true,     // Capture consent dialog for debugging
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
});

/// PageView policy - simple page loading with verification.
pub static PAGEVIEW_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 120_000, // Pageview runtime budget
    permissions: TaskPermissions::default(),
});

/// TwitterActivity policy - complex social media automation.
pub static TWITTERACTIVITY_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::utils::twitter::DEFAULT_TWITTERACTIVITY_DURATION_MS,
    permissions: TaskPermissions {
        allow_export_cookies: true,    // Verify login session
        allow_session_clipboard: true, // Copy tweet text, paste replies
        allow_read_data: true,         // Read persona files from config/
        allow_screenshot: true,        // Debug screenshots
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
});

/// Base policy for most Twitter tasks.
pub static TWITTER_BASE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 45_000, // 45 seconds default for Twitter tasks
    permissions: TaskPermissions {
        allow_screenshot: true,        // Debug failures
        allow_export_cookies: true,    // Auth verification
        allow_session_clipboard: true, // Copy/paste tweets
        ..Default::default()
    },
});

/// DemoKeyboard policy - default policy.
pub static DEMO_KEYBOARD_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::demo_keyboard::DEFAULT_DEMO_KEYBOARD_TASK_DURATION_MS,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// DemoMouse policy - default policy.
pub static DEMO_MOUSE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::demo_mouse::DEFAULT_DEMO_MOUSE_TASK_DURATION_MS,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// DemoQA policy - default policy.
pub static DEMO_QA_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::demoqa::DEFAULT_DEMOQA_TASK_DURATION_MS,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// TaskExample policy - default policy.
pub static TASK_EXAMPLE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::task_example::DEFAULT_TASK_EXAMPLE_DURATION_MS,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// TwitterDive policy - extends Twitter base policy.
pub static TWITTERDIVE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        allow_read_data: true, // Read persona files
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    max_duration_ms: crate::task::twitterdive::DEFAULT_TWITTERDIVE_DURATION_MS,
});

/// TwitterFollow policy - same as Twitter base policy.
pub static TWITTERFOLLOW_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::twitterfollow::DEFAULT_TWITTERFOLLOW_TASK_DURATION_MS,
    permissions: TWITTER_BASE_POLICY.permissions.clone(),
});

/// TwitterIntent policy - same as Twitter base policy.
pub static TWITTERINTENT_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::twitterintent::DEFAULT_TWITTERINTENT_TASK_DURATION_MS,
    permissions: TWITTER_BASE_POLICY.permissions.clone(),
});

/// TwitterLike policy - extends Twitter base policy.
pub static TWITTERLIKE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::twitterlike::DEFAULT_TWITTERLIKE_TASK_DURATION_MS,
    permissions: TWITTER_BASE_POLICY.permissions.clone(),
});

/// TwitterQuote policy - extends Twitter base policy.
pub static TWITTERQUOTE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        allow_read_data: true, // Read persona files
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    max_duration_ms: crate::task::twitterquote::DEFAULT_TWITTERQUOTE_TASK_DURATION_MS,
});

/// TwitterReply policy - extends Twitter base policy.
pub static TWITTERREPLY_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        allow_read_data: true, // Read persona files
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    max_duration_ms: crate::task::twitterreply::DEFAULT_TWITTERREPLY_TASK_DURATION_MS,
});

/// TwitterRetweet policy - same as Twitter base policy.
pub static TWITTERRETWEET_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::twitterretweet::DEFAULT_TWITTERRETWEET_TASK_DURATION_MS,
    permissions: TWITTER_BASE_POLICY.permissions.clone(),
});

/// TwitterTest policy - extends Twitter base policy (allows all read operations).
pub static TWITTERTEST_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: crate::task::twittertest::DEFAULT_TWITTERTEST_TASK_DURATION_MS,
    permissions: crate::task::policy::TaskPermissions {
        allow_screenshot: true,
        allow_export_cookies: true,
        allow_session_clipboard: true,
        allow_read_data: true,
        ..Default::default()
    },
});

/// Return the policy for a given task name.
///
/// Looks up a task‑specific policy if one is registered,
/// otherwise falls back to `DEFAULT_TASK_POLICY`.
pub fn get_policy(task_name: &str) -> &'static TaskPolicy {
    match task_name {
        "cookiebot" => &COOKIEBOT_POLICY,
        "pageview" => &PAGEVIEW_POLICY,
        "twitteractivity" => &TWITTERACTIVITY_POLICY,
        "demo-keyboard" => &DEMO_KEYBOARD_POLICY,
        "demo-mouse" => &DEMO_MOUSE_POLICY,
        "demoqa" => &DEMO_QA_POLICY,
        "task-example" => &TASK_EXAMPLE_POLICY,
        "twitterdive" => &TWITTERDIVE_POLICY,
        "twitterfollow" => &TWITTERFOLLOW_POLICY,
        "twitterintent" => &TWITTERINTENT_POLICY,
        "twitterlike" => &TWITTERLIKE_POLICY,
        "twitterquote" => &TWITTERQUOTE_POLICY,
        "twitterreply" => &TWITTERREPLY_POLICY,
        "twitterretweet" => &TWITTERRETWEET_POLICY,
        "twittertest" => &TWITTERTEST_POLICY,
        _ => &DEFAULT_TASK_POLICY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_timeout() {
        let policy = DEFAULT_TASK_POLICY;
        assert_eq!(policy.max_duration_ms, 60_000);
    }

    #[test]
    fn test_cookiebot_uses_task_duration_constant() {
        let policy = get_policy("cookiebot");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::cookiebot::DEFAULT_COOKIEBOT_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_demoqa_uses_task_duration_constant() {
        let policy = get_policy("demoqa");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::demoqa::DEFAULT_DEMOQA_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_task_example_uses_task_duration_constant() {
        let policy = get_policy("task-example");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::task_example::DEFAULT_TASK_EXAMPLE_DURATION_MS
        );
    }

    #[test]
    fn test_demo_keyboard_uses_task_duration_constant() {
        let policy = get_policy("demo-keyboard");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::demo_keyboard::DEFAULT_DEMO_KEYBOARD_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_demo_mouse_uses_task_duration_constant() {
        let policy = get_policy("demo-mouse");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::demo_mouse::DEFAULT_DEMO_MOUSE_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_default_policy_permissions_all_false() {
        let p = &DEFAULT_TASK_POLICY.permissions;
        assert!(!p.allow_screenshot);
        assert!(!p.allow_export_cookies);
        assert!(!p.allow_import_cookies);
        assert!(!p.allow_export_session);
        assert!(!p.allow_import_session);
        assert!(!p.allow_session_clipboard);
        assert!(!p.allow_read_data);
        assert!(!p.allow_write_data);
    }

    #[test]
    fn test_task_policy_default_impl() {
        let policy = TaskPolicy::default();
        assert_eq!(policy.max_duration_ms, 60_000);
    }

    #[test]
    fn test_session_data_creation() {
        let data = SessionData {
            cookies: vec![],
            local_storage: HashMap::new(),
            exported_at: Utc::now(),
            url: "https://example.com".to_string(),
        };
        assert_eq!(data.url, "https://example.com");
    }

    #[test]
    fn test_effective_permissions_screenshot_implies_write_data() {
        let mut policy = DEFAULT_TASK_POLICY;
        policy.permissions.allow_screenshot = true;
        let eff = policy.effective_permissions();
        assert!(eff.allow_write_data);
    }

    #[test]
    fn test_get_policy_cookiebot() {
        let policy = get_policy("cookiebot");
        assert_eq!(policy.max_duration_ms, 30_000);
        assert!(policy.permissions.allow_export_cookies);
    }

    #[test]
    fn test_get_policy_unknown_task() {
        let policy = get_policy("unknown_task");
        assert_eq!(policy.max_duration_ms, 60_000);
    }

    #[test]
    fn test_policy_validation_zero_timeout_fails() {
        let policy = TaskPolicy {
            max_duration_ms: 0,
            permissions: TaskPermissions::default(),
        };
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_policy_validation_valid_timeout_passes() {
        let policy = TaskPolicy {
            max_duration_ms: 60_000,
            permissions: TaskPermissions::default(),
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_effective_permissions_export_session_implies_export_cookies() {
        let mut policy = DEFAULT_TASK_POLICY;
        policy.permissions.allow_export_session = true;
        policy.permissions.allow_export_cookies = false; // explicitly false
        let eff = policy.effective_permissions();
        assert!(eff.allow_export_cookies); // implied by export_session
    }

    #[test]
    fn test_effective_permissions_import_session_implies_import_cookies() {
        let mut policy = DEFAULT_TASK_POLICY;
        policy.permissions.allow_import_session = true;
        policy.permissions.allow_import_cookies = false; // explicitly false
        let eff = policy.effective_permissions();
        assert!(eff.allow_import_cookies); // implied by import_session
    }

    #[test]
    fn test_effective_permissions_no_implications_when_base_false() {
        let policy = DEFAULT_TASK_POLICY;
        let eff = policy.effective_permissions();
        // All should remain false when base permissions are false
        assert!(!eff.allow_screenshot);
        assert!(!eff.allow_write_data); // not implied because screenshot is false
    }

    #[test]
    fn test_all_task_policies_have_valid_timeouts() {
        // Verify all registered policies have valid timeouts
        let task_names = [
            "cookiebot",
            "pageview",
            "twitteractivity",
            "demo-keyboard",
            "demo-mouse",
            "demoqa",
            "task-example",
            "twitterdive",
            "twitterfollow",
            "twitterintent",
            "twitterlike",
            "twitterquote",
            "twitterreply",
            "twitterretweet",
            "twittertest",
        ];

        for task_name in &task_names {
            let policy = get_policy(task_name);
            assert!(
                policy.max_duration_ms > 0,
                "Task '{}' has invalid timeout: {}",
                task_name,
                policy.max_duration_ms
            );
            assert!(
                policy.validate().is_ok(),
                "Task '{}' policy validation failed",
                task_name
            );
        }
    }

    #[test]
    fn test_twitter_tasks_inherit_base_policy() {
        // Twitter tasks should inherit base permissions
        let twitter_tasks = [
            "twitterlike",
            "twitterquote",
            "twitterreply",
            "twitterdive",
            "twitterfollow",
            "twitterintent",
            "twitterretweet",
            "twittertest",
        ];

        for task_name in &twitter_tasks {
            let policy = get_policy(task_name);
            // All should have screenshot enabled (from base)
            assert!(
                policy.permissions.allow_screenshot,
                "Task '{}' missing screenshot permission",
                task_name
            );
            // All should have export_cookies enabled (from base)
            assert!(
                policy.permissions.allow_export_cookies,
                "Task '{}' missing export_cookies permission",
                task_name
            );
        }
    }

    #[test]
    fn test_twitteractivity_has_extended_permissions() {
        let policy = get_policy("twitteractivity");
        // Should have all the extended permissions
        assert!(policy.permissions.allow_export_cookies);
        assert!(policy.permissions.allow_session_clipboard);
        assert!(policy.permissions.allow_read_data);
        assert!(policy.permissions.allow_screenshot);
        assert_eq!(
            policy.max_duration_ms,
            crate::utils::twitter::DEFAULT_TWITTERACTIVITY_DURATION_MS
        );
    }

    #[test]
    fn test_twitterintent_uses_task_duration_constant() {
        let policy = get_policy("twitterintent");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterintent::DEFAULT_TWITTERINTENT_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twitterfollow_uses_task_duration_constant() {
        let policy = get_policy("twitterfollow");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterfollow::DEFAULT_TWITTERFOLLOW_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twitterreply_uses_task_duration_constant() {
        let policy = get_policy("twitterreply");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterreply::DEFAULT_TWITTERREPLY_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twitterdive_uses_task_duration_constant() {
        let policy = get_policy("twitterdive");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterdive::DEFAULT_TWITTERDIVE_DURATION_MS
        );
    }

    #[test]
    fn test_twitterlike_uses_task_duration_constant() {
        let policy = get_policy("twitterlike");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterlike::DEFAULT_TWITTERLIKE_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twitterquote_uses_task_duration_constant() {
        let policy = get_policy("twitterquote");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterquote::DEFAULT_TWITTERQUOTE_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twitterretweet_uses_task_duration_constant() {
        let policy = get_policy("twitterretweet");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twitterretweet::DEFAULT_TWITTERRETWEET_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_twittertest_uses_task_duration_constant() {
        let policy = get_policy("twittertest");
        assert_eq!(
            policy.max_duration_ms,
            crate::task::twittertest::DEFAULT_TWITTERTEST_TASK_DURATION_MS
        );
    }

    #[test]
    fn test_pageview_has_default_permissions_and_120s_timeout() {
        let policy = get_policy("pageview");
        let perms = &policy.permissions;

        assert_eq!(policy.max_duration_ms, 120_000);
        assert!(!perms.allow_screenshot);
        assert!(!perms.allow_export_cookies);
        assert!(!perms.allow_import_cookies);
        assert!(!perms.allow_export_session);
        assert!(!perms.allow_import_session);
        assert!(!perms.allow_session_clipboard);
        assert!(!perms.allow_read_data);
        assert!(!perms.allow_write_data);
        assert!(!perms.allow_http_requests);
        assert!(!perms.allow_dom_inspection);
        assert!(!perms.allow_browser_export);
        assert!(!perms.allow_browser_import);
    }

    #[test]
    fn test_demo_tasks_have_no_permissions() {
        let demo_tasks = ["demo-keyboard", "demo-mouse", "demoqa"];

        for task_name in &demo_tasks {
            let policy = get_policy(task_name);
            assert!(!policy.permissions.allow_screenshot);
            assert!(!policy.permissions.allow_export_cookies);
            assert!(!policy.permissions.allow_session_clipboard);
        }
    }

    #[test]
    fn test_session_data_serialization_roundtrip() {
        let data = SessionData {
            cookies: vec![serde_json::json!({"name": "session", "value": "abc123"})],
            local_storage: {
                let mut map = std::collections::HashMap::new();
                map.insert("key".to_string(), "value".to_string());
                map
            },
            exported_at: Utc::now(),
            url: "https://example.com/dashboard".to_string(),
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SessionData = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.url, data.url);
        assert_eq!(restored.cookies.len(), data.cookies.len());
    }

    #[test]
    fn test_permissions_default_all_false() {
        let perms = TaskPermissions::default();
        assert!(!perms.allow_screenshot);
        assert!(!perms.allow_export_cookies);
        assert!(!perms.allow_import_cookies);
        assert!(!perms.allow_export_session);
        assert!(!perms.allow_import_session);
        assert!(!perms.allow_session_clipboard);
        assert!(!perms.allow_read_data);
        assert!(!perms.allow_write_data);
    }

    #[test]
    fn test_policy_implements_clone() {
        let policy = DEFAULT_TASK_POLICY.clone();
        let cloned = policy.clone();
        assert_eq!(policy.max_duration_ms, cloned.max_duration_ms);
    }
}
