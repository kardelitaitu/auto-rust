//! CDP utility functions for consistent error handling.
//!
//! Provides helper functions and extension traits for mapping CDP errors
//! to application-specific error types with consistent formatting.

use chromiumoxide::error::CdpError;

/// Map a CDP error to a consistent TaskError.
///
/// Provides consistent error formatting and classification for CDP operations.
///
/// # Arguments
///
/// * `operation` - The CDP operation name (e.g., "Network.getCookies")
/// * `error` - The CDP error to map
///
/// # Returns
///
/// `TaskError::CdpError` with formatted message.
///
/// # Examples
///
/// ```ignore
/// use auto::task::cdp_utils::map_cdp_error;
/// use chromiumoxide::error::CdpError;
///
/// let err = CdpError::Timeout;
/// let task_err = map_cdp_error("Page.navigate", err);
/// ```
pub fn map_cdp_error(operation: &str, error: CdpError) -> crate::error::TaskError {
    let reason = match &error {
        CdpError::NotFound => "Page or target not found".to_string(),
        CdpError::Timeout => "Operation timed out".to_string(),
        _ => format!("CDP operation failed: {}", error),
    };

    crate::error::TaskError::CdpError {
        operation: operation.to_string(),
        reason,
    }
}

/// Extension trait for Result<T, CdpError> to easily map errors.
///
/// This trait allows chaining error mapping directly on CDP operation results.
///
/// # Examples
///
/// ```ignore
/// use auto::task::cdp_utils::CdpResultExt;
///
/// let result = page.execute(params).await.map_cdp_err("Network.getCookies");
/// ```
pub trait CdpResultExt<T> {
    /// Map a CDP error to a TaskError with the given operation name.
    fn map_cdp_err(self, operation: &str) -> Result<T, crate::error::TaskError>;
}

impl<T> CdpResultExt<T> for Result<T, CdpError> {
    fn map_cdp_err(self, operation: &str) -> Result<T, crate::error::TaskError> {
        self.map_err(|e| map_cdp_error(operation, e))
    }
}

/// Extension trait for Option<T> to provide context on None values.
///
/// Useful when a CDP operation returns None where a value was expected.
///
/// # Examples
///
/// ```ignore
/// use auto::task::cdp_utils::CdpOptionExt;
///
/// let result = page.find_element("#btn").await.ok_or_cdp_err("Page.find_element")?;
/// ```
pub trait CdpOptionExt<T> {
    /// Convert None to a CdpError with the given operation name.
    fn ok_or_cdp_err(self, operation: &str) -> Result<T, crate::error::TaskError>;
}

impl<T> CdpOptionExt<T> for Option<T> {
    fn ok_or_cdp_err(self, operation: &str) -> Result<T, crate::error::TaskError> {
        self.ok_or_else(|| crate::error::TaskError::CdpError {
            operation: operation.to_string(),
            reason: "Operation returned no result".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_cdp_error_not_found() {
        let err = CdpError::NotFound;
        let mapped = map_cdp_error("Page.find_element", err);
        match mapped {
            crate::error::TaskError::CdpError { operation, reason } => {
                assert_eq!(operation, "Page.find_element");
                assert!(reason.contains("not found"));
            }
            _ => panic!("Expected CdpError variant"),
        }
    }

    #[test]
    fn test_map_cdp_error_timeout() {
        let err = CdpError::Timeout;
        let mapped = map_cdp_error("Page.navigate", err);
        match mapped {
            crate::error::TaskError::CdpError { operation, reason } => {
                assert_eq!(operation, "Page.navigate");
                assert!(reason.contains("timed out"));
            }
            _ => panic!("Expected CdpError variant"),
        }
    }

    #[test]
    fn test_cdp_result_ext_success() {
        let result: Result<i32, CdpError> = Ok(42);
        let mapped = result.map_cdp_err("Test.op");
        assert!(mapped.is_ok());
        assert_eq!(mapped.unwrap(), 42);
    }

    #[test]
    fn test_cdp_result_ext_error() {
        let result: Result<i32, CdpError> = Err(CdpError::Timeout);
        let mapped = result.map_cdp_err("Page.click");
        assert!(mapped.is_err());
        match mapped {
            Err(crate::error::TaskError::CdpError { operation, .. }) => {
                assert_eq!(operation, "Page.click");
            }
            _ => panic!("Expected CdpError variant"),
        }
    }

    #[test]
    fn test_cdp_option_ext_some() {
        let opt: Option<i32> = Some(42);
        let result = opt.ok_or_cdp_err("Page.find");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_cdp_option_ext_none() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_cdp_err("Page.find");
        assert!(result.is_err());
        match result {
            Err(crate::error::TaskError::CdpError { operation, reason }) => {
                assert_eq!(operation, "Page.find");
                assert!(reason.contains("no result"));
            }
            _ => panic!("Expected CdpError variant"),
        }
    }
}
