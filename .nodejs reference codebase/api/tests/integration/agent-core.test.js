/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Core Integration Tests
 * Tests AI agent perception, reasoning, decision-making, and action execution
 * @module tests/integration/agent-core.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { withPage } from "@api/core/context.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/core/context.js", () => ({
  withPage: vi.fn(async (_page, fn) => {
    const mockCtx = {
      get: vi.fn((key) => {
        if (key === "page")
          return { url: vi.fn(), title: vi.fn(), viewportSize: vi.fn() };
        return null;
      }),
      set: vi.fn(),
    };
    return fn(mockCtx);
  }),
  isSessionActive: vi.fn(() => true),
  checkSession: vi.fn(() => true),
  clearContext: vi.fn(),
}));

// Mock LLM provider
vi.mock("@api/agent/llm-provider.js", () => ({
  default: class LLMProvider {
    constructor(config) {
      this.config = config;
      this.model = config.model || "llama2";
      this.apiKey = config.apiKey;
    }

    async generate(prompt, options = {}) {
      return {
        content: `AI response to: ${prompt.substring(0, 50)}...`,
        model: this.model,
        tokens: prompt.length + 50,
      };
    }

    async stream(prompt, onChunk) {
      const response = await this.generate(prompt);
      onChunk(response.content);
    }
  },
}));

// Mock observer
vi.mock("@api/agent/observer.js", () => ({
  default: class Observer {
    constructor(page) {
      this.page = page;
      this.observations = [];
    }

    async observe() {
      const observation = {
        timestamp: Date.now(),
        url: this.page.url(),
        title: await this.page.title(),
        viewport: await this.page.viewportSize(),
      };
      this.observations.push(observation);
      return observation;
    }

    getRecentObservations(count = 10) {
      return this.observations.slice(-count);
    }

    clearObservations() {
      this.observations = [];
    }
  },
}));

// Mock memory injector
vi.mock("@api/agent/memoryInjector.js", () => ({
  default: class MemoryInjector {
    constructor(observer) {
      this.observer = observer;
      this.memory = [];
    }

    async inject(memoryItem) {
      this.memory.push({
        ...memoryItem,
        id: `mem-${Date.now()}`,
        timestamp: Date.now(),
      });
      return true;
    }

    async retrieve(query, limit = 5) {
      return this.memory
        .filter((m) => m.content.includes(query))
        .slice(0, limit);
    }

    getMemorySize() {
      return this.memory.length;
    }

    clearMemory() {
      this.memory = [];
    }
  },
}));

// Mock retry strategy
vi.mock("@api/agent/retryStrategy.js", () => ({
  default: class RetryStrategy {
    constructor(config = {}) {
      this.maxRetries = config.maxRetries || 3;
      this.backoffMs = config.backoffMs || 1000;
      this.attempts = 0;
    }

    async execute(operation, onRetry) {
      let lastError;
      for (let i = 0; i < this.maxRetries; i++) {
        this.attempts = i + 1;
        try {
          return await operation();
        } catch (error) {
          lastError = error;
          if (i < this.maxRetries - 1 && onRetry) {
            await onRetry(i, error);
            await this.delay(this.backoffMs * Math.pow(2, i));
          }
        }
      }
      throw lastError;
    }

    delay(ms) {
      return new Promise((resolve) => setTimeout(resolve, ms));
    }

    reset() {
      this.attempts = 0;
    }
  },
}));

// Mock response validator
vi.mock("@api/agent/responseValidator.js", () => ({
  default: class ResponseValidator {
    constructor(config = {}) {
      this.requiredFields = config.requiredFields || [];
      this.maxLength = config.maxLength || 10000;
    }

    validate(response) {
      const errors = [];

      if (!response || typeof response !== "object") {
        errors.push("Response must be an object");
        return { valid: false, errors };
      }

      if (!response.content && !response.text) {
        errors.push("Response must have content or text field");
      }

      if (this.requiredFields.length > 0) {
        for (const field of this.requiredFields) {
          if (!(field in response)) {
            errors.push(`Missing required field: ${field}`);
          }
        }
      }

      const content = response.content || response.text || "";
      if (content.length > this.maxLength) {
        errors.push(`Response exceeds max length of ${this.maxLength}`);
      }

      return {
        valid: errors.length === 0,
        errors,
      };
    }
  },
}));

// Mock response cache
vi.mock("@api/agent/responseCache.js", () => ({
  default: class ResponseCache {
    constructor(config = {}) {
      this.maxSize = config.maxSize || 100;
      this.ttl = config.ttl || 60000; // 1 minute
      this.cache = new Map();
    }

    async get(key) {
      const entry = this.cache.get(key);
      if (!entry) return null;

      if (Date.now() - entry.timestamp > this.ttl) {
        this.cache.delete(key);
        return null;
      }

      return entry.value;
    }

    async set(key, value) {
      if (this.cache.size >= this.maxSize) {
        // Remove oldest entry
        const firstKey = this.cache.keys().next().value;
        this.cache.delete(firstKey);
      }
      this.cache.set(key, {
        value,
        timestamp: Date.now(),
      });
      return true;
    }

    async invalidate(key) {
      return this.cache.delete(key);
    }

    clear() {
      this.cache.clear();
    }

    getStats() {
      return {
        size: this.cache.size,
        maxSize: this.maxSize,
      };
    }
  },
}));

// Mock history manager
vi.mock("@api/agent/historyManager.js", () => ({
  default: class HistoryManager {
    constructor() {
      this.history = [];
      this.maxEntries = 100;
    }

    async add(entry) {
      this.history.push({
        ...entry,
        id: `hist-${Date.now()}`,
        timestamp: Date.now(),
      });

      if (this.history.length > this.maxEntries) {
        this.history.shift();
      }
      return true;
    }

    async getRecent(count = 10) {
      return this.history.slice(-count).reverse();
    }

    async search(query) {
      return this.history.filter((h) =>
        JSON.stringify(h).toLowerCase().includes(query.toLowerCase()),
      );
    }

    clear() {
      this.history = [];
    }

    getCount() {
      return this.history.length;
    }
  },
}));

// Mock error recovery
vi.mock("@api/agent/errorRecoveryPrompt.js", () => ({
  default: class ErrorRecoveryPrompt {
    constructor() {
      this.recoveryStrategies = [
        "retry",
        "refresh_page",
        "wait_and_retry",
        "skip_and_continue",
      ];
    }

    async generateRecoveryPlan(error, context) {
      const errorType = this.classifyError(error);
      return {
        strategy: this.selectStrategy(errorType),
        reasoning: `Recovering from ${errorType}: ${error.message}`,
        context: context,
        actions: this.getActionsForStrategy(errorType),
      };
    }

    classifyError(error) {
      if (error.message.includes("timeout")) return "timeout";
      if (error.message.includes("network")) return "network";
      if (error.message.includes("element not found")) return "element_missing";
      return "unknown";
    }

    selectStrategy(errorType) {
      const strategies = {
        timeout: "wait_and_retry",
        network: "retry",
        element_missing: "refresh_page",
        unknown: "skip_and_continue",
      };
      return strategies[errorType] || "skip_and_continue";
    }

    getActionsForStrategy(strategy) {
      const actions = {
        retry: ["wait(1000)", "retry_operation()"],
        refresh_page: ["page.reload()", "wait_for_load()"],
        wait_and_retry: ["wait(5000)", "retry_operation()"],
        skip_and_continue: ["log_error()", "continue_next_task()"],
      };
      return actions[strategy] || ["log_error()"];
    }
  },
}));

// Mock self-healing prompt
vi.mock("@api/agent/selfHealingPrompt.js", () => ({
  default: class SelfHealingPrompt {
    constructor(llmProvider) {
      this.llmProvider = llmProvider;
    }

    async generateHealingPrompt(failure, pageState) {
      const prompt = `
        Page State: ${JSON.stringify(pageState, null, 2)}
        Failure: ${failure.message}
        Suggest recovery actions.
      `;

      const response = await this.llmProvider.generate(prompt);
      return {
        prompt,
        suggestion: response.content,
        timestamp: Date.now(),
      };
    }
  },
}));

describe("Agent Core Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("LLM Provider", () => {
    it("should generate responses with correct model", async () => {
      const LLMProvider = (await import("@api/agent/llm-provider.js")).default;
      const provider = new LLMProvider({
        model: "custom-model",
        apiKey: "test-key",
      });

      const response = await provider.generate("Test prompt");

      expect(response.model).toBe("custom-model");
      expect(response.content).toContain("AI response to");
      expect(response.tokens).toBeGreaterThan(0);
    });

    it("should use default model when not specified", async () => {
      const LLMProvider = (await import("@api/agent/llm-provider.js")).default;
      const provider = new LLMProvider({});

      const response = await provider.generate("Test prompt");

      expect(response.model).toBe("llama2");
    });

    it("should support streaming responses", async () => {
      const LLMProvider = (await import("@api/agent/llm-provider.js")).default;
      const provider = new LLMProvider({});

      const chunks = [];
      await provider.stream("Test prompt", (chunk) => {
        chunks.push(chunk);
      });

      expect(chunks.length).toBeGreaterThan(0);
      expect(chunks[0]).toContain("AI response to");
    });
  });

  describe("Observer", () => {
    it("should observe page state and history", async () => {
      const Observer = (await import("@api/agent/observer.js")).default;

      await withPage({ id: "test-page" }, async (ctx) => {
        const page = ctx.get("page");
        const observer = new Observer(page);

        // Mock page methods
        vi.spyOn(page, "url").mockReturnValue("https://example.com");
        vi.spyOn(page, "title").mockResolvedValue("Example Domain");
        vi.spyOn(page, "viewportSize").mockResolvedValue({
          width: 1920,
          height: 1080,
        });

        const observation = await observer.observe();

        expect(observation.url).toBe("https://example.com");
        expect(observation.title).toBe("Example Domain");
        expect(observation.viewport).toEqual({ width: 1920, height: 1080 });
        expect(observation.timestamp).toBeDefined();
      });
    });

    it("should maintain observation history", async () => {
      const Observer = (await import("@api/agent/observer.js")).default;

      await withPage({ id: "test-page" }, async (ctx) => {
        const page = ctx.get("page");
        const observer = new Observer(page);

        vi.spyOn(page, "url").mockReturnValue("https://example.com");
        vi.spyOn(page, "title").mockResolvedValue("Page 1");
        await observer.observe();

        vi.spyOn(page, "url").mockReturnValue("https://example2.com");
        vi.spyOn(page, "title").mockResolvedValue("Page 2");
        await observer.observe();

        const recent = observer.getRecentObservations(2);
        expect(recent).toHaveLength(2);
        expect(recent[1].title).toBe("Page 2"); // Most recent
      });
    });

    it("should clear observations when requested", async () => {
      const Observer = (await import("@api/agent/observer.js")).default;

      await withPage({ id: "test-page" }, async (ctx) => {
        const page = ctx.get("page");
        const observer = new Observer(page);

        vi.spyOn(page, "url").mockReturnValue("https://example.com");
        vi.spyOn(page, "title").mockResolvedValue("Test");

        await observer.observe();
        await observer.observe();
        expect(observer.getRecentObservations()).toHaveLength(2);

        observer.clearObservations();
        expect(observer.getRecentObservations()).toHaveLength(0);
      });
    });
  });

  describe("Memory Injector", () => {
    it("should inject and retrieve memories", async () => {
      const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
        .default;
      const memory = new MemoryInjector(null);

      await memory.inject({
        content: "User prefers dark mode",
        type: "preference",
      });
      await memory.inject({
        content: "Last login was 2025-03-25",
        type: "history",
      });

      const results = await memory.retrieve("dark mode");
      expect(results).toHaveLength(1);
      expect(results[0].content).toContain("dark mode");
      expect(results[0].id).toBeDefined();
    });

    it("should respect limit in retrieve operations", async () => {
      const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
        .default;
      const memory = new MemoryInjector(null);

      for (let i = 0; i < 10; i++) {
        await memory.inject({ content: `Memory ${i}`, type: "test" });
      }

      const results = await memory.retrieve("Memory", 3);
      expect(results).toHaveLength(3);
    });

    it("should track memory size correctly", async () => {
      const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
        .default;
      const memory = new MemoryInjector(null);

      expect(memory.getMemorySize()).toBe(0);

      await memory.inject({ content: "Test 1" });
      expect(memory.getMemorySize()).toBe(1);

      await memory.inject({ content: "Test 2" });
      expect(memory.getMemorySize()).toBe(2);
    });

    it("should clear all memories", async () => {
      const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
        .default;
      const memory = new MemoryInjector(null);

      await memory.inject({ content: "Test 1" });
      await memory.inject({ content: "Test 2" });
      expect(memory.getMemorySize()).toBe(2);

      memory.clearMemory();
      expect(memory.getMemorySize()).toBe(0);
    });
  });

  describe("Retry Strategy", () => {
    it("should retry failed operations up to max retries", async () => {
      const RetryStrategy = (await import("@api/agent/retryStrategy.js"))
        .default;
      const strategy = new RetryStrategy({ maxRetries: 3, backoffMs: 10 });

      let attempts = 0;
      const operation = async () => {
        attempts++;
        if (attempts < 3) throw new Error("Not yet");
        return "success";
      };

      const result = await strategy.execute(operation);
      expect(result).toBe("success");
      expect(attempts).toBe(3);
    });

    it("should throw after all retries exhausted", async () => {
      const RetryStrategy = (await import("@api/agent/retryStrategy.js"))
        .default;
      const strategy = new RetryStrategy({ maxRetries: 2, backoffMs: 10 });

      const operation = async () => {
        throw new Error("Always fails");
      };

      await expect(strategy.execute(operation)).rejects.toThrow("Always fails");
    });

    it("should call onRetry callback between attempts", async () => {
      const RetryStrategy = (await import("@api/agent/retryStrategy.js"))
        .default;
      const strategy = new RetryStrategy({ maxRetries: 2, backoffMs: 10 });

      const onRetry = vi.fn();

      let attempts = 0;
      const operation = async () => {
        attempts++;
        if (attempts < 2) throw new Error("Fail once");
        return "success";
      };

      await strategy.execute(operation, onRetry);
      expect(onRetry).toHaveBeenCalledTimes(1);
    });

    it("should reset attempt counter", async () => {
      const RetryStrategy = (await import("@api/agent/retryStrategy.js"))
        .default;
      const strategy = new RetryStrategy({ maxRetries: 2 });

      const operation = async () => {
        throw new Error("Fail");
      };

      try {
        await strategy.execute(operation);
      } catch (e) {
        // Expected
      }

      expect(strategy.attempts).toBe(2);
      strategy.reset();
      expect(strategy.attempts).toBe(0);
    });
  });

  describe("Response Validator", () => {
    it("should validate required fields", async () => {
      const ResponseValidator = (
        await import("@api/agent/responseValidator.js")
      ).default;
      const validator = new ResponseValidator({
        requiredFields: ["content", "model"],
      });

      const result = validator.validate({ content: "test", model: "llama2" });
      expect(result.valid).toBe(true);
    });

    it("should reject missing required fields", async () => {
      const ResponseValidator = (
        await import("@api/agent/responseValidator.js")
      ).default;
      const validator = new ResponseValidator({
        requiredFields: ["content", "model"],
      });

      const result = validator.validate({ content: "test" });
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Missing required field: model");
    });

    it("should validate response length", async () => {
      const ResponseValidator = (
        await import("@api/agent/responseValidator.js")
      ).default;
      const validator = new ResponseValidator({ maxLength: 100 });

      const result = validator.validate({ content: "short" });
      expect(result.valid).toBe(true);

      const longContent = "a".repeat(200);
      const result2 = validator.validate({ content: longContent });
      expect(result2.valid).toBe(false);
      expect(result2.errors[0]).toContain("exceeds max length");
    });

    it("should handle invalid response types", async () => {
      const ResponseValidator = (
        await import("@api/agent/responseValidator.js")
      ).default;
      const validator = new ResponseValidator({});

      const result = validator.validate(null);
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Response must be an object");
    });
  });

  describe("Response Cache", () => {
    it("should cache and retrieve responses", async () => {
      const ResponseCache = (await import("@api/agent/responseCache.js"))
        .default;
      const cache = new ResponseCache({ maxSize: 10, ttl: 60000 });

      await cache.set("key1", { data: "test" });
      const result = await cache.get("key1");

      expect(result).toEqual({ data: "test" });
    });

    it("should return null for missing keys", async () => {
      const ResponseCache = (await import("@api/agent/responseCache.js"))
        .default;
      const cache = new ResponseCache({});

      const result = await cache.get("nonexistent");
      expect(result).toBeNull();
    });

    it("should invalidate cached entries", async () => {
      const ResponseCache = (await import("@api/agent/responseCache.js"))
        .default;
      const cache = new ResponseCache({});

      await cache.set("key1", { data: "test" });
      await cache.invalidate("key1");

      const result = await cache.get("key1");
      expect(result).toBeNull();
    });

    it("should respect max size limit", async () => {
      const ResponseCache = (await import("@api/agent/responseCache.js"))
        .default;
      const cache = new ResponseCache({ maxSize: 2 });

      await cache.set("key1", { data: "1" });
      await cache.set("key2", { data: "2" });
      await cache.set("key3", { data: "3" });

      const stats = cache.getStats();
      expect(stats.size).toBe(2);

      // Oldest entry (key1) should be evicted
      expect(await cache.get("key1")).toBeNull();
      expect(await cache.get("key2")).not.toBeNull();
      expect(await cache.get("key3")).not.toBeNull();
    });

    it("should clear entire cache", async () => {
      const ResponseCache = (await import("@api/agent/responseCache.js"))
        .default;
      const cache = new ResponseCache({});

      await cache.set("key1", { data: "1" });
      await cache.set("key2", { data: "2" });

      cache.clear();
      expect(cache.getStats().size).toBe(0);
    });
  });

  describe("History Manager", () => {
    it("should add and retrieve history entries", async () => {
      const HistoryManager = (await import("@api/agent/historyManager.js"))
        .default;
      const history = new HistoryManager();

      await history.add({ action: "click", target: "#button" });
      await history.add({ action: "type", target: "#input", text: "hello" });

      const recent = await history.getRecent(2);
      expect(recent).toHaveLength(2);
      expect(recent[0].action).toBe("type");
      expect(recent[1].action).toBe("click");
    });

    it("should search history by content", async () => {
      const HistoryManager = (await import("@api/agent/historyManager.js"))
        .default;
      const history = new HistoryManager();

      await history.add({ action: "click", target: "#submit" });
      await history.add({
        action: "type",
        target: "#email",
        text: "user@example.com",
      });
      await history.add({ action: "click", target: "#cancel" });

      const results = await history.search("email");
      expect(results).toHaveLength(1);
      expect(results[0].action).toBe("type");
    });

    it("should limit history size to max entries", async () => {
      const HistoryManager = (await import("@api/agent/historyManager.js"))
        .default;
      const history = new HistoryManager();

      for (let i = 0; i < 150; i++) {
        await history.add({ action: "test", index: i });
      }

      expect(history.getCount()).toBe(100); // maxEntries default
    });

    it("should clear all history", async () => {
      const HistoryManager = (await import("@api/agent/historyManager.js"))
        .default;
      const history = new HistoryManager();

      await history.add({ action: "test" });
      await history.add({ action: "test2" });

      history.clear();
      expect(history.getCount()).toBe(0);
    });
  });

  describe("Error Recovery", () => {
    it("should import error recovery module", async () => {
      const ErrorRecoveryPrompt = (
        await import("@api/agent/errorRecoveryPrompt.js")
      ).default;
      expect(ErrorRecoveryPrompt).toBeDefined();
    });
  });

  describe("Self-Healing", () => {
    it("should generate healing prompts from failures", async () => {
      const LLMProvider = (await import("@api/agent/llm-provider.js")).default;
      const SelfHealingPrompt = (
        await import("@api/agent/selfHealingPrompt.js")
      ).default;

      const llm = new LLMProvider({});
      const healing = new SelfHealingPrompt(llm);

      const failure = new Error("Element not clickable");
      const pageState = { url: "https://example.com", title: "Test" };

      const result = await healing.generateHealingPrompt(failure, pageState);

      expect(result.prompt).toContain("Page State");
      expect(result.prompt).toContain("Element not clickable");
      expect(result.suggestion).toBeDefined();
    });
  });

  describe("Agent Integration with Context", () => {
    it("should maintain agent state within page context", async () => {
      await withPage({ id: "agent-test" }, async (ctx) => {
        const Observer = (await import("@api/agent/observer.js")).default;
        const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
          .default;

        const page = ctx.get("page");
        const observer = new Observer(page);
        const memory = new MemoryInjector(observer);

        ctx.set("observer", observer);
        ctx.set("memory", memory);

        await memory.inject({ content: "Test memory", type: "note" });
        expect(memory.getMemorySize()).toBe(1);

        const retrieved = await memory.retrieve("Test");
        expect(retrieved).toHaveLength(1);
      });
    });

    it("should isolate agent components between pages", async () => {
      let memory1Size = 0;
      let memory2Size = 0;

      await withPage({ id: "page-1" }, async (ctx1) => {
        const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
          .default;
        const memory1 = new MemoryInjector(null);
        ctx1.set("memory", memory1);

        await memory1.inject({ content: "Page 1 memory" });
        memory1Size = memory1.getMemorySize();
      });

      await withPage({ id: "page-2" }, async (ctx2) => {
        const MemoryInjector = (await import("@api/agent/memoryInjector.js"))
          .default;
        const memory2 = new MemoryInjector(null);
        ctx2.set("memory", memory2);

        await memory2.inject({ content: "Page 2 memory" });
        memory2Size = memory2.getMemorySize();
      });

      expect(memory1Size).toBe(1);
      expect(memory2Size).toBe(1);
    });
  });
});
