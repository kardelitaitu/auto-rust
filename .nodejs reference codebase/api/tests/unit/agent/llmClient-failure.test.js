/**
 * Unit tests for api/agent/llmClient.js
 * Tests for failure scenarios and error handling
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock dependencies
vi.mock("@api/tests/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/tests/core/config.js", () => ({
  configManager: {
    init: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockReturnValue({
      baseUrl: "http://localhost:11434",
      model: "test-model",
      temperature: 0.7,
      contextLength: 2048,
      maxTokens: 512,
      timeoutMs: 30000,
      serverType: "ollama",
      useVision: false,
    }),
    _getDefaults: vi.fn().mockReturnValue({
      agent: {
        llm: {
          baseUrl: "http://localhost:11434",
          model: "llama2",
          temperature: 0.7,
          contextLength: 2048,
          maxTokens: 512,
          timeoutMs: 30000,
          serverType: "ollama",
        },
      },
    }),
  },
}));

describe("llmClient.js - Failure Scenarios", () => {
  let LLMClient;
  let client;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.restoreAllMocks();

    // Dynamic import to get fresh module
    const module = await import("../../../agent/llmClient.js");
    LLMClient = module.LLMClient || module.default;
    client = new LLMClient();
  });

  describe("_convertToOllamaFormat", () => {
    it("should convert OpenAI format to Ollama format", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Hello" },
            {
              type: "image_url",
              image_url: { url: "data:image/png;base64,abc123" },
            },
          ],
        },
      ];

      const result = client._convertToOllamaFormat(messages);

      expect(result[0].role).toBe("user");
      expect(result[0].content).toBe("Hello");
      expect(result[0].images).toEqual(["abc123"]);
    });

    it("should handle text-only messages", () => {
      const messages = [{ role: "user", content: "Simple text" }];

      const result = client._convertToOllamaFormat(messages);

      expect(result[0]).toEqual({ role: "user", content: "Simple text" });
    });

    it("should handle multiple images", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Look at these" },
            {
              type: "image_url",
              image_url: { url: "data:image/png;base64,img1" },
            },
            {
              type: "image_url",
              image_url: { url: "data:image/png;base64,img2" },
            },
          ],
        },
      ];

      const result = client._convertToOllamaFormat(messages);

      expect(result[0].images).toEqual(["img1", "img2"]);
    });

    it("should handle non-data URLs gracefully", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Check this" },
            {
              type: "image_url",
              image_url: { url: "https://example.com/image.png" },
            },
          ],
        },
      ];

      const result = client._convertToOllamaFormat(messages);

      expect(result[0].images).toBeUndefined();
    });

    it("should handle empty content arrays", () => {
      const messages = [{ role: "user", content: [] }];

      const result = client._convertToOllamaFormat(messages);

      expect(result[0].content).toBe("");
    });
  });

  describe("checkAvailability", () => {
    it("should return true when LLM is available", async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({ data: [{ id: "model1" }] }),
      });

      await client.init();
      const result = await client.checkAvailability();

      expect(result).toBe(true);
    });

    it("should return false when LLM responds with error", async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        text: vi.fn().mockResolvedValue("Internal Server Error"),
      });

      await client.init();
      const result = await client.checkAvailability();

      expect(result).toBe(false);
    });

    it("should return false on network error", async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));

      await client.init();
      const result = await client.checkAvailability();

      expect(result).toBe(false);
    });

    it("should return true when bypassHealthCheck is enabled", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        bypassHealthCheck: true,
      };

      global.fetch = vi.fn().mockRejectedValue(new Error("Connection refused"));

      const result = await client.checkAvailability();

      expect(result).toBe(true);
    });

    it("should handle abort timeout", async () => {
      global.fetch = vi.fn().mockImplementation(
        () =>
          new Promise((_, reject) => {
            setTimeout(
              () => reject(new DOMException("Aborted", "AbortError")),
              100,
            );
          }),
      );

      await client.init();
      const result = await client.checkAvailability();

      expect(result).toBe(false);
    });
  });

  describe("generateCompletion", () => {
    it("should generate completion with ollama format", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        model: "test-model",
        temperature: 0.7,
        contextLength: 2048,
        maxTokens: 512,
        timeoutMs: 30000,
        serverType: "ollama",
        useVision: false,
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: { content: '{"response": "test"}' },
          usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        }),
      });

      const result = await client.generateCompletion([
        { role: "user", content: "Hello" },
      ]);

      expect(result).toBeDefined();
    });

    it("should generate completion with openai format", async () => {
      client.config = {
        baseUrl: "http://localhost:8080",
        model: "test-model",
        temperature: 0.7,
        maxTokens: 512,
        timeoutMs: 30000,
        serverType: "openai",
        useVision: false,
      };

      const jsonContent = JSON.stringify({ answer: "test response" });
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          choices: [{ message: { content: jsonContent } }],
          usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        }),
      });

      const result = await client.generateCompletion([
        { role: "user", content: "Hello" },
      ]);

      expect(result).toBeDefined();
    });

    it("should throw on HTTP error", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        model: "test-model",
        temperature: 0.7,
        maxTokens: 512,
        timeoutMs: 30000,
        serverType: "ollama",
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        text: vi.fn().mockResolvedValue("Bad Request"),
      });

      await expect(
        client.generateCompletion([{ role: "user", content: "Hello" }]),
      ).rejects.toThrow("HTTP Error");
    });

    it("should throw on timeout", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        model: "test-model",
        temperature: 0.7,
        maxTokens: 512,
        timeoutMs: 100,
        serverType: "ollama",
      };

      global.fetch = vi.fn().mockImplementation(
        () =>
          new Promise((_, reject) => {
            setTimeout(
              () => reject(new DOMException("Aborted", "AbortError")),
              150,
            );
          }),
      );

      await expect(
        client.generateCompletion([{ role: "user", content: "Hello" }]),
      ).rejects.toThrow();
    });

    it("should parse JSON response correctly", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        model: "test-model",
        temperature: 0.7,
        maxTokens: 512,
        timeoutMs: 30000,
        serverType: "ollama",
        useVision: false,
      };

      const jsonResponse = { answer: "test answer" };
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: { content: JSON.stringify(jsonResponse) },
          usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        }),
      });

      const result = await client.generateCompletion([
        { role: "user", content: "Hello" },
      ]);

      expect(result).toBeDefined();
    });
  });

  describe("ensureModelRunning", () => {
    it("should not throw when model is running", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        bypassHealthCheck: false,
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({ data: [] }),
      });

      await expect(client.ensureModelRunning()).resolves.not.toThrow();
    });

    it("should warn when model is not running", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        bypassHealthCheck: false,
      };

      global.fetch = vi.fn().mockRejectedValue(new Error("Connection refused"));

      // Should not throw, just warn
      await expect(client.ensureModelRunning()).resolves.not.toThrow();
    });
  });

  describe("error recovery", () => {
    it("should handle restart state correctly", async () => {
      client.config = {
        baseUrl: "http://localhost:11434",
        model: "test-model",
        temperature: 0.7,
        maxTokens: 512,
        timeoutMs: 30000,
        serverType: "ollama",
      };

      client.isRestarting = true;
      client.restartPromise = Promise.resolve();

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: { content: '{"response": "test"}' },
          usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        }),
      });

      // Should wait for restart to complete
      await expect(
        client.generateCompletion([{ role: "user", content: "Hello" }]),
      ).resolves.toBeDefined();
    });
  });
});
