/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/progressTracker.js
 * @module tests/unit/agent/progressTracker.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("api/agent/progressTracker.js", () => {
  let progressTracker;
  let mockWsServer;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    const module = await import("@api/agent/progressTracker.js");
    progressTracker = module.progressTracker || module.default;
    progressTracker.reset();

    // Clear all listeners
    progressTracker.listeners.clear();

    mockWsServer = {
      emit: vi.fn(),
    };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with null wsServer", () => {
      expect(progressTracker.wsServer).toBeNull();
    });

    it("should initialize with default metrics", () => {
      const metrics = progressTracker.getMetrics();
      expect(metrics.stepsCompleted).toBe(0);
      expect(metrics.actionsAttempted).toBe(0);
      expect(metrics.actionsSucceeded).toBe(0);
      expect(metrics.actionsFailed).toBe(0);
      expect(metrics.llmCalls).toBe(0);
      expect(metrics.llmFailures).toBe(0);
      expect(metrics.errors).toBe(0);
    });

    it("should initialize with empty listeners set", () => {
      expect(progressTracker.listeners).toBeInstanceOf(Set);
      expect(progressTracker.listeners.size).toBe(0);
    });
  });

  describe("init()", () => {
    it("should set wsServer", () => {
      progressTracker.init(mockWsServer);
      expect(progressTracker.wsServer).toBe(mockWsServer);
    });
  });

  describe("addListener() and removeListener()", () => {
    it("should add listener", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);
      expect(progressTracker.listeners.has(listener)).toBe(true);
    });

    it("should remove listener", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);
      progressTracker.removeListener(listener);
      expect(progressTracker.listeners.has(listener)).toBe(false);
    });

    it("should add multiple listeners", () => {
      const listener1 = vi.fn();
      const listener2 = vi.fn();
      progressTracker.addListener(listener1);
      progressTracker.addListener(listener2);
      expect(progressTracker.listeners.size).toBe(2);
    });
  });

  describe("startSession()", () => {
    it("should reset metrics", () => {
      progressTracker.startSession("Test goal");

      const metrics = progressTracker.getMetrics();
      expect(metrics.stepsCompleted).toBe(0);
      expect(metrics.actionsAttempted).toBe(0);
    });

    it("should set current goal", () => {
      progressTracker.startSession("Test goal");

      const metrics = progressTracker.getMetrics();
      expect(metrics.currentGoal).toBe("Test goal");
    });

    it("should set startTime", () => {
      progressTracker.startSession("Test goal");

      const metrics = progressTracker.getMetrics();
      expect(metrics.startTime).toBeDefined();
    });

    it("should emit session:start event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test goal");

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "session:start",
          data: expect.objectContaining({
            goal: "Test goal",
          }),
        }),
      );
    });

    it("should emit to wsServer if initialized", () => {
      progressTracker.init(mockWsServer);
      progressTracker.startSession("Test goal");

      expect(mockWsServer.emit).toHaveBeenCalledWith(
        "agent:progress",
        expect.objectContaining({
          event: "session:start",
        }),
      );
    });
  });

  describe("updateUrl()", () => {
    it("should update currentUrl in metrics", () => {
      progressTracker.updateUrl("https://example.com");

      const metrics = progressTracker.getMetrics();
      expect(metrics.currentUrl).toBe("https://example.com");
    });

    it("should emit url:change event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.updateUrl("https://example.com");

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "url:change",
          data: expect.objectContaining({
            url: "https://example.com",
          }),
        }),
      );
    });
  });

  describe("recordStep()", () => {
    it("should increment stepsCompleted", () => {
      progressTracker.recordStep(1, { action: "click" }, { success: true });

      const metrics = progressTracker.getMetrics();
      expect(metrics.stepsCompleted).toBe(1);
    });

    it("should increment actionsAttempted", () => {
      progressTracker.recordStep(1, { action: "click" }, { success: true });

      const metrics = progressTracker.getMetrics();
      expect(metrics.actionsAttempted).toBe(1);
    });

    it("should increment actionsSucceeded for successful actions", () => {
      progressTracker.recordStep(1, { action: "click" }, { success: true });

      const metrics = progressTracker.getMetrics();
      expect(metrics.actionsSucceeded).toBe(1);
    });

    it("should increment actionsFailed for failed actions", () => {
      progressTracker.recordStep(
        1,
        { action: "click" },
        { success: false, error: "Failed" },
      );

      const metrics = progressTracker.getMetrics();
      expect(metrics.actionsFailed).toBe(1);
    });

    it("should emit step:complete event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      progressTracker.recordStep(1, { action: "click" }, { success: true });

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "step:complete",
          data: expect.objectContaining({
            step: 1,
            action: "click",
            success: true,
          }),
        }),
      );
    });

    it("should include elapsed time in event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      vi.advanceTimersByTime(1000);
      progressTracker.recordStep(1, { action: "click" }, { success: true });

      const event = listener.mock.calls.find(
        (call) => call[0].event === "step:complete",
      );
      expect(event[0].data.elapsed).toBeGreaterThanOrEqual(1000);
    });
  });

  describe("recordLlmCall()", () => {
    it("should increment llmCalls", () => {
      progressTracker.recordLlmCall(true, 100);

      const metrics = progressTracker.getMetrics();
      expect(metrics.llmCalls).toBe(1);
    });

    it("should increment llmFailures for failed calls", () => {
      progressTracker.recordLlmCall(false, 100);

      const metrics = progressTracker.getMetrics();
      expect(metrics.llmFailures).toBe(1);
    });

    it("should not increment llmFailures for successful calls", () => {
      progressTracker.recordLlmCall(true, 100);

      const metrics = progressTracker.getMetrics();
      expect(metrics.llmFailures).toBe(0);
    });

    it("should emit llm:call event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.recordLlmCall(true, 100);

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "llm:call",
          data: expect.objectContaining({
            success: true,
            duration: 100,
          }),
        }),
      );
    });

    it("should calculate failure rate", () => {
      progressTracker.recordLlmCall(true, 100);
      progressTracker.recordLlmCall(false, 100);

      const metrics = progressTracker.getMetrics();
      expect(metrics.llmFailureRate).toBe(0.5);
    });
  });

  describe("recordError()", () => {
    it("should increment errors count", () => {
      progressTracker.recordError("network", "Connection failed");

      const metrics = progressTracker.getMetrics();
      expect(metrics.errors).toBe(1);
    });

    it("should emit error event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.recordError("network", "Connection failed");

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "error",
          data: expect.objectContaining({
            type: "network",
            message: "Connection failed",
          }),
        }),
      );
    });

    it("should track total errors in event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.recordError("type1", "msg1");
      progressTracker.recordError("type2", "msg2");

      const lastErrorEvent = listener.mock.calls
        .map((call) => call[0])
        .filter((e) => e.event === "error")
        .pop();

      expect(lastErrorEvent.data.totalErrors).toBe(2);
    });
  });

  describe("recordStuck()", () => {
    it("should emit agent:stuck event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      progressTracker.recordStuck(5, "No progress");

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "agent:stuck",
          data: expect.objectContaining({
            step: 5,
            reason: "No progress",
          }),
        }),
      );
    });

    it("should include metrics in event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      progressTracker.recordStuck(5, "No progress");

      const event = listener.mock.calls.find(
        (call) => call[0].event === "agent:stuck",
      );
      expect(event[0].data.metrics).toBeDefined();
    });
  });

  describe("completeSession()", () => {
    it("should emit session:complete event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      progressTracker.completeSession(true, "Goal achieved");

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "session:complete",
          data: expect.objectContaining({
            success: true,
            reason: "Goal achieved",
          }),
        }),
      );
    });

    it("should include duration in event", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      vi.advanceTimersByTime(5000);
      progressTracker.completeSession(true, "Done");

      const event = listener.mock.calls.find(
        (call) => call[0].event === "session:complete",
      );
      expect(event[0].data.duration).toBeGreaterThanOrEqual(5000);
    });

    it("should include final metrics", () => {
      const listener = vi.fn();
      progressTracker.addListener(listener);

      progressTracker.startSession("Test");
      progressTracker.recordStep(1, { action: "click" }, { success: true });
      progressTracker.completeSession(true, "Done");

      const event = listener.mock.calls.find(
        (call) => call[0].event === "session:complete",
      );
      expect(event[0].data.metrics.actionsSucceeded).toBe(1);
    });
  });

  describe("getMetrics()", () => {
    it("should return elapsed time", () => {
      progressTracker.startSession("Test");
      vi.advanceTimersByTime(2000);

      const metrics = progressTracker.getMetrics();
      expect(metrics.elapsed).toBeGreaterThanOrEqual(2000);
    });

    it("should return 0 elapsed when no session started", () => {
      const metrics = progressTracker.getMetrics();
      expect(metrics.elapsed).toBe(0);
    });

    it("should calculate successRate", () => {
      progressTracker.recordStep(1, { action: "click" }, { success: true });
      progressTracker.recordStep(2, { action: "click" }, { success: true });
      progressTracker.recordStep(3, { action: "click" }, { success: false });

      const metrics = progressTracker.getMetrics();
      expect(metrics.successRate).toBeCloseTo(2 / 3, 2);
    });

    it("should return 0 successRate when no actions", () => {
      const metrics = progressTracker.getMetrics();
      expect(metrics.successRate).toBe(0);
    });

    it("should calculate llmFailureRate", () => {
      progressTracker.recordLlmCall(true, 100);
      progressTracker.recordLlmCall(true, 100);
      progressTracker.recordLlmCall(false, 100);

      const metrics = progressTracker.getMetrics();
      expect(metrics.llmFailureRate).toBeCloseTo(1 / 3, 2);
    });

    it("should return 0 llmFailureRate when no LLM calls", () => {
      const metrics = progressTracker.getMetrics();
      expect(metrics.llmFailureRate).toBe(0);
    });
  });

  describe("reset()", () => {
    it("should reset all metrics", () => {
      progressTracker.startSession("Test");
      progressTracker.recordStep(1, { action: "click" }, { success: true });
      progressTracker.recordError("type", "msg");

      progressTracker.reset();

      const metrics = progressTracker.getMetrics();
      expect(metrics.stepsCompleted).toBe(0);
      expect(metrics.actionsAttempted).toBe(0);
      expect(metrics.errors).toBe(0);
      expect(metrics.startTime).toBeNull();
    });
  });

  describe("_emit()", () => {
    it("should handle wsServer emit errors gracefully", () => {
      const failingServer = {
        emit: vi.fn(() => {
          throw new Error("WS Error");
        }),
      };

      progressTracker.init(failingServer);

      // Should not throw
      expect(() => {
        progressTracker.recordError("test", "message");
      }).not.toThrow();
    });

    it("should handle listener errors gracefully", () => {
      const failingListener = vi.fn(() => {
        throw new Error("Listener Error");
      });
      progressTracker.addListener(failingListener);

      // Should not throw
      expect(() => {
        progressTracker.recordError("test", "message");
      }).not.toThrow();
    });
  });
});
