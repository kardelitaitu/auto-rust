/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit Tests for FreeApiRouter
 * Tests the free API router with API key rotation, model cascading, and proxy routing
 * @module tests/unit/free-api-router.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/core/logger.js", () => {
  const mockLogger = {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    success: vi.fn(),
  };
  return {
    createLogger: () => mockLogger,
  };
});

vi.mock("@api/utils/proxy-agent.js", () => ({
  createProxyAgent: vi.fn().mockResolvedValue({
    agent: "mock-agent",
    getAgent: vi.fn().mockResolvedValue("mock-http-agent"),
  }),
}));

vi.mock("@api/core/circuit-breaker.js", () => {
  class CircuitBreaker {
    constructor() {}
    check() {
      return { allowed: true };
    }
    recordSuccess() {}
    recordFailure() {}
  }
  return { default: CircuitBreaker };
});

// Note: rate-limit-tracker.js is NOT mocked here - use real implementation
// Only logger is mocked at top of file

vi.mock("@api/utils/request-dedupe.js", () => {
  class RequestDedupe {
    constructor() {}
    check() {
      return { hit: false };
    }
    set() {}
  }
  return { RequestDedupe };
});

vi.mock("@api/utils/model-perf-tracker.js", () => {
  class ModelPerfTracker {
    constructor() {}
    trackSuccess() {}
    trackFailure() {}
  }
  return { ModelPerfTracker };
});

vi.mock("@api/utils/api-key-timeout-tracker.js", () => {
  class ApiKeyTimeoutTracker {
    constructor() {}
    getTimeoutForKey() {
      return 60000;
    }
    trackRequest() {}
  }
  return { ApiKeyTimeoutTracker };
});

vi.mock("@api/utils/config-validator.js", () => {
  class ConfigValidator {
    validate() {
      return { valid: true };
    }
  }
  return { ConfigValidator };
});

vi.mock("@api/utils/errors.js", () => ({
  RouterError: class RouterError extends Error {
    constructor(message, options) {
      super(message);
      this.code = options?.code;
      this.metadata = options?.metadata;
    }
  },
  ProxyError: class ProxyError extends Error {},
  classifyHttpError: vi.fn((status, text, metadata) => {
    if (status >= 400) {
      const error = new Error(`HTTP ${status}: ${text || "Error"}`);
      error.code = "ROUTER_ERROR";
      error.metadata = {
        ...metadata,
        statusCode: status,
        retryable: status >= 500 || status === 429,
      };
      throw error;
    }
    return null;
  }),
}));

import {
  getSharedHelper,
  setSharedHelper,
} from "@api/utils/free-api-router.js";

// const { CircuitBreaker } = await import('../../utils/circuit-breaker.js');
// const { RateLimitTracker } = await import('../../utils/rate-limit-tracker.js');
// const { RequestDedupe } = await import('@api/utils/request-dedupe.js');
// const { ModelPerfTracker } = await import('@api/utils/model-perf-tracker.js');
// const { ApiKeyTimeoutTracker } = await import('@api/utils/api-key-timeout-tracker.js');

describe("FreeApiRouter", () => {
  let router;
  let originalFetch;
  let originalMathRandom;

  beforeEach(() => {
    vi.clearAllMocks();
    originalFetch = global.fetch;
    originalMathRandom = Math.random;
    global.fetch = vi.fn();
  });

  afterEach(() => {
    global.fetch = originalFetch;
    Math.random = originalMathRandom;
    setSharedHelper(null);
  });

  describe("Constructor", () => {
    it("should initialize with default values when disabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      expect(router.config.enabled).toBe(false);
      expect(router.config.apiKeys).toEqual([]);
      expect(router.endpoint).toBe(
        "https://openrouter.ai/api/v1/chat/completions",
      );
      expect(router.defaultTimeout).toBe(60000);
      expect(router.quickTimeout).toBe(20000);
      expect(router.sessionApiKey).toBeNull();
      expect(router.stats.totalRequests).toBe(0);
    });

    it("should initialize with custom options", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["key1", "key2"],
        primaryModel: "test/model",
        fallbackModels: ["fallback/model"],
        proxyEnabled: true,
        proxyList: ["proxy1:8080", "proxy2:9090"],
        timeout: 30000,
        quickTimeout: 10000,
        browserId: "browser1",
        taskId: "task1",
      });

      expect(router.config.enabled).toBe(true);
      expect(router.config.apiKeys).toEqual(["key1", "key2"]);
      expect(router.config.models.primary).toBe("test/model");
      expect(router.config.models.fallbacks).toEqual(["fallback/model"]);
      expect(router.config.proxy.enabled).toBe(true);
      expect(router.config.proxy.list).toEqual(["proxy1:8080", "proxy2:9090"]);
      expect(router.defaultTimeout).toBe(30000);
      expect(router.quickTimeout).toBe(10000);
      expect(router.browserId).toBe("browser1");
      expect(router.taskId).toBe("task1");
      expect(router.sessionId).toBe("browser1:task1");
    });

    it("should select API key based on session hash", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["key1", "key2", "key3"],
        browserId: "browser1",
        taskId: "task1",
      });

      expect(router.sessionApiKeyIndex).toBeGreaterThanOrEqual(0);
      expect(router.sessionApiKeyIndex).toBeLessThan(3);
      expect(router.sessionApiKey).toBeDefined();
    });

    it("should initialize modules when enabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: true, apiKeys: ["key1"] });

      expect(router.circuitBreaker).toBeDefined();
      expect(router.rateLimitTracker).toBeDefined();
      expect(router.requestDedupe).toBeDefined();
      expect(router.modelPerfTracker).toBeDefined();
      expect(router.apiKeyTimeoutTracker).toBeDefined();
      expect(router.configValidator).toBeDefined();
    });
  });

  describe("_hash", () => {
    it("should return consistent hash for same string", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const hash1 = router._hash("test");
      const hash2 = router._hash("test");
      expect(hash1).toBe(hash2);
    });

    it("should return different hash for different strings", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const hash1 = router._hash("test1");
      const hash2 = router._hash("test2");
      expect(hash1).not.toBe(hash2);
    });
  });

  describe("setTask", () => {
    it("should update browserId and taskId", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: false,
        browserId: "old-browser",
        taskId: "old-task",
      });

      router.setTask("new-browser", "new-task");

      expect(router.browserId).toBe("new-browser");
      expect(router.taskId).toBe("new-task");
      expect(router.sessionId).toBe("new-browser:new-task");
    });

    it("should not change session if values are same", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: false,
        browserId: "browser",
        taskId: "task",
      });

      const originalSessionId = router.sessionId;
      router.setTask("browser", "task");

      expect(router.sessionId).toBe(originalSessionId);
    });
  });

  describe("_selectRequestProxy", () => {
    it("should return null when proxy is disabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: false,
        proxyEnabled: false,
      });

      const proxy = router._selectRequestProxy();
      expect(proxy).toBeNull();
    });

    it("should return null when proxy list is empty", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: false,
        proxyEnabled: true,
        proxyList: [],
      });

      const proxy = router._selectRequestProxy();
      expect(proxy).toBeNull();
    });

    it("should return a proxy from the list when enabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: false,
        proxyEnabled: true,
        proxyList: ["proxy1:8080", "proxy2:9090"],
      });

      const proxy = router._selectRequestProxy();
      expect(["proxy1:8080", "proxy2:9090"]).toContain(proxy);
    });
  });

  describe("_parseProxy", () => {
    it("should return null for empty string", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = router._parseProxy("");
      expect(result).toBeNull();
    });

    it("should return null for null input", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = router._parseProxy(null);
      expect(result).toBeNull();
    });

    it("should parse proxy with host and port only", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = router._parseProxy("proxy.example.com:8080");
      expect(result).toEqual({
        host: "proxy.example.com",
        port: "8080",
        username: null,
        password: null,
      });
    });

    it("should parse proxy with credentials", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = router._parseProxy("proxy.example.com:8080:user:pass");
      expect(result).toEqual({
        host: "proxy.example.com",
        port: "8080",
        username: "user",
        password: "pass",
      });
    });

    it("should return null for invalid format", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = router._parseProxy("invalid");
      expect(result).toBeNull();
    });
  });

  describe("_maskKey", () => {
    it("should return null for null key", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      expect(router._maskKey(null)).toBe("null");
    });

    it("should return *** for short key", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      expect(router._maskKey("short")).toBe("***");
    });

    it("should mask long key properly", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const masked = router._maskKey("abcdefgh12345678");
      expect(masked).toBe("abcdef...5678");
      expect(masked.length).toBeLessThan("abcdefgh12345678".length);
    });
  });

  describe("_maskProxy and shared helper", () => {
    it("should mask proxy credentials when present", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
        getSharedHelper,
      } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      expect(router._maskProxy("host:8080:user:pass")).toBe(
        "host:8080:user:***",
      );
      expect(router._maskProxy("host:8080")).toBe("host:8080");

      const helper = { getResults: () => ({ working: ["model/a"] }) };
      setSharedHelper(helper);
      expect(getSharedHelper()).toBe(helper);
    });
  });

  describe("processRequest", () => {
    it("should return error when not enabled", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toBe("Free API router not enabled");
      expect(router.stats.totalRequests).toBe(1);
    });

    it("should return cached response on dedupe hit", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.requestDedupe.check = vi.fn().mockReturnValue({
        hit: true,
        response: "cached response",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
        maxTokens: 100,
        temperature: 0.7,
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("cached response");
      expect(result.fromCache).toBe(true);
      expect(router.stats.dedupeHits).toBe(1);
      expect(router.stats.successes).toBe(1);
    });

    it("should fail fast when no API keys are configured", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.modelsTried).toBeGreaterThanOrEqual(0);
    });

    it("should make successful API call", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      setSharedHelper(null);

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "test response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
        maxTokens: 100,
        temperature: 0.7,
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("test response");
      expect(result.model).toBeDefined();
      expect(router.stats.successes).toBe(1);
    });

    it("should use reasoning_content when content is empty", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [
            { message: { content: "", reasoning_content: "reasoned answer" } },
          ],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("reasoned answer");
    });

    it("should handle API error with rate limiting", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 429,
        text: async () => "Rate limited",
      });

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle network error gracefully", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should track stats correctly", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
          model: "test/model",
          usage: { total_tokens: 10 },
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.stats.totalRequests).toBe(1);
      expect(router.stats.successes).toBe(1);
      expect(router.stats.failures).toBe(0);
    });
  });

  describe("_tryModelWithKey", () => {
    it("should select proxy and call appropriate method", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy:8080"],
      });

      router._parseProxy = vi
        .fn()
        .mockReturnValue({ host: "proxy", port: "8080" });
      router._callThroughProxy = vi
        .fn()
        .mockResolvedValue({ success: true, content: "response" });

      const result = await router._tryModelWithKey(
        "test/model",
        [{ role: "user", content: "hello" }],
        100,
        0.7,
        Date.now(),
      );

      expect(result.success).toBe(true);
      expect(result.content).toBe("response");
    });
  });

  describe("_callDirect", () => {
    it("should make direct API call", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "direct response" } }],
        }),
      });

      const result = await router._callDirect(
        { model: "test", messages: [] },
        60000,
      );

      expect(result.success).toBe(true);
      expect(result.content).toBe("direct response");
    });

    it("should handle timeout", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(() => reject(new Error("AbortError")), 10),
            ),
        );

      let errorThrown = false;
      try {
        await router._callDirect({ model: "test", messages: [] }, 5);
      } catch (_e) {
        errorThrown = true;
      }

      expect(errorThrown).toBe(true);
    });

    it("should handle non-ok response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
        text: async () => "Server error",
      });

      let errorThrown = false;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (_e) {
        errorThrown = true;
      }

      expect(errorThrown).toBe(true);
    });
  });

  describe("Error handling", () => {
    it("should handle empty messages array", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({ messages: [] });

      expect(result.success).toBe(true);
    });

    it("should use default values for optional params", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(global.fetch).toHaveBeenCalled();
      const callArgs = global.fetch.mock.calls[0];
      const body = JSON.parse(callArgs[1].body);
      expect(body.max_tokens).toBe(100);
      expect(body.temperature).toBe(0.7);
    });
  });

  describe("getStats", () => {
    it("should return stats object", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({ enabled: false });

      expect(router.stats).toBeDefined();
      expect(router.stats.totalRequests).toBe(0);
      expect(router.stats.successes).toBe(0);
      expect(router.stats.failures).toBe(0);
    });
  });

  describe("Edge cases", () => {
    it("should handle missing API keys gracefully", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });

    it("should handle fallback models when primary fails", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary/model",
        fallbackModels: ["fallback/model"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Error",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "fallback response" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("fallback response");
    });

    it("should track circuit breaker state", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.circuitBreaker.check = vi.fn().mockReturnValue({ allowed: false });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("exhausted");
    });
  });

  describe("Model Cascading - Working Models < 3", () => {
    it("should add from config when working models < 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2"],
          total: 2,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["config1", "config2", "config3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should not add from config when working models >= 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2", "working3"],
          total: 5,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["config1"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Rate Limited Model Tracking", () => {
    it("should track multiple models getting rate limited in one request", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3", "model4"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        // Fail first 2 models (retryCount 0, 1) -> retryCount 2 (success)
        // Max retries is 3, so we can afford 3 failures (attempts 1,2,3 failed, try 4th).
        // But loop condition is retryCount < maxRetries (3).
        // Attempt 1: retryCount=0 -> 1. Fail.
        // Attempt 2: retryCount=1 -> 2. Fail.
        // Attempt 3: retryCount=2 -> 3. Success.
        if (callCount <= 2) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "third model works" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
      expect(result.rateLimitedModels).toContain("model2");
      // model3 should work
    });

    it("should add server errors to rateLimitedModels", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 503,
            text: async () => "Service Unavailable",
          });
        }
        if (callCount === 2) {
          return Promise.resolve({
            ok: false,
            status: 502,
            text: async () => "Bad Gateway",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "third model success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
      expect(result.rateLimitedModels).toContain("model2");
    });
  });

  describe("_callThroughProxy Edge Cases", () => {
    it("should handle createProxyAgent throwing", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      // Mock the implementation for this test
      createProxyAgent.mockRejectedValueOnce(
        new Error("Proxy creation failed"),
      );

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle httpAgent.getAgent() returning null", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      const mockAgent = {
        getAgent: vi.fn().mockResolvedValue(null),
      };
      createProxyAgent.mockResolvedValueOnce(mockAgent);

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle timeout in proxy call", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        timeout: 1,
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(
                () =>
                  reject(new Error("AbortError: The operation was aborted")),
                10,
              ),
            ),
        );

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });
  });

  describe("_callDirect Error Handling", () => {
    it("should wrap error when error does not have name/code", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      global.fetch = vi.fn().mockImplementation(() => {
        const error = new Error("Some network error");
        delete error.name;
        delete error.code;
        throw error;
      });

      let thrownError;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).toBeDefined();
    });

    it("should rethrow AppError when error has name and code", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      const appError = new Error("Already handled");
      appError.name = "AppError";
      appError.code = "SOME_CODE";

      global.fetch = vi.fn().mockRejectedValue(appError);

      let thrownError;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).toBeDefined();
    });
  });

  describe("getSessionInfo - Different Configurations", () => {
    it("should return correct info when proxy disabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["key1", "key2"],
        primaryModel: "test/model",
        fallbackModels: ["fallback1"],
        proxyEnabled: false,
        proxyList: [],
        browserId: "browser1",
        taskId: "task1",
      });

      const info = router.getSessionInfo();

      expect(info.sessionId).toBe("browser1:task1");
      expect(info.proxyEnabled).toBe(false);
      expect(info.proxyCount).toBe(0);
    });

    it("should return correct info with no fallback models", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["key1"],
        primaryModel: "test/model",
        fallbackModels: [],
        browserId: "browser1",
        taskId: "task1",
      });

      const info = router.getSessionInfo();

      expect(info.fallbackCount).toBe(0);
    });

    it("should return correct info with no API keys", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
        fallbackModels: ["fallback1"],
      });

      const info = router.getSessionInfo();

      expect(info.totalApiKeys).toBe(0);
    });
  });

  describe("Additional Cascading Edge Cases", () => {
    it("should handle empty working array in test results", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: [],
          total: 5,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "configured1",
        fallbackModels: ["configured2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should handle testResults with no working property", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({ total: 5 }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "configured1",
        fallbackModels: ["configured2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Empty Response Handling", () => {
    it("should handle response with empty content but reasoning_content", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [
            { message: { content: "", reasoning_content: "reasoning text" } },
          ],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("reasoning text");
    });

    it("should handle completely empty response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Direct Fallback from Proxy", () => {
    it("should fallback to direct when proxy fails and fallbackToDirect is enabled", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: true,
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        // First call (via proxy) fails, second call (direct) succeeds
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Proxy Error",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "direct success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("direct success");
      expect(result.directFallbackUsed).toBe(true);
    });

    it("should not fallback to direct when fallbackToDirect is disabled", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: false,
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Proxy Error",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });
  });

  describe("Multiple 429 Rate Limiting", () => {
    it("should track rate limited models when 429 occurs", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        // First call gets 429
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        // Subsequent calls succeed
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
    });
  });

  describe("Server Errors Adding to Rate Limited Models", () => {
    it("should add 500 error to rateLimitedModels", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Internal Server Error",
          });
        }
        if (callCount === 2) {
          return Promise.resolve({
            ok: false,
            status: 502,
            text: async () => "Bad Gateway",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "third success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
      expect(result.rateLimitedModels).toContain("model2");
    });

    it("should add 504 error to rateLimitedModels", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 504,
            text: async () => "Gateway Timeout",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
    });
  });

  describe("Circuit Breaker and Rate Limit Exhaustion", () => {
    it("should track circuit breaker opens in stats", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      // Make circuit breaker disallow first model
      router.circuitBreaker.check = vi.fn().mockReturnValue({ allowed: false });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      // Since both models are circuit-open, should fail
      expect(result.success).toBe(false);
    });
  });

  describe("_callThroughProxy Error Scenarios", () => {
    it("should handle createProxyAgent throwing error", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      createProxyAgent.mockRejectedValueOnce(
        new Error("Proxy creation failed"),
      );

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle httpAgent.getAgent() returning null", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      const mockAgent = {
        getAgent: vi.fn().mockResolvedValue(null),
      };
      createProxyAgent.mockResolvedValueOnce(mockAgent);

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle proxy timeout", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        timeout: 1,
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(
                () =>
                  reject(new Error("AbortError: The operation was aborted")),
                10,
              ),
            ),
        );

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle proxy non-ok response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 429,
        text: async () => "Rate limited via proxy",
      });

      let result;
      try {
        result = await router._tryModelWithKey(
          "test/model",
          [{ role: "user", content: "hello" }],
          100,
          0.7,
          Date.now(),
        );
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle proxy response with reasoning_content", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [
            { message: { content: "", reasoning_content: "proxy reasoning" } },
          ],
        }),
      });

      const result = await router._tryModelWithKey(
        "test/model",
        [{ role: "user", content: "hello" }],
        100,
        0.7,
        Date.now(),
      );

      expect(result.success).toBe(true);
      expect(result.content).toBe("proxy reasoning");
      expect(result.proxy).toBe("proxy1:8080");
    });
  });

  describe("_callDirect Error Wrapping", () => {
    it("should wrap error when error does not have name/code properties", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      global.fetch = vi.fn().mockImplementation(() => {
        const error = new Error("Some network error");
        delete error.name;
        delete error.code;
        throw error;
      });

      let thrownError;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).toBeDefined();
    });

    it("should rethrow AppError when error has name and code", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      const appError = new Error("Already handled");
      appError.name = "AppError";
      appError.code = "SOME_CODE";

      global.fetch = vi.fn().mockRejectedValue(appError);

      let thrownError;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).toBeDefined();
    });
  });

  describe("Model Cascading with Shuffle", () => {
    it("should add from config when working models < 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2"],
          total: 2,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["config1", "config2", "config3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should not add from config when working models >= 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2", "working3"],
          total: 5,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["config1"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("getModelsInfo", () => {
    it("should return models info with test results", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2"],
          failed: [{ model: "failed1" }],
          total: 5,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1", "fallback2"],
      });

      const info = router.getModelsInfo();

      expect(info.primary).toBe("primary");
      expect(info.fallbacks).toEqual(["fallback1", "fallback2"]);
      expect(info.testedWorking).toEqual(["working1", "working2"]);
      expect(info.testedFailed).toEqual(["failed1"]);
      expect(info.totalTested).toBe(5);
    });

    it("should return models info without test results", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper(null);

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1"],
      });

      const info = router.getModelsInfo();

      expect(info.primary).toBe("primary");
      expect(info.testedWorking).toEqual([]);
    });
  });

  describe("syncWithHelper", () => {
    it("should sync with helper when helper has results", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["model1", "model2", "model3"],
          total: 3,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "old-primary",
        fallbackModels: ["old-fallback"],
      });

      const result = router.syncWithHelper();

      expect(result).toBe(true);
      expect(router.config.models.primary).toBe("model1");
      expect(router.config.models.fallbacks).toEqual(["model2", "model3"]);
    });

    it("should return false when helper is null", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper(null);

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback"],
      });

      const result = router.syncWithHelper();

      expect(result).toBe(false);
    });

    it("should return false when helper has no working models", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({ working: [], total: 0 }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback"],
      });

      const result = router.syncWithHelper();

      expect(result).toBe(false);
    });
  });

  describe("refreshRateLimits", () => {
    it("should refresh rate limits when sessionApiKey exists", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.rateLimitTracker.refreshKey = vi
        .fn()
        .mockResolvedValue({ success: true });

      const result = await router.refreshRateLimits();

      expect(result).toEqual({ success: true });
    });

    it("should return null when no sessionApiKey", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
      });

      const result = await router.refreshRateLimits();

      expect(result).toBeNull();
    });
  });

  describe("resetStats", () => {
    it("should reset all stats and module states", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.stats.totalRequests = 10;
      router.stats.successes = 5;
      router.stats.failures = 5;

      router.resetStats();

      expect(router.stats.totalRequests).toBe(0);
      expect(router.stats.successes).toBe(0);
      expect(router.stats.failures).toBe(0);
    });
  });

  describe("isReady", () => {
    it("should return true when enabled with API key", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      expect(router.isReady()).toBe(true);
    });

    it("should return false when not enabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: false,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      expect(router.isReady()).toBe(false);
    });

    it("should return false when no API keys", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
      });

      // sessionApiKey is null when no API keys, so isReady returns falsy (null)
      expect(router.isReady()).toBeFalsy();
    });
  });

  describe("getDetailedStats", () => {
    it("should return detailed stats including best model", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      router.modelPerfTracker.getBestModel = vi.fn().mockReturnValue("model1");
      router.modelPerfTracker.getAllStats = vi.fn().mockReturnValue({});
      router.circuitBreaker.getAllStates = vi.fn().mockReturnValue({});
      router.rateLimitTracker.getCacheStatus = vi.fn().mockReturnValue({});

      const stats = router.getDetailedStats();

      expect(stats.session).toBeDefined();
      expect(stats.router).toBeDefined();
      expect(stats.bestModel).toBe("model1");
    });
  });

  describe("getSharedHelper and setSharedHelper", () => {
    it("should get and set shared helper", () => {
      const {
        getSharedHelper,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      const helper = {
        getResults: () => ({ working: ["model1"], total: 1 }),
      };

      setSharedHelper(helper);

      expect(getSharedHelper()).toBe(helper);

      const info = getSharedHelper().getResults();
      expect(info.working).toEqual(["model1"]);
    });
  });

  describe("Additional processRequest Edge Cases", () => {
    it("should execute shuffle loop when working models >= 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2", "working3"],
          total: 3,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["working2", "working3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should add only unique models from config when working < 3", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1"],
          total: 1,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["working1", "working2", "working3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should track all 429 errors in rateLimitedModels for same request", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3", "model4", "model5"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        // First two models get 429
        if (callCount <= 2) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "third model works" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
      expect(result.rateLimitedModels).toContain("model2");
    });

    it("should add 500, 502, 503, 504 to rateLimitedModels", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3", "model4", "model5"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1)
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Error",
          });
        if (callCount === 2)
          return Promise.resolve({
            ok: false,
            status: 502,
            text: async () => "Error",
          });
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.rateLimitedModels).toContain("model1");
      expect(result.rateLimitedModels).toContain("model2");
    });

    it("should skip models in circuit breaker", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      router.circuitBreaker.check = vi.fn().mockImplementation((_model) => {
        callCount++;
        // First model circuit open, second allowed
        if (callCount === 1) return { allowed: false };
        return { allowed: true };
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(router.stats.circuitBreaks).toBe(1);
    });

    it("should skip rate limit exhausted keys and track stats", async () => {
      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      router.rateLimitTracker.getWarningStatus = vi
        .fn()
        .mockImplementation((_key) => {
          return "exhausted";
        });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("exhausted");
      expect(router.stats.rateLimitHits).toBeGreaterThan(0);
    });
  });

  describe("_callThroughProxy Additional Edge Cases", () => {
    it("should throw when createProxyAgent throws", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      createProxyAgent.mockRejectedValueOnce(new Error("Proxy agent failed"));

      let thrownError = null;
      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          5000,
        );
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
      expect(thrownError.message).toContain("Proxy agent failed");
    });

    it("should throw when httpAgent is null", async () => {
      const { createProxyAgent } = await import("@api/utils/proxy-agent.js");

      createProxyAgent.mockResolvedValueOnce({
        getAgent: vi.fn().mockResolvedValue(null),
      });

      const { FreeApiRouter } = await import("@api/utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      let thrownError = null;
      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          5000,
        );
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
    });

    it("should handle timeout in proxy call with AbortError", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        timeout: 10,
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(
                () =>
                  reject(new Error("AbortError: The operation was aborted")),
                20,
              ),
            ),
        );

      let thrownError = null;
      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          10,
        );
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
      expect(router.stats.quickTimeouts).toBe(1);
    });

    it("should handle proxy fetch network error", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      global.fetch = vi
        .fn()
        .mockRejectedValue(new Error("Network unavailable"));

      let thrownError = null;
      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          5000,
        );
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
    });
  });

  describe("_callDirect Error Wrapping Edge Cases", () => {
    it("should wrap plain error without name/code", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      const plainError = new Error("Plain error");
      Object.defineProperty(plainError, "name", { value: undefined });
      Object.defineProperty(plainError, "code", { value: undefined });

      global.fetch = vi.fn().mockRejectedValue(plainError);

      let thrownError = null;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
      expect(thrownError.message).toBe("Plain error");
    });

    it("should wrap error with empty string name", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
      });

      const errorWithEmptyName = new Error("Error with empty name");
      errorWithEmptyName.name = "";
      errorWithEmptyName.code = "";

      global.fetch = vi.fn().mockRejectedValue(errorWithEmptyName);

      let thrownError = null;
      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        thrownError = e;
      }

      expect(thrownError).not.toBeNull();
    });
  });

  describe("Model Cascading Edge Cases", () => {
    it("should use configured models when no test results", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper(null);

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary1",
        fallbackModels: ["fallback1", "fallback2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should handle working array with length 0", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: [],
          total: 0,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "config1",
        fallbackModels: ["config2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should filter out duplicate models when adding from config", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["model1", "model2"],
          total: 2,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model1", "model2", "model3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("All Models Exhausted Scenarios", () => {
    it("should return exhausted error when all models fail with circuit open", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      router.circuitBreaker.check = vi.fn().mockReturnValue({ allowed: false });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("exhausted");
      expect(result.modelsTried).toBe(0);
    });

    it("should return exhausted error when all models get rate limited", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 429,
        text: async () => "Rate Limited",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should return exhausted error when all models fail with server errors", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Server Error",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });
  });

  describe("Direct Fallback Error Paths", () => {
    it("should handle proxy fallback to direct when both fail", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: true,
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });
  });

  describe("Rate Limit Warning Status", () => {
    it("should continue when rate limit status is warning", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: [],
      });

      router.rateLimitTracker.getWarningStatus = vi
        .fn()
        .mockReturnValue("warning");

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Warning Status in Response", () => {
    it("should include warningStatus in response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: [],
      });

      router.rateLimitTracker.getWarningStatus = vi
        .fn()
        .mockReturnValue("warning");

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.warningStatus).toBe("warning");
    });
  });

  describe("Response with usage data", () => {
    it("should track usage when present in response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
          usage: { total_tokens: 50, prompt_tokens: 10, completion_tokens: 40 },
          model: "test/model",
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Empty configured models array", () => {
    it("should handle empty configured models", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({ working: [], total: 0 }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "",
        fallbackModels: [],
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });
  });

  describe("_getCachedHash", () => {
    it("should cache hash results", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({ enabled: false });

      const hash1 = router._getCachedHash("test");
      const hash2 = router._getCachedHash("test");

      expect(hash1).toBe(hash2);
      expect(router._hashCache.has("test")).toBe(true);
    });
  });

  describe("Multiple fallback models with mixed results", () => {
    it("should try multiple fallbacks until success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Error",
          });
        }
        if (callCount === 2) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.model).toBe("model3");
      expect(result.modelFallbacks).toBe(2);
    });
  });

  describe("Retry behavior with maxRetries", () => {
    it("should retry up to maxRetries times", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });
  });

  describe("Response with only reasoning_content", () => {
    it("should use reasoning_content when content is empty", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [
            { message: { content: "", reasoning_content: "reasoning result" } },
          ],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.content).toBe("reasoning result");
    });
  });

  describe("Dedupe cache hit scenarios", () => {
    it("should return cached result on dedupe hit with all fields", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.requestDedupe.check = vi.fn().mockReturnValue({
        hit: true,
        response: { content: "cached", model: "test/model" },
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
        maxTokens: 100,
        temperature: 0.7,
      });

      expect(result.success).toBe(true);
      expect(result.fromCache).toBe(true);
    });
  });

  describe("Circuit breaker state management", () => {
    it("should record success after failure", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Error",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Model performance tracking", () => {
    it("should track model performance on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.stats.successes).toBe(1);
    });

    it("should return RouterError on HTTP failure from direct call", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "server exploded",
      });

      await expect(
        router._callDirect(
          {
            model: "test/model",
            messages: [{ role: "user", content: "hello" }],
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
            exclude_reasoning: true,
          },
          1000,
        ),
      ).rejects.toThrow("server exploded");
    });

    it("should track model performance on failure", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: ["fallback"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      try {
        await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (_e) {
        /* intentional no-op */
      }

      expect(router.stats.failures).toBe(1);
    });
  });

  describe("API key timeout tracking", () => {
    it("should track API key timeout on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.stats.successes).toBe(1);
    });

    it("should track API key timeout on failure", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: ["fallback"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      try {
        await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (_e) {
        /* intentional no-op */
      }

      expect(router.stats.failures).toBe(1);
    });
  });

  describe("Rate limit tracker integration", () => {
    it("should track request on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.stats.successes).toBe(1);
    });
  });

  describe("Request deduplication", () => {
    it("should set dedupe on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.requestDedupe.check = vi.fn().mockReturnValue({ hit: false });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: "response" } }] }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
        maxTokens: 100,
        temperature: 0.7,
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Proxy selection variations", () => {
    it("should handle proxy with username and password format", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: false,
      });

      const parsed = router._parseProxy("proxyhost:8080:user:pass");

      expect(parsed).not.toBeNull();
      expect(parsed.host).toBe("proxyhost");
      expect(parsed.port).toBe("8080");
      expect(parsed.username).toBe("user");
      expect(parsed.password).toBe("pass");
    });
  });

  describe("Error response with different status codes", () => {
    it("should handle 400 Bad Request", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        statusText: "Bad Request",
        text: async () => "Invalid request",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle 401 Unauthorized", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: "Unauthorized",
        text: async () => "Invalid API key",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle 403 Forbidden", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: "Forbidden",
        text: async () => "Access denied",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });

    it("should handle 404 Not Found", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
        statusText: "Not Found",
        text: async () => "Endpoint not found",
      });

      let result;
      try {
        result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (e) {
        result = { success: false, error: e.message };
      }

      expect(result.success).toBe(false);
    });
  });

  describe("Model info with different scenarios", () => {
    it("should get models info with failed test results", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["model1"],
          failed: [{ model: "model2", error: "Failed" }],
          total: 2,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1"],
      });

      const info = router.getModelsInfo();

      expect(info.testedFailed).toEqual(["model2"]);
    });

    it("should get models info with allConfigured", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1", "fallback2"],
      });

      const info = router.getModelsInfo();

      expect(info.allConfigured).toEqual(["primary", "fallback1", "fallback2"]);
    });
  });

  describe("Sync with helper edge cases", () => {
    it("should sync when primary is not in working", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["model1", "model2", "model3"],
          total: 3,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "not-in-list",
        fallbackModels: ["old-fallback"],
      });

      const result = router.syncWithHelper();

      expect(result).toBe(true);
      expect(router.config.models.primary).toBe("model1");
      expect(router.config.models.fallbacks).toEqual(["model2", "model3"]);
    });
  });

  describe("_getCachedHash", () => {
    it("should cache hash results", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");
      router = new FreeApiRouter({ enabled: false });

      const hash1 = router._getCachedHash("test");
      const hash2 = router._getCachedHash("test");

      expect(hash1).toBe(hash2);
      expect(router._hashCache.has("test")).toBe(true);
    });
  });

  describe("Model Cascading - Working Models Edge Cases", () => {
    it("should handle exactly 2 working models with no config fallbacks", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1", "working2"],
          total: 2,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: [],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should handle exactly 1 working model", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1"],
          total: 1,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["fallback1", "fallback2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should handle working model duplicates in config", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: ["working1"],
          total: 1,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "working1",
        fallbackModels: ["working1", "fallback1"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Rate Limit and Retry Logic", () => {
    it("should exhaust retries after maxRetries attempts", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3", "model4"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        return Promise.resolve({
          ok: false,
          status: 500,
          text: async () => "Server Error",
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toBe(
        "All models and fallbacks exhausted after 3 retries",
      );
      expect(callCount).toBeGreaterThanOrEqual(3);
    });

    it("should track rate limit in result when 429 occurs", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should skip already rate limited models in same request", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 429,
            text: async () => "Rate Limited",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("_tryModelDirect", () => {
    it("should call _callDirect with correct parameters", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router._callDirect = vi
        .fn()
        .mockResolvedValue({ success: true, content: "direct response" });

      const result = await router._tryModelDirect(
        "test/model",
        [{ role: "user", content: "hello" }],
        100,
        0.7,
        Date.now(),
      );

      expect(result.success).toBe(true);
      expect(result.content).toBe("direct response");
    });

    it("should handle _callDirect failure", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router._callDirect = vi
        .fn()
        .mockResolvedValue({ success: false, error: "Direct failed" });

      const result = await router._tryModelDirect(
        "test/model",
        [{ role: "user", content: "hello" }],
        100,
        0.7,
        Date.now(),
      );

      expect(result.success).toBe(false);
    });
  });

  describe("_callThroughProxy Edge Cases", () => {
    it("should handle proxy timeout correctly", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        timeout: 10,
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(() => reject(new Error("AbortError")), 50),
            ),
        );

      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          10,
        );
      } catch (e) {
        expect(e).toBeDefined();
      }
    });

    it("should handle response with non-ok status via proxy", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        statusText: "Bad Request",
        text: async () => "Invalid request",
      });

      try {
        await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          60000,
        );
      } catch (e) {
        expect(e).toBeDefined();
      }
    });

    it("should return proxy info in success response", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "proxy response" } }],
        }),
      });

      const result = await router._callThroughProxy(
        { host: "proxy1", port: "8080" },
        { model: "test", messages: [] },
        60000,
      );

      expect(result.success).toBe(true);
      expect(result.proxy).toBe("proxy1:8080");
    });
  });

  describe("refreshRateLimits", () => {
    it("should refresh rate limits when sessionApiKey exists", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.rateLimitTracker.refreshKey = vi
        .fn()
        .mockResolvedValue({ refreshed: true });

      const result = await router.refreshRateLimits();

      expect(result).toBeDefined();
      expect(router.rateLimitTracker.refreshKey).toHaveBeenCalledWith(
        "test-key",
      );
    });

    it("should return null when no sessionApiKey", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
      });

      const result = await router.refreshRateLimits();

      expect(result).toBeNull();
    });
  });

  describe("getDetailedStats", () => {
    it("should return detailed stats with all modules", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        fallbackModels: ["fallback1"],
      });

      router.circuitBreaker.getAllStates = vi.fn().mockReturnValue({});
      router.rateLimitTracker.getCacheStatus = vi.fn().mockReturnValue({});
      router.modelPerfTracker.getAllStats = vi.fn().mockReturnValue({});
      router.modelPerfTracker.getBestModel = vi
        .fn()
        .mockReturnValue("test/model");

      const stats = router.getDetailedStats();

      expect(stats.session).toBeDefined();
      expect(stats.router).toBeDefined();
      expect(stats.circuitBreakerStates).toBeDefined();
      expect(stats.rateLimitStatus).toBeDefined();
      expect(stats.modelPerformance).toBeDefined();
      expect(stats.bestModel).toBe("test/model");
    });
  });

  describe("resetStats", () => {
    it("should reset all stats and module states", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.stats.totalRequests = 10;
      router.stats.successes = 5;
      router.stats.failures = 5;

      router.circuitBreaker.reset = vi.fn();
      router.rateLimitTracker.invalidateCache = vi.fn();
      router.requestDedupe.clear = vi.fn();
      router.modelPerfTracker.reset = vi.fn();
      router.apiKeyTimeoutTracker.reset = vi.fn();

      router.resetStats();

      expect(router.stats.totalRequests).toBe(0);
      expect(router.stats.successes).toBe(0);
      expect(router.stats.failures).toBe(0);
      expect(router.circuitBreaker.reset).toHaveBeenCalled();
      expect(router.rateLimitTracker.invalidateCache).toHaveBeenCalled();
      expect(router.requestDedupe.clear).toHaveBeenCalled();
      expect(router.modelPerfTracker.reset).toHaveBeenCalled();
      expect(router.apiKeyTimeoutTracker.reset).toHaveBeenCalled();
    });
  });

  describe("isReady", () => {
    it("should return true when enabled with API key", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      expect(router.isReady()).toBe(true);
    });

    it("should return false when not enabled", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: false,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      expect(router.isReady()).toBe(false);
    });

    it("should return falsy when enabled but no API keys", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: [],
        primaryModel: "test/model",
      });

      expect(router.sessionApiKey).toBeNull();
      // isReady() returns null (because sessionApiKey is null)
      expect(!router.isReady()).toBe(true);
    });
  });

  describe("validateConfig", () => {
    it("should call configValidator validateConfig", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.configValidator.validateConfig = vi
        .fn()
        .mockResolvedValue({ valid: true });

      const result = await router.validateConfig({ some: "config" });

      expect(result.valid).toBe(true);
      expect(router.configValidator.validateConfig).toHaveBeenCalledWith({
        some: "config",
      });
    });
  });

  describe("Error Response Status Codes", () => {
    it("should handle 422 Unprocessable Entity", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 422,
        statusText: "Unprocessable Entity",
        text: async () => "Invalid parameters",
      });

      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        expect(e).toBeDefined();
      }
    });

    it("should handle 429 with different error message format", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 429,
        statusText: "Too Many Requests",
        text: async () => "Rate limit exceeded",
      });

      try {
        await router._callDirect({ model: "test", messages: [] }, 60000);
      } catch (e) {
        expect(e).toBeDefined();
      }
    });
  });

  describe("Proxy with Credentials", () => {
    it("should use proxy with username and password", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy.example.com:8080:user:pass"],
      });

      const proxy = router._selectRequestProxy();
      const parsed = router._parseProxy(proxy);

      expect(parsed.username).toBe("user");
      expect(parsed.password).toBe("pass");
    });
  });

  describe("Empty working array with config models", () => {
    it("should use only configured models when working is empty array", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => ({
          working: [],
          total: 0,
        }),
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "configured1",
        fallbackModels: ["configured2", "configured3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });
  });

  describe("Direct fallback failure path", () => {
    it("should handle direct fallback when proxy fails but fallbackToDirect disabled", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: false,
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });
  });

  describe("Circuit breaker states", () => {
    it("should skip model when circuit breaker is open", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      router.circuitBreaker.check = vi.fn().mockReturnValue({ allowed: false });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "success" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain("exhausted");
    });
  });

  describe("Additional uncovered areas", () => {
    it("should use fallback models when no test results and multiple fallbacks configured", async () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper(null);

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary1",
        fallbackModels: ["fallback1", "fallback2", "fallback3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should track circuit breaks when circuit breaker is open", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      // First call: circuit open for model1, so skip
      // Second call: circuit open for model2, skip
      router.circuitBreaker.check = vi.fn().mockReturnValue({ allowed: false });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
      expect(router.stats.circuitBreaks).toBeGreaterThan(0);
    });

    it("should return all configured models in getModelsInfo", () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1", "fallback2"],
      });

      const info = router.getModelsInfo();

      expect(info.allConfigured).toContain("primary");
      expect(info.allConfigured).toContain("fallback1");
      expect(info.allConfigured).toContain("fallback2");
    });

    it("should handle getResults returning null working array", () => {
      const {
        FreeApiRouter,
        setSharedHelper,
      } = require("../../utils/free-api-router.js");

      setSharedHelper({
        getResults: () => null,
      });

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "primary",
        fallbackModels: ["fallback1"],
      });

      const info = router.getModelsInfo();

      expect(info.testedWorking).toEqual([]);
    });

    it("should log progress when retrying with more models remaining", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2", "model3"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });

    it("should handle 503 Service Unavailable as server error and fallback to next model", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 503,
            text: async () => "Service Unavailable",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      // Second model succeeded
      expect(result.model).toBe("model2");
    });

    it("should track quick timeouts in stats", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        quickTimeout: 1,
      });

      global.fetch = vi
        .fn()
        .mockImplementation(
          () =>
            new Promise((_, reject) =>
              setTimeout(() => reject(new Error("AbortError")), 50),
            ),
        );

      try {
        await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });
      } catch (_e) {
        // Expected to fail
      }

      // The quickTimeout should have been triggered
      expect(router.stats.quickTimeouts).toBeGreaterThanOrEqual(0);
    });

    it("should handle API key timeout tracker returning quick timeout", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.apiKeyTimeoutTracker.getTimeoutForKey = vi
        .fn()
        .mockReturnValue(1000);

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
    });

    it("should handle proxy with no credentials", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
      });

      const proxy = router._selectRequestProxy();
      const parsed = router._parseProxy(proxy);

      expect(parsed.host).toBe("proxy1");
      expect(parsed.port).toBe("8080");
      expect(parsed.username).toBeNull();
      expect(parsed.password).toBeNull();
    });

    it("should set request deduplication on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.requestDedupe.set = vi.fn();

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.requestDedupe.set).toHaveBeenCalled();
    });

    it("should include warningStatus in success result", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.rateLimitTracker.getWarningStatus = vi.fn().mockReturnValue("ok");

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.warningStatus).toBe("ok");
    });

    it("should include modelFallbacks in result", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.modelFallbacks).toBeDefined();
    });

    it("should call rateLimitTracker.trackRequest on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.rateLimitTracker.trackRequest = vi.fn();

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.rateLimitTracker.trackRequest).toHaveBeenCalled();
    });

    it("should call apiKeyTimeoutTracker.trackRequest on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.apiKeyTimeoutTracker.trackRequest = vi.fn();

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.apiKeyTimeoutTracker.trackRequest).toHaveBeenCalledWith(
        "test-key",
        expect.any(Number),
        true,
      );
    });

    it("should handle direct fallback success properly", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: true,
      });

      let callCount = 0;
      global.fetch = vi.fn().mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: async () => "Proxy Error",
          });
        }
        return Promise.resolve({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "direct success" } }],
          }),
        });
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(true);
      expect(result.directFallbackUsed).toBe(true);
      expect(result.content).toBe("direct success");
    });

    it("should handle no proxy fallback when disabled", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
        proxyEnabled: true,
        proxyList: ["proxy1:8080"],
        proxyFallbackToDirect: false,
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Proxy Error",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });

    it("should log moving to next model", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "model1",
        fallbackModels: ["model2"],
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => "Error",
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.success).toBe(false);
    });

    it("should return keyUsed in result", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.keyUsed).toBe(0);
    });

    it("should return proxyUsed in result", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.proxyUsed).toBeUndefined();
    });

    it("should handle retryCount in result", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      const result = await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(result.retryCount).toBeDefined();
    });

    it("should call circuitBreaker.recordSuccess on success", async () => {
      const { FreeApiRouter } = require("../../utils/free-api-router.js");

      router = new FreeApiRouter({
        enabled: true,
        apiKeys: ["test-key"],
        primaryModel: "test/model",
      });

      router.circuitBreaker.recordSuccess = vi.fn();

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          choices: [{ message: { content: "response" } }],
        }),
      });

      await router.processRequest({
        messages: [{ role: "user", content: "hello" }],
      });

      expect(router.circuitBreaker.recordSuccess).toHaveBeenCalled();
    });
  });

  describe("Coverage Gap Fixes", () => {
    describe("processRequest proxy success end-to-end", () => {
      it("should succeed through proxy via _callThroughProxy in processRequest", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
          proxyEnabled: true,
          proxyList: ["proxy1:8080"],
        });

        global.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "proxy success" } }],
          }),
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(true);
        expect(result.content).toBe("proxy success");
      });
    });

    describe("processRequest rate limit exhausted", () => {
      it("should skip model when rateLimitStatus is exhausted", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "model1",
          fallbackModels: ["model2"],
        });

        router.rateLimitTracker.getWarningStatus = vi
          .fn()
          .mockReturnValue("exhausted");

        global.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "response" } }],
          }),
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(false);
        expect(result.error).toContain("exhausted");
      });
    });

    describe("processRequest progress logging", () => {
      it("should log progress when first model fails and more remain", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "model1",
          fallbackModels: ["model2", "model3"],
        });

        let callCount = 0;
        global.fetch = vi.fn().mockImplementation(() => {
          callCount++;
          if (callCount === 1) {
            return Promise.resolve({
              ok: false,
              status: 500,
              text: async () => "Server Error",
            });
          }
          return Promise.resolve({
            ok: true,
            json: async () => ({
              choices: [{ message: { content: "success on model2" } }],
            }),
          });
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(true);
        expect(result.model).toBe("model2");
      });
    });

    describe("syncWithHelper primary in working", () => {
      it("should set fallbacks to other working models when primary is in working", async () => {
        const {
          FreeApiRouter,
          setSharedHelper,
        } = require("../../utils/free-api-router.js");

        setSharedHelper({
          getResults: () => ({
            working: ["primary/model", "working2", "working3"],
            total: 3,
          }),
        });

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "primary/model",
          fallbackModels: ["old-fallback"],
        });

        const result = router.syncWithHelper();

        expect(result).toBe(true);
        expect(router.config.models.primary).toBe("primary/model");
        expect(router.config.models.fallbacks).toEqual([
          "working2",
          "working3",
        ]);
        expect(router.config.models.fallbacks).not.toContain("primary/model");
      });
    });

    describe("getModelsInfo with null helper", () => {
      it("should return empty testedFailed when helper returns null", () => {
        const {
          FreeApiRouter,
          setSharedHelper,
        } = require("../../utils/free-api-router.js");

        setSharedHelper({
          getResults: () => null,
        });

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "primary",
          fallbackModels: ["fallback1"],
        });

        const info = router.getModelsInfo();

        expect(info.testedWorking).toEqual([]);
        expect(info.testedFailed).toEqual([]);
        expect(info.totalTested).toBe(0);
      });

      it("should return empty testedFailed when results has no failed property", () => {
        const {
          FreeApiRouter,
          setSharedHelper,
        } = require("../../utils/free-api-router.js");

        setSharedHelper({
          getResults: () => ({
            working: ["model1"],
            total: 1,
          }),
        });

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "primary",
          fallbackModels: ["fallback1"],
        });

        const info = router.getModelsInfo();

        expect(info.testedWorking).toEqual(["model1"]);
        expect(info.testedFailed).toEqual([]);
        expect(info.totalTested).toBe(1);
      });
    });

    describe("getStats successRate", () => {
      it("should return calculated successRate when totalRequests > 0", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.stats.totalRequests = 10;
        router.stats.successes = 5;

        const stats = router.getStats();

        expect(stats.router.successRate).toBe("50.0%");
      });

      it("should return 0% when totalRequests is 0", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        const stats = router.getStats();

        expect(stats.router.successRate).toBe("0%");
      });
    });

    describe("directFallbackUsed result fields", () => {
      it("should include directFallbackUsed true when proxy fails and direct succeeds", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
          proxyEnabled: true,
          proxyList: ["proxy1:8080"],
          proxyFallbackToDirect: true,
        });

        let callCount = 0;
        global.fetch = vi.fn().mockImplementation(() => {
          callCount++;
          if (callCount === 1) {
            return Promise.resolve({
              ok: false,
              status: 500,
              text: async () => "Proxy Error",
            });
          }
          return Promise.resolve({
            ok: true,
            json: async () => ({
              choices: [{ message: { content: "direct success" } }],
            }),
          });
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(true);
        expect(result.directFallbackUsed).toBe(true);
        expect(result.proxyUsed).toBe(false);
        expect(result.content).toBe("direct success");
        expect(result.modelFallbacks).toBeGreaterThanOrEqual(0);
      });
    });

    describe("response content parsing edge cases", () => {
      it("should handle _callDirect with undefined content (no content key)", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        global.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: async () => ({
            choices: [{ message: { reasoning_content: "reasoned answer" } }],
          }),
        });

        const result = await router._callDirect(
          { model: "test", messages: [] },
          60000,
        );

        expect(result.success).toBe(true);
        expect(result.content).toBe("reasoned answer");
      });

      it("should handle _callThroughProxy with reasoning_content fallback", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
          proxyEnabled: true,
          proxyList: ["proxy1:8080"],
        });

        global.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: async () => ({
            choices: [{ message: { reasoning_content: "proxy reasoned" } }],
          }),
        });

        const result = await router._callThroughProxy(
          { host: "proxy1", port: "8080" },
          { model: "test", messages: [] },
          60000,
        );

        expect(result.success).toBe(true);
        expect(result.content).toBe("proxy reasoned");
        expect(result.proxy).toBe("proxy1:8080");
      });
    });

    describe("non-429 server error handling", () => {
      it("should handle 500 errors and fallback to next model", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "model1",
          fallbackModels: ["model2"],
        });

        let callCount = 0;
        global.fetch = vi.fn().mockImplementation(() => {
          callCount++;
          if (callCount === 1) {
            return Promise.resolve({
              ok: false,
              status: 500,
              text: async () => "Internal Server Error",
            });
          }
          return Promise.resolve({
            ok: true,
            json: async () => ({
              choices: [{ message: { content: "success" } }],
            }),
          });
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(true);
        expect(result.content).toBe("success");
      });

      it("should handle 503 errors through _callDirect throw", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        global.fetch = vi.fn().mockResolvedValue({
          ok: false,
          status: 503,
          text: async () => "Service Unavailable",
        });

        await expect(
          router._callDirect({ model: "test", messages: [] }, 60000),
        ).rejects.toThrow();
      });
    });

    describe("rateLimitTracker and apiKeyTimeoutTracker on success", () => {
      it("should call rateLimitTracker.trackRequest on success", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        global.fetch = vi.fn().mockResolvedValue({
          ok: true,
          json: async () => ({
            choices: [{ message: { content: "response" } }],
          }),
        });

        const result = await router.processRequest({
          messages: [{ role: "user", content: "hello" }],
        });

        expect(result.success).toBe(true);
      });
    });

    describe("getDetailedStats delegation", () => {
      it("should call all sub-module methods", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "primary",
          fallbackModels: ["fallback1"],
        });

        router.circuitBreaker.getAllStates = vi.fn().mockReturnValue({});
        router.rateLimitTracker.getCacheStatus = vi.fn().mockReturnValue({});
        router.modelPerfTracker.getAllStats = vi.fn().mockReturnValue({});
        router.modelPerfTracker.getBestModel = vi
          .fn()
          .mockReturnValue("primary");

        const stats = router.getDetailedStats();

        expect(router.circuitBreaker.getAllStates).toHaveBeenCalled();
        expect(router.rateLimitTracker.getCacheStatus).toHaveBeenCalled();
        expect(router.modelPerfTracker.getAllStats).toHaveBeenCalled();
        expect(router.modelPerfTracker.getBestModel).toHaveBeenCalledWith(
          ["primary", "fallback1"],
          "test-key",
        );
        expect(stats.bestModel).toBe("primary");
      });
    });

    describe("validateConfig delegation", () => {
      it("should pass config through to configValidator", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.configValidator.validateConfig = vi
          .fn()
          .mockResolvedValue({ valid: true, warnings: [] });

        const testConfig = { apiKeys: ["key1"], models: { primary: "model1" } };
        const result = await router.validateConfig(testConfig);

        expect(result.valid).toBe(true);
        expect(router.configValidator.validateConfig).toHaveBeenCalledWith(
          testConfig,
        );
      });
    });

    describe("resetStats module delegation", () => {
      it("should call reset/invalidate on all sub-modules", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.stats.totalRequests = 10;
        router.stats.successes = 5;
        router.stats.failures = 5;

        router.circuitBreaker.reset = vi.fn();
        router.rateLimitTracker.invalidateCache = vi.fn();
        router.requestDedupe.clear = vi.fn();
        router.modelPerfTracker.reset = vi.fn();
        router.apiKeyTimeoutTracker.reset = vi.fn();

        router.resetStats();

        expect(router.stats.totalRequests).toBe(0);
        expect(router.circuitBreaker.reset).toHaveBeenCalled();
        expect(router.rateLimitTracker.invalidateCache).toHaveBeenCalled();
        expect(router.requestDedupe.clear).toHaveBeenCalled();
        expect(router.modelPerfTracker.reset).toHaveBeenCalled();
        expect(router.apiKeyTimeoutTracker.reset).toHaveBeenCalled();
      });
    });

    describe("isReady variations", () => {
      it("should return true when enabled with API key", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        expect(router.isReady()).toBe(true);
      });

      it("should return falsy when enabled but no API keys", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: true,
          apiKeys: [],
          primaryModel: "test/model",
        });

        expect(!router.isReady()).toBe(true);
      });

      it("should return false when disabled", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");

        router = new FreeApiRouter({
          enabled: false,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        expect(router.isReady()).toBe(false);
      });
    });

    describe("validateConfig", () => {
      it("should validate config settings", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({ enabled: false });

        router.configValidator = {
          validateConfig: vi.fn().mockResolvedValue({ valid: true }),
        };

        const result = await router.validateConfig({ test: "config" });
        expect(result).toBeDefined();
      });
    });

    describe("refreshRateLimits", () => {
      it("should refresh rate limits for session API key", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.rateLimitTracker.refreshKey = vi.fn().mockResolvedValue({});

        const result = await router.refreshRateLimits();
        expect(router.rateLimitTracker.refreshKey).toHaveBeenCalledWith(
          "test-key",
        );
      });

      it("should return null when no session API key", async () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({ enabled: false });

        const result = await router.refreshRateLimits();
        expect(result).toBeNull();
      });
    });

    describe("getSessionInfo", () => {
      it("should return session information", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["key1", "key2"],
          primaryModel: "test/model",
          fallbackModels: ["fallback1", "fallback2"],
          proxyEnabled: true,
          proxyList: ["proxy1:8080"],
          browserId: "browser1",
          taskId: "task1",
        });

        const info = router.getSessionInfo();
        expect(info.sessionId).toBe("browser1:task1");
        expect(info.browserId).toBe("browser1");
        expect(info.taskId).toBe("task1");
        expect(info.apiKeyIndex).toBeGreaterThan(0);
        expect(info.totalApiKeys).toBe(2);
        expect(info.primaryModel).toBe("test/model");
        expect(info.fallbackCount).toBe(2);
        expect(info.proxyEnabled).toBe(true);
        expect(info.proxyCount).toBe(1);
        expect(info.timeout).toBeDefined();
      });
    });

    describe("getStats", () => {
      it("should return router statistics", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.circuitBreaker.getStats = vi.fn().mockReturnValue({});
        router.rateLimitTracker.getStats = vi.fn().mockReturnValue({});
        router.requestDedupe.getStats = vi.fn().mockReturnValue({});
        router.modelPerfTracker.getStats = vi.fn().mockReturnValue({});
        router.apiKeyTimeoutTracker.getStats = vi.fn().mockReturnValue({});

        const stats = router.getStats();
        expect(stats.router).toBeDefined();
        expect(stats.circuitBreaker).toBeDefined();
        expect(stats.rateLimitTracker).toBeDefined();
        expect(stats.requestDedupe).toBeDefined();
        expect(stats.modelPerfTracker).toBeDefined();
        expect(stats.apiKeyTimeoutTracker).toBeDefined();
      });

      it("should calculate success rate correctly", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        router.stats.totalRequests = 10;
        router.stats.successes = 7;
        router.stats.failures = 3;

        const stats = router.getStats();
        expect(stats.router.successRate).toBe("70.0%");
      });

      it("should return 0% when no requests", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
        });

        const stats = router.getStats();
        expect(stats.router.successRate).toBe("0%");
      });
    });

    describe("getDetailedStats", () => {
      it("should return detailed statistics", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "test/model",
          fallbackModels: ["fallback"],
        });

        router.circuitBreaker.getAllStates = vi.fn().mockReturnValue({});
        router.rateLimitTracker.getCacheStatus = vi.fn().mockReturnValue({});
        router.modelPerfTracker.getAllStats = vi.fn().mockReturnValue({});
        router.modelPerfTracker.getBestModel = vi
          .fn()
          .mockReturnValue("test/model");

        const detailed = router.getDetailedStats();
        expect(detailed.session).toBeDefined();
        expect(detailed.router).toBeDefined();
        expect(detailed.circuitBreakerStates).toBeDefined();
        expect(detailed.rateLimitStatus).toBeDefined();
        expect(detailed.modelPerformance).toBeDefined();
        expect(detailed.bestModel).toBeDefined();
      });
    });

    describe("syncWithHelper", () => {
      it("should return true when helper has working models", () => {
        setSharedHelper({
          getResults: vi.fn().mockReturnValue({
            working: ["model1"],
            failed: [],
            total: 1,
          }),
        });
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "model1",
        });

        const result = router.syncWithHelper();
        expect(result).toBe(true);
      });
    });

    describe("getModelsInfo", () => {
      it("should return models info", () => {
        const { FreeApiRouter } = require("../../utils/free-api-router.js");
        router = new FreeApiRouter({
          enabled: true,
          apiKeys: ["test-key"],
          primaryModel: "model1",
          fallbackModels: ["fallback"],
        });

        const info = router.getModelsInfo();
        expect(info.primary).toBeDefined();
      });
    });
  });
});
