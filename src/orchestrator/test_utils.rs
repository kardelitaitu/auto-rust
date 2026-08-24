/// Shared test helpers for orchestrator submodule tests.
///
/// Provides `create_test_config()` and `connect_test_session()` used by
/// tests across guards, execution, retry, and health submodules.
use crate::config::{
    Config, OrchestratorConfig, TaskDiscoveryConfig, TracingConfig, TwitterActivityConfig,
};
use crate::session::DurationMs;

pub(crate) fn create_test_config() -> Config {
    Config {
        orchestrator: OrchestratorConfig {
            max_global_concurrency: 10,
            group_timeout_ms: DurationMs::new_const(5000),
            task_timeout_ms: DurationMs::new_const(30000),
            task_stagger_delay_ms: 100,
            worker_wait_timeout_ms: DurationMs::new_const(5000),
            retry_delay_ms: DurationMs::new_const(1000),
            max_retries: 0,
        },
        browser: Default::default(),
        tracing: TracingConfig::default(),
        twitter_activity: TwitterActivityConfig::default(),
        task_discovery: TaskDiscoveryConfig::default(),
    }
}

pub(crate) async fn connect_test_session() -> anyhow::Result<Option<crate::session::Session>> {
    use crate::session::Session;

    let ws_url = match std::env::var("TASK_API_TEST_WS") {
        Ok(url) => url,
        Err(_) => return Ok(None),
    };

    let (browser, handler) = chromiumoxide::Browser::connect(&ws_url).await?;
    let session = Session::new(
        "orchestrator-test-session".to_string(),
        "Orchestrator Test Session".to_string(),
        "brave".to_string(),
        browser,
        handler,
        1,
        0,
        None,
        ws_url,
    );

    Ok(Some(session))
}
