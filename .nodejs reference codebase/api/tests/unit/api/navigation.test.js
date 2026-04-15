/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  goto,
  reload,
  back,
  forward,
  setExtraHTTPHeaders,
} from "@api/interactions/navigation.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/context-state.js", () => ({
  getContextState: vi.fn(),
  setPreviousUrl: vi.fn(),
  getAutoBanners: vi.fn().mockReturnValue(true),
}));

vi.mock("@api/behaviors/warmup.js", () => ({
  beforeNavigate: vi.fn().mockResolvedValue(),
  randomMouse: vi.fn(),
  fakeRead: vi.fn(),
  pause: vi.fn(),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  delay: vi.fn().mockResolvedValue(),
  randomInRange: vi.fn().mockReturnValue(100),
}));

vi.mock("@api/behaviors/persona.js", () => ({
  getPersona: vi
    .fn()
    .mockReturnValue({ scrollDelay: 100, scrollMin: 100, scrollMax: 300 }),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

import { getPage } from "@api/core/context.js";
import { beforeNavigate } from "@api/behaviors/warmup.js";

describe("api/interactions/navigation.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://example.com"),
      goto: vi.fn().mockResolvedValue(),
      waitForSelector: vi.fn().mockResolvedValue(),
      reload: vi.fn().mockResolvedValue(),
      goBack: vi.fn().mockResolvedValue({}),
      goForward: vi.fn().mockResolvedValue(),
      setExtraHTTPHeaders: vi.fn().mockResolvedValue(),
      mouse: {
        wheel: vi.fn().mockResolvedValue(),
      },
      evaluate: vi.fn().mockResolvedValue(),
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("goto", () => {
    it("should navigate to url with default options", async () => {
      await goto("http://example.com");

      expect(mockPage.goto).toHaveBeenCalledWith("http://example.com", {
        waitUntil: "domcontentloaded",
        timeout: 30000,
      });
      expect(beforeNavigate).toHaveBeenCalled();
    });

    it("should disable warmup if requested", async () => {
      await goto("http://example.com", { warmup: false });

      expect(beforeNavigate).not.toHaveBeenCalled();
    });

    it("should resolve on selector", async () => {
      mockPage.goto.mockImplementation(
        () => new Promise((r) => setTimeout(r, 100)),
      );
      mockPage.waitForSelector.mockResolvedValue(true);

      await goto("http://example.com", { resolveOnSelector: "#target" });

      expect(mockPage.waitForSelector).toHaveBeenCalledWith(
        "#target",
        expect.any(Object),
      );
    });

    it("should perform post-navigation scroll", async () => {
      // Mock random to ensure wheel is called (Math.random > 0.3)
      const originalRandom = Math.random;
      Math.random = () => 0.5;

      await goto("http://example.com");

      expect(mockPage.mouse.wheel).toHaveBeenCalled();

      Math.random = originalRandom;
    });

    it("should fallback to window.scrollBy on wheel error", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0.5;

      mockPage.mouse.wheel.mockRejectedValue(new Error("Wheel error"));

      await goto("http://example.com");

      expect(mockPage.evaluate).toHaveBeenCalled();

      Math.random = originalRandom;
    });

    it("should skip wheel if random is low", async () => {
      const originalRandom = Math.random;
      Math.random = () => 0.1; // <= 0.3

      await goto("http://example.com");

      expect(mockPage.mouse.wheel).not.toHaveBeenCalled();

      Math.random = originalRandom;
    });
  });

  describe("reload", () => {
    it("should reload the page", async () => {
      await reload();
      expect(mockPage.reload).toHaveBeenCalled();
    });
  });

  describe("back", () => {
    it("should go back and return true on success", async () => {
      const result = await back();
      expect(mockPage.goBack).toHaveBeenCalled();
      expect(result).toBe(true);
    });

    it("should return false if goBack fails/returns null", async () => {
      mockPage.goBack.mockRejectedValue(new Error("No history"));
      const result = await back();
      expect(result).toBe(false);
    });
  });

  describe("forward", () => {
    it("should go forward", async () => {
      await forward();
      expect(mockPage.goForward).toHaveBeenCalled();
    });
  });

  describe("setExtraHTTPHeaders", () => {
    it("should set headers", async () => {
      const headers = { "X-Test": "1" };
      await setExtraHTTPHeaders(headers);
      expect(mockPage.setExtraHTTPHeaders).toHaveBeenCalledWith(headers);
    });
  });
});
