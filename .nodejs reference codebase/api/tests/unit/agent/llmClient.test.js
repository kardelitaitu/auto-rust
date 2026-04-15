import { describe, it, expect, vi, beforeEach } from "vitest";
import { LLMClient } from "@api/agent/llmClient.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/core/config.js", () => ({
  configManager: {
    init: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockReturnValue({
      baseUrl: "http://localhost:11434",
      model: "llama2",
      serverType: "ollama",
      temperature: 0.7,
      contextLength: 2048,
      maxTokens: 512,
      timeoutMs: 30000,
      useVision: false,
      bypassHealthCheck: false,
    }),
    _getDefaults: vi.fn().mockReturnValue({
      agent: {
        llm: {
          baseUrl: "http://localhost:11434",
          model: "llama2",
          serverType: "ollama",
          temperature: 0.7,
          contextLength: 2048,
          maxTokens: 512,
          timeoutMs: 30000,
        },
      },
    }),
  },
}));

global.fetch = vi.fn();

describe("llmClient.js", () => {
  let client;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new LLMClient();
    client.config = null;
  });

  describe("constructor", () => {
    it("should initialize with default values", () => {
      expect(client.config).toBeNull();
      expect(client.isRestarting).toBe(false);
      expect(client.restartPromise).toBeNull();
    });
  });

  describe("init()", () => {
    it("should initialize config from configManager", async () => {
      await client.init();
      expect(client.config).not.toBeNull();
      expect(client.config.baseUrl).toBe("http://localhost:11434");
    });

    it("should not re-initialize if config already exists", async () => {
      client.config = { model: "custom" };
      const { configManager } = await import("@api/core/config.js");
      await client.init();
      expect(configManager.init).not.toHaveBeenCalled();
    });

    it("should use defaults if configManager.init fails", async () => {
      const { configManager } = await import("@api/core/config.js");
      configManager.init.mockRejectedValueOnce(new Error("Config error"));
      client.config = null;
      await client.init();
      expect(client.config).not.toBeNull();
    });
  });

  describe("_convertToOllamaFormat()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should pass through non-array content unchanged", () => {
      const messages = [{ role: "user", content: "Hello" }];
      const result = client._convertToOllamaFormat(messages);
      expect(result).toEqual([{ role: "user", content: "Hello" }]);
    });

    it("should convert array content with text parts", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Hello" },
            { type: "text", text: "World" },
          ],
        },
      ];
      const result = client._convertToOllamaFormat(messages);
      expect(result[0].content).toBe("Hello\nWorld");
    });

    it("should convert array content with image_url parts", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Describe this" },
            {
              type: "image_url",
              image_url: { url: "data:image/png;base64,aGVsbG8=" },
            },
          ],
        },
      ];
      const result = client._convertToOllamaFormat(messages);
      expect(result[0].content).toBe("Describe this");
      expect(result[0].images).toEqual(["aGVsbG8="]);
    });

    it("should handle image_url as string", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "image_url", image_url: "data:image/png;base64,dGVzdA==" },
          ],
        },
      ];
      const result = client._convertToOllamaFormat(messages);
      expect(result[0].images).toEqual(["dGVzdA=="]);
    });

    it("should not include images array if no valid images", () => {
      const messages = [
        {
          role: "user",
          content: [
            { type: "text", text: "Hello" },
            { type: "image_url", image_url: { url: "invalid-url" } },
          ],
        },
      ];
      const result = client._convertToOllamaFormat(messages);
      expect(result[0].images).toBeUndefined();
    });

    it("should handle empty content array", () => {
      const messages = [{ role: "user", content: [] }];
      const result = client._convertToOllamaFormat(messages);
      expect(result[0].content).toBe("");
      expect(result[0].images).toBeUndefined();
    });
  });

  describe("checkAvailability()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should return true when health check succeeds", async () => {
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ data: [{ id: "llama2" }] }),
      });
      const result = await client.checkAvailability();
      expect(result).toBe(true);
    });

    it("should return false when health check returns non-ok", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Internal Server Error"),
      });
      const result = await client.checkAvailability();
      expect(result).toBe(false);
    });

    it("should return true when fetch fails but bypassHealthCheck is true", async () => {
      client.config.bypassHealthCheck = true;
      fetch.mockRejectedValueOnce(new Error("Connection refused"));
      const result = await client.checkAvailability();
      expect(result).toBe(true);
    });

    it("should return false when fetch fails and bypassHealthCheck is false", async () => {
      client.config.bypassHealthCheck = false;
      fetch.mockRejectedValueOnce(new Error("Connection refused"));
      const result = await client.checkAvailability();
      expect(result).toBe(false);
    });
  });

  describe("ensureModelRunning()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should not throw when model is running", async () => {
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ data: [] }),
      });
      await expect(client.ensureModelRunning()).resolves.not.toThrow();
    });

    it("should not throw when model is not running", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Error"),
      });
      await expect(client.ensureModelRunning()).resolves.not.toThrow();
    });
  });

  describe("generateCompletion()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should generate completion for ollama server type", async () => {
      client.config.serverType = "ollama";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            message: { content: '{"result": "test"}' },
            usage: {
              prompt_tokens: 10,
              completion_tokens: 5,
              total_tokens: 15,
            },
          }),
      });
      const result = await client.generateCompletion([
        { role: "user", content: "test" },
      ]);
      expect(result.result).toBe("test");
    });

    it("should generate completion for openai server type", async () => {
      client.config.serverType = "openai";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [{ message: { content: '{"response": "ok"}' } }],
            usage: {
              prompt_tokens: 20,
              completion_tokens: 10,
              total_tokens: 30,
            },
          }),
      });
      const result = await client.generateCompletion([
        { role: "user", content: "test" },
      ]);
      expect(result.response).toBe("ok");
    });

    it("should throw on HTTP error", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Server Error"),
      });
      await expect(
        client.generateCompletion([{ role: "user", content: "test" }]),
      ).rejects.toThrow("HTTP Error 500");
    });

    it("should throw on unexpected response format", async () => {
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ unknown: "format" }),
      });
      await expect(
        client.generateCompletion([{ role: "user", content: "test" }]),
      ).rejects.toThrow("Unexpected API response format");
    });

    it("should handle JSON parse failure with robust extraction", async () => {
      client.config.serverType = "openai";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [
              { message: { content: 'Some text {"key": "value"} more text' } },
            ],
          }),
      });
      const result = await client.generateCompletion([
        { role: "user", content: "test" },
      ]);
      expect(result.key).toBe("value");
    });

    it("should wait if restarting", async () => {
      client.isRestarting = true;
      client.restartPromise = Promise.resolve();
      client.config.serverType = "openai";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [{ message: { content: '{"done": true}' } }],
          }),
      });
      const result = await client.generateCompletion([
        { role: "user", content: "test" },
      ]);
      expect(result.done).toBe(true);
    });
  });

  describe("generateCompletionWithRetry()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should succeed on first attempt", async () => {
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [{ message: { content: '{"success": true}' } }],
          }),
      });
      const result = await client.generateCompletionWithRetry([
        { role: "user", content: "test" },
      ]);
      expect(result.success).toBe(true);
    });

    it("should retry on transient failure", async () => {
      fetch.mockRejectedValueOnce(new Error("Network error"));
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [{ message: { content: '{"success": true}' } }],
          }),
      });
      const result = await client.generateCompletionWithRetry(
        [{ role: "user", content: "test" }],
        2,
      );
      expect(result.success).toBe(true);
    });

    it("should throw on 400 error without retry", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        text: () => Promise.resolve("Bad Request"),
      });
      await expect(
        client.generateCompletionWithRetry(
          [{ role: "user", content: "test" }],
          3,
        ),
      ).rejects.toThrow("HTTP Error 400");
    });

    it("should throw on 401 error without retry", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        text: () => Promise.resolve("Unauthorized"),
      });
      await expect(
        client.generateCompletionWithRetry(
          [{ role: "user", content: "test" }],
          3,
        ),
      ).rejects.toThrow("HTTP Error 401");
    });

    it("should throw on 403 error without retry", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 403,
        text: () => Promise.resolve("Forbidden"),
      });
      await expect(
        client.generateCompletionWithRetry(
          [{ role: "user", content: "test" }],
          3,
        ),
      ).rejects.toThrow("HTTP Error 403");
    });

    it("should exhaust retries and throw last error", async () => {
      fetch.mockRejectedValue(new Error("Persistent error"));
      await expect(
        client.generateCompletionWithRetry(
          [{ role: "user", content: "test" }],
          2,
        ),
      ).rejects.toThrow("Persistent error");
    });
  });

  describe("generateCompletionStructured()", () => {
    beforeEach(async () => {
      await client.init();
    });

    it("should generate structured completion for ollama", async () => {
      client.config.serverType = "ollama";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            message: { content: '{"action": "click", "selector": "#btn"}' },
          }),
      });
      const result = await client.generateCompletionStructured([
        { role: "user", content: "test" },
      ]);
      expect(result.action).toBe("click");
    });

    it("should generate structured completion for openai", async () => {
      client.config.serverType = "openai";
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            choices: [
              { message: { content: '{"action": "wait", "value": "1000"}' } },
            ],
          }),
      });
      const result = await client.generateCompletionStructured([
        { role: "user", content: "test" },
      ]);
      expect(result.action).toBe("wait");
    });

    it("should throw on HTTP error", async () => {
      fetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Server Error"),
      });
      await expect(
        client.generateCompletionStructured([
          { role: "user", content: "test" },
        ]),
      ).rejects.toThrow("HTTP Error 500");
    });

    it("should throw on unexpected response format", async () => {
      fetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ unknown: "format" }),
      });
      await expect(
        client.generateCompletionStructured([
          { role: "user", content: "test" },
        ]),
      ).rejects.toThrow("Unexpected API response format");
    });
  });

  describe("getUsageStats()", () => {
    it("should return usage stats", async () => {
      await client.init();
      client.isRestarting = false;
      const stats = client.getUsageStats();
      expect(stats).toHaveProperty("model");
      expect(stats).toHaveProperty("baseUrl");
      expect(stats).toHaveProperty("useVision");
      expect(stats).toHaveProperty("isRestarting");
      expect(stats.isRestarting).toBe(false);
    });

    it("should return stats with null config", () => {
      client.config = null;
      const stats = client.getUsageStats();
      expect(stats.model).toBeUndefined();
    });
  });
});
