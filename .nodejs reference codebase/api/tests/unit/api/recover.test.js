/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  recover,
  urlChanged,
  goBack,
  findElement,
  smartClick,
  undo,
} from "@api/behaviors/recover.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/behaviors/timing.js", () => ({
  think: vi.fn().mockResolvedValue(),
  delay: vi.fn().mockResolvedValue(),
  randomInRange: vi.fn((min, max) => min),
  gaussian: vi.fn((mean) => mean),
}));

vi.mock("@api/interactions/scroll.js", () => ({
  scroll: vi.fn().mockResolvedValue(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

import { getPage } from "@api/core/context.js";
import { scroll } from "@api/interactions/scroll.js";

describe("api/behaviors/recover.js", () => {
  let mockPage;
  let mockLocator;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLocator = {
      first: vi.fn().mockReturnThis(),
      isVisible: vi.fn().mockResolvedValue(true),
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 0, y: 0, width: 100, height: 100 }),
      click: vi.fn().mockResolvedValue(),
      count: vi.fn().mockResolvedValue(1),
    };

    mockPage = {
      url: vi.fn().mockReturnValue("http://example.com"),
      goBack: vi.fn().mockResolvedValue(),
      locator: vi.fn().mockReturnValue(mockLocator),
      mouse: {
        click: vi.fn().mockResolvedValue(),
      },
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("recover", () => {
    it("should return false (placeholder)", async () => {
      expect(await recover()).toBe(false);
    });
  });

  describe("urlChanged", () => {
    it("should return true if url changed", async () => {
      mockPage.url.mockReturnValue("http://new.com");
      expect(await urlChanged("http://old.com")).toBe(true);
    });

    it("should return false if url same", async () => {
      mockPage.url.mockReturnValue("http://same.com");
      expect(await urlChanged("http://same.com")).toBe(false);
    });
  });

  describe("goBack", () => {
    it("should navigate back", async () => {
      await goBack();
      expect(mockPage.goBack).toHaveBeenCalled();
    });
  });

  describe("findElement", () => {
    it("should return selector if element found", async () => {
      const result = await findElement("#target");
      expect(result).toBe("#target");
      expect(scroll).not.toHaveBeenCalled();
    });

    it("should scroll and retry if not found", async () => {
      mockLocator.count
        .mockResolvedValueOnce(0) // Not found first
        .mockResolvedValueOnce(1); // Found second

      const result = await findElement("#target");
      expect(result).toBe("#target");
      expect(scroll).toHaveBeenCalled();
    });

    it("should return null after max retries", async () => {
      mockLocator.count.mockResolvedValue(0);

      const result = await findElement("#target", { maxRetries: 2 });
      expect(result).toBe(null);
      expect(scroll).toHaveBeenCalled(); // Called between attempts
    });
  });

  describe("smartClick", () => {
    it("should click element using coordinates", async () => {
      const result = await smartClick("#target");

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).toHaveBeenCalled(); // Gaussian click
      expect(mockLocator.click).not.toHaveBeenCalled();
    });

    it("should fallback to standard click if no bounding box", async () => {
      mockLocator.boundingBox.mockResolvedValue(null);

      const result = await smartClick("#target");

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).not.toHaveBeenCalled();
      expect(mockLocator.click).toHaveBeenCalled();
    });

    it("should detect unexpected navigation and recover", async () => {
      mockPage.url
        .mockReturnValueOnce("http://initial.com") // previousUrl
        .mockReturnValue("http://unexpected.com"); // currentUrl

      const result = await smartClick("#target", { recovery: true });

      expect(result.success).toBe(false);
      expect(result.recovered).toBe(true);
      expect(mockPage.goBack).toHaveBeenCalled();
    });

    it("should retry if click fails completely", async () => {
      // Both mouse click and fallback locator click fail
      mockPage.mouse.click.mockRejectedValue(new Error("Mouse Click failed"));
      mockLocator.click.mockRejectedValue(new Error("Locator Click failed"));

      // smartClick catches error in outer loop and retries

      // We want it to succeed eventually? Or fail?
      // If we want it to retry, we need it to fail first attempt.
      // But if it fails consistently, it will retry maxRetries times and return false.

      // Let's make it succeed on second attempt.
      // Attempt 1: mouse fail, locator fail -> outer catch -> retry
      // Attempt 2: mouse succeed

      mockPage.mouse.click
        .mockRejectedValueOnce(new Error("Mouse Click failed"))
        .mockResolvedValueOnce();

      mockLocator.click.mockRejectedValueOnce(
        new Error("Locator Click failed"),
      );

      const result = await smartClick("#target", { maxRetries: 2 });

      expect(result.success).toBe(true);
      expect(mockPage.mouse.click).toHaveBeenCalledTimes(2);
    });

    it("should scroll if element not visible", async () => {
      mockLocator.isVisible
        .mockResolvedValueOnce(false)
        .mockResolvedValueOnce(true);

      const result = await smartClick("#target");

      expect(result.success).toBe(true);
      expect(scroll).toHaveBeenCalled();
    });

    it("should return false if element never becomes visible", async () => {
      mockLocator.isVisible.mockResolvedValue(false);
      const result = await smartClick("#target", { maxRetries: 2 });
      expect(result.success).toBe(false);
      expect(scroll).toHaveBeenCalledTimes(1);
    });

    it("should fallback to standard click if coordinate click throws", async () => {
      mockPage.mouse.click.mockRejectedValueOnce(new Error("Coord fail"));
      const result = await smartClick("#target");
      expect(result.success).toBe(true);
      expect(mockLocator.click).toHaveBeenCalled();
    });
  });

  describe("undo", () => {
    it("should call goBack", async () => {
      await undo();
      expect(mockPage.goBack).toHaveBeenCalled();
    });
  });
});
