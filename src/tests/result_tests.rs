#[cfg(test)]
mod tests {

    #[test]
    fn test_task_result_success() {
        let result = crate::result::TaskResult::success(100);
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.attempt, 1);
    }

    #[test]
    fn test_task_result_with_retry() {
        let result =
            crate::result::TaskResult::success(50).with_retry(3, 5, "previous error".to_string());

        assert!(result.is_success());
        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
        assert_eq!(result.last_error, Some("previous error".to_string()));
    }

    #[test]
    fn test_task_error_kind_classify() {
        assert_eq!(
            crate::result::TaskErrorKind::classify("deadline has elapsed"),
            crate::result::TaskErrorKind::Timeout
        );

        assert_eq!(
            crate::result::TaskErrorKind::classify("validation failed"),
            crate::result::TaskErrorKind::Validation
        );

        assert_eq!(
            crate::result::TaskErrorKind::classify("navigation error"),
            crate::result::TaskErrorKind::Navigation
        );

        assert_eq!(
            crate::result::TaskErrorKind::classify("session closed"),
            crate::result::TaskErrorKind::Session
        );
    }
}
