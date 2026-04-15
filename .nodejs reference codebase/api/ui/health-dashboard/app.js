/**
 * Auto-AI Health Dashboard Application
 * Real-time health monitoring via WebSocket
 */

// Configuration
const WS_URL = `ws://${window.location.hostname}:3002/health-ws`;
const HTTP_URL = `http://${window.location.hostname}:3001/api/health`;
const ALERTS_URL = `http://${window.location.hostname}:3001/api/health/alerts`;
const RECONNECT_DELAY = 3000;
const POLL_INTERVAL = 5000;

// State
let ws = null;
let healthHistory = [];
let alerts = [];
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
let isPolling = false;
let pollTimer = null;
let reconnectTimer = null;

// DOM Elements
const elements = {
  connectionStatus: document.getElementById('connectionStatus'),
  overallStatus: document.getElementById('overallStatus'),
  lastUpdated: document.getElementById('lastUpdated'),
  providersGrid: document.getElementById('providersGrid'),
  browsersGrid: document.getElementById('browsersGrid'),
  memoryUsage: document.getElementById('memoryUsage'),
  memoryBar: document.getElementById('memoryBar'),
  uptime: document.getElementById('uptime'),
  healthChart: document.getElementById('healthChart'),
  alertsContainer: document.getElementById('alertsContainer'),
};

/**
 * Initialize dashboard
 */
function init() {
  console.log('Initializing Health Dashboard...');
  connectWebSocket();
  updateConnectionStatus('connecting');
}

/**
 * Connect to WebSocket server
 */
function connectWebSocket() {
  try {
    ws = new WebSocket(WS_URL);
    
    ws.onopen = () => {
      console.log('WebSocket connected');
      reconnectAttempts = 0;
      isPolling = false;
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
      updateConnectionStatus('connected');
      
      // Subscribe to health updates
      ws.send(JSON.stringify({ type: 'health:subscribe' }));
      
      // Fetch initial alerts
      fetchAlerts();
    };
    
    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        handleMessage(message);
      } catch (error) {
        console.error('Failed to parse message:', error);
      }
    };
    
    ws.onclose = () => {
      console.log('WebSocket closed');
      updateConnectionStatus('disconnected');
      startReconnect();
    };
    
    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      updateConnectionStatus('disconnected');
    };
    
  } catch (error) {
    console.error('Failed to create WebSocket:', error);
    startHttpPolling();
  }
}

/**
 * Start reconnection attempts
 */
function startReconnect() {
  if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
    reconnectAttempts++;
    console.log(`Reconnecting in ${RECONNECT_DELAY}ms (attempt ${reconnectAttempts})...`);
    reconnectTimer = setTimeout(connectWebSocket, RECONNECT_DELAY);
  } else {
    console.error('Max reconnection attempts reached, switching to HTTP polling');
    startHttpPolling();
  }
}

/**
 * Start HTTP polling fallback
 */
function startHttpPolling() {
  if (isPolling) return;
  
  isPolling = true;
  updateConnectionStatus('polling');
  console.log('Starting HTTP polling fallback...');
  
  // Fetch immediately
  fetchHealth();
  fetchAlerts();
  
  // Then poll at interval
  pollTimer = setInterval(() => {
    fetchHealth();
    fetchAlerts();
  }, POLL_INTERVAL);
}

/**
 * Stop HTTP polling
 */
function stopHttpPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
  isPolling = false;
}

/**
 * Update connection status UI
 */
function updateConnectionStatus(status) {
  const dot = elements.connectionStatus.querySelector('.status-dot');
  const text = elements.connectionStatus.querySelector('.status-text');
  
  dot.className = 'status-dot';
  
  switch (status) {
    case 'connected':
      dot.classList.add('connected');
      text.textContent = 'Connected (WebSocket)';
      break;
    case 'polling':
      dot.classList.add('polling');
      text.textContent = 'Connected (HTTP Polling)';
      break;
    case 'disconnected':
      dot.classList.add('disconnected');
      text.textContent = 'Disconnected';
      break;
    case 'connecting':
    default:
      text.textContent = 'Connecting...';
      break;
  }
}

/**
 * Fetch alerts from API
 */
async function fetchAlerts() {
  try {
    const response = await fetch(ALERTS_URL);
    if (response.ok) {
      const data = await response.json();
      if (data.alerts) {
        alerts = data.alerts;
        updateAlerts();
      }
    }
  } catch (error) {
    // Alerts endpoint may not exist yet, silently fail
    console.debug('Alerts fetch failed (endpoint may not exist):', error.message);
  }
}

/**
 * Handle WebSocket message
 */
function handleMessage(message) {
  switch (message.type) {
    case 'health:update':
    case 'health:changed':
      updateDashboard(message.data);
      addToHistory(message.data);
      
      if (message.type === 'health:changed' && message.previous) {
        addAlert(message.previous, message.current);
      }
      break;
      
    case 'health:subscribed':
      console.log('Subscribed to health updates (interval:', message.interval, 'ms)');
      break;
      
    case 'error':
      console.error('Server error:', message.message);
      addAlert('error', message.message);
      break;
  }
}

/**
 * Update dashboard with health data
 */
function updateDashboard(health) {
  // Update overall status
  updateOverallStatus(health.overall);
  
  // Update last updated time
  elements.lastUpdated.textContent = new Date(health.timestamp).toLocaleTimeString();
  
  // Update providers
  updateProviders(health.providers || health.circuitBreakers);
  
  // Update browsers
  updateBrowsers(health.browsers);
  
  // Update system resources
  updateSystem(health.system);
  
  // Update chart
  updateChart();
}

/**
 * Update overall status badge
 */
function updateOverallStatus(status) {
  elements.overallStatus.className = `status-badge ${status}`;
  
  const icons = {
    healthy: '🟢',
    degraded: '🟡',
    unhealthy: '🔴',
    unknown: '⚪'
  };
  
  const labels = {
    healthy: 'Healthy',
    degraded: 'Degraded',
    unhealthy: 'Unhealthy',
    unknown: 'Unknown'
  };
  
  elements.overallStatus.innerHTML = `
    <span class="status-icon">${icons[status] || '⚪'}</span>
    <span class="status-label">${labels[status] || 'Unknown'}</span>
  `;
}

/**
 * Update providers grid
 */
function updateProviders(providers) {
  if (!providers || Object.keys(providers).length === 0) {
    elements.providersGrid.innerHTML = `
      <div class="card">
        <div class="card-header">
          <span class="card-title">No providers configured</span>
        </div>
      </div>
    `;
    return;
  }
  
  elements.providersGrid.innerHTML = Object.entries(providers)
    .map(([name, data]) => createProviderCard(name, data))
    .join('');
}

/**
 * Create provider card HTML
 */
function createProviderCard(name, data) {
  const status = data.status || data.state?.toLowerCase() || 'unknown';
  const icons = {
    healthy: '🟢',
    degraded: '🟡',
    unhealthy: '🔴',
    unknown: '⚪'
  };
  
  return `
    <div class="card">
      <div class="card-header">
        <span class="card-title">${name}</span>
        <span class="card-status ${status}">
          ${icons[status] || '⚪'} ${capitalize(status)}
        </span>
      </div>
      <div class="card-body">
        <div class="card-stat">
          <div class="card-stat-label">Failure Rate</div>
          <div class="card-stat-value">${data.failureRate || 'N/A'}</div>
        </div>
        <div class="card-stat">
          <div class="card-stat-label">State</div>
          <div class="card-stat-value">${data.state || 'N/A'}</div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Update browsers grid
 */
function updateBrowsers(browsers) {
  if (!browsers || Object.keys(browsers).length === 0) {
    elements.browsersGrid.innerHTML = `
      <div class="card">
        <div class="card-header">
          <span class="card-title">No browsers connected</span>
        </div>
      </div>
    `;
    return;
  }
  
  elements.browsersGrid.innerHTML = Object.entries(browsers)
    .map(([name, data]) => createBrowserCard(name, data))
    .join('');
}

/**
 * Create browser card HTML
 */
function createBrowserCard(name, data) {
  const status = data.status || data.state?.toLowerCase() || 'unknown';
  const icons = {
    healthy: '🟢',
    degraded: '🟡',
    unhealthy: '🔴',
    unknown: '⚪'
  };
  
  return `
    <div class="card">
      <div class="card-header">
        <span class="card-title">${name}</span>
        <span class="card-status ${status}">
          ${icons[status] || '⚪'} ${capitalize(status)}
        </span>
      </div>
      <div class="card-body">
        <div class="card-stat">
          <div class="card-stat-label">Failures</div>
          <div class="card-stat-value">${data.details?.failures || data.failures || 0}</div>
        </div>
        <div class="card-stat">
          <div class="card-stat-label">Successes</div>
          <div class="card-stat-value">${data.details?.successes || data.successes || 0}</div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Update system resources
 */
function updateSystem(system) {
  if (!system) return;
  
  // Update memory
  if (system.memory) {
    elements.memoryUsage.textContent = system.memory.usagePercent;
    const percent = parseFloat(system.memory.usagePercent) || 0;
    elements.memoryBar.style.width = `${Math.min(percent, 100)}%`;
    
    // Update bar color based on usage
    elements.memoryBar.className = 'progress-fill';
    if (percent > 90) {
      elements.memoryBar.classList.add('critical');
    } else if (percent > 70) {
      elements.memoryBar.classList.add('warning');
    }
  }
  
  // Update uptime
  if (system.uptime) {
    elements.uptime.textContent = formatUptime(system.uptime);
  }
}

/**
 * Format uptime string
 */
function formatUptime(uptimeStr) {
  const seconds = parseInt(uptimeStr) || 0;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

/**
 * Add health data to history
 */
function addToHistory(health) {
  healthHistory.push({
    timestamp: Date.now(),
    status: health.overall
  });
  
  // Keep only last hour (assuming 2-second intervals = 1800 data points)
  const oneHourAgo = Date.now() - 3600000;
  healthHistory = healthHistory.filter(h => h.timestamp > oneHourAgo);
  
  if (healthHistory.length > 1800) {
    healthHistory = healthHistory.slice(-1800);
  }
}

/**
 * Update health chart
 */
function updateChart() {
  if (healthHistory.length < 2) return;
  
  const svg = elements.healthChart;
  const width = svg.clientWidth || 800;
  const height = svg.clientHeight || 150;
  const padding = 10;
  
  // Map status to Y values
  const statusY = {
    healthy: padding,
    degraded: height / 2,
    unhealthy: height - padding,
    unknown: height / 2
  };
  
  // Create path
  const points = healthHistory.map((h, i) => {
    const x = padding + (i / (healthHistory.length - 1)) * (width - 2 * padding);
    const y = statusY[h.status] || statusY.unknown;
    return `${x},${y}`;
  });
  
  // Create SVG content
  svg.innerHTML = `
    <polyline
      points="${points.join(' ')}"
      fill="none"
      stroke="var(--accent-blue)"
      stroke-width="2"
    />
    <rect x="${padding}" y="${padding}" width="${width - 2*padding}" height="${height/3}" 
          fill="rgba(34, 197, 94, 0.1)" />
    <rect x="${padding}" y="${height/3}" width="${width - 2*padding}" height="${height/3}" 
          fill="rgba(234, 179, 8, 0.1)" />
    <rect x="${padding}" y="${2*height/3}" width="${width - 2*padding}" height="${height/3}" 
          fill="rgba(239, 68, 68, 0.1)" />
  `;
}

/**
 * Add alert for health change
 */
function addAlert(previous, current) {
  const alert = {
    id: Date.now(),
    previous,
    current,
    timestamp: new Date()
  };
  
  alerts.unshift(alert);
  
  // Keep only last 20 alerts
  if (alerts.length > 20) {
    alerts = alerts.slice(0, 20);
  }
  
  updateAlerts();
}

/**
 * Update alerts display
 */
function updateAlerts() {
  if (alerts.length === 0) {
    elements.alertsContainer.innerHTML = '<div class="alert-info">No recent alerts</div>';
    return;
  }

  elements.alertsContainer.innerHTML = alerts.map(alert => {
    const severity = alert.current === 'unhealthy' ? 'critical' : 'warning';
    const icon = alert.current === 'unhealthy' ? '🔴' : '🟡';

    return `
      <div class="alert ${severity}">
        <div class="alert-icon">${icon}</div>
        <div class="alert-content">
          <div class="alert-message">
            Status changed: ${capitalize(alert.previous)} → ${capitalize(alert.current)}
          </div>
          <div class="alert-time">${alert.timestamp.toLocaleTimeString()}</div>
        </div>
      </div>
    `;
  }).join('');
}

/**
 * Fetch health data via HTTP
 */
async function fetchHealth() {
  try {
    const response = await fetch(HTTP_URL);
    const health = await response.json();
    updateDashboard(health);
    addToHistory(health);
  } catch (error) {
    console.error('HTTP health fetch failed:', error);
  }
}

/**
 * Utility: Capitalize first letter
 */
function capitalize(str) {
  if (!str) return '';
  return str.charAt(0).toUpperCase() + str.slice(1);
}

/**
 * Clean up on page unload
 */
window.addEventListener('beforeunload', () => {
  if (ws) {
    ws.close();
  }
  stopHttpPolling();
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
  }
});

// Initialize on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
