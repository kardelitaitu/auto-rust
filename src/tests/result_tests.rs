#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    fn sample_metadata() -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();
        metadata.insert("source".to_string(), "manual".to_string());
        metadata.insert("run_id".to_string(), "abc123".to_string());
        metadata
    }

    #[test]
    fn test_task_result_success() {
        let result = crate::result::TaskResult::success(100);
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.attempt, 1);
        assert_eq!(result.error_kind, None);
        assert_eq!(result.metadata, None);
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
    fn test_task_result_metadata_round_trip() {
        let result = crate::result::TaskResult {
            status: crate::result::TaskStatus::Success,
            attempt: 2,
            max_retries: 4,
            last_error: None,
            error_kind: None,
            duration_ms: 42,
            metadata: Some(sample_metadata()),
        };

        let json = serde_json::to_string(&result).expect("serialize result");
        let round_trip: crate::result::TaskResult =
            serde_json::from_str(&json).expect("deserialize result");

        assert_eq!(round_trip.metadata, result.metadata);
        assert_eq!(round_trip.attempt, 2);
        assert_eq!(round_trip.duration_ms, 42);
    }

    #[test]
    fn test_task_result_deserializes_without_metadata() {
        let json = r#"{"status":"Success","attempt":1,"max_retries":0,"last_error":null,"error_kind":null,"duration_ms":100}"#;
        let result: crate::result::TaskResult =
            serde_json::from_str(json).expect("deserialize legacy result");
        assert_eq!(result.metadata, None);
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

        assert!(crate::result::TaskErrorKind::Timeout.is_retryable());
        assert!(!crate::result::TaskErrorKind::Validation.is_retryable());
    }
}
