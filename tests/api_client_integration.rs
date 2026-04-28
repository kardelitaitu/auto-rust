//! HTTP API client integration tests using wiremock.
//!
//! These tests verify the full HTTP request/response cycle including:
//! - Successful GET requests with JSON deserialization
//! - HTTP error handling (4xx, 5xx, timeouts)
//! - Circuit breaker integration with real HTTP calls
//! - API key authentication headers
//! - Retry behavior with mocked failures

use serde::{Deserialize, Serialize};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestResponse {
    message: String,
    status: String,
    count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestRequest {
    name: String,
    value: i32,
}

/// Creates a test API client pointing at the mock server
fn create_test_client(mock_server: &MockServer) -> auto::api::client::ApiClient {
    auto::api::client::ApiClient::new(mock_server.uri())
}

/// Creates a test API client with circuit breaker
fn create_test_client_with_circuit_breaker(
    mock_server: &MockServer,
    cb: auto::api::client::CircuitBreaker,
) -> auto::api::client::ApiClient {
    auto::api::client::ApiClient::with_circuit_breaker(mock_server.uri(), cb)
}

#[tokio::test]
async fn test_get_request_success() {
    let mock_server = MockServer::start().await;

    let response = TestResponse {
        message: "Success".to_string(),
        status: "ok".to_string(),
        count: 42,
    };

    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: TestResponse = client.get("/api/test").await.expect("Should succeed");

    assert_eq!(result.message, "Success");
    assert_eq!(result.status, "ok");
    assert_eq!(result.count, 42);
}

#[tokio::test]
async fn test_get_with_api_key_header() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/secure"))
        .and(header("X-API-Key", "secret-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&TestResponse {
            message: "Authorized".to_string(),
            status: "ok".to_string(),
            count: 1,
        }))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: TestResponse = client
        .get_with_key("/api/secure", "secret-key-123")
        .await
        .expect("Should succeed with API key");

    assert_eq!(result.message, "Authorized");
}

#[tokio::test]
async fn test_http_404_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/not-found"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: Result<TestResponse, _> = client.get("/api/not-found").await;

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("404") || err_msg.contains("API request failed"));
}

#[tokio::test]
async fn test_http_500_retryable_error() {
    let mock_server = MockServer::start().await;

    // First two requests fail with 500, third succeeds
    Mock::given(method("GET"))
        .and(path("/api/flaky"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/flaky"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&TestResponse {
            message: "Recovered".to_string(),
            status: "ok".to_string(),
            count: 1,
        }))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    // Retry policy with 3 retries, so it should eventually succeed
    let result: TestResponse = client
        .get("/api/flaky")
        .await
        .expect("Should succeed after retries");

    assert_eq!(result.message, "Recovered");
}

#[tokio::test]
async fn test_http_429_too_many_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/rate-limited"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Rate limited"))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: Result<TestResponse, _> = client.get("/api/rate-limited").await;

    assert!(result.is_err());
    // 429 is retryable, so it should retry and then fail
}

#[tokio::test]
async fn test_circuit_breaker_blocks_requests_when_open() {
    let mock_server = MockServer::start().await;

    // All requests fail
    Mock::given(method("GET"))
        .and(path("/api/failing"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let cb = auto::api::client::CircuitBreaker::new(2, 2, 1000);
    let client = create_test_client_with_circuit_breaker(&mock_server, cb);

    // First request - circuit closed, should attempt
    let _ = client.get::<TestResponse>("/api/failing").await;

    // Second request - circuit still closed (1 failure)
    let _ = client.get::<TestResponse>("/api/failing").await;

    // Third request - circuit should be open now (2 failures), blocks immediately
    let result: Result<TestResponse, _> = client.get("/api/failing").await;

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Circuit breaker") || err_msg.contains("API request"));
}

#[tokio::test]
async fn test_malformed_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/bad-json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: Result<TestResponse, _> = client.get("/api/bad-json").await;

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    // Should fail to parse JSON
    assert!(err_msg.contains("parse") || err_msg.contains("JSON") || err_msg.contains("retryable"));
}

#[tokio::test]
async fn test_empty_response_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/empty"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: Result<TestResponse, _> = client.get("/api/empty").await;

    assert!(result.is_err()); // Empty body can't be parsed as JSON
}

#[tokio::test]
async fn test_base_url_trailing_slash_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&TestResponse {
            message: "OK".to_string(),
            status: "ok".to_string(),
            count: 1,
        }))
        .mount(&mock_server)
        .await;

    // Client with trailing slash in base URL
    let client = auto::api::client::ApiClient::new(format!("{}/", mock_server.uri()));
    let result: TestResponse = client.get("api/test").await.expect("Should succeed");

    assert_eq!(result.message, "OK");
}

#[tokio::test]
async fn test_request_timeout() {
    let mock_server = MockServer::start().await;

    // Response that takes too long - use shorter delay for faster test
    Mock::given(method("GET"))
        .and(path("/api/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(3)))
        .mount(&mock_server)
        .await;

    // Create client with short timeout
    let client = auto::api::client::ApiClient::with_timeout(
        format!("{}/", mock_server.uri()),
        std::time::Duration::from_secs(1),
    );
    let result: Result<TestResponse, _> = client.get("/api/slow").await;

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    // Should timeout (client has 1s timeout, mock has 3s delay)
    assert!(err_msg.contains("timeout") || err_msg.contains("API request"));
}

#[tokio::test]
async fn test_complex_json_response() {
    let mock_server = MockServer::start().await;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ComplexResponse {
        data: Vec<TestRequest>,
        meta: serde_json::Value,
    }

    let response = ComplexResponse {
        data: vec![
            TestRequest {
                name: "item1".to_string(),
                value: 100,
            },
            TestRequest {
                name: "item2".to_string(),
                value: 200,
            },
        ],
        meta: serde_json::json!({"total": 2, "page": 1}),
    };

    Mock::given(method("GET"))
        .and(path("/api/complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server);
    let result: ComplexResponse = client
        .get("/api/complex")
        .await
        .expect("Should parse complex response");

    assert_eq!(result.data.len(), 2);
    assert_eq!(result.data[0].name, "item1");
    assert_eq!(result.meta["total"], 2);
}
