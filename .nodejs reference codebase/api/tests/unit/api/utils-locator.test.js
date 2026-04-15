/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { getLocator, stringify } from "@api/utils/locator.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

import { getPage } from "@api/core/context.js";

describe("api/utils/locator.js", () => {
  let mockPage;
  let mockLocator;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLocator = {
      click: vi.fn(),
      waitFor: vi.fn(),
      isVisible: vi.fn(),
    };

    mockPage = {
      locator: vi.fn().mockReturnValue(mockLocator),
    };

    getPage.mockReturnValue(mockPage);
  });

  describe("getLocator", () => {
    it("should return locator when given a string selector", () => {
      const result = getLocator(".my-selector");
      expect(mockPage.locator).toHaveBeenCalledWith(".my-selector");
      expect(result).toBe(mockLocator);
    });

    it("should return same locator when given a Locator instance", () => {
      const result = getLocator(mockLocator);
      expect(mockPage.locator).not.toHaveBeenCalled();
      expect(result).toBe(mockLocator);
    });

    it("should throw error for null input", () => {
      expect(() => getLocator(null)).toThrow("Selector or Locator is required");
    });

    it("should throw error for undefined input", () => {
      expect(() => getLocator(undefined)).toThrow(
        "Selector or Locator is required",
      );
    });

    it("should throw error for empty string", () => {
      expect(() => getLocator("")).toThrow("Selector or Locator is required");
    });

    it("should work with different selector types", () => {
      const cssSelector = getLocator("#id");
      expect(cssSelector).toBe(mockLocator);

      const classSelector = getLocator(".class");
      expect(classSelector).toBe(mockLocator);

      const attributeSelector = getLocator('[data-testid="test"]');
      expect(attributeSelector).toBe(mockLocator);
    });
  });

  describe("stringify", () => {
    it("should return string as-is for string input", () => {
      const result = stringify(".my-selector");
      expect(result).toBe(".my-selector");
    });

    it("should return toString for Locator input", () => {
      const mockLocatorWithToString = {
        toString: vi.fn().mockReturnValue("[Locator]"),
      };
      const result = stringify(mockLocatorWithToString);
      expect(result).toBe("[Locator]");
    });

    it("should return [Locator] placeholder when toString returns falsy", () => {
      const mockLocatorNoToString = {
        toString: vi.fn().mockReturnValue(null),
      };
      const result = stringify(mockLocatorNoToString);
      expect(result).toBe("[Locator]");
    });

    it("should handle locator with no custom toString", () => {
      const result = stringify({});
      expect(result).not.toBe("");
    });
  });
});
