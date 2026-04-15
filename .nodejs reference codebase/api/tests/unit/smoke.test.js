/**
 * Auto-AI Framework - Smoke Tests
 * Basic sanity checks to verify test infrastructure works
 * @module tests/unit/smoke
 */

import { describe, it, expect } from "vitest";
import {
  createMockPage,
  createMockLogger,
  wait,
} from "@api/tests/utils/test-helpers.js";

describe("Test Infrastructure Smoke Tests", () => {
  describe("Mock Factories", () => {
    it("should create a mock page with required methods", () => {
      const page = createMockPage();

      expect(page).toBeDefined();
      expect(typeof page.goto).toBe("function");
      expect(typeof page.click).toBe("function");
      expect(typeof page.locator).toBe("function");
      expect(typeof page.screenshot).toBe("function");
      expect(typeof page.evaluate).toBe("function");
    });

    it("should create a mock logger with all log levels", () => {
      const logger = createMockLogger("test");

      expect(logger).toBeDefined();
      expect(typeof logger.info).toBe("function");
      expect(typeof logger.warn).toBe("function");
      expect(typeof logger.error).toBe("function");
      expect(typeof logger.debug).toBe("function");
      expect(typeof logger.success).toBe("function");
    });

    it("should allow overriding mock page methods", () => {
      const customUrl = "https://custom.example.com";
      const page = createMockPage({
        url: () => customUrl,
      });

      expect(page.url()).toBe(customUrl);
    });
  });

  describe("Async Utilities", () => {
    it("should wait for specified duration", async () => {
      const start = Date.now();
      await wait(50);
      const elapsed = Date.now() - start;

      expect(elapsed).toBeGreaterThanOrEqual(40);
    });
  });

  describe("Basic Assertions", () => {
    it("should pass basic equality checks", () => {
      expect(1 + 1).toBe(2);
      expect("hello").toBe("hello");
      expect(true).toBe(true);
    });

    it("should handle object comparisons", () => {
      const obj = { a: 1, b: 2 };
      expect(obj).toEqual({ a: 1, b: 2 });
    });

    it("should handle array operations", () => {
      const arr = [1, 2, 3];
      expect(arr).toHaveLength(3);
      expect(arr).toContain(2);
    });
  });

  describe("Mock Function Behavior", () => {
    it("should track mock function calls", async () => {
      const mockFn = createMockPage().goto;

      await mockFn("https://example.com");

      expect(mockFn).toHaveBeenCalledTimes(1);
      expect(mockFn).toHaveBeenCalledWith("https://example.com");
    });

    it("should allow mock return value configuration", async () => {
      const mockFn = createMockPage().evaluate;
      mockFn.mockResolvedValue({ result: "test" });

      const result = await mockFn();

      expect(result).toEqual({ result: "test" });
    });
  });
});
