# Reliability Patterns

Building resilient automation with circuit breakers, retries, and health monitoring.

---

## Table of Contents

1. [Overview](#overview)
2. [Circuit Breaker Pattern](#circuit-breaker-pattern)
3. [Retry with Backoff](#retry-with-backoff)
4. [Health Monitoring](#health-monitoring)
5. [Provider Fallback](#provider-fallback)
6. [Failure Scenarios](#failure-scenarios)
7. [Configuration](#configuration)
8. [Best Practices](#best-practices)

---

## Overview

Reliability in automation means your system continues working even when individual components fail.

**Key Principles:**

1. **Expect failures** - Everything fails eventually
2. **Detect quickly** - Monitor health continuously
3. **Respond gracefully** - Fail over, don't crash
4. **Recover automatically** - Self-healing where possible

---

## Circuit Breaker Pattern

### Purpose

Prevents cascading failures by stopping requests to failing providers.

### How It Works

```
Request → Check Circuit → [CLOSED: Allow] → Execute → Record Result
                          [OPEN: Block] → Use Backup
                          [HALF_OPEN: Test] → Limited Allow
```

### States

| State | Behavior | When |
|-------|----------|------|
| CLOSED | Normal operation | Failure rate < threshold |
| OPEN | Blocking requests | Failure rate >= threshold |
| HALF_OPEN | Testing recovery | After timeout period |

### Usage

```javascript
import CircuitBreaker from './api/core/circuit-breaker.js';

const breaker = new CircuitBreaker({
    failureThreshold: 50,
    successThreshold: 2,
    halfOpenTime: 30000
});

// Execute with protection
try {
    const result = await breaker.execute('provider-id', async () => {
        return await callProvider();
    });
} catch (error) {
    if (error.code === 'CIRCUIT_OPEN') {
        return await callBackup();
    }
    throw error;
}
```

See [`docs/CIRCUIT-BREAKER.md`](CIRCUIT-BREAKER.md) for complete guide.

---

## Retry with Backoff

### Purpose

Handles transient failures by retrying with increasing delays.

### Exponential Backoff with Jitter

```
Attempt 1: Fail → Wait 1s + jitter
Attempt 2: Fail → Wait 2s + jitter  
Attempt 3: Fail → Wait 4s + jitter
Attempt 4: Success
```

**Jitter** prevents thundering herd by randomizing delays.

### Usage

```javascript
import { withRetry, withProviderFallback } from './api/utils/retry.js';

// Basic retry with backoff
const result = await withRetry(
    () => callAPI(),
    {
        retries: 3,
        delay: 1000,
        factor: 2
    }
);

// Retry with circuit breaker integration
const result = await withRetry(
    () => callProvider(provider),
    {
        retries: 3,
        circuitBreaker: breaker,
        providerId: provider
    }
);

// Retry with provider fallback
const result = await withProviderFallback(
    [
        () => callProvider('primary'),
        () => callProvider('backup1'),
        () => callProvider('backup2')
    ],
    { retries: 2 }
);
```

### When NOT to Retry

| Error Type | Retry? | Reason |
|------------|--------|--------|
| Network timeout | Yes | Transient |
| Rate limit (429) | Yes | Temporary |
| Server error (500) | Yes | May recover |
| Invalid request (400) | No | Won't fix itself |
| Authentication (401) | No | Credentials won't change |
| Not found (404) | No | Resource doesn't exist |

---

## Health Monitoring

### Purpose

Provides real-time visibility into system health for proactive issue detection.

### Usage

```javascript
import { healthMonitor, getHealth, isProviderHealthy } from './api/core/health-monitor.js';

// Get current health
const health = getHealth();
console.log(`Overall: ${health.overall}`);

// Check specific provider
if (isProviderHealthy('anthropic/claude-3.5')) {
    await callProvider('anthropic/claude-3.5');
} else {
    await callProvider('openai/gpt-4');
}

// Get recommended provider
const recommended = healthMonitor.getRecommendedProvider();
console.log(`Recommended: ${recommended}`);
```

### Health Status

| Status | Meaning | Action |
|--------|---------|--------|
| healthy | All systems operational | Normal operation |
| degraded | Some issues detected | Monitor closely |
| unhealthy | Critical issues | Immediate action |

### Health Endpoint

```javascript
// Express endpoint
app.get('/api/health', (req, res) => {
    const health = healthMonitor.toJSON();
    res.json(health);
});
```

See [`docs/HEALTH-MONITORING.md`](HEALTH-MONITORING.md) for complete guide.

---

## Provider Fallback

### Purpose

Automatically switch to backup providers when primary fails.

### Implementation

```javascript
import { withProviderFallback } from './api/utils/retry.js';

// Define provider chain
const providers = [
    () => callProvider('primary'),
    () => callProvider('backup1'),
    () => callProvider('backup2')
];

// Execute with automatic fallback
const result = await withProviderFallback(providers, {
    retries: 2
});
```

### Health-Based Selection

```javascript
// Select provider based on health
async function selectProvider() {
    const recommended = healthMonitor.getRecommendedProvider();
    if (recommended) {
        return recommended;
    }
    return 'default-provider';
}
```

---

## Failure Scenarios

### Scenario 1: LLM Provider Down

**Detection:**
- Circuit breaker failure rate exceeds threshold
- Health monitor marks provider as unhealthy

**Response:**
```javascript
if (!isProviderHealthy(currentProvider)) {
    const backup = healthMonitor.getRecommendedProvider();
    if (backup) {
        await switchProvider(backup);
    }
}
```

**Recovery:**
- Circuit breaker transitions to HALF_OPEN after timeout
- Limited requests test provider health
- On success, circuit closes, normal operation resumes

### Scenario 2: Browser Connection Lost

**Detection:**
- Browser circuit breaker opens
- Connection health check fails

**Response:**
```javascript
try {
    await browserBreaker.execute('profile-1', async () => {
        return await performAutomation();
    });
} catch (error) {
    await performAutomation('profile-2');
}
```

### Scenario 3: Rate Limit Exceeded

**Detection:**
- HTTP 429 response
- Rate limit error code

**Response:**
```javascript
const result = await withRetry(
    () => callAPI(),
    {
        retries: 5,
        delay: 2000,
        factor: 2
    }
);
```

---

## Configuration

### Complete Reliability Config

```json
{
  "circuitBreaker": {
    "failureThreshold": 50,
    "successThreshold": 2,
    "halfOpenTime": 30000,
    "monitoringWindow": 60000,
    "minSamples": 5
  },
  "retry": {
    "retries": 3,
    "delay": 1000,
    "factor": 2,
    "maxDelay": 30000,
    "jitterMin": 0.5,
    "jitterMax": 1.5
  },
  "healthMonitoring": {
    "enabled": true,
    "checkInterval": 30000,
    "maxHistory": 100
  }
}
```

### Environment-Specific Settings

**Production:**
```json
{
  "circuitBreaker": { "failureThreshold": 30, "halfOpenTime": 60000 },
  "retry": { "retries": 5, "delay": 2000 },
  "healthMonitoring": { "checkInterval": 15000 }
}
```

**Development:**
```json
{
  "circuitBreaker": { "failureThreshold": 70, "halfOpenTime": 15000 },
  "retry": { "retries": 2, "delay": 500 },
  "healthMonitoring": { "checkInterval": 60000 }
}
```

---

## Best Practices

### 1. Always Have Backups

```javascript
// Good: Multiple providers
const providers = ['primary', 'backup1', 'backup2'];

// Bad: Single point of failure
const provider = 'only-provider';
```

### 2. Monitor Continuously

```javascript
// Start health monitoring on startup
healthMonitor.startPeriodicChecks();
healthMonitor.checkInterval = 30000;
```

### 3. Log Circuit Events

```javascript
breaker.onStateChange = (model, oldState, newState) => {
    logger.info(`Circuit ${model}: ${oldState} → ${newState}`);
};
```

### 4. Test Failure Scenarios

```javascript
// Chaos testing: randomly fail providers
if (Math.random() < 0.1) {
    throw new Error('Simulated failure');
}
```

### 5. Track Reliability Metrics

```javascript
import { retryStats } from './api/utils/retry.js';

const stats = retryStats.getStats();
console.log(`Success rate: ${stats.successRate}`);
console.log(`Average retries: ${stats.averageRetries}`);
```

### 6. Use Health for Routing

```javascript
// Smart routing based on health
async function callLLM(prompt) {
    const recommended = healthMonitor.getRecommendedProvider();
    if (recommended) {
        return await callProvider(recommended, prompt);
    }
    return await callProvider('default', prompt);
}
```

---

## Related Documentation

- [`docs/CIRCUIT-BREAKER.md`](CIRCUIT-BREAKER.md) - Circuit breaker guide
- [`docs/HEALTH-MONITORING.md`](HEALTH-MONITORING.md) - Health monitoring
- [`docs/ERROR-HANDLING.md`](ERROR-HANDLING.md) - Error handling patterns
- [`api/utils/retry.js`](../api/utils/retry.js) - Enhanced retry
- [`api/core/health-monitor.js`](../api/core/health-monitor.js) - Health monitor

---

*Last updated: 2026-03-31*
