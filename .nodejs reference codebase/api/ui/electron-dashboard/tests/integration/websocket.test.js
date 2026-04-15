/**
 * Integration tests for WebSocket communication
 * Tests Socket.io connection, events, and real-time updates
 */

import {
  describe,
  it,
  expect,
  beforeAll,
  afterAll,
  beforeEach,
  afterEach,
  vi,
} from "vitest";
import { DashboardServer } from "../../dashboard.js";
import { io as ioClient } from "socket.io-client";
import { createServer } from "net";

/**
 * Find an available port by trying to bind to port 0
 */
function getAvailablePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

describe("WebSocket Integration Tests", () => {
  let server;
  let serverPort;
  let clientSocket;

  beforeAll(async () => {
    // Get a unique port for all tests in this describe block
    serverPort = await getAvailablePort();
  });

  beforeEach(async () => {
    server = new DashboardServer(serverPort);

    // Disable auth for most tests
    process.env.DASHBOARD_AUTH_ENABLED = "false";

    await server.start();
    // Give server time to fully initialize
    await new Promise((resolve) => setTimeout(resolve, 150));
  });

  afterEach(async () => {
    if (clientSocket && clientSocket.connected) {
      clientSocket.disconnect();
      clientSocket = null;
    }
    if (server) {
      await server.stop();
      server = null;
    }
    delete process.env.DASHBOARD_AUTH_ENABLED;
    // Wait for port to be released
    await new Promise((resolve) => setTimeout(resolve, 100));
  });

  const connectClient = (options = {}) => {
    return new Promise((resolve, reject) => {
      const socket = ioClient(`http://localhost:${serverPort}`, {
        transports: ["websocket"],
        reconnection: false,
        ...options,
      });

      socket.on("connect", () => {
        resolve(socket);
      });

      socket.on("connect_error", (err) => {
        reject(err);
      });

      // Timeout after 5 seconds
      setTimeout(() => {
        reject(new Error("Connection timeout"));
      }, 5000);
    });
  };

  describe("Connection Management", () => {
    it("should accept client connections", async () => {
      clientSocket = await connectClient();
      expect(clientSocket.connected).toBe(true);
    });

    it("should emit metrics immediately on connection", async () => {
      clientSocket = await connectClient();

      const metrics = await new Promise((resolve) => {
        clientSocket.once("metrics", (data) => {
          resolve(data);
        });
      });

      expect(metrics).toBeDefined();
      expect(metrics.timestamp).toBeDefined();
    });

    it("should track connected clients", async () => {
      clientSocket = await connectClient();

      // Give server time to process connection
      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      const health = await healthResponse.json();

      expect(health.clients).toBe(1);
    });

    it("should handle client disconnection", async () => {
      clientSocket = await connectClient();

      // Give server time to process connection
      await new Promise((resolve) => setTimeout(resolve, 100));

      clientSocket.disconnect();
      clientSocket = null;

      // Give server time to process disconnection
      await new Promise((resolve) => setTimeout(resolve, 200));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      const health = await healthResponse.json();

      expect(health.clients).toBe(0);
    });

    it("should handle multiple concurrent connections", async () => {
      const client1 = await connectClient();
      const client2 = await connectClient();
      const client3 = await connectClient();

      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      const health = await healthResponse.json();

      expect(health.clients).toBe(3);

      client1.disconnect();
      client2.disconnect();
      client3.disconnect();
    });
  });

  describe("Socket Events", () => {
    it("should respond to requestUpdate event", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Request update
      clientSocket.emit("requestUpdate");

      const metrics = await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      expect(metrics).toBeDefined();
      expect(metrics.timestamp).toBeDefined();
    });

    it("should broadcast metrics to all clients", async () => {
      const client1 = await connectClient();
      const client2 = await connectClient();

      // Both clients should receive metrics
      const metrics1 = await new Promise((resolve) => {
        client1.once("metrics", resolve);
      });
      const metrics2 = await new Promise((resolve) => {
        client2.once("metrics", resolve);
      });

      expect(metrics1).toBeDefined();
      expect(metrics2).toBeDefined();
      expect(metrics1.timestamp).toBeDefined();
      expect(metrics2.timestamp).toBeDefined();

      client1.disconnect();
      client2.disconnect();
    });

    it("should handle task-update event", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      const taskData = {
        taskName: "test-task",
        status: "completed",
        success: true,
        timestamp: Date.now(),
      };

      // Emit task-update and verify server is still responsive
      clientSocket.emit("task-update", taskData);

      // Wait for processing
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Verify server is still responsive
      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });

    it("should handle clear-history event", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // First add a task so we can verify it's cleared
      clientSocket.emit("task-update", {
        taskName: "test-task-for-clear",
        status: "completed",
        success: true,
        timestamp: Date.now(),
      });

      await new Promise((resolve) => setTimeout(resolve, 200));

      // Emit clear-history and wait for response
      clientSocket.emit("clear-history");

      // Give server time to process
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Verify history was cleared by checking server state
      expect(server.dashboardData.tasks).toEqual([]);
    });

    it("should handle send-notification event", async () => {
      clientSocket = await connectClient();

      const notificationPromise = new Promise((resolve) => {
        clientSocket.once("notification", resolve);
      });

      clientSocket.emit("send-notification", {
        type: "info",
        title: "Test",
        message: "Test notification",
      });

      const notification = await notificationPromise;

      expect(notification.type).toBe("info");
      expect(notification.title).toBe("Test");
      expect(notification.message).toBe("Test notification");
      expect(notification.timestamp).toBeDefined();
    });
  });

  describe("Authentication", () => {
    it("should handle push_metrics event (auth tested in unit tests)", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Push metrics without auth (auth is disabled by default in tests)
      clientSocket.emit("push_metrics", {
        sessions: [{ id: "test-session", status: "online" }],
      });

      // Wait for processing
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Verify session was added (auth is disabled)
      const hasSession =
        server.latestMetrics.sessions?.some((s) => s.id === "test-session") ||
        false;
      expect(hasSession).toBe(true);
    });

    it("should handle task-update event (auth tested in unit tests)", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Send task-update (auth is disabled by default)
      clientSocket.emit("task-update", {
        taskName: "test-task-auth",
        status: "completed",
        success: true,
        timestamp: Date.now(),
      });

      // Wait for processing
      await new Promise((resolve) => setTimeout(resolve, 200));

      // Verify server is still responsive
      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });
  });

  describe("Broadcast Management", () => {
    it("should have broadcast manager initialized", async () => {
      // Broadcast manager should be defined
      expect(server.broadcastManager).toBeDefined();
      expect(typeof server.broadcastManager.start).toBe("function");
      expect(typeof server.broadcastManager.stop).toBe("function");

      // Connect a client to trigger broadcast
      clientSocket = await connectClient();
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Now broadcast should be running
      expect(server.broadcastManager.broadcastInterval).not.toBeNull();
    });

    it("should stop broadcast when all clients disconnect", async () => {
      clientSocket = await connectClient();

      // Wait for broadcast to start
      await new Promise((resolve) => setTimeout(resolve, 200));
      expect(server.broadcastManager.broadcastInterval).not.toBeNull();

      clientSocket.disconnect();
      clientSocket = null;

      // Wait for broadcast to stop
      await new Promise((resolve) => setTimeout(resolve, 400));

      expect(server.broadcastManager.broadcastInterval).toBeNull();
    });

    it("should resume broadcast on new connection after error pause", async () => {
      clientSocket = await connectClient();

      // Wait for connection to be established
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Simulate error pause
      server.broadcastManager.broadcastPaused = true;
      server.broadcastManager.consecutiveErrors =
        server.broadcastManager.maxConsecutiveErrors;

      // Disconnect and reconnect
      clientSocket.disconnect();
      clientSocket = null;
      await new Promise((resolve) => setTimeout(resolve, 100));

      clientSocket = await connectClient();

      // Wait for resume
      await new Promise((resolve) => setTimeout(resolve, 200));

      expect(server.broadcastManager.broadcastPaused).toBe(false);
      expect(server.broadcastManager.consecutiveErrors).toBe(0);
    });

    it("should have broadcast running", async () => {
      clientSocket = await connectClient();

      // Verify broadcast is running
      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(server.broadcastManager.broadcastInterval).not.toBeNull();
      expect(server.broadcastManager.broadcastPaused).toBe(false);
    });
  });

  describe("Error Handling", () => {
    it("should handle malformed push_metrics payload", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Send malformed data
      clientSocket.emit("push_metrics", "not-an-object");

      // Should not crash - wait and verify server is still responsive
      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });

    it("should handle malformed task-update payload", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Send malformed data
      clientSocket.emit("task-update", null);

      // Should not crash
      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });

    it("should handle missing required fields in task-update", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Send task without required fields
      clientSocket.emit("task-update", {
        someRandomField: "value",
      });

      // Should not crash
      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });

    it("should handle notification with missing message", async () => {
      clientSocket = await connectClient();

      // Send notification without message
      clientSocket.emit("send-notification", {
        type: "info",
        // missing message
      });

      // Should not crash and not emit notification
      await new Promise((resolve) => setTimeout(resolve, 100));

      const healthResponse = await fetch(
        `http://localhost:${serverPort}/health`,
      );
      expect(healthResponse.ok).toBe(true);
    });
  });

  describe("Data Sanitization", () => {
    it("should sanitize notification messages (remove control characters)", async () => {
      clientSocket = await connectClient();

      const notificationPromise = new Promise((resolve) => {
        clientSocket.once("notification", resolve);
      });

      // Message with control characters
      clientSocket.emit("send-notification", {
        type: "info",
        title: "Test",
        message: "Normal message\x00\x1Fwith\x7Fcontrol chars",
      });

      const notification = await notificationPromise;

      // Control characters should be removed
      expect(notification.message).not.toContain("\x00");
      expect(notification.message).not.toContain("\x1F");
      expect(notification.message).not.toContain("\x7F");
      expect(notification.message).toContain("Normal message");
      expect(notification.message).toContain("with");
      expect(notification.message).toContain("control chars");
    });

    it("should sanitize task data (remove control characters)", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      const metricsPromise = new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      clientSocket.emit("task-update", {
        taskName: "Task\x00with\x1Fcontrol\x7Fchars",
        status: "completed",
        success: true,
        timestamp: Date.now(),
      });

      const metrics = await metricsPromise;

      // Control characters should be removed in the task
      const task = metrics.recentTasks?.find((t) =>
        t.taskName?.includes("Task"),
      );
      if (task) {
        expect(task.taskName).not.toContain("\x00");
        expect(task.taskName).not.toContain("\x1F");
        expect(task.taskName).not.toContain("\x7F");
        expect(task.taskName).toContain("Task");
        expect(task.taskName).toContain("with");
      }
    });
  });

  describe("Metrics Collection", () => {
    it("should include system metrics", async () => {
      clientSocket = await connectClient();

      const metrics = await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      expect(metrics.system).toBeDefined();
      expect(metrics.system.cpu).toBeDefined();
      expect(metrics.system.memory).toBeDefined();
    });

    it("should include cumulative metrics", async () => {
      clientSocket = await connectClient();

      const metrics = await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      expect(metrics.cumulative).toBeDefined();
      expect(metrics.cumulative.completedTasks).toBeDefined();
      expect(metrics.cumulative.sessionUptimeMs).toBeDefined();
      expect(metrics.cumulative.engineUptimeMs).toBeDefined();
    });

    it("should update metrics when pushed", async () => {
      clientSocket = await connectClient();

      // Wait for initial metrics
      await new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      const testSession = {
        id: "test-session-123",
        status: "online",
        lastSeen: Date.now(),
      };

      server.updateMetrics({
        sessions: [testSession],
      });

      const metricsPromise = new Promise((resolve) => {
        clientSocket.once("metrics", resolve);
      });

      // Trigger broadcast
      server.io.emit("metrics", server.collectMetrics());

      const metrics = await metricsPromise;

      expect(metrics.sessions).toBeDefined();
      expect(metrics.sessions.some((s) => s.id === "test-session-123")).toBe(
        true,
      );
    });
  });
});
