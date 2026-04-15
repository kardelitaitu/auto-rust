/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { ModelPerfTracker } from "@api/utils/model-perf-tracker.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("ModelPerfTracker", () => {
  let tracker;

  beforeEach(() => {
    vi.clearAllMocks();
    tracker = new ModelPerfTracker({ windowSize: 10, minSamples: 3 });
  });

  describe("constructor", () => {
    it("should create with default options", () => {
      const defaultTracker = new ModelPerfTracker();
      expect(defaultTracker.windowSize).toBe(100);
      expect(defaultTracker.minSamples).toBe(5);
    });

    it("should create with custom options", () => {
      expect(tracker.windowSize).toBe(10);
      expect(tracker.minSamples).toBe(3);
    });
  });

  describe("trackSuccess", () => {
    it("should track success for new model", () => {
      tracker.trackSuccess("gpt-4", 100);

      const stats = tracker.getModelStats("gpt-4");
      expect(stats).not.toBeNull();
      expect(stats.successes).toBe(1);
      expect(stats.total).toBe(1);
      expect(stats.avgDuration).toBe(50);
    });

    it("should track success for existing model", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("gpt-4", 200);

      const stats = tracker.getModelStats("gpt-4");
      expect(stats.successes).toBe(2);
      expect(stats.total).toBe(2);
    });

    it("should track success with apiKey", () => {
      tracker.trackSuccess("gpt-4", 100, "sk-key123");

      const stats = tracker.getModelStats("gpt-4", "sk-key123");
      expect(stats).not.toBeNull();
      expect(stats.apiKey).toBe("sk-k...y123");
    });

    it("should maintain recent durations window", () => {
      const smallWindow = new ModelPerfTracker({ windowSize: 3 });

      smallWindow.trackSuccess("gpt-4", 100);
      smallWindow.trackSuccess("gpt-4", 200);
      smallWindow.trackSuccess("gpt-4", 300);
      smallWindow.trackSuccess("gpt-4", 400);

      const stats = smallWindow.getModelStats("gpt-4");
      expect(stats.recentDurations.length).toBe(3);
      expect(stats.recentDurations).toContain(400);
      expect(stats.recentDurations).not.toContain(100);
    });
  });

  describe("trackFailure", () => {
    it("should track failure for new model", () => {
      tracker.trackFailure("gpt-4", "Rate limit exceeded");

      const stats = tracker.getModelStats("gpt-4");
      expect(stats).not.toBeNull();
      expect(stats.failures).toBe(1);
      expect(stats.total).toBe(1);
      expect(stats.lastError).toBe("Rate limit exceeded");
    });

    it("should track failure for existing model", () => {
      tracker.trackFailure("gpt-4", "Error 1");
      tracker.trackFailure("gpt-4", "Error 2");

      const stats = tracker.getModelStats("gpt-4");
      expect(stats.failures).toBe(2);
      expect(stats.total).toBe(2);
    });

    it("should truncate long error messages", () => {
      const longError = "A".repeat(200);
      tracker.trackFailure("gpt-4", longError);

      const stats = tracker.getModelStats("gpt-4");
      expect(stats.lastError.length).toBe(100);
    });
  });

  describe("getModelStats", () => {
    it("should return stats for existing model", () => {
      tracker.trackSuccess("gpt-4", 100);

      const stats = tracker.getModelStats("gpt-4");
      expect(stats.model).toBe("gpt-4");
    });

    it("should return null for non-existing model", () => {
      const stats = tracker.getModelStats("nonexistent");
      expect(stats).toBeNull();
    });
  });

  describe("getAllStats", () => {
    it("should return empty object when no stats", () => {
      const allStats = tracker.getAllStats();
      expect(Object.keys(allStats).length).toBe(0);
    });

    it("should return formatted stats for all models", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackFailure("gpt-4", "Error");

      const allStats = tracker.getAllStats();
      expect(allStats["gpt-4"]).not.toBeUndefined();
      expect(allStats["gpt-4"].successRate).toBe("50.0%");
    });

    it("should return N/A for zero totals in getAllStats", () => {
      const freshTracker = new ModelPerfTracker();
      const allStats = freshTracker.getAllStats();
      expect(Object.keys(allStats).length).toBe(0);
    });
  });

  describe("getBestModel", () => {
    it("should return null for empty model list", () => {
      const best = tracker.getBestModel([]);
      expect(best).toBeNull();
    });

    it("should return null if no model meets minSamples", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("gpt-4", 100);

      const best = tracker.getBestModel(["gpt-4", "claude"]);
      expect(best).toBeNull();
    });

    it("should return best model based on score", () => {
      tracker.trackSuccess("fast-model", 50);
      tracker.trackSuccess("fast-model", 50);
      tracker.trackSuccess("fast-model", 50);
      tracker.trackFailure("slow-model", "Error");
      tracker.trackFailure("slow-model", "Error");
      tracker.trackFailure("slow-model", "Error");

      const best = tracker.getBestModel(["fast-model", "slow-model"]);
      expect(best).toBe("fast-model");
    });
  });

  describe("getWorstModel", () => {
    it("should return null for empty model list", () => {
      const worst = tracker.getWorstModel([]);
      expect(worst).toBeNull();
    });

    it("should return null if no model meets minSamples", () => {
      tracker.trackSuccess("gpt-4", 100);

      const worst = tracker.getWorstModel(["gpt-4", "claude"]);
      expect(worst).toBeNull();
    });

    it("should return worst model based on score", () => {
      tracker.trackSuccess("fast-model", 50);
      tracker.trackSuccess("fast-model", 50);
      tracker.trackSuccess("fast-model", 50);
      tracker.trackFailure("slow-model", "Error");
      tracker.trackFailure("slow-model", "Error");
      tracker.trackFailure("slow-model", "Error");

      const worst = tracker.getWorstModel(["fast-model", "slow-model"]);
      expect(worst).toBe("slow-model");
    });
  });

  describe("getSortedModels", () => {
    it("should return empty array when no stats", () => {
      const sorted = tracker.getSortedModels();
      expect(sorted).toEqual([]);
    });

    it("should return models sorted by success rate", () => {
      tracker.trackSuccess("good-model", 100);
      tracker.trackSuccess("good-model", 100);
      tracker.trackFailure("bad-model", "Error");

      const sorted = tracker.getSortedModels();
      expect(sorted[0].model).toBe("good-model");
    });

    it("should handle zero total for successRate calculation", () => {
      tracker.trackFailure("empty-model", "Error");

      const sorted = tracker.getSortedModels();
      expect(sorted[0].successRate).toBe(0);
    });

    it("should filter by apiKey", () => {
      tracker.trackSuccess("model1", 100, "key1");
      tracker.trackSuccess("model2", 100, "key2");

      const sorted = tracker.getSortedModels("key1");
      expect(sorted.length).toBe(1);
      expect(sorted[0].model).toBe("model1");
    });
  });

  describe("getHistory", () => {
    it("should return default 50 entries", () => {
      for (let i = 0; i < 60; i++) {
        tracker.trackSuccess("gpt-4", 100);
      }

      const history = tracker.getHistory();
      expect(history.length).toBe(50);
    });

    it("should respect limit parameter", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("gpt-4", 100);

      const history = tracker.getHistory(1);
      expect(history.length).toBe(1);
    });

    it("should trim history when exceeding 1000 entries", () => {
      for (let i = 0; i < 1005; i++) {
        tracker.trackSuccess("gpt-4", 100);
      }

      expect(tracker.history.length).toBeLessThanOrEqual(1000);
    });
  });

  describe("reset", () => {
    it("should reset specific model", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("claude", 100);

      tracker.reset("gpt-4");

      expect(tracker.getModelStats("gpt-4")).toBeNull();
      expect(tracker.getModelStats("claude")).not.toBeNull();
    });

    it("should reset all models", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("claude", 100);

      tracker.reset();

      expect(tracker.getModelStats("gpt-4")).toBeNull();
      expect(tracker.getModelStats("claude")).toBeNull();
      expect(tracker.history.length).toBe(0);
    });
  });

  describe("getStats", () => {
    it("should return empty stats when no data", () => {
      const stats = tracker.getStats();
      expect(stats.modelsTracked).toBe(0);
      expect(stats.totalRequests).toBe(0);
      expect(stats.overallSuccessRate).toBe("N/A");
    });

    it("should aggregate all stats", () => {
      tracker.trackSuccess("gpt-4", 100);
      tracker.trackSuccess("claude", 200);
      tracker.trackFailure("gpt-4", "Error");

      const stats = tracker.getStats();
      expect(stats.modelsTracked).toBe(2);
      expect(stats.totalRequests).toBe(3);
      expect(stats.totalSuccesses).toBe(2);
      expect(stats.totalFailures).toBe(1);
    });
  });

  describe("private methods", () => {
    it("_maskKey should mask long keys", () => {
      const key = tracker._maskKey("sk-1234567890abcdef");
      expect(key).toBe("sk-1...cdef");
    });

    it("_maskKey should handle short keys", () => {
      const key = tracker._maskKey("short");
      expect(key).toBe("***");
    });

    it("_maskKey should handle null", () => {
      const key = tracker._maskKey(null);
      expect(key).toBe("null");
    });

    it("_calculateAvg should compute running average", () => {
      const avg = tracker._calculateAvg(100, 2, 200);
      expect(avg).toBe(133.33333333333334);
    });

    it("_calculateRecentAvg should handle empty array", () => {
      const avg = tracker._calculateRecentAvg([]);
      expect(avg).toBe(0);
    });

    it("_calculateRecentAvg should compute average", () => {
      const avg = tracker._calculateRecentAvg([100, 200, 300]);
      expect(avg).toBe(200);
    });
  });
});
