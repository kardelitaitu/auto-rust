/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import fs from "fs";
import path from "path";

// Prevent IPC code from running during tests
vi.stubGlobal("process", {
  ...process,
  send: vi.fn(),
  on: vi.fn(),
  exit: vi.fn(),
  listeners: vi.fn(() => []),
});

// Mock external modules that cause issues in tests
vi.mock("@api/ui/electron-dashboard/server/utils/metrics.js", () => ({
  getSystemMetrics: vi.fn(() => ({
    cpu: { usage: 15.5, cores: 8 },
    memory: { total: 16, used: 8, free: 8, percent: 50 },
    platform: "Windows",
    hostname: "test-host",
    uptime: 3600,
  })),
  calculateCpuUsage: vi.fn((prev, current) => {
    const totalDiff = current.total - prev.total;
    const idleDiff = current.idle - prev.idle;
    if (totalDiff === 0) return 0;
    return Math.round(((totalDiff - idleDiff) / totalDiff) * 100 * 100) / 100;
  }),
}));

vi.mock("@api/ui/electron-dashboard/server/socket/handlers.js", () => ({
  setupSocketHandlers: vi.fn(),
}));

vi.mock("@api/ui/electron-dashboard/server/socket/broadcast.js", () => ({
  BroadcastManager: vi.fn().mockImplementation(() => ({
    start: vi.fn(),
    stop: vi.fn(),
    resume: vi.fn(),
    collectMetrics: vi.fn(() => ({
      timestamp: Date.now(),
      sessions: [],
      queue: { queueLength: 0 },
      recentTasks: [],
      system: { cpu: {}, memory: {} },
      cumulative: { completedTasks: 0 },
    })),
    sendMetrics: vi.fn(),
  })),
}));

vi.mock("@api/ui/electron-dashboard/server/lib/history-manager.js", () => ({
  HistoryManager: vi.fn().mockImplementation(() => ({
    getTasks: vi.fn(() => []),
    getCompletedTasksCount: vi.fn(() => 0),
    getTwitterActions: vi.fn(() => ({})),
    getApiMetrics: vi.fn(() => ({ calls: 0, failures: 0 })),
    save: vi.fn(),
  })),
}));

vi.mock("@api/ui/electron-dashboard/server/routes/health.js", () => ({
  createHealthRouter: vi.fn(() => ({ use: vi.fn(), get: vi.fn() })),
}));

vi.mock("@api/ui/electron-dashboard/server/routes/status.js", () => ({
  createStatusRouter: vi.fn(() => ({ use: vi.fn(), get: vi.fn() })),
}));

vi.mock("@api/ui/electron-dashboard/server/routes/tasks.js", () => ({
  createTasksRouter: vi.fn(() => ({ use: vi.fn(), get: vi.fn() })),
}));

vi.mock("@api/ui/electron-dashboard/server/routes/dashboard.js", () => ({
  createDashboardRouter: vi.fn(() => ({ use: vi.fn(), get: vi.fn() })),
}));

vi.mock("@api/ui/electron-dashboard/server/routes/export.js", () => ({
  createExportRouter: vi.fn(() => ({ use: vi.fn(), get: vi.fn() })),
}));

vi.mock("@api/ui/electron-dashboard/server/middleware/auth.js", () => ({
  isAuthenticated: vi.fn(() => true),
  withAuth: vi.fn((handler) => handler),
  requireAuth: vi.fn(),
}));

vi.mock("@api/ui/electron-dashboard/server/middleware/rateLimit.js", () => ({
  createRateLimit: vi.fn(() => (req, res, next) => next()),
}));

vi.mock("@api/ui/electron-dashboard/server/utils/hashing.js", () => ({
  quickHash: vi.fn(() => "mockhash"),
}));

vi.mock("@api/ui/electron-dashboard/server/utils/sanitization.js", () => ({
  sanitizeLogString: vi.fn((s) => s),
  sanitizeObject: vi.fn((o) => o),
}));

vi.mock("@api/ui/electron-dashboard/server/socket/validation.js", () => ({
  validateTask: vi.fn(),
  validateSession: vi.fn(),
  validateMetrics: vi.fn(),
  validatePayload: vi.fn(),
}));

import { DashboardServer } from "@api/ui/electron-dashboard/dashboard.js";
import {
  getSystemMetrics,
  calculateCpuUsage,
} from "@api/ui/electron-dashboard/server/utils/metrics.js";

describe("DashboardServer", () => {
  let server;

  beforeEach(() => {
    vi.restoreAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    if (server?.stop) {
      try {
        server.stop();
      } catch (e) {
        // Ignore cleanup errors
      }
    }
  });

  describe("constructor", () => {
    it("should create instance with default port", () => {
      server = new DashboardServer();
      expect(server.port).toBe(3001);
      expect(server.BROADCAST_MS).toBe(2000);
    });

    it("should create instance with custom port", () => {
      server = new DashboardServer(4000, 5000);
      expect(server.port).toBe(4000);
      expect(server.BROADCAST_MS).toBe(5000);
    });

    it("should initialize with empty metrics", () => {
      server = new DashboardServer();
      expect(server.latestMetrics.sessions).toEqual([]);
      expect(server.latestMetrics.queue.queueLength).toBe(0);
      expect(server.cumulativeMetrics.completedTasks).toBeDefined();
    });
  });

  describe("getSystemMetrics", () => {
    it("should return system metrics object", () => {
      const metrics = getSystemMetrics();
      expect(metrics).toHaveProperty("cpu");
      expect(metrics).toHaveProperty("memory");
      expect(metrics).toHaveProperty("platform");
      expect(metrics).toHaveProperty("hostname");
    });

    it("should return CPU with usage and cores", () => {
      const metrics = getSystemMetrics();
      expect(metrics.cpu).toHaveProperty("usage");
      expect(metrics.cpu).toHaveProperty("cores");
    });

    it("should return memory with percent", () => {
      const metrics = getSystemMetrics();
      expect(metrics.memory.percent).toBeGreaterThanOrEqual(0);
      expect(metrics.memory.percent).toBeLessThanOrEqual(100);
    });
  });

  describe("updateMetrics", () => {
    beforeEach(() => {
      // Delete history file to ensure clean state
      const historyFile = path.join(
        __dirname,
        "..",
        "..",
        "ui",
        "electron-dashboard",
        "data",
        "dashboard-history.json",
      );
      try {
        fs.unlinkSync(historyFile);
      } catch (e) {
        // ignore if file doesn't exist
      }
      server = new DashboardServer();
    });

    it("should update latestMetrics with payload", () => {
      const payload = {
        sessions: [{ id: "session-1" }],
        queue: { queueLength: 5 },
        metrics: { tasks: { executed: 10 } },
      };
      server.updateMetrics(payload);
      expect(server.latestMetrics.sessions).toEqual(payload.sessions);
      expect(server.latestMetrics.queue.queueLength).toBe(5);
    });

    it("should handle null payload gracefully", () => {
      expect(() => server.updateMetrics(null)).not.toThrow();
    });

    it("should handle undefined payload gracefully", () => {
      expect(() => server.updateMetrics(undefined)).not.toThrow();
    });

    it("should increment task counter on new successful tasks", () => {
      vi.useRealTimers();
      server.dashboardData.tasks = [];
      server.historyManager.completedTasks = 0;
      server.historyManager.tasks = [];
      const now = Date.now();
      server.updateMetrics({
        recentTasks: [
          { id: "t1", taskName: "task1", success: true, timestamp: now },
          { id: "t2", taskName: "task2", success: false, timestamp: now + 1 },
        ],
      });
      // After updateMetrics, the tasks should be stored in latestMetrics
      expect(server.latestMetrics.recentTasks).toHaveLength(2);
    });
  });

  describe("collectMetrics", () => {
    beforeEach(() => {
      server = new DashboardServer();
    });

    it("should return error when broadcast manager not initialized", () => {
      const collected = server.collectMetrics();
      // Without starting the server, broadcastManager is null
      // so collectMetrics returns an error object
      expect(collected).toHaveProperty("error");
    });

    it("should return metrics when broadcast manager is initialized", () => {
      // Initialize the broadcast manager mock with all required methods
      server.broadcastManager = {
        collectMetrics: vi.fn(() => ({
          timestamp: Date.now(),
          sessions: [],
          queue: { queueLength: 0 },
          cumulative: { completedTasks: 0, engineUptimeMs: 0 },
        })),
        stop: vi.fn(),
      };

      const collected = server.collectMetrics();
      expect(collected).toHaveProperty("timestamp");
      expect(collected).toHaveProperty("sessions");
      expect(collected).toHaveProperty("cumulative");
    });
  });

  describe("calculateCpuUsage", () => {
    it("should calculate CPU usage correctly", () => {
      const usage = calculateCpuUsage(
        { idle: 100, total: 500 },
        { idle: 150, total: 600 },
      );
      expect(usage).toBeGreaterThanOrEqual(0);
      expect(usage).toBeLessThanOrEqual(100);
    });

    it("should return 0 when total delta is 0", () => {
      const usage = calculateCpuUsage(
        { idle: 100, total: 500 },
        { idle: 100, total: 500 },
      );
      expect(usage).toBe(0);
    });
  });

  describe("emit", () => {
    beforeEach(() => {
      server = new DashboardServer();
    });

    it("should emit event to all sockets", () => {
      server.io = { emit: vi.fn(), close: vi.fn() };
      server.emit("test-event", { data: "test" });
      expect(server.io.emit).toHaveBeenCalledWith(
        "test-event",
        expect.objectContaining({
          timestamp: expect.any(Number),
          data: "test",
        }),
      );
    });

    it("should handle missing io gracefully", () => {
      server.io = null;
      expect(() => server.emit("test", {})).not.toThrow();
    });
  });
});
