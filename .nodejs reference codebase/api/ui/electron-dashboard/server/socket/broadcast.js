/**
 * Broadcast management for dashboard server
 */

import { createLogger } from "../../lib/logger.js";
import { quickHash } from "../utils/hashing.js";
import { getSystemMetrics } from "../utils/metrics.js";
import os from "os";

// Import health monitoring
import { getHealth } from "../../../../core/health-monitor.js";

const logger = createLogger("server/socket/broadcast.js");

/**
 * Broadcast manager for handling periodic metrics broadcasting.
 */
export class BroadcastManager {
  constructor(options = {}) {
    this.io = options.io;
    this.server = options.server;
    this.BROADCAST_MS = options.broadcastIntervalMs || 2000;
    this.SESSION_TTL_MS = options.sessionTTL || 300000;

    this.broadcastInterval = null;
    this.consecutiveErrors = 0;
    this.maxConsecutiveErrors = 5;
    this.broadcastPaused = false;
    this.lastBroadcastHashes = null;
    this.lastActiveCheck = Date.now();
    this.lastCpuInfo = null;

    // Initialize CPU info
    this.initializeCpuInfo();
  }

  /**
   * Initialize CPU info for delta calculations.
   */
  initializeCpuInfo() {
    const initialCpus = os.cpus();
    let initialIdle = 0,
      initialTotal = 0;
    for (const cpu of initialCpus) {
      for (const type in cpu.times) {
        initialTotal += cpu.times[type];
      }
      initialIdle += cpu.times.idle;
    }
    this.lastCpuInfo = { idle: initialIdle, total: initialTotal };
  }

  /**
   * Start the broadcast interval.
   */
  start() {
    if (this.broadcastInterval) {
      clearInterval(this.broadcastInterval);
    }

    // Reset error tracking on fresh start
    this.consecutiveErrors = 0;
    this.broadcastPaused = false;
    this.lastBroadcastHashes = null;

    this.broadcastInterval = setInterval(() => {
      // Circuit breaker: stop if too many consecutive errors
      if (this.broadcastPaused) {
        return;
      }

      const clientCount = this.io?.sockets?.sockets?.size || 0;
      if (clientCount > 0) {
        try {
          const metrics = this.collectMetrics();

          // Check for error in metrics
          if (metrics.error) {
            this.consecutiveErrors++;
            logger.warn(
              `Broadcast error (${this.consecutiveErrors}/${this.maxConsecutiveErrors}): ${metrics.error}`,
            );

            if (this.consecutiveErrors >= this.maxConsecutiveErrors) {
              this.broadcastPaused = true;
              logger.error(
                "Broadcast paused due to repeated errors. Will resume on next client connection.",
              );
            }
            return;
          }

          // Reset error counter on success
          this.consecutiveErrors = 0;

          // Efficient change detection using hashes
          const currentHash = {
            sessions: quickHash(metrics.sessions),
            queue: quickHash(metrics.queue),
            recentTasks: quickHash(metrics.recentTasks),
          };

          const hasChanged =
            !this.lastBroadcastHashes ||
            currentHash.sessions !== this.lastBroadcastHashes.sessions ||
            currentHash.queue !== this.lastBroadcastHashes.queue ||
            currentHash.recentTasks !== this.lastBroadcastHashes.recentTasks;

          if (hasChanged) {
            this.io.emit("metrics", metrics);
            this.lastBroadcastHashes = currentHash;
          }
        } catch (err) {
          this.consecutiveErrors++;
          logger.error(
            `Broadcast exception (${this.consecutiveErrors}/${this.maxConsecutiveErrors}):`,
            err.message,
          );

          if (this.consecutiveErrors >= this.maxConsecutiveErrors) {
            this.broadcastPaused = true;
            logger.error("Broadcast paused due to repeated exceptions.");
          }
        }
      } else {
        clearInterval(this.broadcastInterval);
        this.broadcastInterval = null;
      }
    }, this.BROADCAST_MS);

    logger.info(`Broadcast interval set to ${this.BROADCAST_MS}ms`);
  }

  /**
   * Resume broadcast after it was paused due to errors.
   */
  resume() {
    if (this.broadcastPaused) {
      this.broadcastPaused = false;
      this.consecutiveErrors = 0;
      logger.info("Broadcast resumed after error recovery");
    }
  }

  /**
   * Stop the broadcast interval.
   */
  stop() {
    if (this.broadcastInterval) {
      clearInterval(this.broadcastInterval);
      this.broadcastInterval = null;
      logger.info("Broadcast stopped (no clients connected)");
    }
  }

  /**
   * Collect metrics for broadcasting.
   * @returns {Object} - Metrics object
   */
  collectMetrics() {
    try {
      const now = Date.now();

      // Clean up stale sessions
      if (
        this.SESSION_TTL_MS > 0 &&
        this.server.dashboardData.sessions.length > 0
      ) {
        const cutoff = now - this.SESSION_TTL_MS;
        const beforeCount = this.server.dashboardData.sessions.length;
        this.server.dashboardData.sessions =
          this.server.dashboardData.sessions.filter((s) => {
            const lastSeen = s.lastSeen || s.firstSeen || 0;
            return lastSeen > cutoff;
          });
        if (this.server.dashboardData.sessions.length < beforeCount) {
          logger.debug(
            `Cleaned up ${beforeCount - this.server.dashboardData.sessions.length} stale sessions`,
          );
        }
      }

      const elapsed = now - this.lastActiveCheck;
      this.lastActiveCheck = now;

      // Only increment uptime if there's an active session or queue
      const hasOnlineSession =
        this.server.latestMetrics?.sessions?.some(
          (s) => s?.status === "online",
        ) || false;
      const hasActivity =
        hasOnlineSession || this.server.latestMetrics?.queue?.queueLength > 0;

      if (hasActivity) {
        this.server.cumulativeMetrics.engineUptimeMs += elapsed;
        this.server.cumulativeMetrics.sessionUptimeMs += elapsed;
      }

      // Get system metrics
      const systemMetrics = getSystemMetrics(this.lastCpuInfo);
      this.lastCpuInfo = systemMetrics.cpuInfo;
      delete systemMetrics.cpuInfo; // Remove from output

      // Get health data
      const healthData = getHealth();

      const result = {
        timestamp: now,
        ...this.server.latestMetrics,
        cumulative: {
          ...this.server.cumulativeMetrics,
          completedTasks: this.server.historyManager.getCompletedTasksCount(),
        },
        system: systemMetrics,
        health: healthData
      };

      return result;
    } catch (error) {
      logger.error("Error collecting metrics:", error);
      return { error: error.message, timestamp: Date.now() };
    }
  }

  /**
   * Send metrics to a specific socket.
   * @param {Object} socket - Socket to send metrics to
   */
  sendMetrics(socket) {
    try {
      socket.emit("metrics", this.collectMetrics());
    } catch (error) {
      logger.error("Error sending metrics:", error);
    }
  }
}
