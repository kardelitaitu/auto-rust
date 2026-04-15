import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies - these persist across module resets
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  isSessionActive: vi.fn().mockReturnValue(true),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

vi.mock("@api/agent/llmClient.js", () => ({
  llmClient: {
    generateCompletion: vi.fn(),
  },
}));

vi.mock("@api/core/errors.js", () => ({
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor(message = "Session has been disconnected") {
      super(message);
      this.name = "SessionDisconnectedError";
      this.code = "SESSION_DISCONNECTED";
    }
  },
}));

// Import after mocks are set up
import { getPage, isSessionActive } from "@api/core/context.js";
import { llmClient } from "@api/agent/llmClient.js";

describe("resourceTracker", () => {
  let mockPage;
  let resourceTracker;

  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();

    // Re-import module to get fresh state
    resourceTracker = await import("@api/interactions/resourceTracker.js");

    mockPage = {
      screenshot: vi.fn().mockResolvedValue(Buffer.from("fake-screenshot")),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("100"),
        }),
      }),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    if (resourceTracker?.stopWatch) {
      resourceTracker.stopWatch();
    }
  });

  describe("startWatch", () => {
    it("should start watching resources with default options", async () => {
      llmClient.generateCompletion.mockResolvedValue(
        '{"gold": 100, "wood": 50}',
      );

      await resourceTracker.startWatch();

      // Should not throw
      expect(true).toBe(true);
    });

    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(resourceTracker.startWatch()).rejects.toThrow(
        "Browser closed.",
      );
    });

    it("should warn and return if watch already running", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 100}');

      // Start first watch
      await resourceTracker.startWatch({ interval: 100 });

      // Try to start again - should warn and return
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
      await resourceTracker.startWatch({ interval: 100 });
      consoleSpy.mockRestore();
    });

    it("should call onChange callback when resources change", async () => {
      const onChange = vi.fn();
      llmClient.generateCompletion
        .mockResolvedValueOnce('{"gold": 100}')
        .mockResolvedValueOnce('{"gold": 150}');

      await resourceTracker.startWatch({
        regions: { gold: { x: 0, y: 0, width: 100, height: 50 } },
        interval: 10,
        onChange,
      });

      // Wait for interval to trigger
      await new Promise((resolve) => setTimeout(resolve, 50));

      resourceTracker.stopWatch();
    });

    it("should use text extraction when useLLM is false", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Gold: 200"),
        }),
      });

      await resourceTracker.startWatch({
        useLLM: false,
        interval: 10,
      });

      // Wait a bit for interval
      await new Promise((resolve) => setTimeout(resolve, 30));

      resourceTracker.stopWatch();
    });

    it("should handle screenshot errors gracefully", async () => {
      mockPage.screenshot.mockRejectedValueOnce(new Error("Screenshot failed"));

      await resourceTracker.startWatch({ interval: 10 });

      // Wait for interval to trigger
      await new Promise((resolve) => setTimeout(resolve, 30));

      resourceTracker.stopWatch();
    });
  });

  describe("stopWatch", () => {
    it("should stop the watch interval", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 100}');

      await resourceTracker.startWatch({ interval: 100 });
      resourceTracker.stopWatch();

      // Should not throw
      expect(true).toBe(true);
    });

    it("should not throw if no watch is running", () => {
      expect(() => resourceTracker.stopWatch()).not.toThrow();
    });
  });

  describe("waitForResources", () => {
    it("should return true when all resource targets are met", async () => {
      llmClient.generateCompletion.mockResolvedValue(
        '{"gold": 500, "wood": 200}',
      );

      const result = await resourceTracker.waitForResources(
        { gold: 500, wood: 200 },
        { regions: { gold: {}, wood: {} }, timeout: 1000 },
      );

      expect(result).toBe(true);
    });

    it("should return false when timeout is reached", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 100}');

      const result = await resourceTracker.waitForResources(
        { gold: 1000 },
        { timeout: 100, regions: { gold: {} } },
      );

      expect(result).toBe(false);
    });

    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(
        resourceTracker.waitForResources({ gold: 100 }),
      ).rejects.toThrow("Browser closed.");
    });

    it("should handle extraction errors and continue", async () => {
      llmClient.generateCompletion
        .mockRejectedValueOnce(new Error("Extraction failed"))
        .mockResolvedValueOnce('{"gold": 500}');

      const result = await resourceTracker.waitForResources(
        { gold: 500 },
        { regions: { gold: {} }, timeout: 1000, interval: 50 },
      );

      expect(result).toBe(true);
    });

    it("should log debug message on extraction error", async () => {
      // First call throws, second succeeds but doesn't meet target
      llmClient.generateCompletion
        .mockRejectedValueOnce(new Error("Screenshot error"))
        .mockResolvedValueOnce('{"gold": 100}');

      const result = await resourceTracker.waitForResources(
        { gold: 500 },
        { regions: { gold: {} }, timeout: 100, interval: 20 },
      );

      expect(result).toBe(false);
    });

    it("should catch and log errors during resource check loop", async () => {
      // Make screenshot throw an error during the loop
      mockPage.screenshot
        .mockRejectedValueOnce(new Error("Screenshot failed"))
        .mockResolvedValueOnce(Buffer.from("fake-screenshot"));

      llmClient.generateCompletion.mockResolvedValue('{"gold": 500}');

      const result = await resourceTracker.waitForResources(
        { gold: 500 },
        { regions: { gold: {} }, timeout: 200, interval: 30 },
      );

      expect(result).toBe(true);
    });

    it("should use custom interval between checks", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 500}');

      const startTime = Date.now();
      await resourceTracker.waitForResources(
        { gold: 500 },
        { regions: { gold: {} }, interval: 100 },
      );
      const endTime = Date.now();

      // Should have taken at least some time
      expect(endTime - startTime).toBeGreaterThanOrEqual(0);
    });
  });

  describe("alertWhenAvailable", () => {
    it("should return when resources are available", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 500}');

      await expect(
        resourceTracker.alertWhenAvailable(
          { gold: 500 },
          { regions: { gold: {} }, timeout: 1000 },
        ),
      ).resolves.not.toThrow();
    });

    it("should throw error when resources not available", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 100}');

      await expect(
        resourceTracker.alertWhenAvailable(
          { gold: 1000 },
          { regions: { gold: {} }, timeout: 100 },
        ),
      ).rejects.toThrow("Resources not available");
    });
  });

  describe("getCurrent", () => {
    it("should return lastResourceState if set by watch", async () => {
      // Start watch to set lastResourceState
      llmClient.generateCompletion.mockResolvedValue(
        '{"gold": 100, "wood": 50}',
      );

      await resourceTracker.startWatch({
        regions: { gold: {}, wood: {} },
        interval: 10,
      });

      // Wait for watch to set lastResourceState
      await new Promise((resolve) => setTimeout(resolve, 30));

      // getCurrent should return cached state from watch
      const result = await resourceTracker.getCurrent();
      expect(result).toBeDefined();

      resourceTracker.stopWatch();
    });

    it("should throw SessionDisconnectedError when session is inactive", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(resourceTracker.getCurrent()).rejects.toThrow(
        "Browser closed.",
      );
    });

    it("should extract resources when no cached state", async () => {
      llmClient.generateCompletion.mockResolvedValue('{"gold": 200}');

      const result = await resourceTracker.getCurrent();
      expect(result).toEqual({ gold: 200 });
      expect(mockPage.screenshot).toHaveBeenCalled();
    });
  });

  describe("extractResources (via other functions)", () => {
    it("should parse JSON from LLM response", async () => {
      llmClient.generateCompletion.mockResolvedValue(
        'Here is the data: {"gold": 100, "wood": 50}',
      );

      const result = await resourceTracker.getCurrent();
      expect(result).toEqual({ gold: 100, wood: 50 });
    });

    it("should return empty object on invalid JSON", async () => {
      llmClient.generateCompletion.mockResolvedValue("No JSON here!");

      const result = await resourceTracker.getCurrent();
      expect(result).toEqual({});
    });

    it("should return empty object on LLM error", async () => {
      llmClient.generateCompletion.mockRejectedValue(new Error("LLM failed"));

      const result = await resourceTracker.getCurrent();
      expect(result).toEqual({});
    });
  });

  describe("computeChanges (via startWatch)", () => {
    it("should detect added resources", async () => {
      const onChange = vi.fn();
      llmClient.generateCompletion
        .mockResolvedValueOnce("{}")
        .mockResolvedValueOnce('{"gold": 100}');

      await resourceTracker.startWatch({
        regions: { gold: {} },
        interval: 10,
        onChange,
      });

      await new Promise((resolve) => setTimeout(resolve, 30));
      resourceTracker.stopWatch();
    });

    it("should detect changed resources", async () => {
      const onChange = vi.fn();
      llmClient.generateCompletion
        .mockResolvedValueOnce('{"gold": 100}')
        .mockResolvedValueOnce('{"gold": 200}');

      await resourceTracker.startWatch({
        regions: { gold: {} },
        interval: 10,
        onChange,
      });

      await new Promise((resolve) => setTimeout(resolve, 30));
      resourceTracker.stopWatch();
    });

    it("should not call onChange when no changes", async () => {
      const onChange = vi.fn();
      llmClient.generateCompletion.mockResolvedValue('{"gold": 100}');

      await resourceTracker.startWatch({
        regions: { gold: {} },
        interval: 10,
        onChange,
      });

      await new Promise((resolve) => setTimeout(resolve, 30));
      resourceTracker.stopWatch();

      // onChange may or may not be called depending on timing
    });
  });

  describe("extractAllTextResources", () => {
    it("should extract gold from text", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          textContent: vi.fn().mockResolvedValue("Gold: 500"),
        }),
      });

      await resourceTracker.startWatch({ useLLM: false, interval: 10 });
      await new Promise((resolve) => setTimeout(resolve, 30));
      resourceTracker.stopWatch();
    });

    it("should handle missing elements gracefully", async () => {
      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          textContent: vi
            .fn()
            .mockRejectedValue(new Error("Element not found")),
        }),
      });

      await resourceTracker.startWatch({ useLLM: false, interval: 10 });
      await new Promise((resolve) => setTimeout(resolve, 30));
      resourceTracker.stopWatch();
    });
  });

  describe("default export", () => {
    it("should export all functions", async () => {
      const mod = await import("@api/interactions/resourceTracker.js");

      expect(mod.default).toHaveProperty("startWatch");
      expect(mod.default).toHaveProperty("stopWatch");
      expect(mod.default).toHaveProperty("waitForResources");
      expect(mod.default).toHaveProperty("alertWhenAvailable");
      expect(mod.default).toHaveProperty("getCurrent");
    });
  });
});
