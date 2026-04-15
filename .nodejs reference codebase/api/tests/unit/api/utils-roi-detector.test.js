/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { identifyROI } from "@api/utils/roi-detector.js";

describe("api/utils/roi-detector.js", () => {
  let mockPage;

  beforeEach(() => {
    mockPage = {
      isClosed: vi.fn(),
      $: vi.fn(),
      boundingBox: vi.fn(),
      viewportSize: vi.fn(),
    };
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("identifyROI", () => {
    it("should return null if page is closed", async () => {
      mockPage.isClosed.mockReturnValue(true);
      const result = await identifyROI(mockPage);
      expect(result).toBeNull();
    });

    it("should return null if page is null", async () => {
      const result = await identifyROI(null);
      expect(result).toBeNull();
    });

    it("should return null if no matching element found", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockResolvedValue(null);
      mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });

      const result = await identifyROI(mockPage);
      expect(result).toBeNull();
    });

    it("should return null for elements too small", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockResolvedValueOnce({}).mockResolvedValue(null);
      mockPage.boundingBox.mockResolvedValue({
        x: 100,
        y: 100,
        width: 30,
        height: 30,
      });
      mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });

      const result = await identifyROI(mockPage);
      expect(result).toBeNull();
    });

    it("should use the first matching actionable selector", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockImplementation(async (selector) => {
        if (selector === '[role="dialog"]') return null;
        if (selector === '[aria-modal="true"]') return null;
        if (selector === ".modal") return null;
        if (selector === "form") {
          return {
            boundingBox: vi
              .fn()
              .mockResolvedValue({ x: 100, y: 120, width: 200, height: 160 }),
          };
        }
        return null;
      });
      mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });

      const result = await identifyROI(mockPage);

      expect(result).toEqual({
        x: 80,
        y: 100,
        width: 240,
        height: 200,
      });
      expect(mockPage.$).toHaveBeenCalledWith("form");
    });

    it("should fall back to default viewport size when viewport is missing", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockImplementation(async (selector) => {
        if (selector === "main") {
          return {
            boundingBox: vi
              .fn()
              .mockResolvedValue({ x: 10, y: 15, width: 100, height: 120 }),
          };
        }
        return null;
      });
      mockPage.viewportSize.mockReturnValue(null);

      const result = await identifyROI(mockPage);

      expect(result).toEqual({
        x: 0,
        y: 0,
        width: 140,
        height: 160,
      });
    });

    it("should clamp ROI dimensions to viewport bounds", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockImplementation(async (selector) => {
        if (selector === '[role="main"]') {
          return {
            boundingBox: vi
              .fn()
              .mockResolvedValue({ x: 1200, y: 680, width: 200, height: 100 }),
          };
        }
        return null;
      });
      mockPage.viewportSize.mockReturnValue({ width: 1280, height: 720 });

      const result = await identifyROI(mockPage);

      expect(result.width).toBeLessThanOrEqual(100);
      expect(result.height).toBeLessThanOrEqual(60);
    });

    it("should handle errors gracefully", async () => {
      mockPage.isClosed.mockReturnValue(false);
      mockPage.$.mockRejectedValue(new Error("Selector error"));

      const result = await identifyROI(mockPage);
      expect(result).toBeNull();
    });
  });
});
