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
}

impl Default for TaskPolicy {
    fn default() -> Self {
        DEFAULT_TASK_POLICY
    }
}

/// Simple boolean permissions that control task capabilities.
#[derive(Debug, Clone)]
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
}

impl Default for TaskPermissions {
    fn default() -> Self {
        Self {
            allow_screenshot: false,
            allow_export_cookies: false,
            allow_import_cookies: false,
            allow_export_session: false,
            allow_import_session: false,
            allow_session_clipboard: false,
            allow_read_data: false,
            allow_write_data: false,
        }
    }
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

// ============================================================================
// Task-specific Policies
// ============================================================================

/// CookieBot policy - handles cookie consent dialogs.
pub static COOKIEBOT_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 30_000, // 30 seconds max for consent handling
    permissions: TaskPermissions {
        allow_export_cookies: true,   // Export to verify consent state
        allow_screenshot: true,       // Capture consent dialog for debugging
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
});

/// PageView policy - simple page loading with verification.
pub static PAGEVIEW_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 30_000, // Page load timeout
    permissions: TaskPermissions {
        allow_screenshot: true,       // Verify page loaded correctly
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
});

/// TwitterActivity policy - complex social media automation.
pub static TWITTERACTIVITY_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 300_000, // 5 minutes for feed scanning
    permissions: TaskPermissions {
        allow_export_cookies: true,       // Verify login session
        allow_session_clipboard: true,     // Copy tweet text, paste replies
        allow_read_data: true,            // Read persona files from config/
        allow_screenshot: true,           // Debug screenshots
        // allow_write_data implied by allow_screenshot
        ..Default::default()
    },
});

/// Base policy for most Twitter tasks.
pub static TWITTER_BASE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 45_000, // 45 seconds default for Twitter tasks
    permissions: TaskPermissions {
        allow_screenshot: true,           // Debug failures
        allow_export_cookies: true,       // Auth verification
        allow_session_clipboard: true,    // Copy/paste tweets
        ..Default::default()
    },
});

/// DemoKeyboard policy - default policy.
pub static DEMO_KEYBOARD_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 60_000,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// DemoMouse policy - default policy.
pub static DEMO_MOUSE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 60_000,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// DemoQA policy - default policy.
pub static DEMO_QA_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 60_000,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// TaskExample policy - default policy.
pub static TASK_EXAMPLE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 60_000,
    permissions: TaskPermissions {
        ..Default::default()
    },
});

/// TwitterDive policy - extends Twitter base policy.
pub static TWITTERDIVE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterFollow policy - extends Twitter base policy.
pub static TWITTERFOLLOW_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterIntent policy - extends Twitter base policy.
pub static TWITTERINTENT_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterLike policy - extends Twitter base policy.
pub static TWITTERLIKE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 30_000,  // Override: faster timeout
    permissions: crate::task::policy::TaskPermissions {
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterQuote policy - extends Twitter base policy.
pub static TWITTERQUOTE_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        allow_read_data: true,  // Read persona files
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterReply policy - extends Twitter base policy.
pub static TWITTERREPLY_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        allow_read_data: true, // Read persona files
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterRetweet policy - extends Twitter base policy.
pub static TWITTERRETWEET_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    permissions: crate::task::policy::TaskPermissions {
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
});

/// TwitterTest policy - extends Twitter base policy (allows all read operations).
pub static TWITTERTEST_POLICY: Lazy<TaskPolicy> = Lazy::new(|| TaskPolicy {
    max_duration_ms: 120_000,  // Longer timeout for comprehensive tests
    permissions: crate::task::policy::TaskPermissions {
        allow_screenshot: true,
        allow_export_cookies: true,
        allow_session_clipboard: true,
        allow_read_data: true,
        ..TWITTER_BASE_POLICY.permissions.clone()
    },
    ..*TWITTER_BASE_POLICY
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
}
