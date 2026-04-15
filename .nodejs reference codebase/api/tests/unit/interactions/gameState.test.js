import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
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

vi.mock("@api/core/errors.js", () => ({
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor(message = "Browser closed") {
      super(message);
      this.name = "SessionDisconnectedError";
    }
  },
}));

// Import after mocks
import * as gameState from "@api/interactions/gameState.js";
import { getPage, isSessionActive } from "@api/core/context.js";

describe("gameState", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://example.com"),
      locator: vi.fn().mockReturnValue({
        waitFor: vi.fn().mockResolvedValue(undefined),
        isVisible: vi.fn().mockResolvedValue(true),
        isDisabled: vi.fn().mockResolvedValue(false),
        textContent: vi.fn().mockResolvedValue("test text"),
        inputValue: vi.fn().mockResolvedValue("100"),
      }),
      evaluate: vi.fn().mockResolvedValue({ r: 255, g: 0, b: 0 }),
      waitForTimeout: vi.fn(),
      screenshot: vi.fn().mockResolvedValue(Buffer.from("screenshot")),
      accessibility: {
        snapshot: vi.fn().mockResolvedValue({ role: "document" }),
      },
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("session validation", () => {
    it("should throw SessionDisconnectedError when session is inactive for waitForElementState", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForElementState(".element")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForEnabled", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForEnabled(".element")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForText", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForText(".element", "text")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForValueChange", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(
        gameState.waitForValueChange(".element", "old"),
      ).rejects.toThrow("Browser closed");
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForPixelChange", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForPixelChange(100, 100)).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForPopup", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForPopup(".popup")).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for waitForResources", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.waitForResources({ gold: 100 })).rejects.toThrow(
        "Browser closed",
      );
    });

    it("should throw SessionDisconnectedError when session is inactive for getGameState", async () => {
      isSessionActive.mockReturnValue(false);

      await expect(gameState.getGameState()).rejects.toThrow("Browser closed");
    });
  });

  describe("waitForElementState", () => {
    it("should return true when element reaches state", async () => {
      const result = await gameState.waitForElementState(".element");

      expect(result).toBe(true);
    });

    it("should return false on timeout without throwOnTimeout", async () => {
      mockPage.locator.mockReturnValue({
        waitFor: vi.fn().mockRejectedValue(new Error("Timeout")),
      });

      const result = await gameState.waitForElementState(".element", {
        timeout: 100,
      });

      expect(result).toBe(false);
    });

    it("should throw on timeout when throwOnTimeout is true", async () => {
      mockPage.locator.mockReturnValue({
        waitFor: vi.fn().mockRejectedValue(new Error("Timeout")),
      });

      await expect(
        gameState.waitForElementState(".element", {
          timeout: 100,
          throwOnTimeout: true,
        }),
      ).rejects.toThrow("Timeout");
    });

    it("should wait for hidden state", async () => {
      const result = await gameState.waitForElementState(".element", {
        state: "hidden",
      });

      expect(result).toBe(true);
    });
  });

  describe("waitForEnabled", () => {
    it("should return true when element becomes enabled", async () => {
      const result = await gameState.waitForEnabled(".element");

      expect(result).toBe(true);
    });

    it("should return false on timeout", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
        isDisabled: vi.fn().mockResolvedValue(true),
      });

      const result = await gameState.waitForEnabled(".element", true, 100);

      expect(result).toBe(false);
    });

    it("should wait for disabled state", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
        isDisabled: vi.fn().mockResolvedValue(true),
      });

      const result = await gameState.waitForEnabled(".element", false, 100);

      expect(result).toBe(true);
    });

    it("should handle invisible elements", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
        isDisabled: vi.fn().mockResolvedValue(false),
      });

      const result = await gameState.waitForEnabled(".element", true, 100);

      expect(result).toBe(false);
    });
  });

  describe("waitForText", () => {
    it("should return true when text matches", async () => {
      const result = await gameState.waitForText(".element", "test");

      expect(result).toBe(true);
    });

    it("should return false on timeout", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("different"),
      });

      const result = await gameState.waitForText(".element", "not found", 100);

      expect(result).toBe(false);
    });

    it("should handle invisible elements", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
        textContent: vi.fn().mockResolvedValue("test"),
      });

      const result = await gameState.waitForText(".element", "test", 100);

      expect(result).toBe(false);
    });
  });

  describe("waitForValueChange", () => {
    it("should return true when value changes", async () => {
      mockPage.locator.mockReturnValue({
        inputValue: vi.fn().mockResolvedValue("changed"),
      });

      const result = await gameState.waitForValueChange(".element", "initial");

      expect(result).toBe(true);
    });

    it("should return false on timeout", async () => {
      mockPage.locator.mockReturnValue({
        inputValue: vi.fn().mockResolvedValue("initial"),
        textContent: vi.fn().mockResolvedValue("initial"),
      });

      const result = await gameState.waitForValueChange(
        ".element",
        "initial",
        100,
      );

      expect(result).toBe(false);
    });

    it("should fallback to textContent when inputValue fails", async () => {
      mockPage.locator.mockReturnValue({
        inputValue: vi.fn().mockRejectedValue(new Error("Not input")),
        textContent: vi.fn().mockResolvedValue("changed"),
      });

      const result = await gameState.waitForValueChange(".element", "initial");

      expect(result).toBe(true);
    });
  });

  describe("waitForPixelChange", () => {
    it("should return true when pixel changes", async () => {
      mockPage.evaluate
        .mockResolvedValueOnce({ r: 255, g: 0, b: 0 })
        .mockResolvedValueOnce({ r: 0, g: 255, b: 0 });

      const result = await gameState.waitForPixelChange(100, 100, {
        timeout: 200,
      });

      expect(result).toBe(true);
    });

    it("should return false on timeout when pixel does not change", async () => {
      mockPage.evaluate.mockResolvedValue({ r: 255, g: 0, b: 0 });

      const result = await gameState.waitForPixelChange(100, 100, {
        timeout: 100,
      });

      expect(result).toBe(false);
    });

    it("should respect threshold option", async () => {
      mockPage.evaluate
        .mockResolvedValueOnce({ r: 255, g: 0, b: 0 })
        .mockResolvedValueOnce({ r: 250, g: 5, b: 0 });

      const result = await gameState.waitForPixelChange(100, 100, {
        threshold: 100,
        timeout: 200,
      });

      expect(result).toBe(false);
    });
  });

  describe("waitForPopup", () => {
    it("should return true when popup appears", async () => {
      const result = await gameState.waitForPopup(".popup", true);

      expect(result).toBe(true);
    });

    it("should return false when popup does not appear", async () => {
      mockPage.locator.mockReturnValue({
        waitFor: vi.fn().mockRejectedValue(new Error("Not found")),
      });

      const result = await gameState.waitForPopup(".popup", true, 100);

      expect(result).toBe(false);
    });

    it("should return true when popup disappears", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
      });

      const result = await gameState.waitForPopup(".popup", false);

      expect(result).toBe(true);
    });

    it("should return false when popup does not disappear", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
      });

      const result = await gameState.waitForPopup(".popup", false, 100);

      expect(result).toBe(false);
    });
  });

  describe("waitForResources", () => {
    it("should return true when resources are met", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("Gold: 1000"),
      });

      const result = await gameState.waitForResources({ Gold: 500 });

      expect(result).toBe(true);
    });

    it("should return false on timeout", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("Gold: 100"),
      });

      const result = await gameState.waitForResources(
        { Gold: 1000 },
        '[class*="resource"]',
        100,
      );

      expect(result).toBe(false);
    });

    it("should return false when resource element not visible", async () => {
      mockPage.locator.mockReturnValue({
        isVisible: vi.fn().mockResolvedValue(false),
      });

      const result = await gameState.waitForResources(
        { Gold: 100 },
        '[class*="resource"]',
        100,
      );

      expect(result).toBe(false);
    });
  });

  describe("getGameState", () => {
    it("should return game state with screenshot", async () => {
      const result = await gameState.getGameState();

      expect(result).toHaveProperty("url");
      expect(result).toHaveProperty("timestamp");
      expect(result).toHaveProperty("screenshot");
      expect(result).toHaveProperty("axTree");
    });

    it("should return game state without screenshot when disabled", async () => {
      const result = await gameState.getGameState({ screenshot: false });

      expect(result).toHaveProperty("url");
      expect(result).toHaveProperty("timestamp");
      expect(result).not.toHaveProperty("screenshot");
    });

    it("should handle screenshot failure", async () => {
      mockPage.screenshot.mockRejectedValue(new Error("Screenshot failed"));

      const result = await gameState.getGameState();

      expect(result).toHaveProperty("url");
      expect(result).not.toHaveProperty("screenshot");
    });

    it("should handle accessibility tree failure", async () => {
      mockPage.accessibility.snapshot.mockRejectedValue(new Error("AX failed"));

      const result = await gameState.getGameState();

      expect(result).toHaveProperty("url");
      expect(result).not.toHaveProperty("axTree");
    });
  });

  describe("default export", () => {
    it("should export all functions as default object", async () => {
      const mod = await import("@api/interactions/gameState.js");

      expect(mod.default).toHaveProperty("waitForElementState");
      expect(mod.default).toHaveProperty("waitForEnabled");
      expect(mod.default).toHaveProperty("waitForText");
      expect(mod.default).toHaveProperty("waitForValueChange");
      expect(mod.default).toHaveProperty("waitForPixelChange");
      expect(mod.default).toHaveProperty("waitForPopup");
      expect(mod.default).toHaveProperty("waitForResources");
      expect(mod.default).toHaveProperty("getGameState");
    });
  });
});
