mod blockmedia_policy_tests;
mod click_policy_tests;
mod page_manager_policy_tests;
mod result_tests;
mod task_registry_policy_tests;

#[cfg(test)]
mod config_tests {
    use crate::config::{
        validate_config, BrowserConfig, CircuitBreakerConfig, Config, OrchestratorConfig,
        RoxybrowserConfig, TwitterActivityConfig,
    };
    use std::collections::BTreeMap;

    #[test]
    fn test_validate_config_valid() {
        let config = Config {
            browser: BrowserConfig {
                connectors: vec![],
                connection_timeout_ms: 10000,
                max_discovery_retries: 3,
                discovery_retry_delay_ms: 5000,
                circuit_breaker: CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 5,
                    success_threshold: 3,
                    half_open_time_ms: 30000,
                },
                profiles: vec![],
                roxybrowser: RoxybrowserConfig {
                    enabled: true,
                    api_url: "http://localhost".to_string(),
                    api_key: "key".to_string(),
                },
                user_agent: None,
                extra_http_headers: BTreeMap::new(),
                cursor_overlay_ms: 0,
                max_workers_per_session: 5,
            },
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 5,
                task_timeout_ms: 60_000,
                group_timeout_ms: 120_000,
                worker_wait_timeout_ms: 10000,
                stuck_worker_threshold_ms: 60_000,
                task_stagger_delay_ms: 1000,
                max_retries: 2,
                retry_delay_ms: 500,
            },
            twitter_activity: TwitterActivityConfig::default(),
        };

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_concurrency() {
        let config = Config {
            browser: BrowserConfig {
                connectors: vec![],
                connection_timeout_ms: 10000,
                max_discovery_retries: 3,
                discovery_retry_delay_ms: 5000,
                circuit_breaker: CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 5,
                    success_threshold: 3,
                    half_open_time_ms: 30000,
                },
                profiles: vec![],
                roxybrowser: RoxybrowserConfig {
                    enabled: true,
                    api_url: "http://localhost".to_string(),
                    api_key: "key".to_string(),
                },
                user_agent: None,
                extra_http_headers: BTreeMap::new(),
                cursor_overlay_ms: 0,
                max_workers_per_session: 5,
            },
            orchestrator: OrchestratorConfig {
                max_global_concurrency: 0,
                task_timeout_ms: 60_000,
                group_timeout_ms: 120_000,
                worker_wait_timeout_ms: 10000,
                stuck_worker_threshold_ms: 60_000,
                task_stagger_delay_ms: 1000,
                max_retries: 2,
                retry_delay_ms: 500,
            },
            twitter_activity: TwitterActivityConfig::default(),
        };

        assert!(validate_config(&config).is_err());
    }
}

#[cfg(test)]
mod metrics_tests {
    use crate::metrics::{MetricsCollector, TaskMetrics, TaskStatus};

    #[test]
    fn test_metrics_collector_new() {
        let mc = MetricsCollector::new(100);
        let stats = mc.get_stats();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.active_tasks, 0);
    }

    #[test]
    fn test_metrics_success_rate() {
        let mc = MetricsCollector::new(100);
        mc.task_started();
        mc.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Success,
            duration_ms: 100,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });

        assert_eq!(mc.success_rate(), 100.0);
    }
}
