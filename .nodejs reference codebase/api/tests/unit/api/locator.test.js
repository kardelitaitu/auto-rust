/**
 * Auto-AI Framework - Locator Utils Tests
 * @module tests/unit/api/locator.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

const mockLocator = {
  click: vi.fn(),
  fill: vi.fn(),
  innerText: vi.fn(),
};

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => ({
    locator: vi.fn(() => mockLocator),
  })),
}));

let getLocator;
let stringify;

describe("api/utils/locator.js", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("../../../utils/locator.js");
    getLocator = module.getLocator;
    stringify = module.stringify;
  });

  describe("getLocator", () => {
    it("should return page.locator for string selector", () => {
      const result = getLocator(".my-selector");
      expect(result).toBeDefined();
    });

    it("should return the same locator if passed a locator", () => {
      const result = getLocator(mockLocator);
      expect(result).toBe(mockLocator);
    });

    it("should throw for null selector", () => {
      expect(() => getLocator(null)).toThrow("Selector or Locator is required");
    });

    it("should throw for undefined selector", () => {
      expect(() => getLocator(undefined)).toThrow(
        "Selector or Locator is required",
      );
    });

    it("should throw for empty string selector", () => {
      expect(() => getLocator("")).toThrow("Selector or Locator is required");
    });

    it("should handle CSS ID selector", () => {
      const result = getLocator("#id");
      expect(result).toBeDefined();
    });

    it("should handle CSS class selector", () => {
      const result = getLocator(".class");
      expect(result).toBeDefined();
    });

    it("should handle attribute selector", () => {
      const result = getLocator('[data-testid="test"]');
      expect(result).toBeDefined();
    });
  });

  describe("stringify", () => {
    it("should return string as-is", () => {
      expect(stringify(".my-selector")).toBe(".my-selector");
    });

    it("should call toString on locator", () => {
      const result = stringify(mockLocator);
      expect(result).toBeDefined();
    });
  });
});
