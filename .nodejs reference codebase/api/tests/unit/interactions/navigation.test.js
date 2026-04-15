/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/context-state.js", () => ({
  setPreviousUrl: vi.fn(),
  getAutoBanners: vi.fn().mockReturnValue(true),
}));

vi.mock("@api/behaviors/warmup.js", () => ({
  beforeNavigate: vi.fn().mockResolvedValue(undefined),
  randomMouse: vi.fn().mockResolvedValue(undefined),
  fakeRead: vi.fn().mockResolvedValue(undefined),
  pause: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  delay: vi.fn().mockResolvedValue(undefined),
  randomInRange: vi.fn((min, max) => (min + max) / 2),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi.fn().mockReturnValue({ timeouts: { navigation: 30000 } }),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

vi.mock("@api/core/plugins/index.js", () => ({
  getPluginManager: vi.fn().mockReturnValue({
    executeHook: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock("@api/interactions/banners.js", () => ({
  handleBanners: vi.fn().mockResolvedValue(undefined),
}));

describe("api/interactions/navigation.js", () => {
  let navigation;
  let mockPage;

  beforeEach(async () => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://example.com"),
      goto: vi.fn().mockResolvedValue(undefined),
      goBack: vi.fn().mockResolvedValue(undefined),
      goForward: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
      setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
      waitForLoadState: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn().mockResolvedValue(undefined),
      mouse: {
        wheel: vi.fn().mockResolvedValue(undefined),
      },
      bringToFront: vi.fn().mockResolvedValue(undefined),
    };

    const { getPage } = await import("@api/core/context.js");
    getPage.mockReturnValue(mockPage);

    navigation = await import("@api/interactions/navigation.js");
  });

  describe("goto()", () => {
    it("should navigate to URL", async () => {
      await navigation.goto("https://test.com");
      expect(mockPage.goto).toHaveBeenCalledWith(
        "https://test.com",
        expect.any(Object),
      );
    });

    it("should accept options", async () => {
      await navigation.goto("https://test.com", {
        waitUntil: "networkidle",
        timeout: 5000,
      });
      expect(mockPage.goto).toHaveBeenCalled();
    });

    it("should handle navigation errors", async () => {
      mockPage.goto.mockRejectedValue(new Error("Navigation failed"));
      await expect(navigation.goto("https://invalid")).rejects.toThrow();
    });
  });

  describe("back()", () => {
    it("should navigate back", async () => {
      await navigation.back();
      expect(mockPage.goBack).toHaveBeenCalled();
    });
  });

  describe("forward()", () => {
    it("should navigate forward", async () => {
      await navigation.forward();
      expect(mockPage.goForward).toHaveBeenCalled();
    });
  });

  describe("reload()", () => {
    it("should reload the page", async () => {
      await navigation.reload();
      expect(mockPage.reload).toHaveBeenCalled();
    });
  });

  describe("setExtraHTTPHeaders()", () => {
    it("should set HTTP headers", async () => {
      await navigation.setExtraHTTPHeaders({ "X-Custom": "value" });
      expect(mockPage.setExtraHTTPHeaders).toHaveBeenCalledWith({
        "X-Custom": "value",
      });
    });
  });

  describe("exported warmup functions", () => {
    it("should export beforeNavigate", () => {
      expect(typeof navigation.beforeNavigate).toBe("function");
    });

    it("should export randomMouse", () => {
      expect(typeof navigation.randomMouse).toBe("function");
    });

    it("should export fakeRead", () => {
      expect(typeof navigation.fakeRead).toBe("function");
    });

    it("should export pause", () => {
      expect(typeof navigation.pause).toBe("function");
    });
  });
});
