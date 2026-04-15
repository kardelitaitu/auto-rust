/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/lru-cache.js
 * @module tests/unit/lru-cache.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  LRUCache,
  createCache,
  createCachedFunction,
  queryCache,
  selectorCache,
  contentCache,
} from "@api/utils/lru-cache.js";

describe("utils/lru-cache", () => {
  describe("LRUCache", () => {
    let cache;

    beforeEach(() => {
      cache = new LRUCache({ maxSize: 3, ttl: 0 });
    });

    describe("Constructor", () => {
      it("should create cache with default options", () => {
        const defaultCache = new LRUCache();

        expect(defaultCache.maxSize).toBe(100);
        expect(defaultCache.ttl).toBe(0);
        expect(defaultCache.size).toBe(0);
      });

      it("should create cache with custom options", () => {
        const customCache = new LRUCache({ maxSize: 50, ttl: 5000 });

        expect(customCache.maxSize).toBe(50);
        expect(customCache.ttl).toBe(5000);
      });
    });

    describe("set", () => {
      it("should add new entry", () => {
        cache.set("key1", "value1");

        expect(cache.size).toBe(1);
        expect(cache.get("key1")).toBe("value1");
      });

      it("should update existing entry", () => {
        cache.set("key1", "value1");
        cache.set("key1", "value2");

        expect(cache.size).toBe(1);
        expect(cache.get("key1")).toBe("value2");
      });

      it("should evict oldest when over maxSize", () => {
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        cache.set("key3", "value3");
        cache.set("key4", "value4");

        expect(cache.size).toBe(3);
        expect(cache.get("key1")).toBeNull();
        expect(cache.get("key2")).toBe("value2");
        expect(cache.get("key3")).toBe("value3");
        expect(cache.get("key4")).toBe("value4");
      });
    });

    describe("get", () => {
      it("should return value for existing key", () => {
        cache.set("key1", "value1");

        expect(cache.get("key1")).toBe("value1");
      });

      it("should return null for non-existing key", () => {
        expect(cache.get("nonexistent")).toBeNull();
      });

      it("should track hits and misses", () => {
        cache.set("key1", "value1");

        cache.get("key1");
        cache.get("key1");
        cache.get("nonexistent");

        expect(cache.hits).toBe(2);
        expect(cache.misses).toBe(1);
      });
    });

    describe("has", () => {
      it("should return true for existing key", () => {
        cache.set("key1", "value1");

        expect(cache.has("key1")).toBe(true);
      });

      it("should return false for non-existing key", () => {
        expect(cache.has("nonexistent")).toBe(false);
      });
    });

    describe("delete", () => {
      it("should delete existing key", () => {
        cache.set("key1", "value1");

        const result = cache.delete("key1");

        expect(result).toBe(true);
        expect(cache.get("key1")).toBeNull();
        expect(cache.size).toBe(0);
      });

      it("should return false for non-existing key", () => {
        const result = cache.delete("nonexistent");

        expect(result).toBe(false);
      });
    });

    describe("clear", () => {
      it("should clear all entries", () => {
        cache.set("key1", "value1");
        cache.set("key2", "value2");

        cache.clear();

        expect(cache.size).toBe(0);
        expect(cache.get("key1")).toBeNull();
        expect(cache.get("key2")).toBeNull();
      });

      it("should reset hits and misses", () => {
        cache.set("key1", "value1");
        cache.get("key1");
        cache.get("nonexistent");

        cache.clear();

        expect(cache.hits).toBe(0);
        expect(cache.misses).toBe(0);
      });
    });

    describe("stats", () => {
      it("should return cache statistics", () => {
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        cache.get("key1");
        cache.get("nonexistent");

        const stats = cache.stats();

        expect(stats.size).toBe(2);
        expect(stats.maxSize).toBe(3);
        expect(stats.hits).toBe(1);
        expect(stats.misses).toBe(1);
        expect(stats.ttl).toBe(0);
        expect(stats.keys).toContain("key1");
        expect(stats.keys).toContain("key2");
      });
    });

    describe("LRU behavior", () => {
      it("should evict least recently used when full", () => {
        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);

        cache.get("a");

        cache.set("d", 4);

        expect(cache.get("a")).toBe(1);
        expect(cache.get("b")).toBeNull();
        expect(cache.get("c")).toBe(3);
        expect(cache.get("d")).toBe(4);
      });
    });
  });

  describe("createCache", () => {
    it("should create LRUCache instance", () => {
      const cache = createCache({ maxSize: 10, ttl: 1000 });

      expect(cache).toBeInstanceOf(LRUCache);
      expect(cache.maxSize).toBe(10);
      expect(cache.ttl).toBe(1000);
    });

    it("should use default options when not provided", () => {
      const cache = createCache();

      expect(cache.maxSize).toBe(100);
      expect(cache.ttl).toBe(0);
    });
  });

  describe("createCachedFunction", () => {
    it("should execute function on first call", async () => {
      const fn = vi.fn().mockImplementation(async (x) => x * 2);
      const cachedFn = createCachedFunction(fn, { maxSize: 5 });

      const result = await cachedFn(5);

      expect(result).toBe(10);
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should return cached result on subsequent calls", async () => {
      const fn = vi.fn().mockImplementation(async (x) => x * 2);
      const cachedFn = createCachedFunction(fn, { maxSize: 5 });

      const result1 = await cachedFn(5);
      const result2 = await cachedFn(5);

      expect(result1).toBe(10);
      expect(result2).toBe(10);
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should execute function with different arguments", async () => {
      const fn = vi.fn().mockImplementation(async (x) => x * 2);
      const cachedFn = createCachedFunction(fn, { maxSize: 5 });

      const result1 = await cachedFn(5);
      const result2 = await cachedFn(10);

      expect(result1).toBe(10);
      expect(result2).toBe(20);
      expect(fn).toHaveBeenCalledTimes(2);
    });
  });

  describe("Pre-configured caches", () => {
    describe("queryCache", () => {
      it("should be LRUCache instance", () => {
        expect(queryCache).toBeInstanceOf(LRUCache);
      });

      it("should have default settings", () => {
        expect(queryCache.maxSize).toBe(200);
        expect(queryCache.ttl).toBe(10000);
      });
    });

    describe("selectorCache", () => {
      it("should be LRUCache instance", () => {
        expect(selectorCache).toBeInstanceOf(LRUCache);
      });

      it("should have default settings", () => {
        expect(selectorCache.maxSize).toBe(500);
        expect(selectorCache.ttl).toBe(0);
      });
    });

    describe("contentCache", () => {
      it("should be LRUCache instance", () => {
        expect(contentCache).toBeInstanceOf(LRUCache);
      });

      it("should have default settings", () => {
        expect(contentCache.maxSize).toBe(100);
        expect(contentCache.ttl).toBe(30000);
      });
    });
  });

  describe("Edge cases", () => {
    it("should handle maxSize of 1", () => {
      const singleCache = new LRUCache({ maxSize: 1 });

      singleCache.set("a", 1);
      singleCache.set("b", 2);

      expect(singleCache.size).toBe(1);
      expect(singleCache.get("a")).toBeNull();
      expect(singleCache.get("b")).toBe(2);
    });

    it("should handle setting undefined value", () => {
      const testCache = new LRUCache({ maxSize: 3 });
      testCache.set("key", undefined);

      expect(testCache.get("key")).toBeUndefined();
    });

    it("should handle setting null value", () => {
      const testCache = new LRUCache({ maxSize: 3 });
      testCache.set("key", null);

      const result = testCache.get("key");
      expect(result).toBeNull();
      expect(testCache.hits).toBe(1);
    });
  });
});
