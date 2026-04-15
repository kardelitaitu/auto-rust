# Circuit Breaker Guide

Protect your automation from cascading failures with the circuit breaker pattern.

---

## Table of Contents

1. [What is a Circuit Breaker?](#what-is-a-circuit-breaker)
2. [How It Works](#how-it-works)
3. [Configuration](#configuration)
4. [Usage](#usage)
5. [Monitoring](#monitoring)
6. [Troubleshooting](#troubleshooting)
7. [Best Practices](#best-practices)

---

## What is a Circuit Breaker?

A circuit breaker prevents cascading failures by monitoring provider health and temporarily stopping requests when failure rate exceeds a threshold.

**Analogy:** Like an electrical circuit breaker that trips when too many devices draw power.

**Without Circuit Breaker:**
```
Provider fails → Retry → Fail → Retry → Fail → System Crash
```

**With Circuit Breaker:**
```
Provider fails → Retry → Fail → Circuit opens → Switch to backup → Continue operating
```

---

## How It Works

### Three States

```
CLOSED (Normal) → OPEN (Protecting) → HALF_OPEN (Testing) → CLOSED (Recovered)
```

| State | Behavior | When |
|-------|----------|------|
| **CLOSED** | Normal operation, requests flow through | Failure rate < threshold |
| **OPEN** | Requests blocked immediately, use backup | Failure rate >= threshold |
| **HALF_OPEN** | Limited requests test recovery | After timeout period |

### State Transitions

1. **CLOSED → OPEN**: When failure rate exceeds threshold (default 50%)
2. **OPEN → HALF_OPEN**: After timeout period (default 30 seconds)
3. **HALF_OPEN → CLOSED**: After success threshold met (default 2 successes)
4. **HALF_OPEN → OPEN**: If test request fails

---

## Configuration

### Default Settings

Located in `config/timeouts.json`:

```json
{
  "circuitBreaker": {
    "failureThreshold": 50,
    "successThreshold": 2,
    "halfOpenTime": 30000,
    "monitoringWindow": 60000,
    "minSamples": 5
  }
}
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `failureThreshold` | 50 | Failure % to trip circuit |
| `successThreshold` | 2 | Successes needed to close |
| `halfOpenTime` | 30000 | Wait time before testing (ms) |
| `monitoringWindow` | 60000 | Time window for calculating rate (ms) |
| `minSamples` | 5 | Minimum operations before tripping |

### Recommended Settings

**Production (Conservative):**
```json
{
  "failureThreshold": 30,
  "successThreshold": 3,
  "halfOpenTime": 60000,
  "monitoringWindow": 120000,
  "minSamples": 10
}
```

**Development (Lenient):**
```json
{
  "failureThreshold": 70,
  "successThreshold": 1,
  "halfOpenTime": 15000
}
```

---

## Usage

### Basic Usage

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
        return await callLLM();
    });
} catch (error) {
    if (error.code === 'CIRCUIT_OPEN') {
        // Circuit is open, use backup
        const result = await callBackupProvider();
    }
}
```

### With Retry Integration

```javascript
import { withRetry } from './api/utils/retry.js';

// Retry automatically checks circuit breaker
const result = await withRetry(
    () => callProvider(provider),
    { 
        retries: 3,
        circuitBreaker: breaker,
        providerId: provider
    }
);
```

### Manual Control

```javascript
// Check if allowed
const check = breaker.check('provider-id');
if (check.allowed) {
    // Proceed with request
} else {
    // Use backup - check.retryAfter tells you when to retry
    console.log(`Retry after ${check.retryAfter}ms`);
}

// Record outcomes
breaker.recordSuccess('provider-id');
breaker.recordFailure('provider-id');

// Get health status
const health = breaker.getHealth('provider-id');
console.log(health);
// { status: 'CLOSED', failureRate: 5, recentOperations: 20 }

// Reset breaker
breaker.reset('provider-id');
```

### Provider Fallback

```javascript
import { withProviderFallback } from './api/utils/retry.js';

// Automatically tries providers in order
const result = await withProviderFallback(
    [
        () => callProvider('primary'),
        () => callProvider('backup1'),
        () => callProvider('backup2')
    ],
    { retries: 2 }
);
```

---

## Monitoring

### Get Health Status

```javascript
import { healthMonitor } from './api/core/health-monitor.js';

// Single provider health
const health = healthMonitor.getCircuitBreakerHealth();
console.log(health['anthropic/claude-3.5']);

// All providers
const allHealth = healthMonitor.getCircuitBreakerHealth();
for (const [provider, data] of Object.entries(allHealth)) {
    console.log(`${provider}: ${data.state} (${data.failureRate})`);
}
```

### Health API Endpoint

```javascript
// Express endpoint
import { createHealthEndpoints } from './api/core/health-endpoint.js';

const app = express();
createHealthEndpoints(app, { basePath: '/api' });

// GET /api/health returns:
{
  "status": "healthy",
  "providers": {
    "anthropic/claude-3.5": {
      "state": "CLOSED",
      "failureRate": "5%",
      "status": "healthy"
    }
  }
}
```

### Health Change Listener

```javascript
healthMonitor.onHealthChange('my-app', (previous, current, health) => {
    if (current === 'degraded' || current === 'unhealthy') {
        // Send alert
        sendAlert(`Health degraded: ${previous} → ${current}`);
    }
});
```

---

## Troubleshooting

### Circuit Keeps Opening

**Symptoms:** Frequent "CIRCUIT_OPEN" errors

**Causes:**
1. Provider is actually failing (check logs)
2. Threshold too sensitive
3. Network issues

**Solutions:**
```json
// Increase threshold
{ "failureThreshold": 70, "minSamples": 10 }

// Or increase monitoring window
{ "monitoringWindow": 120000 }
```

### Circuit Never Opens

**Symptoms:** Failures continue without protection

**Solutions:**
```json
// Lower threshold
{ "failureThreshold": 30 }

// Reduce minSamples
{ "minSamples": 3 }
```

### Slow Recovery

**Symptoms:** Circuit stays open too long

**Solutions:**
```json
// Shorten half-open time
{ "halfOpenTime": 15000 }

// Reduce success threshold
{ "successThreshold": 1 }
```

### Common Error: CIRCUIT_OPEN

```javascript
// Error: Circuit breaker OPEN for provider. Retry after 30s

// Solution 1: Use backup provider
if (error.code === 'CIRCUIT_OPEN') {
    return await callBackupProvider();
}

// Solution 2: Wait and retry
if (error.code === 'CIRCUIT_OPEN') {
    await wait(error.retryAfter);
    return await callProvider();
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
// Start health monitoring
healthMonitor.startPeriodicChecks();
healthMonitor.checkInterval = 30000; // 30 seconds
```

### 3. Log Circuit Events

```javascript
breaker.onStateChange = (model, oldState, newState) => {
    logger.info(`Circuit ${model}: ${oldState} → ${newState}`);
};
```

### 4. Tune for Your Use Case

| Use Case | failureThreshold | halfOpenTime |
|----------|-----------------|--------------|
| Critical production | 30% | 60000ms |
| Standard automation | 50% | 30000ms |
| Development | 70% | 15000ms |

### 5. Reset on Deployment

```javascript
// Reset breakers when deploying
breaker.resetAll();
```

---

## Related Documentation

- [`api/core/circuit-breaker.js`](../api/core/circuit-breaker.js) - Implementation
- [`docs/HEALTH-MONITORING.md`](HEALTH-MONITORING.md) - Health monitoring
- [`docs/RELIABILITY.md`](RELIABILITY.md) - Reliability patterns
- [`api/utils/retry.js`](../api/utils/retry.js) - Retry with circuit breaker

---

*Last updated: 2026-03-31*
