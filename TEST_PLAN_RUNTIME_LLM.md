# Test Implementation Plan: Runtime & LLM Integration

**Focus Areas:**
1. Runtime - retry logic, circuit breaker, cancellation token handling
2. LLM Integration - error handling, timeout, fallback switching

**Status:** Planning Phase - Awaiting Implementation Approval

---

## 🎯 AREA 1: RUNTIME TESTS

### Current State Analysis
**File:** `src/runtime/task_context.rs`  
**Current Tests:** ~60 tests (mostly click learning, timing, sanitization)  
**Coverage Gaps:** Retry logic, circuit breaker, cancellation tokens

### Missing Test Areas

#### 1.1 Retry Logic Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
pub async fn click_with_retry(&self, selector: &str) -> Result<ClickOutcome>
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_click_retry_success_on_second_attempt` | First click fails, second succeeds | Returns success, attempt=2 |
| `test_click_retry_exhausts_all_attempts` | All 3 attempts fail | Returns last error, attempt=3 |
| `test_retry_backoff_timing` | Measure delay between retries | Delay increases exponentially |
| `test_retry_resets_on_success` | Success after failure resets counter | Next operation starts at attempt=1 |
| `test_retry_preserves_error_context` | Multiple failures | Last error includes all previous errors |

**Test Implementation Approach:**
```rust
#[tokio::test]
async fn test_click_retry_success_on_second_attempt() {
    // Mock: first click fails (element not found), second succeeds
    // Verify: outcome.success == true, outcome.attempts == 2
}
```

**Confidence:** 90% - Clear requirements, mockable dependencies

---

#### 1.2 Circuit Breaker Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
pub struct CircuitBreaker {
    failure_count: AtomicUsize,
    state: AtomicU8, // Closed=0, Open=1, HalfOpen=2
    last_failure_time: AtomicU64,
}
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_circuit_opens_after_threshold` | 5 failures in 60s | State transitions to Open |
| `test_circuit_closes_after_timeout` | Wait 30s after open | State becomes HalfOpen, then Closed on success |
| `test_circuit_half_open_allows_probe` | In HalfOpen state | Single request allowed through |
| `test_circuit_reopens_on_probe_failure` | Probe fails | Returns to Open immediately |
| `test_circuit_failure_count_decay` | 1 failure/min for 10 min | Old failures expire, count decreases |
| `test_circuit_fast_fail_when_open` | Request when Open | Immediate error, no attempt |

**Test Implementation Approach:**
```rust
#[test]
fn test_circuit_opens_after_threshold() {
    let cb = CircuitBreaker::new(5, Duration::from_secs(60));
    for _ in 0..5 {
        cb.record_failure();
    }
    assert_eq!(cb.state(), CircuitState::Open);
}
```

**Confidence:** 85% - State machine testing is straightforward

---

#### 1.3 Cancellation Token Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
pub async fn pause(&self, ms: u64) {
    // Should end early if cancellation token triggered
}
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_pause_ends_early_on_cancel` | Cancel during 5000ms pause | Returns before 5000ms |
| `test_click_aborts_on_cancel` | Cancel during click operation | Operation aborts gracefully |
| `test_navigate_aborts_on_cancel` | Cancel during navigation | Page loading stops |
| `test_task_cleanup_on_cancel` | Cancel mid-task | Resources released properly |
| `test_cascade_cancel_to_subtasks` | Parent cancelled | Child operations also cancelled |
| `test_cancel_token_thread_safety` | Concurrent cancel + operations | No race conditions |

**Test Implementation Approach:**
```rust
#[tokio::test]
async fn test_pause_ends_early_on_cancel() {
    let token = CancellationToken::new();
    let start = Instant::now();
    
    tokio::spawn({
        let token = token.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token.cancel();
        }
    });
    
    pause_with_cancel(5000, &token).await;
    assert!(start.elapsed() < Duration::from_millis(200));
}
```

**Confidence:** 80% - Async timing tests can be flaky, need careful design

---

### Runtime Test Implementation Priority

```
Phase 1 (This Week) - 6 tests
├── Circuit breaker state transitions (3 tests)
├── Retry success/failure paths (2 tests)
└── Basic cancellation (1 test)

Phase 2 (Next Week) - 8 tests
├── Retry backoff timing (2 tests)
├── Circuit decay/reopen (3 tests)
├── Cancellation edge cases (3 tests)

Phase 3 (Following Week) - 4 tests
├── Complex retry + circuit interaction (2 tests)
└── Resource cleanup verification (2 tests)
```

---

## 🎯 AREA 2: LLM INTEGRATION TESTS

### Current State Analysis
**Files:** `src/llm/client.rs`, `src/llm/unified_processor.rs`, `src/llm/reply_engine.rs`  
**Current Tests:** ~80 tests (response parsing, sentiment analysis, batch processing)  
**Coverage Gaps:** Error handling, timeouts, fallback switching

### Missing Test Areas

#### 2.1 Error Handling Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
pub async fn ollama_chat(&self, request: ChatRequest) -> Result<ChatResponse>
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_ollama_connection_refused` | Server down | Returns ConnectionError with retryable=true |
| `test_ollama_404_endpoint` | Wrong endpoint | Returns ApiError with specific message |
| `test_ollama_malformed_json` | Invalid response | Returns ParseError with raw body |
| `test_ollama_rate_limited` | 429 response | Returns RateLimitError with Retry-After header |
| `test_ollama_server_error` | 500/503 response | Returns ServerError, triggers retry |
| `test_openrouter_auth_failure` | Invalid API key | Returns AuthError, no retry |
| `test_openrouter_quota_exceeded` | 402 response | Returns QuotaError with upgrade link |
| `test_error_classification_accuracy` | Various errors | Each error type correctly classified |

**Test Implementation Approach:**
```rust
#[tokio::test]
async fn test_ollama_connection_refused() {
    let client = LlmClient::with_base_url("http://localhost:99999"); // Invalid port
    let result = client.ollama_chat(create_test_request()).await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_retryable());
    assert!(err.to_string().contains("connection"));
}
```

**Confidence:** 85% - Can use mock servers or invalid URLs

---

#### 2.2 Timeout Handling Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
.timeout(Duration::from_secs(120))
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_request_timeout_slow_response` | Response > timeout | Returns TimeoutError |
| `test_request_timeout_during_stream` | Stream stalls | Aborts, returns partial + error |
| `test_timeout_doesnt_abort_fast_response` | Response < timeout | Success, no timeout |
| `test_timeout_retry_exhaustion` | Timeout on all retries | Returns TimeoutError after N attempts |
| `test_timeout_with_different_providers` | Timeout on provider A | Falls back to provider B |
| `test_connect_timeout_vs_read_timeout` | Connection slow vs response slow | Distinguishes in error |

**Test Implementation Approach:**
```rust
#[tokio::test]
async fn test_request_timeout_slow_response() {
    let mut server = mockito::Server::new_async().await;
    let mock = server.mock("POST", "/api/chat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_delay(Duration::from_secs(10)) // Slow response
        .with_body(r#"{"response": "too late"}"#)
        .create_async()
        .await;
    
    let client = LlmClient::with_timeout(Duration::from_millis(100));
    let result = client.chat(server.url() + "/api/chat").await;
    
    assert!(matches!(result.unwrap_err(), LlmError::Timeout));
    mock.assert_async().await;
}
```

**Confidence:** 75% - Timeout testing requires mock servers with delays

---

#### 2.3 Fallback Provider Tests (Priority: 🔴 HIGH)

**Current Implementation:**
```rust
pub struct MultiProviderClient {
    providers: Vec<ProviderConfig>,
    current: AtomicUsize,
}
```

**Missing Tests:**

| Test Name | Scenario | Expected Behavior |
|-----------|----------|-------------------|
| `test_fallback_on_primary_failure` | Primary fails | Switches to secondary |
| `test_fallback_recovery_to_primary` | Primary recovers | Switches back after health check |
| `test_fallback_exhaustion_all_fail` | All providers fail | Returns aggregate error |
| `test_fallback_priority_order` | Multiple fallbacks | Tries in priority order |
| `test_fallback_preserves_context` | Switch mid-conversation | Context/thread maintained |
| `test_fallback_load_balancing` | Both healthy | Distributes requests |
| `test_fallback_health_check_interval` | Periodic checks | Marks unhealthy providers |
| `test_fallback_provider_blacklist` | Provider fails 3x | Temporarily blacklisted |

**Test Implementation Approach:**
```rust
#[tokio::test]
async fn test_fallback_on_primary_failure() {
    let primary = MockProvider::new().fails_with(500);
    let secondary = MockProvider::new().succeeds_with("Hello");
    
    let client = MultiProviderClient::new(vec![
        ProviderConfig::new("primary", primary.url()),
        ProviderConfig::new("secondary", secondary.url()),
    ]);
    
    let response = client.chat("Hi").await.unwrap();
    assert_eq!(response.content, "Hello");
    assert!(primary.was_called());
    assert!(secondary.was_called());
}
```

**Confidence:** 70% - Complex multi-provider setup required

---

### LLM Test Implementation Priority

```
Phase 1 (This Week) - 5 tests
├── Connection refused error (1 test)
├── Rate limit handling (1 test)
├── Basic timeout (1 test)
├── Fallback on failure (1 test)
└── Error classification (1 test)

Phase 2 (Next Week) - 7 tests
├── Various HTTP error codes (3 tests)
├── Timeout edge cases (2 tests)
├── Fallback priority order (2 tests)

Phase 3 (Following Week) - 5 tests
├── Fallback recovery (2 tests)
├── Provider health checks (2 tests)
└── Load balancing (1 test)
```

---

## 📊 SUMMARY COMPARISON

| Metric | Runtime | LLM Integration |
|--------|---------|-----------------|
| **Current Tests** | ~60 | ~80 |
| **Missing Tests** | ~18 | ~17 |
| **Priority** | 🔴 High | 🔴 High |
| **Complexity** | Medium | Medium-High |
| **Dependencies** | Mock browser | Mock HTTP servers |
| **Time Estimate** | 3-4 days | 4-5 days |
| **Confidence** | 85% | 77% |

---

## 🛠️ TEST INFRASTRUCTURE NEEDS

### Required Dependencies
```toml
[dev-dependencies]
mockito = "1.2"           # HTTP mocking for LLM tests
wiremock = "0.5"         # Alternative HTTP mocking
tokio-test = "0.4"       # Async test utilities
serial_test = "3.0"      # Prevent concurrent test conflicts
```

### Test Utilities to Create

1. **`test_utils/mock_browser.rs`**
   - Mock Page interactions
   - Simulate element found/not found
   - Simulate navigation success/failure

2. **`test_utils/mock_llm_server.rs`**
   - Start/stop mock LLM server
   - Configure responses
   - Simulate delays/errors

3. **`test_utils/circuit_breaker_assertions.rs`**
   - Helper to assert state transitions
   - Timing assertions

---

## ✅ ACCEPTANCE CRITERIA

### Runtime Tests Complete When:
- [ ] All retry scenarios tested (success, failure, backoff)
- [ ] Circuit breaker state machine fully verified
- [ ] Cancellation token tested in all async operations
- [ ] Edge cases: concurrent cancels, nested retries

### LLM Tests Complete When:
- [ ] All HTTP error codes handled correctly
- [ ] Timeout scenarios verified (connect, read, total)
- [ ] Fallback switching works across providers
- [ ] Error classification accurate for all error types

---

## 🚀 RECOMMENDED APPROACH

### Option A: Parallel Implementation
**Pros:** Faster completion, independent work streams  
**Cons:** More cognitive load, potential merge conflicts  
**Timeline:** 5-6 days

### Option B: Sequential (Runtime First)
**Pros:** Focused attention, learn patterns for LLM tests  
**Cons:** Slower overall  
**Timeline:** 7-8 days

### Option C: Quick Wins First
Start with easiest tests from both areas, then tackle complexity.  
**Timeline:** 6-7 days

---

## 🎯 MY RECOMMENDATION

**Approach:** Option C - Quick Wins First

**Reasoning:**
1. Get immediate coverage improvement (2 days)
2. Build confidence with mock infrastructure
3. Tackle complex scenarios with established patterns

**Implementation Order:**
1. **Day 1:** Circuit breaker state transitions (3 tests)
2. **Day 2:** LLM error classification + timeout (3 tests)
3. **Day 3:** Retry logic + fallback switching (4 tests)
4. **Day 4-5:** Edge cases, concurrency, integration

**Confidence:** 85% - Achievable with focused effort

---

## ⏳ DECISION POINT

**Ready to proceed?**

**A)** Yes - Start with Runtime circuit breaker tests  
**B)** Yes - Start with LLM error handling tests  
**C)** Yes - Start with Quick Wins (mixed)  
**D)** No - Need more analysis / different priority

**Additional Requirements:**
- Mock server library preference? (mockito vs wiremock)
- Any specific error scenarios to prioritize?
- Should we create shared test utilities first?
