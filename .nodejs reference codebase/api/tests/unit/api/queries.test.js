/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  text,
  attr,
  visible,
  count,
  exists,
  currentUrl,
} from "@api/interactions/queries.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  isSessionActive: vi.fn(),
}));

import { getPage, isSessionActive } from "@api/core/context.js";

describe("api/interactions/queries.js", () => {
  let mockPage;
  let mockLocator;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLocator = {
      first: vi.fn().mockReturnThis(),
      innerText: vi.fn().mockResolvedValue("Inner Text"),
      getAttribute: vi.fn().mockResolvedValue("value"),
      isVisible: vi.fn().mockResolvedValue(true),
      count: vi.fn().mockResolvedValue(5),
    };

    mockPage = {
      locator: vi.fn().mockReturnValue(mockLocator),
      url: vi.fn().mockReturnValue("http://example.com"),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  describe("text", () => {
    it("should return innerText", async () => {
      const result = await text("#selector");
      expect(result).toBe("Inner Text");
      expect(mockLocator.innerText).toHaveBeenCalled();
    });

    it("should work with a locator object directly", async () => {
      const locator = {
        first: vi.fn().mockReturnThis(),
        innerText: vi.fn().mockResolvedValue("Locator Text"),
      };

      const result = await text(locator);

      expect(result).toBe("Locator Text");
      expect(locator.first).toHaveBeenCalled();
    });

    it("should return empty string on error", async () => {
      mockLocator.innerText.mockRejectedValue(new Error("Fail"));
      const result = await text("#selector");
      expect(result).toBe("");
    });

    it("should return empty string when no elements found", async () => {
      mockLocator.innerText.mockResolvedValue("");
      const result = await text("#nonexistent");
      expect(result).toBe("");
    });
  });

  describe("attr", () => {
    it("should return attribute value", async () => {
      const result = await attr("#selector", "data-id");
      expect(result).toBe("value");
      expect(mockLocator.getAttribute).toHaveBeenCalledWith("data-id");
    });

    it("should return null on error", async () => {
      mockLocator.getAttribute.mockRejectedValue(new Error("Fail"));
      const result = await attr("#selector", "data-id");
      expect(result).toBeNull();
    });
  });

  describe("visible", () => {
    it("should return visibility", async () => {
      const result = await visible("#selector");
      expect(result).toBe(true);
      expect(mockLocator.isVisible).toHaveBeenCalled();
    });

    it("should return false on error", async () => {
      mockLocator.isVisible.mockRejectedValue(new Error("Fail"));
      const result = await visible("#selector");
      expect(result).toBe(false);
    });

    it("should return false when element not found", async () => {
      mockLocator.isVisible.mockResolvedValue(false);
      const result = await visible("#nonexistent");
      expect(result).toBe(false);
    });
  });

  describe("count", () => {
    it("should return element count", async () => {
      const result = await count("#selector");
      expect(result).toBe(5);
      expect(mockLocator.count).toHaveBeenCalled();
    });

    it("should return 0 on error", async () => {
      mockLocator.count.mockRejectedValue(new Error("Fail"));
      const result = await count("#selector");
      expect(result).toBe(0);
    });

    it("should return 0 when no elements found", async () => {
      mockLocator.count.mockResolvedValue(0);
      const result = await count("#nonexistent");
      expect(result).toBe(0);
      expect(mockLocator.count).toHaveBeenCalled();
    });
  });

  describe("exists", () => {
    it("should return true if count > 0", async () => {
      const result = await exists("#selector");
      expect(result).toBe(true);
    });

    it("should return false if count is 0", async () => {
      mockLocator.count.mockResolvedValue(0);
      const result = await exists("#selector");
      expect(result).toBe(false);
    });

    it("should return false when element not found", async () => {
      mockLocator.count.mockResolvedValue(0);
      const result = await exists("#nonexistent");
      expect(result).toBe(false);
    });

    it("should return false when count throws", async () => {
      mockLocator.count.mockRejectedValue(new Error("Fail"));
      const result = await exists("#selector");
      expect(result).toBe(false);
    });
  });

  describe("currentUrl", () => {
    it("should return current URL", async () => {
      const result = await currentUrl();
      expect(result).toBe("http://example.com");
    });

    it("should return the page url directly when session is active", async () => {
      mockPage.url.mockReturnValueOnce("https://example.org/path");

      const result = await currentUrl();

      expect(result).toBe("https://example.org/path");
    });

    it("should return empty string on error", async () => {
      mockPage.url.mockImplementation(() => {
        throw new Error("Fail");
      });
      const result = await currentUrl();
      expect(result).toBe("");
    });
  });

  describe("Session Safety", () => {
    it("should return default value if session inactive", async () => {
      isSessionActive.mockReturnValue(false);

      expect(await text("#selector")).toBe("");
    });

    it("should handle session inactive correctly for text", async () => {
      isSessionActive.mockReturnValue(false);
      expect(await text("#selector")).toBe("");
    });

    it("should handle session inactive correctly for attr", async () => {
      isSessionActive.mockReturnValue(false);
      expect(await attr("#selector", "id")).toBe(null);
    });

    it("should handle session inactive correctly for visible", async () => {
      isSessionActive.mockReturnValue(false);
      expect(await visible("#selector")).toBe(false);
    });

    it("should handle session inactive correctly for count", async () => {
      isSessionActive.mockReturnValue(false);
      expect(await count("#selector")).toBe(0);
    });

    it("should handle session inactive correctly for currentUrl", async () => {
      isSessionActive.mockReturnValue(false);
      expect(await currentUrl()).toBe("");
    });
  });
});
