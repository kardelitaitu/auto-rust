/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import Orchestrator from "@api/core/orchestrator.js";
import * as validator from "@api/utils/validator.js";

// Mock dependencies
vi.mock("@api/core/sessionManager.js", () => ({
  default: class {
    constructor() {
      this.loadConfiguration = vi.fn().mockResolvedValue(undefined);
      this.addSession = vi.fn();
      this.getAllSessions = vi.fn().mockReturnValue([]);
      this.shutdown = vi.fn().mockResolvedValue(undefined);
      this.markSessionFailed = vi.fn();
      this.acquireWorker = vi.fn();
      this.releaseWorker = vi.fn();
      this.acquirePage = vi.fn();
      this.releasePage = vi.fn();
      this.replaceBrowserByEndpoint = vi.fn();
    }
    get activeSessionsCount() {
      return 0;
    }
  },
}));
vi.mock("@api/core/discovery.js", () => ({
  default: class {
    constructor() {
      this.loadConnectors = vi.fn().mockResolvedValue(undefined);
      this.discoverBrowsers = vi.fn().mockResolvedValue([]);
    }
  },
}));
vi.mock("@api/core/automator.js", () => ({
  default: class {
    constructor() {
      this.connectToBrowser = vi.fn();
      this.startHealthChecks = vi.fn();
      this.shutdown = vi.fn().mockResolvedValue(undefined);
      this.checkPageResponsive = vi.fn().mockResolvedValue({ healthy: true });
    }
  },
}));
vi.mock("@api/utils/metrics.js", () => ({
  default: {
    recordBrowserDiscovery: vi.fn(),
    recordTaskExecution: vi.fn(),
    getStats: vi.fn().mockReturnValue({}),
    logStats: vi.fn(),
    generateJsonReport: vi.fn().mockResolvedValue(undefined),
    metrics: { startTime: Date.now(), lastResetTime: Date.now() },
  },
}));
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
  loggerContext: {
    run: vi.fn((ctx, fn) => fn()),
    getStore: vi.fn().mockReturnValue({}),
  },
}));
vi.mock("@api/utils/configLoader.js", () => {
  const mockInstance = {
    getSettings: vi.fn().mockResolvedValue({}),
    getTimeoutValue: vi.fn().mockReturnValue(1000),
    loadConfig: vi.fn().mockResolvedValue({}),
    clearCache: vi.fn(),
  };
  return {
    ConfigLoader: class {
      constructor() {
        return mockInstance;
      }
    },
    getTimeoutValue: vi.fn().mockReturnValue(1000),
    getSettings: vi.fn().mockResolvedValue({}),
  };
});
vi.mock("@api/utils/envLoader.js", () => ({
  isDevelopment: vi.fn(() => false),
}));

// Mock validator
vi.mock("@api/utils/validator.js", () => ({
  validateTaskExecution: vi.fn(() => ({ isValid: true })),
  validatePayload: vi.fn(() => ({ isValid: true })),
}));

// Import the mocked metrics module
import metricsCollector from "@api/utils/metrics.js";

describe("Orchestrator", () => {
  let orchestrator;
  let mockSessionManager;
  let mockDiscovery;
  let mockAutomator;

  beforeEach(() => {
    vi.clearAllMocks();

    // Instantiate Orchestrator
    orchestrator = new Orchestrator();

    // Access the instances attached to orchestrator
    mockSessionManager = orchestrator.sessionManager;
    mockDiscovery = orchestrator.discovery;
    mockAutomator = orchestrator.automator;

    // Fix activeSessionsCount getter mock on the instance
    vi.spyOn(mockSessionManager, "activeSessionsCount", "get").mockReturnValue(
      0,
    );

    // Default mock implementations
    mockAutomator.connectToBrowser.mockResolvedValue({});

    // Reset validator mocks to valid state
    validator.validateTaskExecution.mockReturnValue({ isValid: true });
    validator.validatePayload.mockReturnValue({ isValid: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Initialization", () => {
    it("should initialize with empty queue and default components", () => {
      expect(orchestrator.taskQueue).toEqual([]);
      expect(orchestrator.isProcessingTasks).toBe(false);
      expect(orchestrator.sessionManager).toBeDefined();
      expect(orchestrator.discovery).toBeDefined();
      expect(orchestrator.automator).toBeDefined();
    });
  });

  describe("formatSessionDisplayName", () => {
    it("should format session display names correctly", () => {
      expect(
        orchestrator._formatSessionDisplayName({
          type: "localChrome",
          port: 9222,
        }),
      ).toBe("chrome:9222");
      expect(
        orchestrator._formatSessionDisplayName({
          type: "ixbrowser",
          windowName: "Profile1",
        }),
      ).toBe("ix:Profile1");
      expect(
        orchestrator._formatSessionDisplayName({
          type: "unknown",
          ws: "ws://localhost:1234",
        }),
      ).toBe("unknown");
    });
  });

  describe("Task Processing", () => {
    it("should distribute tasks to sessions", async () => {
      orchestrator.taskQueue = [{ taskName: "mockTask", payload: {} }];
      vi.spyOn(
        mockSessionManager,
        "activeSessionsCount",
        "get",
      ).mockReturnValue(1);
      const mockSession = {
        id: "s1",
        browser: {
          contexts: () => [],
          newContext: vi
            .fn()
            .mockResolvedValue({ newPage: vi.fn(), close: vi.fn() }),
        },
        workers: [],
      };
      mockSessionManager.getAllSessions.mockReturnValue([mockSession]);

      // Mock processSharedChecklistForSession to avoid complex logic in this test
      const checklistSpy = vi
        .spyOn(orchestrator, "processSharedChecklistForSession")
        .mockResolvedValue();

      await orchestrator.processTasks();

      expect(checklistSpy).toHaveBeenCalledWith(
        mockSession,
        expect.arrayContaining([
          expect.objectContaining({ taskName: "mockTask" }),
        ]),
      );
      expect(orchestrator.taskQueue).toHaveLength(0);
      expect(orchestrator.isProcessingTasks).toBe(false);
    });

    it("should exit early when no sessions are active", async () => {
      orchestrator.taskQueue = [{ taskName: "mockTask", payload: {} }];
      vi.spyOn(
        mockSessionManager,
        "activeSessionsCount",
        "get",
      ).mockReturnValue(0);

      const result = await orchestrator.processTasks();

      expect(result).toBeUndefined();
      expect(orchestrator.isProcessingTasks).toBe(false);
      // Queue is cleared so orchestrator can exit gracefully
      expect(orchestrator.taskQueue).toHaveLength(0);
    });

    it("should exit early when already processing", async () => {
      orchestrator.taskQueue = [{ taskName: "mockTask", payload: {} }];
      orchestrator.isProcessingTasks = true;

      const result = await orchestrator.processTasks();

      expect(result).toBeUndefined();
      expect(orchestrator.taskQueue).toHaveLength(1);
    });
  });

  it("should sleep for specified duration", async () => {
    vi.useFakeTimers();
    const setTimeoutSpy = vi.spyOn(global, "setTimeout");

    orchestrator._sleep(100);

    expect(setTimeoutSpy).toHaveBeenCalledWith(expect.any(Function), 100);
    vi.advanceTimersByTime(100);
    vi.useRealTimers();
  });

  describe("startDiscovery", () => {
    it("should load connectors and discover browsers", async () => {
      mockDiscovery.discoverBrowsers.mockResolvedValue([
        {
          ws: "ws://localhost:1234",
          windowName: "Chrome 1",
          type: "localChrome",
        },
      ]);
      mockAutomator.connectToBrowser.mockResolvedValue({ contexts: () => [] });

      // Mock active sessions count to trigger health checks
      vi.spyOn(
        mockSessionManager,
        "activeSessionsCount",
        "get",
      ).mockReturnValue(1);

      await orchestrator.startDiscovery({ browsers: ["chrome"] });

      expect(mockDiscovery.loadConnectors).toHaveBeenCalledWith(["chrome"]);
      expect(mockDiscovery.discoverBrowsers).toHaveBeenCalled();
      expect(mockAutomator.connectToBrowser).toHaveBeenCalledWith(
        "ws://localhost:1234",
      );
      expect(mockSessionManager.addSession).toHaveBeenCalled();
      expect(mockAutomator.startHealthChecks).toHaveBeenCalled();
    });

    it("should handle no browsers discovered", async () => {
      mockDiscovery.discoverBrowsers.mockResolvedValue([]);

      await orchestrator.startDiscovery();

      expect(mockAutomator.connectToBrowser).not.toHaveBeenCalled();
      expect(mockSessionManager.addSession).not.toHaveBeenCalled();
    });
  });

  describe("addTask", () => {
    it("should add valid task to queue and trigger processing", async () => {
      vi.useFakeTimers();
      const processSpy = vi.spyOn(orchestrator, "processTasks");

      orchestrator.addTask("testTask", { foo: "bar" });

      expect(orchestrator.taskQueue).toHaveLength(1);
      expect(orchestrator.taskQueue[0]).toMatchObject({
        taskName: "testTask",
        payload: { foo: "bar" },
      });

      vi.advanceTimersByTime(50);
      expect(processSpy).toHaveBeenCalled();
    });

    it("should debounce task processing", () => {
      vi.useFakeTimers();
      const processSpy = vi.spyOn(orchestrator, "processTasks");

      orchestrator.addTask("task1", {});
      orchestrator.addTask("task2", {});

      expect(orchestrator.taskQueue).toHaveLength(2);

      vi.advanceTimersByTime(50);
      expect(processSpy).toHaveBeenCalledTimes(1);
    });

    it("should throw on invalid task name", () => {
      expect(() => orchestrator.addTask("")).toThrow("Invalid task name");
      expect(() => orchestrator.addTask(null)).toThrow("Invalid task name");
    });

    it("should throw on invalid payload", () => {
      validator.validatePayload.mockReturnValue({
        isValid: false,
        errors: ["Bad payload"],
      });
      expect(() => orchestrator.addTask("task", {})).toThrow(
        "Invalid task payload",
      );
    });

    it("should respect max queue size", () => {
      orchestrator.maxTaskQueueSize = 1;
      orchestrator.taskQueue = [{ taskName: "existing", payload: {} }];

      expect(() => orchestrator.addTask("task", {})).toThrow("Task queue full");
    });
  });

  describe("Metrics", () => {
    it("should delegate getMetrics to metricsCollector", () => {
      orchestrator.getMetrics();
      expect(metricsCollector.getStats).toHaveBeenCalled();
    });

    it("should delegate logMetrics to metricsCollector", () => {
      orchestrator.logMetrics();
      expect(metricsCollector.logStats).toHaveBeenCalled();
    });

    it("should fall back when metricsCollector throws", () => {
      metricsCollector.getStats.mockImplementationOnce(() => {
        throw new Error("metrics failed");
      });

      expect(orchestrator.getMetrics()).toEqual({});
    });
  });

  describe("Shutdown", () => {
    it("should shutdown gracefully", async () => {
      vi.spyOn(orchestrator, "waitForTasksToComplete").mockResolvedValue({
        timedOut: false,
      });

      await orchestrator.shutdown(false);

      expect(orchestrator.isShuttingDown).toBe(true);
      expect(metricsCollector.logStats).toHaveBeenCalled();
      expect(mockSessionManager.shutdown).toHaveBeenCalled();
      expect(mockAutomator.shutdown).toHaveBeenCalled();
    });

    it("should force shutdown", async () => {
      const waitSpy = vi.spyOn(orchestrator, "waitForTasksToComplete");

      await orchestrator.shutdown(true);

      expect(waitSpy).not.toHaveBeenCalled();
      expect(mockSessionManager.shutdown).toHaveBeenCalled();
    });

    it("should force cancel when graceful shutdown times out", async () => {
      const waitSpy = vi
        .spyOn(orchestrator, "waitForTasksToComplete")
        .mockResolvedValue({
          timedOut: true,
        });
      const cancelSpy = vi.spyOn(orchestrator, "_forceCancelAllTasks");

      await orchestrator.shutdown(false);

      expect(waitSpy).toHaveBeenCalled();
      expect(cancelSpy).toHaveBeenCalled();
    });
  });

  describe("waitForTasksToComplete", () => {
    it("should resolve immediately if no tasks and not processing", async () => {
      orchestrator.taskQueue = [];
      orchestrator.isProcessingTasks = false;

      const result = await orchestrator.waitForTasksToComplete();
      expect(result.timedOut).toBe(0); // Actually returns { completed: 0, timedOut: 0, failed: 0, total: 0 }
    });

    it("should wait for tasksProcessed event", async () => {
      orchestrator.taskQueue = [];
      orchestrator.isProcessingTasks = true;

      const waitPromise = orchestrator.waitForTasksToComplete();

      orchestrator.isProcessingTasks = false;
      orchestrator.emit("tasksProcessed");

      const result = await waitPromise;
      expect(result.completed).toBe(true);
    });

    it("should time out and force cancel when tasks do not complete", async () => {
      vi.useFakeTimers();
      orchestrator.taskQueue = [{ taskName: "mockTask", payload: {} }];
      orchestrator.isProcessingTasks = true;

      const waitPromise = orchestrator.waitForTasksToComplete({
        timeoutMs: 1000,
      });
      await vi.advanceTimersByTimeAsync(1000);

      const result = await waitPromise;
      expect(result.completed).toBe(false);
      expect(result.timedOut).toBe(true);
    });
  });

  describe("internal branches", () => {
    it("should record session failure and recovery scores", () => {
      orchestrator._recordSessionOutcome("session-1", false);
      expect(orchestrator._getSessionFailureScore("session-1")).toBe(1);

      orchestrator._recordSessionOutcome("session-1", true);
      expect(orchestrator._getSessionFailureScore("session-1")).toBe(0);
    });

    it("should throw when task module is missing", async () => {
      await expect(
        orchestrator._importTaskModule("missing-task"),
      ).rejects.toThrow("Task module 'missing-task' not found");
    });
  });

  describe("metrics helpers", () => {
    it("should expose session metrics and queue status", () => {
      mockSessionManager.getAllSessions.mockReturnValue([
        {
          id: "s1",
          displayName: "session-1",
          browser: { isConnected: () => true },
          workers: [{ status: "busy" }, { status: "idle" }],
          completedTaskCount: 3,
          currentTaskName: "task-a",
          currentProcessing: "Executing...",
          browserType: "localChrome",
        },
      ]);

      const sessions = orchestrator.getSessionMetrics();
      const queue = orchestrator.getQueueStatus();

      expect(sessions).toHaveLength(1);
      expect(queue.queueLength).toBe(0);
    });

    it("should fall back for recent tasks and breakdown helpers", () => {
      expect(orchestrator.getRecentTasks()).toEqual([]);
      expect(orchestrator.getTaskBreakdown()).toEqual({});
    });

    it("should return empty session metrics on session manager failure", () => {
      mockSessionManager.getAllSessions.mockImplementation(() => {
        throw new Error("session failure");
      });

      expect(orchestrator.getSessionMetrics()).toEqual([]);
    });
  });
});
