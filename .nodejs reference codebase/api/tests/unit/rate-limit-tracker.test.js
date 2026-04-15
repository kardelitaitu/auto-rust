/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { RateLimitTracker } from "@api/utils/rate-limit-tracker.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

global.fetch = vi.fn();

describe("rate-limit-tracker.js", () => {
  let tracker;

  beforeEach(() => {
    tracker = new RateLimitTracker({
      cacheDuration: 60000,
      warningThreshold: 0.2,
    });
    vi.clearAllMocks();
  });

  describe("constructor", () => {
    it("should initialize with default options", () => {
      const defaultTracker = new RateLimitTracker();
      expect(defaultTracker.cacheDuration).toBe(60000);
      expect(defaultTracker.warningThreshold).toBe(0.2);
    });

    it("should accept custom options", () => {
      const customTracker = new RateLimitTracker({
        cacheDuration: 30000,
        warningThreshold: 0.5,
      });
      expect(customTracker.cacheDuration).toBe(30000);
      expect(customTracker.warningThreshold).toBe(0.5);
    });
  });

  describe("checkKey", () => {
    it("should return cached data if within cache duration", async () => {
      const cachedData = { data: { limit_remaining: 100 } };
      tracker.cache.set("test-key", {
        data: cachedData,
        timestamp: Date.now(),
      });

      const result = await tracker.checkKey("test-key");
      expect(result).toEqual(cachedData);
    });

    it("should fetch new data if cache is expired", async () => {
      const apiData = {
        data: { limit_remaining: 50, is_free_tier: false, usage_daily: 10 },
      };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      tracker.cache.set("test-key", {
        data: {},
        timestamp: Date.now() - 70000,
      });

      const result = await tracker.checkKey("test-key");
      expect(result).toEqual(apiData);
    });

    it("should return null on fetch error", async () => {
      global.fetch.mockRejectedValue(new Error("Network error"));

      const result = await tracker.checkKey("test-key");
      expect(result).toBeNull();
    });

    it("should return null on non-ok response", async () => {
      global.fetch.mockResolvedValue({ ok: false, status: 401 });

      const result = await tracker.checkKey("test-key");
      expect(result).toBeNull();
    });

    it("should handle fetch that throws after initial check", async () => {
      tracker.cache.set("test-key", {
        data: {},
        timestamp: Date.now() - 70000,
      });

      global.fetch.mockRejectedValue(new Error("Connection refused"));

      const result = await tracker.checkKey("test-key");
      expect(result).toBeNull();
    });

    it("should cache the fetched data", async () => {
      const apiData = { data: { limit_remaining: 75 } };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      await tracker.checkKey("test-key");

      const cached = tracker.cache.get("test-key");
      expect(cached).toBeDefined();
      expect(cached.data).toEqual(apiData);
    });
  });

  describe("_fetchKeyInfo", () => {
    it("should throw on non-ok response status", async () => {
      global.fetch.mockResolvedValue({
        ok: false,
        status: 429,
        statusText: "Too Many Requests",
      });

      await expect(tracker._fetchKeyInfo("test-key")).rejects.toThrow(
        "HTTP 429",
      );
    });

    it("should throw on JSON parse error", async () => {
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockRejectedValue(new Error("Invalid JSON")),
      });

      await expect(tracker._fetchKeyInfo("test-key")).rejects.toThrow(
        "Invalid JSON",
      );
    });

    it("should return parsed JSON on success", async () => {
      const apiData = { data: { limit_remaining: 100, is_free_tier: true } };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      const result = await tracker._fetchKeyInfo("test-key");
      expect(result).toEqual(apiData);
    });

    it("should include correct authorization header", async () => {
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({ data: {} }),
      });

      await tracker._fetchKeyInfo("my-secret-key");

      expect(global.fetch).toHaveBeenCalledWith(
        "https://openrouter.ai/api/v1/key",
        expect.objectContaining({
          method: "GET",
          headers: expect.objectContaining({
            Authorization: "Bearer my-secret-key",
            "Content-Type": "application/json",
          }),
        }),
      );
    });
  });

  describe("trackRequest", () => {
    it("should track request for an API key", () => {
      tracker.trackRequest("test-key", "gpt-4");

      const history = tracker.requestHistory.get("test-key");
      expect(history).toBeDefined();
      expect(history.total).toBe(1);
      expect(history.requests[0].model).toBe("gpt-4");
    });

    it("should not track request if no API key provided", () => {
      tracker.trackRequest(null, "gpt-4");
      expect(tracker.requestHistory.size).toBe(0);
    });

    it("should append to existing request history", () => {
      tracker.trackRequest("test-key", "gpt-4");
      tracker.trackRequest("test-key", "claude-3");

      const history = tracker.requestHistory.get("test-key");
      expect(history.total).toBe(2);
    });
  });

  describe("getRemaining", () => {
    it("should return remaining from cached data", () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 100 } },
        timestamp: Date.now(),
      });

      expect(tracker.getRemaining("test-key")).toBe(100);
    });

    it("should return null if no cached data", () => {
      expect(tracker.getRemaining("non-existent")).toBeNull();
    });

    it("should return null if limit_remaining is not a number", () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: "unknown" } },
        timestamp: Date.now(),
      });

      expect(tracker.getRemaining("test-key")).toBeNull();
    });
  });

  describe("getIsFreeTier", () => {
    it("should return free tier status from cached data", () => {
      tracker.cache.set("test-key", {
        data: { data: { is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getIsFreeTier("test-key")).toBe(true);
    });

    it("should return null if no cached data", () => {
      expect(tracker.getIsFreeTier("non-existent")).toBeNull();
    });
  });

  describe("getUsageToday", () => {
    it("should return usage from cached data", () => {
      tracker.cache.set("test-key", {
        data: { data: { usage_daily: 150 } },
        timestamp: Date.now(),
      });

      expect(tracker.getUsageToday("test-key")).toBe(150);
    });

    it("should return 0 if usage_daily is undefined", () => {
      tracker.cache.set("test-key", {
        data: { data: {} },
        timestamp: Date.now(),
      });

      expect(tracker.getUsageToday("test-key")).toBe(0);
    });
  });

  describe("getWarningStatus", () => {
    it('should return "unknown" if no cached data', () => {
      expect(tracker.getWarningStatus("non-existent")).toBe("unknown");
    });

    it('should return "unknown" when remaining is null', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: null, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("unknown");
    });

    it('should return "unknown" when limit_remaining is undefined', () => {
      tracker.cache.set("test-key", {
        data: { data: { is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("unknown");
    });

    it('should return "exhausted" if remaining is 0', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 0, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("exhausted");
    });

    it('should return "exhausted" if remaining is negative', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: -5, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("exhausted");
    });

    it('should return "critical" for free tier with less than 10 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 5, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("critical");
    });

    it('should return "critical" for free tier with exactly 9 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 9, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("critical");
    });

    it('should return "warning" for non-free tier with less than 50 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 30, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("warning");
    });

    it('should return "warning" for non-free tier with exactly 49 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 49, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("warning");
    });

    it('should return "ok" for sufficient remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 100, is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("ok");
    });

    it('should return "warning" for free tier with 10 remaining (not enough for ok)', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 10, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("warning");
    });

    it('should return "ok" for free tier with 50 or more remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 50, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("ok");
    });

    it('should return "ok" for free tier with 100 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 100, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("ok");
    });
  });

  describe("getRequestRate", () => {
    it("should return 0 for unknown key", () => {
      expect(tracker.getRequestRate("non-existent")).toBe(0);
    });

    it("should return count of requests in time window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [
          { timestamp: now - 1000, model: "gpt-4" },
          { timestamp: now - 30000, model: "gpt-4" },
          { timestamp: now - 70000, model: "gpt-4" },
        ],
        total: 3,
      });

      expect(tracker.getRequestRate("test-key", 60000)).toBe(2);
    });

    it("should return all requests within larger window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [
          { timestamp: now - 1000, model: "gpt-4" },
          { timestamp: now - 30000, model: "gpt-4" },
          { timestamp: now - 70000, model: "gpt-4" },
        ],
        total: 3,
      });

      expect(tracker.getRequestRate("test-key", 120000)).toBe(3);
    });

    it("should exclude requests exactly at window boundary (strict less than)", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [{ timestamp: now - 60000, model: "gpt-4" }],
        total: 1,
      });

      expect(tracker.getRequestRate("test-key", 60000)).toBe(0);
    });

    it("should exclude requests just outside window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [{ timestamp: now - 60001, model: "gpt-4" }],
        total: 1,
      });

      expect(tracker.getRequestRate("test-key", 60000)).toBe(0);
    });

    it("should use default 60 second window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [{ timestamp: now - 30000, model: "gpt-4" }],
        total: 1,
      });

      expect(tracker.getRequestRate("test-key")).toBe(1);
    });
  });

  describe("refreshKey", () => {
    it("should invalidate cache and fetch new data", async () => {
      tracker.cache.set("test-key", {
        data: { old: true },
        timestamp: Date.now(),
      });

      const apiData = { data: { limit_remaining: 50 } };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      const result = await tracker.refreshKey("test-key");

      expect(result).toEqual(apiData);
    });
  });

  describe("invalidateCache", () => {
    it("should clear all cached data", () => {
      tracker.cache.set("key1", { data: {} });
      tracker.cache.set("key2", { data: {} });

      tracker.invalidateCache();

      expect(tracker.cache.size).toBe(0);
    });
  });

  describe("getCacheStatus", () => {
    it("should return status for all cached keys", () => {
      tracker.cache.set("test-key-12345678", {
        data: { data: { limit_remaining: 100 } },
        timestamp: Date.now() - 10000,
      });

      const status = tracker.getCacheStatus();
      const key = Object.keys(status)[0];
      expect(status[key]).toBeDefined();
      expect(status[key].remaining).toBe(100);
    });

    it("should return empty object when cache is empty", () => {
      const status = tracker.getCacheStatus();
      expect(status).toEqual({});
    });

    it("should return undefined remaining when data is empty object", () => {
      tracker.cache.set("test-key-12345678", {
        data: { data: {} },
        timestamp: Date.now(),
      });

      const status = tracker.getCacheStatus();
      const key = Object.keys(status)[0];
      expect(status[key].remaining).toBeUndefined();
    });

    it("should return undefined remaining when data is undefined", () => {
      tracker.cache.set("test-key-12345678", {
        data: undefined,
        timestamp: Date.now(),
      });

      const status = tracker.getCacheStatus();
      const key = Object.keys(status)[0];
      expect(status[key].remaining).toBeUndefined();
    });

    it("should include age in milliseconds", () => {
      const now = Date.now();
      tracker.cache.set("test-key-12345678", {
        data: { data: { limit_remaining: 50 } },
        timestamp: now - 5000,
      });

      const status = tracker.getCacheStatus();
      const key = Object.keys(status)[0];
      expect(status[key].age).toBeGreaterThanOrEqual(5000);
    });
  });

  describe("_maskKey", () => {
    it("should mask key properly", () => {
      expect(tracker._maskKey("12345678")).toBe("123456...5678");
      expect(tracker._maskKey("short")).toBe("***");
      expect(tracker._maskKey(null)).toBe("null");
    });
  });

  describe("getAllKeyStatus", () => {
    it("should return status for multiple keys", async () => {
      tracker.cache.set("key1", {
        data: {
          data: { limit_remaining: 100, is_free_tier: false, usage_daily: 10 },
        },
        timestamp: Date.now(),
      });
      tracker.cache.set("key2", {
        data: {
          data: { limit_remaining: 50, is_free_tier: true, usage_daily: 5 },
        },
        timestamp: Date.now(),
      });

      const results = await tracker.getAllKeyStatus(["key1", "key2"]);

      expect(results).toHaveLength(2);
      expect(results[0].remaining).toBe(100);
      expect(results[1].remaining).toBe(50);
    });

    it("should return empty array for empty input", async () => {
      const results = await tracker.getAllKeyStatus([]);
      expect(results).toHaveLength(0);
      expect(results).toEqual([]);
    });

    it("should return unknown status for non-existent keys", async () => {
      const results = await tracker.getAllKeyStatus(["non-existent-key"]);

      expect(results).toHaveLength(1);
      expect(results[0].remaining).toBeNull();
      expect(results[0].warning).toBe("unknown");
      expect(results[0].usageToday).toBeNull();
      expect(results[0].isFreeTier).toBeNull();
    });
  });

  describe("getStats", () => {
    it("should return statistics", () => {
      tracker.requestHistory.set("key1", { total: 10 });
      tracker.requestHistory.set("key2", { total: 5 });
      tracker.cache.set("key1", { data: {} });

      const stats = tracker.getStats();

      expect(stats.cachedKeys).toBe(1);
      expect(stats.trackedKeys).toBe(2);
      expect(stats.totalRequests).toBe(15);
    });

    it("should return zeros when no history or cache", () => {
      const stats = tracker.getStats();

      expect(stats.cachedKeys).toBe(0);
      expect(stats.trackedKeys).toBe(0);
      expect(stats.totalRequests).toBe(0);
    });

    it("should handle history with missing total property", () => {
      tracker.requestHistory.set("key1", { requests: [] });

      const stats = tracker.getStats();

      expect(stats.totalRequests).toBeNaN();
    });

    it("should count requests across multiple keys", () => {
      tracker.requestHistory.set("key1", { total: 3, requests: [] });
      tracker.requestHistory.set("key2", { total: 7, requests: [] });
      tracker.requestHistory.set("key3", { total: 2, requests: [] });

      const stats = tracker.getStats();

      expect(stats.trackedKeys).toBe(3);
      expect(stats.totalRequests).toBe(12);
    });

    it("should handle empty requests array", () => {
      tracker.requestHistory.set("key1", { total: 0, requests: [] });

      const stats = tracker.getStats();

      expect(stats.totalRequests).toBe(0);
    });
  });

  describe("checkKey edge cases", () => {
    it("should return cached data even if undefined", async () => {
      tracker.cache.set("test-key", { data: undefined, timestamp: Date.now() });

      const result = await tracker.checkKey("test-key");
      expect(result).toBeUndefined();
    });

    it("should return cached data even if null", async () => {
      tracker.cache.set("test-key", { data: null, timestamp: Date.now() });

      const result = await tracker.checkKey("test-key");
      expect(result).toBeNull();
    });

    it("should fetch new data when cache is expired", async () => {
      const cachedData = { data: { limit_remaining: 100 } };
      tracker.cache.set("test-key", {
        data: cachedData,
        timestamp: Date.now() - 60000,
      });

      const apiData = { data: { limit_remaining: 50 } };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      const result = await tracker.checkKey("test-key");
      expect(result).toEqual(apiData);
    });
  });

  describe("getAllKeyStatus edge cases", () => {
    it("should handle mix of existing and non-existing keys", async () => {
      tracker.cache.set("key1", {
        data: {
          data: { limit_remaining: 100, is_free_tier: false, usage_daily: 10 },
        },
        timestamp: Date.now(),
      });

      const results = await tracker.getAllKeyStatus(["key1", "non-existent"]);

      expect(results).toHaveLength(2);
      expect(results[0].remaining).toBe(100);
      expect(results[1].remaining).toBeNull();
      expect(results[1].warning).toBe("unknown");
    });

    it("should handle empty data structure", async () => {
      tracker.cache.set("key1", {
        data: {},
        timestamp: Date.now(),
      });

      const results = await tracker.getAllKeyStatus(["key1"]);

      expect(results[0].remaining).toBeNull();
      expect(results[0].usageToday).toBe(0);
    });
  });

  describe("getRequestRate edge cases", () => {
    it("should return 0 for empty requests array", () => {
      tracker.requestHistory.set("test-key", {
        requests: [],
        total: 0,
      });

      expect(tracker.getRequestRate("test-key", 60000)).toBe(0);
    });

    it("should filter requests outside window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [
          { timestamp: now - 1000, model: "gpt-4" },
          { timestamp: now - 5000, model: "gpt-4" },
          { timestamp: now - 100000, model: "gpt-4" },
          { timestamp: now - 200000, model: "gpt-4" },
        ],
        total: 4,
      });

      expect(tracker.getRequestRate("test-key", 60000)).toBe(2);
    });
  });

  describe("getWarningStatus edge cases", () => {
    it('should return "ok" for free tier with exactly 50 remaining', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 50, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("ok");
    });

    it('should return "exhausted" for free tier with 0 remaining (not critical)', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 0, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("exhausted");
    });

    it('should return "warning" for free tier below 10', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 8, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("critical");
    });

    it('should return "warning" for free tier at 10', () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 10, is_free_tier: true } },
        timestamp: Date.now(),
      });

      expect(tracker.getWarningStatus("test-key")).toBe("warning");
    });
  });

  describe("_maskKey edge cases", () => {
    it("should handle empty string as null", () => {
      expect(tracker._maskKey("")).toBe("null");
    });

    it("should handle exactly 8 character key", () => {
      expect(tracker._maskKey("12345678")).toBe("123456...5678");
    });

    it("should handle 7 character key", () => {
      expect(tracker._maskKey("1234567")).toBe("***");
    });

    it("should handle 9 character key", () => {
      expect(tracker._maskKey("123456789")).toBe("123456...6789");
    });

    it("should handle very long key", () => {
      const longKey = "a".repeat(50);
      const result = tracker._maskKey(longKey);
      expect(result).toBe("aaaaaa...aaaa");
    });
  });

  describe("getRequestRate additional coverage", () => {
    it("should count all requests in very short window", () => {
      const now = Date.now();
      tracker.requestHistory.set("test-key", {
        requests: [
          { timestamp: now, model: "gpt-4" },
          { timestamp: now - 100, model: "gpt-4" },
          { timestamp: now - 200, model: "gpt-4" },
        ],
        total: 3,
      });

      expect(tracker.getRequestRate("test-key", 1000)).toBe(3);
    });
  });

  describe("getRemaining additional coverage", () => {
    it("should handle nested data structure correctly", () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 0 } },
        timestamp: Date.now(),
      });

      expect(tracker.getRemaining("test-key")).toBe(0);
    });

    it("should return null for undefined limit_remaining", () => {
      tracker.cache.set("test-key", {
        data: { data: {} },
        timestamp: Date.now(),
      });

      expect(tracker.getRemaining("test-key")).toBeNull();
    });
  });

  describe("getIsFreeTier additional coverage", () => {
    it("should return false when is_free_tier is not present", () => {
      tracker.cache.set("test-key", {
        data: { data: { limit_remaining: 100 } },
        timestamp: Date.now(),
      });

      expect(tracker.getIsFreeTier("test-key")).toBe(false);
    });

    it("should return false when is_free_tier is explicitly false", () => {
      tracker.cache.set("test-key", {
        data: { data: { is_free_tier: false } },
        timestamp: Date.now(),
      });

      expect(tracker.getIsFreeTier("test-key")).toBe(false);
    });
  });

  describe("getUsageToday additional coverage", () => {
    it("should handle string usage_daily", () => {
      tracker.cache.set("test-key", {
        data: { data: { usage_daily: "100" } },
        timestamp: Date.now(),
      });

      expect(tracker.getUsageToday("test-key")).toBe("100");
    });

    it("should handle negative usage_daily", () => {
      tracker.cache.set("test-key", {
        data: { data: { usage_daily: -5 } },
        timestamp: Date.now(),
      });

      expect(tracker.getUsageToday("test-key")).toBe(-5);
    });
  });

  describe("refreshKey additional coverage", () => {
    it("should remove old cache entry before fetching", async () => {
      tracker.cache.set("test-key", {
        data: { old: true },
        timestamp: Date.now(),
      });

      const apiData = { data: { limit_remaining: 50 } };
      global.fetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(apiData),
      });

      await tracker.refreshKey("test-key");

      const cached = tracker.cache.get("test-key");
      expect(cached.data).toEqual(apiData);
    });
  });

  describe("getCacheStatus additional coverage", () => {
    it("should handle multiple keys", () => {
      tracker.cache.set("key1-12345678", {
        data: { data: { limit_remaining: 100 } },
        timestamp: Date.now() - 1000,
      });
      tracker.cache.set("key2-12345678", {
        data: { data: { limit_remaining: 50 } },
        timestamp: Date.now() - 2000,
      });
      tracker.cache.set("key3-12345678", {
        data: { data: {} },
        timestamp: Date.now() - 3000,
      });

      const status = tracker.getCacheStatus();
      const keys = Object.keys(status);

      expect(keys).toHaveLength(3);
      expect(keys.some((k) => k.includes("key1"))).toBe(true);
      expect(keys.some((k) => k.includes("key2"))).toBe(true);
      expect(keys.some((k) => k.includes("key3"))).toBe(true);
    });
  });
});
