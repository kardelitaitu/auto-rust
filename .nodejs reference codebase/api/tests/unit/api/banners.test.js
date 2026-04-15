/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  delay: vi.fn().mockResolvedValue(undefined),
  randomInRange: vi.fn(() => 1000),
}));

import { getPage } from "@api/core/context.js";
import { handleBanners } from "@api/interactions/banners.js";

describe("api/interactions/banners.js", () => {
  let mockPage;
  let mockLocator;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLocator = {
      first: vi.fn(() => mockLocator),
      isVisible: vi.fn(),
      click: vi.fn().mockResolvedValue(undefined),
    };

    mockPage = {
      locator: vi.fn(() => mockLocator),
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("handleBanners", () => {
    it("should return false when no banner is visible", async () => {
      mockLocator.isVisible.mockRejectedValue(new Error("Not found"));

      const result = await handleBanners();

      expect(result).toBe(false);
    });

    it("should return true and click when banner is visible", async () => {
      mockLocator.isVisible.mockResolvedValue(true);

      const result = await handleBanners();

      expect(result).toBe(true);
      expect(mockLocator.click).toHaveBeenCalled();
    });

    it("should respect waitAfter option", async () => {
      mockLocator.isVisible.mockResolvedValue(true);

      await handleBanners({ waitAfter: false });

      expect(mockLocator.click).toHaveBeenCalled();
    });

    it("should handle custom timeout option", async () => {
      mockLocator.isVisible.mockResolvedValue(true);

      const result = await handleBanners({ timeout: 5000 });

      expect(result).toBe(true);
    });

    it("should iterate through multiple selectors", async () => {
      mockLocator.isVisible
        .mockRejectedValueOnce(new Error("Not found"))
        .mockRejectedValueOnce(new Error("Not found"))
        .mockResolvedValue(true);

      const result = await handleBanners();

      expect(result).toBe(true);
    });

    it("should return false when all selectors fail", async () => {
      mockLocator.isVisible.mockRejectedValue(new Error("Not found"));

      const result = await handleBanners();

      expect(result).toBe(false);
    });

    it("should use default timeout when not provided", async () => {
      mockLocator.isVisible.mockResolvedValue(true);

      await handleBanners();

      expect(mockLocator.isVisible).toHaveBeenCalled();
    });

    it("should continue when click rejects and still report success", async () => {
      mockLocator.isVisible.mockResolvedValue(true);
      mockLocator.click.mockRejectedValue(new Error("click failed"));

      const result = await handleBanners();

      expect(result).toBe(true);
    });

    it("should not wait after click when waitAfter is false", async () => {
      mockLocator.isVisible.mockResolvedValue(true);

      const result = await handleBanners({ waitAfter: false });

      expect(result).toBe(true);
      expect(mockLocator.click).toHaveBeenCalledWith({ force: true });
    });

    it("should return false when page.locator throws", async () => {
      mockPage.locator.mockImplementation(() => {
        throw new Error("locator failed");
      });

      const result = await handleBanners();

      expect(result).toBe(false);
    });
  });
});
