//! Security utilities for task operations.
//!
//! Provides helper functions for validating paths and other security-sensitive
//! operations to ensure tasks stay within allowed boundaries.

use std::path::{Path, PathBuf};

/// Validate that a data file path is safe for reading/writing.
///
/// Security checks:
/// - Path must be relative (not absolute)
/// - Path must not contain directory traversal ("..")
/// - Path must be within `config/` or `data/` directories
///
/// # Arguments
///
/// * `relative_path` - The user-provided relative path
///
/// # Returns
///
/// `Ok(PathBuf)` with resolved path if valid, `Err(TaskError::InvalidPath)` if not.
///
/// # Examples
///
/// ```ignore
/// use auto::task::security::validate_data_path;
///
/// let path = validate_data_path("personas/default.json")?;
/// // Returns: Ok(PathBuf("config/personas/default.json"))
/// ```
pub fn validate_data_path(relative_path: &str) -> crate::error::Result<PathBuf> {
    // Check 1: Reject empty paths
    if relative_path.is_empty() {
        return Err(crate::error::TaskError::InvalidPath(
            "Path cannot be empty".to_string(),
        )
        .into());
    }

    // Check 2: Reject absolute paths (including Unix-style / and Windows-style C:)
    let path = Path::new(relative_path);
    if path.is_absolute()
        || relative_path.starts_with('/')
        || relative_path.starts_with('\\')
        || (relative_path.len() > 1 && relative_path.as_bytes()[1] == b':')
    {
        return Err(crate::error::TaskError::InvalidPath(format!(
            "Absolute paths not allowed: {}",
            relative_path
        ))
        .into());
    }

    // Check 3: Reject directory traversal attempts
    // Normalize path separators and check for ".."
    let normalized = relative_path.replace('\\', "/");
    let components: Vec<&str> = normalized.split('/').collect();

    for component in &components {
        if *component == ".." {
            return Err(crate::error::TaskError::InvalidPath(format!(
                "Directory traversal not allowed: {}",
                relative_path
            ))
            .into());
        }
    }

    // Check 4: Must resolve within allowed directories
    let allowed_dirs = ["config", "data"];

    for dir in &allowed_dirs {
        let full_path = Path::new(dir).join(relative_path);

        // Check if file exists
        if full_path.exists() {
            // Verify the resolved path doesn't escape the allowed directory
            // by checking that the canonical path still starts with the allowed dir
            let canonical_base = match Path::new(dir).canonicalize() {
                Ok(p) => p,
                Err(_) => continue, // Skip if base dir doesn't exist
            };

            let canonical_path = match full_path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    return Err(crate::error::TaskError::InvalidPath(format!(
                        "Failed to resolve path: {}",
                        e
                    ))
                    .into());
                }
            };

            if !canonical_path.starts_with(&canonical_base) {
                return Err(crate::error::TaskError::InvalidPath(format!(
                    "Path escapes allowed directory: {}",
                    relative_path
                ))
                .into());
            }

            return Ok(canonical_path);
        }
    }

    // File doesn't exist in any allowed directory
    // For write operations, we'll allow it but return the path in config/
    let default_path = Path::new("config").join(relative_path);
    Ok(default_path)
}

/// Check if a path string contains directory traversal patterns.
///
/// This is a lightweight check that doesn't require file system access.
pub fn contains_traversal(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.split('/').any(|c| c == "..")
}

/// Check if a path is safe without requiring the file to exist.
///
/// This performs only syntactic checks (no traversal, not absolute).
/// Useful for write operations where the file doesn't exist yet.
pub fn is_safe_path(relative_path: &str) -> bool {
    if relative_path.is_empty() {
        return false;
    }

    // Check for absolute paths (including Unix-style / and Windows-style C:)
    let path = Path::new(relative_path);
    if path.is_absolute()
        || relative_path.starts_with('/')
        || relative_path.starts_with('\\')
        || (relative_path.len() > 1 && relative_path.as_bytes()[1] == b':')
    {
        return false;
    }

    !contains_traversal(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rejects_absolute_unix() {
        assert!(validate_data_path("/etc/passwd").is_err());
        assert!(validate_data_path("/home/user/file.txt").is_err());
    }

    #[test]
    fn test_validate_rejects_absolute_windows() {
        assert!(validate_data_path("C:\\Windows\\System32").is_err());
        assert!(validate_data_path("D:\\file.txt").is_err());
    }

    #[test]
    fn test_validate_rejects_traversal_prefix() {
        assert!(validate_data_path("../secret.txt").is_err());
    }

    #[test]
    fn test_validate_rejects_traversal_middle() {
        assert!(validate_data_path("foo/../bar/../../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_rejects_traversal_suffix() {
        assert!(validate_data_path("data/subdir/..").is_err());
    }

    #[test]
    fn test_validate_rejects_empty() {
        assert!(validate_data_path("").is_err());
    }

    #[test]
    fn test_validate_accepts_safe_relative() {
        // These should pass syntactic validation
        // (actual file existence check may fail in test environment)
        assert!(is_safe_path("config/test.txt"));
        assert!(is_safe_path("data/subdir/file.json"));
        assert!(is_safe_path("personas/default.toml"));
    }

    #[test]
    fn test_contains_traversal_detects_double_dot() {
        assert!(contains_traversal("../file.txt"));
        assert!(contains_traversal("foo/../bar"));
        assert!(contains_traversal("a/b/c/../../d"));
    }

    #[test]
    fn test_contains_traversal_allows_safe() {
        assert!(!contains_traversal("file.txt"));
        assert!(!contains_traversal("subdir/file.txt"));
        assert!(!contains_traversal("a/b/c/d.json"));
    }

    #[test]
    fn test_contains_traversal_handles_backslash() {
        assert!(contains_traversal("..\\file.txt"));
        assert!(contains_traversal("foo\\..\\bar"));
    }

    #[test]
    fn test_is_safe_path_rejects_absolute() {
        assert!(!is_safe_path("/etc/passwd"));
        assert!(!is_safe_path("C:\\file.txt"));
    }

    #[test]
    fn test_is_safe_path_rejects_empty() {
        assert!(!is_safe_path(""));
    }

    #[test]
    fn test_is_safe_path_rejects_traversal() {
        assert!(!is_safe_path("../file.txt"));
        assert!(!is_safe_path("foo/../../bar"));
    }

    #[test]
    fn test_is_safe_path_accepts_valid() {
        assert!(is_safe_path("file.txt"));
        assert!(is_safe_path("data/file.json"));
        assert!(is_safe_path("config/personas/bot.json"));
    }

    #[test]
    fn test_is_safe_path_allows_single_dot() {
        // Single dot is OK (refers to current directory)
        assert!(is_safe_path("./file.txt"));
        assert!(is_safe_path("data/./file.txt"));
    }

    #[test]
    fn test_validate_allows_single_dot() {
        // Single dot should be allowed
        assert!(is_safe_path("./config.txt"));
    }
}
