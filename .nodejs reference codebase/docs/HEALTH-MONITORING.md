# Health Monitoring Guide

Monitor system health and detect issues before they affect users.

---

## Table of Contents

1. [Overview](#overview)
2. [Health Monitor API](#health-monitor-api)
3. [Health Status Levels](#health-status-levels)
4. [Health Endpoints](#health-endpoints)
5. [Monitoring Providers](#monitoring-providers)
6. [Alerts](#alerts)
7. [Dashboard Integration](#dashboard-integration)
8. [Best Practices](#best-practices)

---

## Overview

The Health Monitor provides real-time visibility into:
- LLM provider health (circuit breaker status)
- Browser connection health
- System resources (memory, uptime)

**Key Benefits:**
- Detect issues before users complain
- Make informed failover decisions
- Track reliability metrics over time

---

## Health Monitor API

### Get Current Health

```javascript
import { healthMonitor, getHealth } from './api/core/health-monitor.js';

// Get full health status
const health = getHealth();
console.log(health);

// Output:
{
  overall: 'healthy',
  timestamp: '2026-03-31T10:00:00.000Z',
  circuitBreakers: {
    'anthropic/claude-3.5': {
      state: 'CLOSED',
      failureRate: '5%',
      status: 'healthy'
    }
  },
  browsers: {
    'profile-1': {
      state: 'CLOSED',
      status: 'healthy'
    }
  },
  system: {
    memory: {
      usagePercent: '29%',
      status: 'healthy'
    }
  }
}
```

### Check Provider Health

```javascript
// Check if specific provider is healthy
const isHealthy = healthMonitor.isProviderHealthy('anthropic/claude-3.5');

// Get recommended provider
const recommended = healthMonitor.getRecommendedProvider();
console.log(`Recommended: ${recommended}`);
```

### Health History

```javascript
// Get last 100 health checks
const history = healthMonitor.getHistory();

// Calculate uptime percentage
const oneHourAgo = Date.now() - 3600000;
const recentHealth = history.filter(h => 
    new Date(h.timestamp).getTime() > oneHourAgo
);
const healthyCount = recentHealth.filter(
    h => h.health.overall === 'healthy'
).length;
const uptimePercent = (healthyCount / recentHealth.length) * 100;
```

---

## Health Status Levels

| Status | Description | Action |
|--------|-------------|--------|
| `healthy` | All systems operational | No action needed |
| `degraded` | Some issues detected | Monitor closely, prepare failover |
| `unhealthy` | Critical issues | Immediate action required |
| `unknown` | No data available | Check monitoring configuration |

---

## Health Endpoints

### Setup

```javascript
import express from 'express';
import { createHealthEndpoints } from './api/core/health-endpoint.js';

const app = express();

// Create health endpoints at /api/health
createHealthEndpoints(app, { basePath: '/api' });
```

### Available Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/health` | Current health status |
| `GET /api/health/history?limit=10` | Health history |
| `GET /api/health/provider/:id` | Provider-specific health |
| `GET /api/health/recommended` | Recommended provider |
| `GET /api/health/summary` | Brief summary for dashboards |

### Example Response

```json
// GET /api/health
{
  "status": "healthy",
  "timestamp": "2026-03-31T10:00:00.000Z",
  "providers": {
    "anthropic/claude-3.5": {
      "state": "CLOSED",
      "failureRate": "5%",
      "status": "healthy"
    }
  },
  "browsers": {
    "profile-1": {
      "state": "CLOSED",
      "status": "healthy"
    }
  },
  "system": {
    "memory": {
      "usagePercent": "29%",
      "status": "healthy"
    }
  },
  "recommendations": []
}
```

### Health Check Middleware

```javascript
import { healthCheckMiddleware } from './api/core/health-endpoint.js';

// Protect critical routes
app.post('/api/automation', 
    healthCheckMiddleware(), 
    runAutomation
);

// Returns 503 if system is unhealthy
```

---

## Monitoring Providers

### Provider Health Thresholds

| Failure Rate | Status | Recommendation |
|--------------|--------|----------------|
| 0-20% | healthy | Normal operation |
| 20-50% | degraded | Monitor closely |
| 50%+ | unhealthy | Switch to backup |
| OPEN circuit | unhealthy | Immediate failover |

### Smart Provider Selection

```javascript
// Select provider based on health
async function selectProvider() {
    const recommended = healthMonitor.getRecommendedProvider();
    
    if (recommended) {
        return recommended;
    }
    
    // Fall back to default
    return 'default-provider';
}
```

---

## Alerts

### Health Change Listener

```javascript
healthMonitor.onHealthChange('alert-system', (previous, current, health) => {
    if (current === 'unhealthy') {
        sendAlert({
            type: 'critical',
            message: `System health degraded: ${previous} → ${current}`,
            health: health
        });
    }
    
    if (previous === 'unhealthy' && current === 'healthy') {
        sendNotification('System recovered to healthy status');
    }
});
```

### Custom Alert Thresholds

```javascript
// Check and alert every minute
setInterval(() => {
    const health = healthMonitor.getHealth();
    
    // Alert on unhealthy providers
    for (const [provider, data] of Object.entries(health.circuitBreakers)) {
        if (data.status === 'unhealthy') {
            logger.error(`ALERT: Provider ${provider} unhealthy`);
        }
    }
    
    // Alert on high memory
    if (parseFloat(health.system.memory.usagePercent) > 90) {
        logger.error('ALERT: Memory usage critical');
    }
}, 60000);
```

---

## Dashboard Integration

### Prometheus Metrics

```javascript
import client from 'prom-client';

const healthGauge = new client.Gauge({
    name: 'autoai_health_status',
    help: 'Current health status (0=healthy, 1=degraded, 2=unhealthy)'
});

const providerFailures = new client.Gauge({
    name: 'autoai_provider_failure_rate',
    help: 'Provider failure rate percentage',
    labelNames: ['provider']
});

// Update metrics
healthMonitor.onHealthChange('metrics', (prev, current, health) => {
    const value = current === 'healthy' ? 0 : 
                  current === 'degraded' ? 1 : 2;
    healthGauge.set(value);
    
    for (const [provider, data] of Object.entries(health.circuitBreakers)) {
        providerFailures.set({ provider }, parseFloat(data.failureRate));
    }
});
```

### Grafana Dashboard

Query examples:

```promql
# Current health status
autoai_health_status

# Provider failure rate over time
autoai_provider_failure_rate{provider="anthropic/claude-3.5"}

# Health uptime (percentage of healthy checks)
avg_over_time(autoai_health_status[1h])
```

---

## Best Practices

### 1. Monitor Continuously

```javascript
// Start on application startup
healthMonitor.startPeriodicChecks();
healthMonitor.checkInterval = 30000; // 30 seconds
```

### 2. Use Health for Routing

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

### 3. Track Health Trends

```javascript
// Calculate uptime over last hour
function getUptimePercent() {
    const history = healthMonitor.getHistory();
    const oneHourAgo = Date.now() - 3600000;
    
    const recentHealth = history.filter(h => 
        new Date(h.timestamp).getTime() > oneHourAgo
    );
    
    const healthyCount = recentHealth.filter(
        h => h.health.overall === 'healthy'
    ).length;
    
    return (healthyCount / recentHealth.length) * 100;
}
```

### 4. Expose Health to Dashboards

```javascript
// Health endpoint for monitoring tools
app.get('/health', (req, res) => {
    const health = healthMonitor.toJSON();
    
    if (health.status === 'unhealthy') {
        res.status(503);
    }
    
    res.json(health);
});
```

### 5. Graceful Degradation

```javascript
// Adjust behavior based on health
async function processAutomation() {
    const health = healthMonitor.getHealth();
    
    if (health.overall === 'unhealthy') {
        // Reduce load, essential operations only
        logger.warn('Operating in reduced capacity');
        await processEssentialOnly();
    } else if (health.overall === 'degraded') {
        // Proceed with extra error handling
        await processWithExtraRetries();
    } else {
        // Normal operation
        await processNormally();
    }
}
```

---

## Troubleshooting

### Health Shows "Unknown"

**Cause:** Monitoring not initialized.

**Solution:**
```javascript
// Ensure circuit breaker is connected
healthMonitor.setCircuitBreaker(circuitBreaker);

// Trigger initial check
healthMonitor.performHealthCheck();
```

### Health Not Updating

**Cause:** Periodic checks stopped.

**Solution:**
```javascript
// Restart periodic checks
healthMonitor.startPeriodicChecks();
healthMonitor.checkInterval = 10000; // 10 seconds
```

---

## Related Documentation

- [`api/core/health-monitor.js`](../api/core/health-monitor.js) - Implementation
- [`api/core/health-endpoint.js`](../api/core/health-endpoint.js) - Health endpoints
- [`docs/CIRCUIT-BREAKER.md`](CIRCUIT-BREAKER.md) - Circuit breaker guide
- [`docs/RELIABILITY.md`](RELIABILITY.md) - Reliability patterns

---

*Last updated: 2026-03-31*
