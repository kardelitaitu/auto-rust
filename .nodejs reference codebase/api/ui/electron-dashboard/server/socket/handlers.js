/**
 * Socket event handlers for dashboard server
 */

import { createLogger } from "../../lib/logger.js";
import { withAuth } from "../middleware/auth.js";
import { sanitizeObject, sanitizeLogString } from "../utils/sanitization.js";
import { validatePayload, validateTask } from "./validation.js";

const logger = createLogger("server/socket/handlers.js");

/**
 * Setup socket event handlers.
 * @param {Object} options - Handler options
 * @param {Object} options.io - Socket.io server instance
 * @param {Object} options.server - DashboardServer instance
 * @param {boolean} options.authEnabled - Whether authentication is enabled
 * @param {string} options.authToken - The configured auth token
 */
export function setupSocketHandlers(options = {}) {
  const { io, server, authEnabled, authToken } = options;

  io.on("connection", (socket) => {
    // Reset session uptime when Electron first connects
    if (!server.firstClientConnected) {
      server.firstClientConnected = true;
      server.cumulativeMetrics.sessionUptimeMs = 0;
      logger.info("Dashboard session started (Electron connected)");
    }

    // Resume broadcast if it was paused due to errors
    server.resumeBroadcast();

    // Start broadcast if this is the first client
    if (!server.broadcastInterval) {
      server.startBroadcast();
    }

    logger.info(
      `Dashboard client connected (total: ${io?.sockets?.sockets?.size || 0})`,
    );

    // Send initial data immediately
    server.sendMetrics(socket);

    socket.on("disconnect", () => {
      const remainingClients = (io?.sockets?.sockets?.size || 0) - 1;
      logger.info(
        `Dashboard client disconnected (remaining: ${remainingClients})`,
      );
      // Stop broadcast if no clients remain
      if (remainingClients <= 0) {
        server.stopBroadcast();
      }
    });

    socket.on("requestUpdate", () => {
      server.sendMetrics(socket);
    });

    // Support metrics push from Orchestrator via Socket (protected)
    socket.on(
      "push_metrics",
      withAuth(
        "push_metrics",
        (payload) => {
          const sanitized = sanitizeObject(payload);
          const validated = validatePayload(sanitized);
          if (validated) {
            server.updateMetrics(validated);
          } else {
            logger.warn("Received invalid metrics payload, ignoring");
          }
        },
        authEnabled,
        authToken,
      ),
    );

    // Task updates (protected)
    socket.on(
      "task-update",
      withAuth(
        "task-update",
        (data) => {
          const sanitized = sanitizeObject(data);
          const validated = validateTask(sanitized);
          if (validated) {
            server.mergeTaskData(validated);
            server.latestMetrics.recentTasks =
              server.dashboardData.tasks.slice(-40);
            io.emit("metrics", server.collectMetrics());
          } else {
            logger.warn("Received invalid task-update payload, ignoring");
          }
        },
        authEnabled,
        authToken,
      ),
    );

    // Clear history (protected - requires auth when enabled)
    socket.on(
      "clear-history",
      withAuth(
        "clear-history",
        () => {
          logger.info("Clearing dashboard history per client request");
          server.historyManager.clearHistory();

          // Reset all data stores to zeros
          server.dashboardData.tasks = [];
          server.dashboardData.twitterActions = {
            likes: 0,
            retweets: 0,
            replies: 0,
            quotes: 0,
            follows: 0,
            bookmarks: 0,
            total: 0,
          };
          server.dashboardData.apiMetrics = {
            calls: 0,
            failures: 0,
            successRate: 100,
            avgResponseTime: 0,
          };
          server.dashboardData.errors = [];
          server.cumulativeMetrics.completedTasks = 0;

          // Also reset latestMetrics to show zeros
          server.latestMetrics.recentTasks = [];
          if (server.latestMetrics.metrics) {
            server.latestMetrics.metrics.twitter = {
              actions: {
                likes: 0,
                retweets: 0,
                replies: 0,
                quotes: 0,
                follows: 0,
                bookmarks: 0,
                total: 0,
              },
            };
            server.latestMetrics.metrics.api = {
              calls: 0,
              failures: 0,
              successRate: 100,
              avgResponseTime: 0,
            };
          }

          // Reset lastSeenMetrics so new incoming data starts from zero
          server.lastSeenMetrics = {
            twitter: {
              actions: {
                likes: 0,
                retweets: 0,
                replies: 0,
                quotes: 0,
                follows: 0,
                bookmarks: 0,
              },
            },
            api: { calls: 0, failures: 0 },
            tasks: { executed: 0, failed: 0 },
          };

          logger.info("History cleared, broadcasting zeros...");
          // Broadcast cleared data immediately
          io.emit("metrics", server.collectMetrics());
        },
        authEnabled,
        authToken,
      ),
    );

    // Notification API - send notification to all clients (protected)
    socket.on(
      "send-notification",
      withAuth(
        "send-notification",
        (data) => {
          if (data && data.message) {
            const notification = {
              type: data.type || "info",
              title: data.title || "Dashboard",
              message: sanitizeLogString(data.message),
              timestamp: Date.now(),
            };
            io.emit("notification", notification);
          }
        },
        authEnabled,
        authToken,
      ),
    );
  });
}
