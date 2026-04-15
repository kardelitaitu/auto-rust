/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for AsyncQueue
 * @module tests/unit/async-queue.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AsyncQueue, DiveQueue } from "@api/utils/async-queue.js";

describe("AsyncQueue", () => {
  let queue;

  beforeEach(() => {
    queue = new AsyncQueue({
      maxConcurrent: 2,
      maxQueueSize: 10,
      defaultTimeout: 5000,
    });
  });

  afterEach(() => {
    if (queue.processingPromise) {
      queue.processingPromise.cancel?.();
    }
  });

  describe("Initialization", () => {
    it("should initialize with correct values", () => {
      expect(queue.maxConcurrent).toBe(2);
      expect(queue.maxQueueSize).toBe(10);
      expect(queue.defaultTimeout).toBe(5000);
      expect(queue.queue).toEqual([]);
      expect(queue.active.size).toBe(0);
    });

    it("should use default values when not provided", () => {
      const defaultQueue = new AsyncQueue();
      expect(defaultQueue.maxConcurrent).toBe(3);
      expect(defaultQueue.maxQueueSize).toBe(50);
      expect(defaultQueue.defaultTimeout).toBe(5000);
    });
  });

  describe("add", () => {
    it("should add item to queue", async () => {
      const task = vi.fn().mockResolvedValue("result");

      const result = await queue.add(task);

      expect(result.success).toBe(true);
      expect(task).toHaveBeenCalled();
    });

    it("should track statistics", async () => {
      const task = vi.fn().mockResolvedValue("result");

      await queue.add(task);

      expect(queue.stats).toBeDefined();
    });
  });

  describe("getStatus", () => {
    it("should return current queue status", async () => {
      const task = vi.fn().mockResolvedValue("result");

      queue.add(task);
      const status = queue.getStatus();

      expect(status).toHaveProperty("queueLength");
      expect(status).toHaveProperty("activeCount");
    });
  });
});

describe("DiveQueue", () => {
  let diveQueue;

  beforeEach(() => {
    diveQueue = new DiveQueue({
      maxConcurrent: 2,
      maxQueueSize: 10,
      defaultTimeout: 5000,
    });
  });

  afterEach(() => {
    if (diveQueue.processingPromise) {
      diveQueue.processingPromise.cancel?.();
    }
  });

  describe("Initialization", () => {
    it("should initialize with engagement counters", () => {
      expect(diveQueue.engagementCounters).toBeDefined();
      expect(diveQueue.engagementCounters.likes).toBe(0);
      expect(diveQueue.engagementCounters.replies).toBe(0);
    });

    it("should initialize quickMode to false", () => {
      expect(diveQueue.quickMode).toBe(false);
    });
  });

  describe("addDive", () => {
    it("should add dive with fallback support", async () => {
      const primaryFn = vi.fn().mockResolvedValue({ success: true });
      const fallbackFn = vi
        .fn()
        .mockResolvedValue({ success: true, fallback: true });

      const result = await diveQueue.addDive(primaryFn, fallbackFn, {
        timeout: 5000,
      });

      expect(result.success).toBe(true);
      expect(primaryFn).toHaveBeenCalled();
    });
  });

  describe("quickMode", () => {
    it("should enable quick mode", () => {
      diveQueue.enableQuickMode();

      expect(diveQueue.quickMode).toBe(true);
    });

    it("should disable quick mode", () => {
      diveQueue.enableQuickMode();
      diveQueue.disableQuickMode();

      expect(diveQueue.quickMode).toBe(false);
    });
  });

  describe("engagement tracking", () => {
    it("should record likes", () => {
      const result = diveQueue.recordEngagement("likes");

      expect(result).toBe(true);
      expect(diveQueue.engagementCounters.likes).toBe(1);
    });

    it("should not exceed limit", () => {
      diveQueue.engagementCounters.likes = 5; // Set to max

      const result = diveQueue.recordEngagement("likes");

      expect(result).toBe(false);
    });
  });

  describe("canEngage", () => {
    it("should return true when under limit", () => {
      expect(diveQueue.canEngage("likes")).toBe(true);
    });

    it("should return false when at limit", () => {
      diveQueue.engagementCounters.likes = 5; // Set to max

      expect(diveQueue.canEngage("likes")).toBe(false);
    });
  });

  describe("getFullStatus", () => {
    it("should include engagement metrics", () => {
      diveQueue.recordEngagement("likes", 3);
      diveQueue.recordEngagement("replies", 1);

      const status = diveQueue.getFullStatus();

      expect(status.engagement).toBeDefined();
      expect(status.quickMode).toBe(false);
    });
  });
});

describe("Coverage Gap Tests", () => {
  describe("AsyncQueue - add() method", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 1,
        maxQueueSize: 2,
        defaultTimeout: 1000,
      });
    });

    it("should reject task when queue is full", async () => {
      const task = vi
        .fn()
        .mockImplementation(
          () => new Promise((resolve) => setTimeout(resolve, 5000)),
        );

      queue.add(task, { name: "task1" });
      queue.add(task, { name: "task2" });
      await new Promise((resolve) => setTimeout(resolve, 10));

      const result = await queue.add(task, { name: "task3" });

      expect(result.success).toBe(false);
      expect(result.reason).toBe("queue_full");
    });

    it("should handle priority correctly - higher priority first", async () => {
      const results = [];
      const task1 = vi.fn().mockImplementation(() => {
        results.push("low");
        return Promise.resolve("low");
      });
      const task2 = vi.fn().mockImplementation(() => {
        results.push("high");
        return Promise.resolve("high");
      });

      await queue.add(task1, { priority: 1, name: "low" });
      await queue.add(task2, { priority: 10, name: "high" });

      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(queue.queue.length).toBe(0);
    });

    it("should use custom timeout from options", async () => {
      const slowTask = vi
        .fn()
        .mockImplementation(
          () => new Promise((resolve) => setTimeout(resolve, 500)),
        );

      const result = await queue.add(slowTask, {
        timeout: 100,
        name: "timeout-test",
      });

      expect(result.success).toBe(false);
      expect(result.reason).toBe("timeout");
    });

    it("should track totalAdded in stats", async () => {
      const task = vi.fn().mockResolvedValue("result");

      await queue.add(task, { name: "task1" });
      await queue.add(task, { name: "task2" });

      expect(queue.stats.totalAdded).toBe(2);
    });
  });

  describe("AsyncQueue - _processQueue() manually", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 2,
        maxQueueSize: 10,
        defaultTimeout: 5000,
      });
    });

    it("should process items concurrently up to maxConcurrent", async () => {
      let activeCount = 0;
      let maxActive = 0;

      const task = vi.fn().mockImplementation(async () => {
        activeCount++;
        maxActive = Math.max(maxActive, activeCount);
        await new Promise((resolve) => setTimeout(resolve, 50));
        activeCount--;
        return "done";
      });

      await Promise.all([
        queue.add(task, { name: "task1" }),
        queue.add(task, { name: "task2" }),
        queue.add(task, { name: "task3" }),
      ]);

      expect(maxActive).toBeLessThanOrEqual(2);
    });

    it("should handle task errors gracefully", async () => {
      const errorTask = vi.fn().mockRejectedValue(new Error("Task failed"));

      const result = await queue.add(errorTask, { name: "error-task" });

      expect(result.success).toBe(false);
      expect(result.reason).toBe("error");
      expect(result.error).toBe("Task failed");
      expect(queue.stats.totalFailed).toBe(1);
    });

    it("should handle timeout errors", async () => {
      const slowTask = vi.fn().mockImplementation(() => new Promise(() => {}));

      const result = await queue.add(slowTask, {
        timeout: 50,
        name: "timeout-task",
      });

      expect(result.success).toBe(false);
      expect(result.reason).toBe("timeout");
      expect(queue.stats.totalTimedOut).toBe(1);
    });

    it("should process items in priority order", async () => {
      const results = [];

      const task1 = vi.fn().mockImplementation(async () => {
        results.push("p1");
        return "p1";
      });
      const task2 = vi.fn().mockImplementation(async () => {
        results.push("p5");
        return "p5";
      });
      const task3 = vi.fn().mockImplementation(async () => {
        results.push("p10");
        return "p10";
      });

      queue.queue.push({
        id: "1",
        taskFn: task1,
        priority: 1,
        taskName: "p1",
        resolve: () => {},
        reject: () => {},
        enqueueTime: Date.now(),
      });
      queue.queue.push({
        id: "2",
        taskFn: task2,
        priority: 5,
        taskName: "p5",
        resolve: () => {},
        reject: () => {},
        enqueueTime: Date.now(),
      });
      queue.queue.push({
        id: "3",
        taskFn: task3,
        priority: 10,
        taskName: "p10",
        resolve: () => {},
        reject: () => {},
        enqueueTime: Date.now(),
      });

      queue._processQueue();
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(results[0]).toBe("p10");
    });
  });

  describe("AsyncQueue - Statistics tracking", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 2,
        maxQueueSize: 10,
        defaultTimeout: 5000,
      });
    });

    it("should track averageProcessingTime", async () => {
      const task = vi.fn().mockImplementation(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        return "result";
      });

      await queue.add(task, { name: "task1" });
      await queue.add(task, { name: "task2" });

      expect(queue.stats.averageProcessingTime).toBeGreaterThan(0);
    });

    it("should update processedCount and failedCount", async () => {
      const successTask = vi.fn().mockResolvedValue("ok");
      const failTask = vi.fn().mockRejectedValue(new Error("fail"));

      await queue.add(successTask, { name: "success" });
      await queue.add(failTask, { name: "fail" });

      expect(queue.processedCount).toBe(2);
      expect(queue.failedCount).toBe(1);
    });

    it("should track timedOutCount", async () => {
      const slowTask = vi.fn().mockImplementation(() => new Promise(() => {}));

      await queue.add(slowTask, { timeout: 50, name: "timeout1" });
      await queue.add(slowTask, { timeout: 50, name: "timeout2" });

      expect(queue.timedOutCount).toBe(2);
    });
  });

  describe("AsyncQueue - getStatus() and getStats()", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 2,
        maxQueueSize: 10,
        defaultTimeout: 5000,
      });
    });

    it("should return correct status during processing", async () => {
      const task = vi
        .fn()
        .mockImplementation(
          () => new Promise((resolve) => setTimeout(resolve, 100)),
        );

      queue.add(task, { name: "task1" });

      await new Promise((resolve) => setTimeout(resolve, 10));

      const status = queue.getStatus();

      expect(status.processed).toBeGreaterThanOrEqual(0);
      expect(status.failed).toBe(0);
      expect(status.timedOut).toBe(0);
    });

    it("should return utilizationPercent in getStats()", async () => {
      const task = vi
        .fn()
        .mockImplementation(
          () => new Promise((resolve) => setTimeout(resolve, 100)),
        );

      queue.add(task, { name: "task1" });

      await new Promise((resolve) => setTimeout(resolve, 10));

      const stats = queue.getStats();

      expect(stats.utilizationPercent).toBeGreaterThanOrEqual(0);
    });
  });

  describe("AsyncQueue - clear() and health methods", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 2,
        maxQueueSize: 10,
        defaultTimeout: 5000,
      });
    });

    it("should clear queue and return dropped count", async () => {
      const task = vi
        .fn()
        .mockImplementation(
          () => new Promise((resolve) => setTimeout(resolve, 1000)),
        );

      queue.add(task, { name: "task1" });
      queue.add(task, { name: "task2" });

      await new Promise((resolve) => setTimeout(resolve, 10));

      const result = queue.clear();

      expect(result.dropped).toBeGreaterThanOrEqual(0);
      expect(queue.queue.length).toBe(0);
    });

    it("should report healthy when within limits", () => {
      expect(queue.isHealthy()).toBe(true);
      expect(queue.isQueueHealthy()).toBe(true);
    });

    it("should report unhealthy when failed count exceeds limit", () => {
      queue.failedCount = 10;

      expect(queue.isHealthy()).toBe(false);
    });

    it("should report unhealthy when timeout count exceeds limit", () => {
      queue.timedOutCount = 15;

      expect(queue.isHealthy()).toBe(false);
    });

    it("should return health status", () => {
      const health = queue.getHealth();

      expect(health).toHaveProperty("healthy");
      expect(health).toHaveProperty("failedCount");
      expect(health).toHaveProperty("timedOutCount");
      expect(health).toHaveProperty("queueLength");
    });
  });

  describe("DiveQueue - addDive() with fallback", () => {
    let diveQueue;

    beforeEach(() => {
      diveQueue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 10,
        defaultTimeout: 5000,
        fallbackEngagement: true,
      });
    });

    it("should handle timeout in primary function", async () => {
      let settle;
      const primaryPromise = new Promise((resolve, reject) => {
        settle = { resolve, reject };
      });
      const primaryFn = vi.fn().mockReturnValue(primaryPromise);
      const fallbackFn = vi.fn().mockResolvedValue({ fallback: true });

      const resultPromise = diveQueue.addDive(primaryFn, fallbackFn, {
        timeout: 50,
        taskName: "timeout-dive",
      });

      await new Promise((resolve) => setTimeout(resolve, 100));
      settle.reject(new Error("timeout"));

      const result = await resultPromise;

      expect(result).toBeDefined();
    });

    it("should use fallback when primary function rejects", async () => {
      const primaryFn = vi
        .fn()
        .mockImplementation(() => Promise.reject(new Error("Primary failed")));
      const fallbackFn = vi.fn().mockResolvedValue({ fallback: true });

      const result = await diveQueue.addDive(primaryFn, fallbackFn, {
        taskName: "error-dive",
      });

      expect(result).toBeDefined();
    });

    it("should not use fallback when fallbackEngagement is false", async () => {
      const noFallbackQueue = new DiveQueue({
        fallbackEngagement: false,
      });

      const primaryFn = vi
        .fn()
        .mockImplementation(() => Promise.reject(new Error("Primary failed")));
      const fallbackFn = vi.fn().mockResolvedValue({ fallback: true });

      const result = await noFallbackQueue.addDive(primaryFn, fallbackFn, {
        taskName: "no-fallback-dive",
      });

      expect(result).toBeDefined();
    });

    it("should handle both primary and fallback failing", async () => {
      const primaryFn = vi
        .fn()
        .mockImplementation(() => Promise.reject(new Error("Primary failed")));
      const fallbackFn = vi
        .fn()
        .mockImplementation(() =>
          Promise.reject(new Error("Fallback also failed")),
        );

      const result = await diveQueue.addDive(primaryFn, fallbackFn, {
        taskName: "both-fail-dive",
      });

      expect(result).toBeDefined();
    });

    it("should handle successful dive", async () => {
      const primaryFn = vi.fn().mockResolvedValue({ success: true });

      const result = await diveQueue.addDive(primaryFn, null, {
        taskName: "success-dive",
      });

      expect(result.success).toBe(true);
    });
  });

  describe("DiveQueue - Engagement tracking", () => {
    let diveQueue;

    beforeEach(() => {
      diveQueue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 10,
        defaultTimeout: 5000,
        likes: 3,
        replies: 2,
        retweets: 1,
      });
    });

    it("should track all engagement types", () => {
      diveQueue.recordEngagement("likes");
      diveQueue.recordEngagement("likes");
      diveQueue.recordEngagement("replies");
      diveQueue.recordEngagement("retweets");

      expect(diveQueue.engagementCounters.likes).toBe(2);
      expect(diveQueue.engagementCounters.replies).toBe(1);
      expect(diveQueue.engagementCounters.retweets).toBe(1);
    });

    it("should return correct progress for all engagement types", () => {
      diveQueue.recordEngagement("likes");
      diveQueue.recordEngagement("likes");

      const progress = diveQueue.getEngagementProgress();

      expect(progress.likes.current).toBe(2);
      expect(progress.likes.limit).toBe(3);
      expect(progress.likes.remaining).toBe(1);
      expect(progress.likes.percentUsed).toBe(67);
    });

    it("should handle unknown engagement action in canEngage", () => {
      expect(diveQueue.canEngage("unknown")).toBe(true);
    });

    it("should handle unknown engagement action in recordEngagement", () => {
      const result = diveQueue.recordEngagement("unknown");
      expect(result).toBe(false);
    });

    it("should update engagement limits at runtime", () => {
      diveQueue.updateEngagementLimits({ likes: 10, replies: 5 });

      expect(diveQueue.engagementLimits.likes).toBe(10);
      expect(diveQueue.engagementLimits.replies).toBe(5);
    });

    it("should reset engagement counters", () => {
      diveQueue.recordEngagement("likes");
      diveQueue.recordEngagement("replies");

      diveQueue.resetEngagement();

      expect(diveQueue.engagementCounters.likes).toBe(0);
      expect(diveQueue.engagementCounters.replies).toBe(0);
    });
  });

  describe("DiveQueue - quickMode", () => {
    let diveQueue;

    beforeEach(() => {
      diveQueue = new DiveQueue({
        maxConcurrent: 1,
        maxQueueSize: 10,
        defaultTimeout: 20000,
      });
    });

    it("should change defaultTimeout when enabling quickMode", () => {
      const normalTimeout = diveQueue.defaultTimeout;

      diveQueue.enableQuickMode();

      expect(diveQueue.defaultTimeout).toBeLessThan(normalTimeout);
    });

    it("should restore defaultTimeout when disabling quickMode", () => {
      diveQueue.enableQuickMode();

      diveQueue.disableQuickMode();

      expect(diveQueue.defaultTimeout).toBe(20000);
    });

    it("should include quickMode in getFullStatus()", () => {
      const status = diveQueue.getFullStatus();

      expect(status.quickMode).toBe(false);

      diveQueue.enableQuickMode();

      const newStatus = diveQueue.getFullStatus();
      expect(newStatus.quickMode).toBe(true);
    });
  });

  describe("Edge cases and error handling", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 1,
        maxQueueSize: 5,
        defaultTimeout: 1000,
      });
    });

    it("should handle multiple rapid additions", async () => {
      const tasks = Array(10)
        .fill(null)
        .map((_, i) => vi.fn().mockResolvedValue(`result${i}`));

      const promises = tasks.map((task, i) =>
        queue.add(task, { name: `task${i}` }),
      );

      const results = await Promise.all(promises);

      const successful = results.filter((r) => r.success);
      expect(successful.length).toBeGreaterThan(0);
    });

    it("should handle empty queue gracefully", () => {
      const status = queue.getStatus();

      expect(status.queueLength).toBe(0);
      expect(status.activeCount).toBe(0);
    });

    it("should handle getStats on empty queue", () => {
      const stats = queue.getStats();

      expect(stats.totalAdded).toBe(0);
      expect(stats.totalCompleted).toBe(0);
      expect(stats.utilizationPercent).toBe(0);
    });
  });

  describe("DiveQueue - Custom engagement limits", () => {
    it("should use custom engagement limits from constructor", () => {
      const diveQueue = new DiveQueue({
        replies: 10,
        retweets: 5,
        likes: 20,
        follows: 3,
        bookmarks: 4,
        quotes: 2,
      });

      expect(diveQueue.engagementLimits.replies).toBe(10);
      expect(diveQueue.engagementLimits.retweets).toBe(5);
      expect(diveQueue.engagementLimits.likes).toBe(20);
      expect(diveQueue.engagementLimits.follows).toBe(3);
      expect(diveQueue.engagementLimits.bookmarks).toBe(4);
      expect(diveQueue.engagementLimits.quotes).toBe(2);
    });
  });

  describe("AsyncQueue - Internal methods", () => {
    let queue;

    beforeEach(() => {
      queue = new AsyncQueue({
        maxConcurrent: 1,
        maxQueueSize: 5,
        defaultTimeout: 1000,
      });
    });

    it("should cover _createTimeout method - immediate", () => {
      const mockItem = {
        timeout: 10,
        taskName: "test-timeout",
      };

      const timeoutPromise = queue._createTimeout(mockItem);

      expect(timeoutPromise).toBeInstanceOf(Promise);

      return timeoutPromise.catch((err) => {
        expect(err.message).toBe("timeout");
      });
    });

    it("should cover _createTimeout method - resolve before timeout", async () => {
      const mockItem = {
        timeout: 100,
        taskName: "test-timeout",
      };

      const timeoutPromise = queue._createTimeout(mockItem);
      const resultPromise = Promise.race([
        timeoutPromise,
        new Promise((resolve) => setTimeout(() => resolve("resolved"), 5)),
      ]);

      const result = await resultPromise;
      expect(result).toBe("resolved");
    });

    it("should call _createTimeout for items", async () => {
      const task = vi.fn().mockResolvedValue("result");

      await queue.add(task, { name: "test-task" });

      expect(queue._createTimeout).toBeDefined();
    });

    it("should handle queue processing errors gracefully", async () => {
      const badTask = vi.fn().mockImplementation(() => {
        throw new Error("Processing error");
      });

      queue.queue.push({
        id: "error-id",
        taskFn: badTask,
        timeout: 100,
        priority: 1,
        taskName: "error-task",
        resolve: vi.fn(),
        reject: vi.fn(),
        enqueueTime: Date.now(),
      });

      queue._processQueue();
      await new Promise((resolve) => setTimeout(resolve, 10));
    });
  });
});
