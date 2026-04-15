/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import Orchestrator from "@api/core/orchestrator.js";
import {
  validatePayload,
  validateTaskExecution,
} from "@api/utils/validator.js";
import { isDevelopment } from "@api/utils/envLoader.js";

// Mock Logger Singleton using vi.hoisted
const { mockLogger, mockMetrics } = vi.hoisted(() => {
  return {
    mockLogger: {
      info: vi.fn(),
      debug: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
    },
    mockMetrics: {
      recordBrowserDiscovery: vi.fn(),
      recordTaskExecution: vi.fn(),
      recordTaskStart: vi.fn(),
      getStats: vi.fn(),
      getRecentTasks: vi.fn(),
      getTaskBreakdown: vi.fn(),
      logStats: vi.fn(),
      generateJsonReport: vi.fn(),
      metrics: {
        startTime: Date.now(),
        lastResetTime: Date.now(),
      },
    },
  };
});

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
vi.mock("@api/index.js", () => ({
  api: {
    withPage: vi.fn((page, fn) => fn()),
    init: vi.fn().mockResolvedValue(),
    getCurrentUrl: vi.fn().mockResolvedValue("http://test.com"),
  },
}));
vi.mock("@api/utils/metrics.js", () => ({
  default: mockMetrics,
}));
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => mockLogger),
  loggerContext: {
    run: vi.fn((ctx, fn) => fn()),
    getStore: vi.fn().mockReturnValue({}),
  },
}));
vi.mock("@api/utils/configLoader.js", () => ({
  ConfigLoader: class {
    constructor() {
      this.getSettings = vi.fn().mockResolvedValue({});
      this.getTimeoutValue = vi.fn().mockReturnValue(1000);
      this.loadConfig = vi.fn().mockResolvedValue({});
      this.clearCache = vi.fn();
    }
  },
  getTimeoutValue: vi.fn().mockResolvedValue({}),
  getSettings: vi.fn().mockResolvedValue({}),
}));
vi.mock("@api/utils/envLoader.js", () => ({
  isDevelopment: vi.fn(() => false),
}));

// Mock validator
vi.mock("@api/utils/validator.js", () => ({
  validateTaskExecution: vi.fn(() => ({ isValid: true })),
  validatePayload: vi.fn(() => ({ isValid: true })),
}));

describe("Orchestrator Coverage Extensions", () => {
  let orchestrator;
  let mockSessionManager;
  let mockDiscovery;
  let mockAutomator;

  beforeEach(() => {
    vi.clearAllMocks();
    orchestrator = new Orchestrator();
    mockSessionManager = orchestrator.sessionManager;
    mockDiscovery = orchestrator.discovery;
    mockAutomator = orchestrator.automator;

    // Default setups
    vi.spyOn(mockSessionManager, "activeSessionsCount", "get").mockReturnValue(
      0,
    );
    mockSessionManager.acquireWorker.mockResolvedValue({
      id: 0,
      status: "busy",
    });
    mockSessionManager.releaseWorker.mockResolvedValue();
    mockSessionManager.acquirePage.mockResolvedValue({
      close: vi.fn(),
      isClosed: vi.fn().mockReturnValue(false),
      evaluate: vi.fn().mockResolvedValue(),
      context: () => ({
        browser: () => ({
          isConnected: vi.fn().mockReturnValue(true),
        }),
      }),
    });
    mockSessionManager.releasePage.mockResolvedValue();

    // Ensure validator mocks are reset to default behavior
    validateTaskExecution.mockReturnValue({ isValid: true });
    validatePayload.mockReturnValue({ isValid: true });

    // Spy on internal method for mocking
    vi.spyOn(orchestrator, "_importTaskModule").mockImplementation(
      async (taskName) => {
        if (taskName === "coverage_valid_task") {
          return { default: async () => ({ success: true }) };
        }
        if (taskName === "coverage_no_default_task") {
          return { someExport: "value" };
        }
        throw new Error(`Cannot find module '../tasks/${taskName}.js'`);
      },
    );
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("executeTask (Coverage)", () => {
    it("should execute successfully when task module is valid", async () => {
      const task = { taskName: "coverage_valid_task", payload: { foo: "bar" } };
      const session = { id: "s1", browserInfo: "test" };
      const page = {
        isClosed: vi.fn().mockReturnValue(false),
        context: vi.fn(() => ({
          browser: () => ({ isConnected: () => true }),
        })),
      };

      await orchestrator.executeTask(task, page, session);

      expect(orchestrator._importTaskModule).toHaveBeenCalledWith(
        "coverage_valid_task",
      );

      // Verify metrics were called with correct structure (flexible matching)
      expect(mockMetrics.recordTaskExecution).toHaveBeenCalled();
      const callArgs = mockMetrics.recordTaskExecution.mock.calls[0];
      expect(callArgs[0]).toBe("coverage_valid_task"); // taskName
      expect(typeof callArgs[1]).toBe("number"); // duration
      expect(callArgs[2]).toBe(true); // success
      expect(callArgs[3]).toBe("s1"); // sessionId
    });

    it("should fail if task does not have default export", async () => {
      const task = { taskName: "coverage_no_default_task", payload: {} };
      const session = { id: "s1" };
      const page = {
        isClosed: vi.fn().mockReturnValue(false),
        context: vi.fn(() => ({
          browser: () => ({ isConnected: () => true }),
        })),
      };

      await orchestrator.executeTask(task, page, session);

      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining(
          "[Orchestrator] Task 'coverage_no_default_task' error:",
        ),
        expect.stringContaining("missing default export"),
      );
    });

    it("should fail if validation fails", async () => {
      const task = { taskName: "coverage_valid_task", payload: { foo: "bar" } };
      const session = { id: "s1", browserInfo: "test" };
      const page = {
        isClosed: vi.fn().mockReturnValue(false),
        context: vi.fn(() => ({
          browser: () => ({ isConnected: () => true }),
        })),
      };

      validateTaskExecution.mockReturnValueOnce({
        isValid: false,
        errors: ["Validation failed"],
      });

      await orchestrator.executeTask(task, page, session);

      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining(
          "[Orchestrator] Task 'coverage_valid_task' error:",
        ),
        expect.stringContaining("Validation failed"),
      );
    });
  });

  describe("processTasks (Coverage)", () => {
    it("should return early if already processing tasks", async () => {
      orchestrator.isProcessingTasks = true;
      await orchestrator.processTasks();
      expect(mockLogger.info).not.toHaveBeenCalled();
    });

    it("should return early if task queue is empty", async () => {
      orchestrator.taskQueue = [];
      await orchestrator.processTasks();
      expect(mockLogger.info).not.toHaveBeenCalled();
    });

    it("should warn and return if no active sessions", async () => {
      orchestrator.taskQueue = [{ taskName: "t1", payload: {} }];
      vi.spyOn(
        mockSessionManager,
        "activeSessionsCount",
        "get",
      ).mockReturnValue(0);

      await orchestrator.processTasks();

      expect(mockLogger.warn).toHaveBeenCalledWith(
        "[Orchestrator] No active sessions available.",
      );
    });
  });

  describe("processSharedChecklistForSession (Edge Cases)", () => {
    it("should handle page.close() failure gracefully", async () => {
      orchestrator.isShuttingDown = false;

      const mockPage = {
        close: vi.fn().mockRejectedValue(new Error("Close failed")),
        isClosed: vi.fn().mockReturnValue(false),
      };

      const mockContext = {
        newPage: vi.fn().mockResolvedValue(mockPage),
        close: vi.fn().mockResolvedValue(),
      };

      const mockSession = {
        id: "s1",
        browser: {
          isConnected: vi.fn().mockReturnValue(true),
          contexts: () => [],
          newContext: vi.fn().mockResolvedValue(mockContext),
        },
        workers: [{ id: 0, status: "idle" }],
      };
      mockSessionManager.acquirePage.mockResolvedValue(mockPage);
      mockSessionManager.releasePage.mockResolvedValue();

      vi.spyOn(orchestrator, "executeTask").mockResolvedValue();

      await orchestrator.processSharedChecklistForSession(mockSession, [
        { taskName: "t1" },
      ]);

      expect(mockSessionManager.releasePage).toHaveBeenCalledWith(
        "s1",
        mockPage,
      );
    });

    it("should handle critical error in worker loop", async () => {
      orchestrator.isShuttingDown = false;
      const mockSession = {
        id: "s1",
        browser: {
          isConnected: vi.fn().mockReturnValue(true),
          contexts: () => [],
          newContext: vi.fn().mockResolvedValue({
            newPage: vi.fn().mockRejectedValue(new Error("Context Crash")),
            close: vi.fn(),
          }),
        },
        workers: [{ id: 0, status: "idle" }],
      };
      mockSessionManager.acquirePage.mockRejectedValue(
        new Error("Context Crash"),
      );

      await orchestrator.processSharedChecklistForSession(mockSession, [
        { taskName: "t1" },
      ]);

      expect(mockLogger.error).toHaveBeenCalledWith(
        expect.stringContaining("[Orchestrator][s1][Worker 0] Task error:"),
        expect.stringContaining("Context Crash"),
      );
    });
  });

  describe("waitForTasksToComplete", () => {
    it("should resolve immediately if queue is empty and not processing", async () => {
      orchestrator.taskQueue = [];
      orchestrator.isProcessingTasks = false;

      const result = await orchestrator.waitForTasksToComplete();
      expect(result.completed).toBe(0);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Queue empty, resolving immediately"),
      );
    });

    it("should wait for tasksProcessed event if queue is not empty", async () => {
      orchestrator.taskQueue = [{ taskName: "t1" }];
      orchestrator.isProcessingTasks = true;

      const promise = orchestrator.waitForTasksToComplete();

      // Simulate tasks processed and queue empty
      orchestrator.taskQueue = [];
      orchestrator.isProcessingTasks = false;
      orchestrator.emit("tasksProcessed");

      const result = await promise;
      expect(result.completed).toBe(true);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("All tasks completed"),
      );
    });
  });

  describe("startDiscovery (Edge Cases)", () => {
    it("should handle no endpoints found", async () => {
      mockDiscovery.discoverBrowsers.mockResolvedValue([]);
      await orchestrator.startDiscovery();
      expect(mockLogger.warn).toHaveBeenCalledWith(
        expect.stringContaining("No browser endpoints discovered"),
      );
    });

    it("should skip browser with no ws endpoint", async () => {
      mockDiscovery.discoverBrowsers.mockResolvedValue([
        { ws: null, windowName: "NoWS" },
      ]);
      await orchestrator.startDiscovery();
      expect(mockLogger.warn).toHaveBeenCalledWith(
        expect.stringContaining("has no 'ws' endpoint"),
      );
    });

    it("should handle top-level discovery error", async () => {
      mockDiscovery.loadConnectors.mockRejectedValue(new Error("Load failed"));
      await orchestrator.startDiscovery();
      expect(mockLogger.error).toHaveBeenCalledWith(
        "Browser discovery failed:",
        "Load failed",
      );
    });
  });

  describe("Metrics Passthrough", () => {
    it("should delegate getMetrics", () => {
      mockMetrics.getStats.mockReturnValue({ total: 10 });
      const result = orchestrator.getMetrics();
      expect(result).toHaveProperty("total", 10);
      expect(result).toHaveProperty("startTime");
      expect(result).toHaveProperty("lastResetTime");
    });

    it("should delegate logMetrics", () => {
      orchestrator.logMetrics();
      expect(mockMetrics.logStats).toHaveBeenCalled();
    });
  });

  describe("Helper Methods", () => {
    it("should wait for specified time in _sleep", async () => {
      vi.useFakeTimers();
      const promise = orchestrator._sleep(1000);
      vi.advanceTimersByTime(1000);
      await expect(promise).resolves.toBeUndefined();
      vi.useRealTimers();
    });
  });

  describe("addTask (Validation & Debounce)", () => {
    it("should debounce processTasks calls", async () => {
      vi.useFakeTimers();
      const processTasksSpy = vi
        .spyOn(orchestrator, "processTasks")
        .mockResolvedValue();

      orchestrator.addTask("task1");
      orchestrator.addTask("task2");
      orchestrator.addTask("task3");

      expect(processTasksSpy).not.toHaveBeenCalled();

      vi.advanceTimersByTime(50);
      // We don't need to run all timers, just advance the time for the debounce
      expect(processTasksSpy).toHaveBeenCalled();
      vi.useRealTimers();
    });
  });

  describe("shutdown", () => {
    it("should handle force shutdown", async () => {
      const waitForTasksSpy = vi
        .spyOn(orchestrator, "waitForTasksToComplete")
        .mockResolvedValue({ timedOut: false });
      vi.spyOn(orchestrator.automator, "shutdown").mockResolvedValue();
      vi.spyOn(mockSessionManager, "shutdown").mockResolvedValue();

      await orchestrator.shutdown(true);

      expect(waitForTasksSpy).not.toHaveBeenCalled();
      expect(orchestrator.isShuttingDown).toBe(true);
    });
  });
});
