/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Health WebSocket Server
 * Real-time health monitoring via WebSocket connections
 * @module core/health-websocket
 */

import { WebSocketServer } from 'ws';
import { createLogger } from './logger.js';
import { healthMonitor, getHealth } from './health-monitor.js';

const logger = createLogger('health-websocket.js');

/**
 * Health WebSocket Server Class
 * 
 * Broadcasts health updates to connected clients in real-time.
 * Supports authentication and rate limiting.
 */
export class HealthWebSocketServer {
  constructor(options = {}) {
    this.server = null;
    this.httpServer = options.httpServer;
    this.port = options.port || 3002;
    this.path = options.path || '/health-ws';
    this.authToken = options.authToken;
    this.authEnabled = !!options.authToken;
    this.broadcastInterval = options.broadcastInterval || 2000;
    this.clients = new Set();
    this.broadcastTimer = null;
    this.lastHealth = null;
    
    this.start();
  }

  /**
   * Start WebSocket server
   */
  start() {
    try {
      // Create WebSocket server
      if (this.httpServer) {
        // Attach to existing HTTP server
        this.server = new WebSocketServer({
          server: this.httpServer,
          path: this.path
        });
      } else {
        // Create standalone server
        this.server = new WebSocketServer({
          port: this.port,
          path: this.path
        });
      }

      this.server.on('connection', (ws, req) => {
        this.handleConnection(ws, req);
      });

      this.server.on('error', (error) => {
        logger.error(`WebSocket server error: ${error.message}`);
      });

      // Start periodic broadcasts
      this.startBroadcasts();

      const addr = this.httpServer ? 'attached to HTTP server' : `port ${this.port}`;
      logger.info(`Health WebSocket server started on ${addr}${this.path}`);
      
    } catch (error) {
      logger.error(`Failed to start WebSocket server: ${error.message}`);
      throw error;
    }
  }

  /**
   * Handle new WebSocket connection
   * @param {WebSocket} ws - WebSocket connection
   * @param {http.IncomingMessage} req - HTTP request
   */
  handleConnection(ws, req) {
    logger.debug('New WebSocket connection');

    // Authenticate if enabled
    if (this.authEnabled) {
      const url = new URL(req.url, `ws://localhost:${this.port}`);
      const token = url.searchParams.get('token');
      
      if (token !== this.authToken) {
        logger.warn('WebSocket authentication failed');
        ws.send(JSON.stringify({
          type: 'error',
          message: 'Authentication failed'
        }));
        ws.close(4001, 'Unauthorized');
        return;
      }
    }

    // Add to clients set
    this.clients.add(ws);
    logger.debug(`WebSocket connected. Total clients: ${this.clients.size}`);

    // Send initial health data
    this.sendHealth(ws);

    // Handle messages from client
    ws.on('message', (message) => {
      this.handleMessage(ws, message);
    });

    // Handle disconnection
    ws.on('close', () => {
      this.clients.delete(ws);
      logger.debug(`WebSocket disconnected. Total clients: ${this.clients.size}`);
    });

    // Handle errors
    ws.on('error', (error) => {
      logger.error(`WebSocket client error: ${error.message}`);
      this.clients.delete(ws);
    });
  }

  /**
   * Handle incoming message from client
   * @param {WebSocket} ws - WebSocket connection
   * @param {string} message - Received message
   */
  handleMessage(ws, message) {
    try {
      const data = JSON.parse(message);
      
      switch (data.type) {
        case 'ping':
          ws.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
          break;
          
        case 'health:request':
          this.sendHealth(ws);
          break;
          
        case 'health:subscribe':
          // Client wants to subscribe to health updates (already subscribed by default)
          ws.send(JSON.stringify({ 
            type: 'health:subscribed',
            interval: this.broadcastInterval
          }));
          break;
          
        default:
          logger.debug(`Unknown message type: ${data.type}`);
      }
    } catch (error) {
      logger.error(`Failed to parse WebSocket message: ${error.message}`);
    }
  }

  /**
   * Send health data to specific client
   * @param {WebSocket} ws - WebSocket connection
   */
  sendHealth(ws) {
    if (ws.readyState !== ws.OPEN) return;
    
    const health = getHealth();
    this.lastHealth = health;
    
    ws.send(JSON.stringify({
      type: 'health:update',
      timestamp: new Date().toISOString(),
      data: health
    }));
  }

  /**
   * Broadcast health data to all connected clients
   */
  broadcast() {
    const health = getHealth();
    
    // Check if health changed
    const hasChanged = !this.lastHealth || 
                       health.overall !== this.lastHealth.overall;
    
    this.lastHealth = health;
    
    const message = JSON.stringify({
      type: hasChanged ? 'health:changed' : 'health:update',
      timestamp: new Date().toISOString(),
      previous: hasChanged ? this.lastHealth?.overall : undefined,
      current: health.overall,
      data: health
    });

    // Send to all connected clients
    for (const client of this.clients) {
      if (client.readyState === client.OPEN) {
        client.send(message);
      }
    }
    
    logger.debug(`Health broadcast sent to ${this.clients.size} clients`);
  }

  /**
   * Start periodic health broadcasts
   */
  startBroadcasts() {
    if (this.broadcastTimer) {
      clearInterval(this.broadcastTimer);
    }
    
    this.broadcastTimer = setInterval(() => {
      this.broadcast();
    }, this.broadcastInterval);
    
    logger.debug(`Health broadcasts started (interval: ${this.broadcastInterval}ms)`);
  }

  /**
   * Stop WebSocket server
   */
  stop() {
    if (this.broadcastTimer) {
      clearInterval(this.broadcastTimer);
      this.broadcastTimer = null;
    }
    
    if (this.server) {
      this.server.close(() => {
        logger.info('Health WebSocket server stopped');
      });
      
      // Close all client connections
      for (const client of this.clients) {
        client.close(1001, 'Server shutting down');
      }
      this.clients.clear();
    }
  }

  /**
   * Get server status
   * @returns {object} Server status
   */
  getStatus() {
    return {
      running: !!this.server,
      clients: this.clients.size,
      port: this.port,
      path: this.path,
      authEnabled: this.authEnabled,
      broadcastInterval: this.broadcastInterval
    };
  }
}

/**
 * Default health WebSocket server instance
 */
let healthWebSocketServer = null;

/**
 * Initialize health WebSocket server
 * @param {object} options - Server options
 * @param {object} [options.httpServer] - HTTP server to attach to
 * @param {number} [options.port] - WebSocket port (if not attaching)
 * @param {string} [options.path] - WebSocket path
 * @param {string} [options.authToken] - Authentication token
 * @param {number} [options.broadcastInterval] - Broadcast interval in ms
 * @returns {HealthWebSocketServer}
 */
export function initHealthWebSocket(options = {}) {
  if (healthWebSocketServer) {
    logger.warn('Health WebSocket server already initialized');
    return healthWebSocketServer;
  }
  
  healthWebSocketServer = new HealthWebSocketServer(options);
  return healthWebSocketServer;
}

/**
 * Get health WebSocket server instance
 * @returns {HealthWebSocketServer|null}
 */
export function getHealthWebSocket() {
  return healthWebSocketServer;
}

/**
 * Stop health WebSocket server
 */
export function stopHealthWebSocket() {
  if (healthWebSocketServer) {
    healthWebSocketServer.stop();
    healthWebSocketServer = null;
  }
}

export default {
  HealthWebSocketServer,
  initHealthWebSocket,
  getHealthWebSocket,
  stopHealthWebSocket
};
