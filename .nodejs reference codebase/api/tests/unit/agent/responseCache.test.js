/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

describe("api/agent/responseCache.js", () => {
  let responseCache;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();
    const module = await import("@api/agent/responseCache.js");
    responseCache = module.responseCache || module.default;
    responseCache.clear();
  });

  afterEach(() => {
    responseCache.clear();
  });

  describe("responseCache", () => {
    it("should be defined", () => {
      expect(responseCache).toBeDefined();
    });

    it("should have all required methods", () => {
      expect(typeof responseCache.get).toBe("function");
      expect(typeof responseCache.set).toBe("function");
      expect(typeof responseCache.clear).toBe("function");
      expect(typeof responseCache.getKey).toBe("function");
      expect(typeof responseCache.getStats).toBe("function");
      expect(typeof responseCache.cleanup).toBe("function");
      expect(typeof responseCache.setSimilarityThreshold).toBe("function");
    });
  });

  describe("getKey", () => {
    it("should generate cache key from context", () => {
      const context = {
        url: "https://example.com/path",
        goal: "click button",
        pageType: "dashboard",
        elementHash: "abc123",
      };

      const key = responseCache.getKey(context);
      expect(key).toBe("example.com/path|click button|dashboard|abc123");
    });

    it("should normalize URL by removing query params and hash", () => {
      const context = {
        url: "https://example.com/path?query=value&other=123#hash",
        goal: "test",
      };

      const key = responseCache.getKey(context);
      expect(key).toBe("example.com/path|test|unknown|none");
    });

    it("should handle invalid URL gracefully", () => {
      const context = {
        url: "not-a-valid-url",
        goal: "test",
      };

      const key = responseCache.getKey(context);
      expect(key).toContain("not-a-valid-url|test");
    });

    it("should use defaults for missing context properties", () => {
      const context = {};

      const key = responseCache.getKey(context);
      expect(key).toBe("||unknown|none");
    });

    it("should normalize goal to lowercase and trim", () => {
      const context = {
        url: "https://example.com",
        goal: "  CLICK BUTTON  ",
      };

      const key = responseCache.getKey(context);
      expect(key).toContain("click button");
    });
  });

  describe("set and get", () => {
    it("should cache and retrieve response", () => {
      const context = { url: "https://example.com", goal: "test" };
      const response = { success: true, data: "test" };

      responseCache.set(context, response);
      const result = responseCache.get(context);

      expect(result).toEqual(response);
    });

    it("should return null for cache miss", () => {
      const context = { url: "https://example.com", goal: "test" };

      const result = responseCache.get(context);
      expect(result).toBeNull();
    });

    it("should respect TTL for cache expiration", async () => {
      const context = { url: "https://example.com", goal: "test" };
      const response = { success: true };

      responseCache.set(context, response, 50);

      expect(responseCache.get(context)).toEqual(response);

      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(responseCache.get(context)).toBeNull();
    });

    it("should use custom TTL when specified", () => {
      const context = { url: "https://example.com", goal: "test" };
      const response = { success: true };

      responseCache.set(context, response, 5000);

      const stats = responseCache.getStats();
      expect(stats.size).toBe(1);
    });

    it("should overwrite existing entry for same key", () => {
      const context = { url: "https://example.com", goal: "test" };

      responseCache.set(context, { data: 1 });
      responseCache.set(context, { data: 2 });

      const result = responseCache.get(context);
      expect(result.data).toBe(2);
    });
  });

  describe("semantic similarity", () => {
    it("should return null for non-matching contexts", () => {
      const context1 = {
        url: "https://example.com/page1",
        goal: "click submit",
      };
      const context2 = {
        url: "https://different.com/page2",
        goal: "find information",
      };
      const response = { success: true };

      responseCache.set(context1, response);
      const result = responseCache.get(context2);

      // Different URLs and goals should not match above threshold
      expect(result).toBeNull();
    });

    it("should return expired entries as cache miss", async () => {
      const context = { url: "https://example.com", goal: "test" };
      const response = { success: true };

      responseCache.set(context, response, 50);

      await new Promise((resolve) => setTimeout(resolve, 100));
      const result = responseCache.get(context);
      expect(result).toBeNull();
    });
  });

  describe("_stringSimilarity", () => {
    it("should return 1 for identical strings", () => {
      const sim = responseCache._stringSimilarity("hello world", "hello world");
      expect(sim).toBe(1);
    });

    it("should return higher similarity for similar strings", () => {
      const sim = responseCache._stringSimilarity(
        "click submit button",
        "click the submit button",
      );
      expect(sim).toBeGreaterThan(0.5);
    });

    it("should return lower similarity for different strings", () => {
      const sim = responseCache._stringSimilarity(
        "hello world",
        "goodbye universe",
      );
      expect(sim).toBeLessThan(0.5);
    });

    it("should normalize case and whitespace", () => {
      const sim = responseCache._stringSimilarity("HELLO WORLD", "hello world");
      expect(sim).toBe(1);
    });

    it("should handle empty strings", () => {
      const sim = responseCache._stringSimilarity("", "");
      expect(sim).toBe(1);
    });
  });

  describe("_calculateSimilarity", () => {
    it("should weight URL at 40%, goal at 40%, pageType at 20%", () => {
      const ctx1 = {
        url: "https://example.com",
        goal: "test",
        pageType: "form",
      };
      const ctx2 = {
        url: "https://example.com",
        goal: "test",
        pageType: "form",
      };

      const sim = responseCache._calculateSimilarity(ctx1, ctx2);
      expect(sim).toBe(1);
    });

    it("should handle partial context matches", () => {
      const ctx1 = { url: "https://example.com", goal: "test" };
      const ctx2 = { url: "https://example.com", goal: "different" };

      const sim = responseCache._calculateSimilarity(ctx1, ctx2);
      expect(sim).toBeGreaterThan(0);
      expect(sim).toBeLessThan(1);
    });

    it("should return 0 when no common factors", () => {
      const ctx1 = {};
      const ctx2 = {};

      const sim = responseCache._calculateSimilarity(ctx1, ctx2);
      expect(sim).toBe(0);
    });
  });

  describe("eviction", () => {
    it("should have _evictOldest method", () => {
      expect(typeof responseCache._evictOldest).toBe("function");
    });

    it("should have configurable maxSize", () => {
      const originalMaxSize = responseCache.maxSize;

      responseCache.maxSize = 50;
      expect(responseCache.maxSize).toBe(50);

      responseCache.maxSize = originalMaxSize;
    });
  });

  describe("clear", () => {
    it("should clear all cached entries", () => {
      responseCache.set(
        { url: "https://example.com/1", goal: "test" },
        { data: 1 },
      );
      responseCache.set(
        { url: "https://example.com/2", goal: "test" },
        { data: 2 },
      );

      responseCache.clear();

      const stats = responseCache.getStats();
      expect(stats.size).toBe(0);
    });
  });

  describe("getStats", () => {
    it("should return correct statistics", () => {
      responseCache.set(
        { url: "https://example.com/1", goal: "test" },
        { data: 1 },
      );
      responseCache.set(
        { url: "https://example.com/2", goal: "test" },
        { data: 2 },
      );

      responseCache.get({ url: "https://example.com/1", goal: "test" });

      const stats = responseCache.getStats();

      expect(stats.size).toBe(2);
      expect(stats.maxSize).toBe(1000);
      expect(stats.totalHits).toBeGreaterThanOrEqual(0);
    });

    it("should return zero stats for empty cache", () => {
      const stats = responseCache.getStats();

      expect(stats.size).toBe(0);
      expect(stats.totalHits).toBe(0);
      expect(stats.expiredCount).toBe(0);
      expect(stats.hitRate).toBe(0);
    });
  });

  describe("cleanup", () => {
    it("should remove expired entries", async () => {
      const context1 = { url: "https://example.com/1", goal: "test" };
      const context2 = { url: "https://example.com/2", goal: "test" };

      responseCache.set(context1, { data: 1 }, 50);
      responseCache.set(context2, { data: 2 }, 10000);

      await new Promise((resolve) => setTimeout(resolve, 100));

      const removed = responseCache.cleanup();
      expect(removed).toBe(1);

      const stats = responseCache.getStats();
      expect(stats.size).toBe(1);
    });

    it("should return 0 when no entries are expired", () => {
      responseCache.set(
        { url: "https://example.com", goal: "test" },
        { data: 1 },
      );

      const removed = responseCache.cleanup();
      expect(removed).toBe(0);
    });
  });

  describe("setSimilarityThreshold", () => {
    it("should update similarity threshold", () => {
      responseCache.setSimilarityThreshold(0.5);
      expect(responseCache.similarityThreshold).toBe(0.5);
    });

    it("should clamp threshold to 0-1 range", () => {
      responseCache.setSimilarityThreshold(-0.5);
      expect(responseCache.similarityThreshold).toBe(0);

      responseCache.setSimilarityThreshold(1.5);
      expect(responseCache.similarityThreshold).toBe(1);
    });
  });

  describe("default export", () => {
    it("should export responseCache as default", async () => {
      const mod = await import("@api/agent/responseCache.js");
      expect(mod.default).toBe(responseCache);
    });
  });
});
