# Health Dashboard Guide

Real-time health monitoring dashboard for Auto-AI.

---

## Table of Contents

1. [Overview](#overview)
2. [Features](#features)
3. [Quick Start](#quick-start)
4. [Accessing the Dashboard](#accessing-the-dashboard)
5. [Dashboard Sections](#dashboard-sections)
6. [WebSocket Connection](#websocket-connection)
7. [Alerts](#alerts)
8. [Configuration](#configuration)
9. [API Reference](#api-reference)
10. [Troubleshooting](#troubleshooting)

---

## Overview

The Health Dashboard provides real-time visibility into your Auto-AI system health:

- **LLM Provider Status** - Monitor OpenRouter, Ollama, and other LLM providers
- **Browser Connections** - Track browser profile health
- **System Resources** - Memory usage and uptime
- **Real-time Updates** - WebSocket-based live updates
- **Alerts** - Notifications for status changes

---

## Features

| Feature | Description |
|---------|-------------|
| Real-time Updates | WebSocket push updates every 2 seconds |
| Visual Status | Color-coded health indicators (🟢🟡🔴) |
| Provider Cards | Individual status for each LLM provider |
| Browser Monitoring | Connection status for each browser profile |
| Memory Tracking | System memory usage with progress bar |
| Health History | Sparkline chart showing health over time |
| Alerts | Notifications for status changes |
| Responsive Design | Works on desktop and mobile |

---

## Quick Start

### 1. Enable Dashboard in Config

Edit `config/settings.json`:

```json
{
  "ui": {
    "dashboard": {
      "enabled": true,
      "port": 3001,
      "broadcastIntervalMs": 2000
    }
  }
}
```

### 2. Start Auto-AI

```bash
node main.js pageview=example.com
```

### 3. Open Dashboard

Navigate to: `http://localhost:3001/health`

---

## Accessing the Dashboard

### Web Dashboard

**URL:** `http://localhost:3001/health`

**Default Port:** 3001 (configurable in `config/settings.json`)

### WebSocket Connection

**URL:** `ws://localhost:3002/health-ws`

**Authentication:** Append `?token=YOUR_TOKEN` to WebSocket URL

---

## Dashboard Sections

### Overall Status

Shows the current overall system health:

- 🟢 **Healthy** - All systems operational
- 🟡 **Degraded** - Some issues detected
- 🔴 **Unhealthy** - Critical issues
- ⚪ **Unknown** - No data available

### LLM Providers

Displays status for each configured LLM provider:

| Field | Description |
|-------|-------------|
| Provider | Provider name (e.g., `anthropic/claude-3.5`) |
| Status | Current health status |
| Failure Rate | Percentage of failed requests |
| State | Circuit breaker state (CLOSED/OPEN/HALF_OPEN) |

### Browser Connections

Shows status for each connected browser profile:

| Field | Description |
|-------|-------------|
| Profile | Browser profile name |
| Status | Connection health status |
| Failures | Number of connection failures |
| Successes | Number of successful connections |

### System Resources

Displays system resource usage:

- **Memory Usage** - Current memory percentage with progress bar
- **Uptime** - System uptime in hours and minutes

### Health History

Shows health status over the last hour as a sparkline chart:

- Green area = Healthy
- Yellow area = Degraded
- Red area = Unhealthy

### Recent Alerts

Lists recent health alerts:

- Status changes (healthy → degraded)
- Provider failures
- Memory warnings

---

## WebSocket Connection

### Connecting

```javascript
const ws = new WebSocket('ws://localhost:3002/health-ws');

ws.onopen = () => {
  console.log('Connected to health dashboard');
  
  // Subscribe to updates
  ws.send(JSON.stringify({ type: 'health:subscribe' }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch (message.type) {
    case 'health:update':
      console.log('Health update:', message.data);
      break;
    case 'health:changed':
      console.log('Health changed:', message.previous, '→', message.current);
      break;
  }
};
```

### Message Types

| Type | Description |
|------|-------------|
| `health:update` | Regular health update |
| `health:changed` | Health status changed |
| `health:subscribed` | Subscription confirmed |
| `error` | Error message |

### Authentication

If authentication is enabled, include token in URL:

```
ws://localhost:3002/health-ws?token=YOUR_AUTH_TOKEN
```

---

## Alerts

### Alert Types

| Type | Severity | Trigger |
|------|----------|---------|
| `status_change` | Varies | System health status changes |
| `provider_failure` | Critical | Provider failure rate > 50% |
| `provider_warning` | Warning | Provider failure rate > 40% |
| `memory_critical` | Critical | Memory usage > 90% |
| `memory_warning` | Warning | Memory usage > 81% |

### Webhook Integration

Configure webhook URL to receive alerts:

```javascript
import { initHealthAlerts } from './api/core/health-alerts.js';

initHealthAlerts({
  webhookUrl: 'https://your-webhook-url.com/alerts',
  providerFailureRate: 50,
  memoryUsage: 90
});
```

**Webhook Payload:**

```json
{
  "type": "health_alert",
  "alert": {
    "id": "status-1234567890",
    "severity": "critical",
    "message": "System health changed: healthy → unhealthy",
    "timestamp": "2026-03-31T10:00:00.000Z",
    "data": {
      "previous": "healthy",
      "current": "unhealthy"
    }
  }
}
```

---

## Configuration

### Settings (config/settings.json)

```json
{
  "ui": {
    "dashboard": {
      "enabled": true,
      "port": 3001,
      "broadcastIntervalMs": 2000,
      "colorScheme": "dark"
    }
  },
  "health": {
    "websocket": {
      "port": 3002,
      "path": "/health-ws",
      "broadcastInterval": 2000
    },
    "alerts": {
      "providerFailureRate": 50,
      "memoryUsage": 90,
      "consecutiveFailures": 5
    }
  }
}
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DASHBOARD_PORT` | Dashboard HTTP port | 3001 |
| `WEBSOCKET_PORT` | WebSocket port | 3002 |
| `DASHBOARD_AUTH_TOKEN` | Authentication token | None |

---

## API Reference

### REST Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Current health status |
| `/api/health/history` | GET | Health history |
| `/api/health/provider/:id` | GET | Provider-specific health |
| `/api/health/recommended` | GET | Recommended provider |
| `/api/health/summary` | GET | Brief health summary |

### Example: Get Health

```bash
curl http://localhost:3001/api/health
```

**Response:**

```json
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
    "ixbrowser-1": {
      "state": "CLOSED",
      "status": "healthy"
    }
  },
  "system": {
    "memory": {
      "usagePercent": "29%"
    }
  }
}
```

---

## Troubleshooting

### Dashboard Won't Load

**Problem:** Dashboard page doesn't load

**Solutions:**
1. Check if dashboard is enabled in `config/settings.json`
2. Verify port 3001 is not in use
3. Check browser console for errors

### WebSocket Won't Connect

**Problem:** WebSocket shows "Disconnected"

**Solutions:**
1. Verify WebSocket server is running (check logs)
2. Check port 3002 is accessible
3. Try HTTP polling fallback (automatic)

### No Data Showing

**Problem:** Dashboard shows "Loading..." forever

**Solutions:**
1. Check if health-monitor is initialized
2. Verify circuit breaker is connected
3. Check browser console for errors

### Alerts Not Triggering

**Problem:** Alerts don't appear for status changes

**Solutions:**
1. Verify health-alerts module is initialized
2. Check alert thresholds in config
3. Check logs for alert errors

---

## Related Documentation

- [`api/core/health-monitor.js`](../api/core/health-monitor.js) - Health monitoring
- [`api/core/health-websocket.js`](../api/core/health-websocket.js) - WebSocket server
- [`api/core/health-alerts.js`](../api/core/health-alerts.js) - Alert system
- [`api/ui/health-dashboard/`](../api/ui/health-dashboard/) - Dashboard UI
- [`docs/RELIABILITY.md`](RELIABILITY.md) - Reliability patterns

---

*Last updated: 2026-03-31*
