/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for core/errors-enhanced.js
 * @module tests/unit/errors-enhanced.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  EnhancedAutomationError,
  EnhancedElementNotFoundError,
  EnhancedContextNotInitializedError,
  EnhancedBrowserNotFoundError,
  EnhancedLLMTimeoutError,
  EnhancedNavigationError,
  EnhancedActionFailedError,
  EnhancedConnectionTimeoutError,
  EnhancedRateLimitError,
  enhanceError,
  withEnhancedError,
  AutomationError,
  ValidationError,
} from "@api/core/errors-enhanced.js";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/error-suggestions.js", () => ({
  getSuggestionsForError: vi.fn(),
  formatSuggestions: vi.fn(),
}));

import {
  getSuggestionsForError,
  formatSuggestions,
} from "@api/utils/error-suggestions.js";

describe("core/errors-enhanced", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("EnhancedAutomationError", () => {
    it("should create error with all options", () => {
      const error = new EnhancedAutomationError(
        "TEST_CODE",
        "Test error message",
        {
          metadata: { key: "value" },
          cause: new Error("cause"),
          suggestions: ["fix1", "fix2"],
          docs: "docs/test.html",
          severity: "high",
        },
      );

      expect(error.code).toBe("TEST_CODE");
      expect(error.message).toBe("Test error message");
      expect(error.metadata.key).toBe("value");
      expect(error.cause.message).toBe("cause");
      expect(error.suggestions).toEqual(["fix1", "fix2"]);
      expect(error.docs).toBe("docs/test.html");
      expect(error.severity).toBe("high");
      expect(error.timestamp).toBeDefined();
    });

    it("should use default values when options not provided", () => {
      const error = new EnhancedAutomationError("CODE", "message");

      expect(error.metadata).toEqual({});
      expect(error.cause).toBeNull();
      expect(error.suggestions).toBeNull();
      expect(error.docs).toBeNull();
      expect(error.severity).toBe("medium");
    });

    it("should have correct name", () => {
      const error = new EnhancedAutomationError("CODE", "message");
      expect(error.name).toBe("EnhancedAutomationError");
    });

    it("should generate toString with suggestions", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: ["fix1", "fix2"],
        docs: "docs/test.html",
        metadata: { key: "value" },
      });

      const str = error.toString();
      expect(str).toContain("[CODE] message");
      expect(str).toContain("Suggestions:");
      expect(str).toContain("fix1");
      expect(str).toContain("fix2");
      expect(str).toContain("Documentation:");
    });

    it("should generate toString without suggestions", () => {
      const error = new EnhancedAutomationError("CODE", "message");

      const str = error.toString();
      expect(str).toContain("[CODE] message");
      expect(str).not.toContain("Suggestions:");
    });

    it("should generate toJSON", () => {
      const causeError = new Error("cause");
      const error = new EnhancedAutomationError("CODE", "message", {
        metadata: { key: "value" },
        cause: causeError,
        suggestions: ["fix1"],
        docs: "docs/test.html",
        severity: "high",
      });

      const json = error.toJSON();
      expect(json.code).toBe("CODE");
      expect(json.message).toBe("message");
      expect(json.metadata.key).toBe("value");
      expect(json.suggestions).toEqual(["fix1"]);
      expect(json.docs).toBe("docs/test.html");
      expect(json.severity).toBe("high");
      expect(json.cause.message).toBe("cause");
    });

    it("should getAutoSuggestions from error-suggestions module", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        code: "BROWSER_NOT_FOUND",
      });
      getSuggestionsForError.mockReturnValue({
        suggestions: ["auto1", "auto2"],
      });

      const suggestions = error.getAutoSuggestions();
      expect(getSuggestionsForError).toHaveBeenCalledWith(error);
      expect(suggestions).toEqual(["auto1", "auto2"]);
    });

    it("should return auto suggestions when available", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: ["own1"],
      });
      getSuggestionsForError.mockReturnValue({
        suggestions: ["auto1"],
      });

      const suggestions = error.getAutoSuggestions();
      expect(suggestions).toEqual(["auto1"]);
    });

    it("should fallback to own suggestions when auto returns null", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: ["own1"],
      });
      getSuggestionsForError.mockReturnValue(null);

      const suggestions = error.getAutoSuggestions();
      expect(suggestions).toEqual(["own1"]);
    });

    it("should print to console", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: ["fix1"],
      });
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      error.print();

      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });

    it("should include metadata in toString", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        metadata: { userId: 123 },
      });

      const str = error.toString();
      expect(str).toContain("userId");
      expect(str).toContain("123");
    });
  });

  describe("EnhancedElementNotFoundError", () => {
    it("should create with selector", () => {
      const error = new EnhancedElementNotFoundError(".button");

      expect(error.code).toBe("ELEMENT_NOT_FOUND");
      expect(error.message).toContain(".button");
      expect(error.selector).toBe(".button");
      expect(error.name).toBe("EnhancedElementNotFoundError");
    });

    it("should include selector in metadata", () => {
      const error = new EnhancedElementNotFoundError(".button");
      expect(error.metadata.selector).toBe(".button");
    });

    it("should accept custom suggestions", () => {
      const error = new EnhancedElementNotFoundError(".button", {
        suggestions: ["custom1"],
      });
      expect(error.suggestions).toEqual(["custom1"]);
    });
  });

  describe("EnhancedContextNotInitializedError", () => {
    it("should create with default message", () => {
      const error = new EnhancedContextNotInitializedError();

      expect(error.code).toBe("CONTEXT_NOT_INITIALIZED");
      expect(error.message).toContain("api.withPage");
      expect(error.name).toBe("EnhancedContextNotInitializedError");
      expect(error.severity).toBe("high");
    });
  });

  describe("EnhancedBrowserNotFoundError", () => {
    it("should create with default message", () => {
      const error = new EnhancedBrowserNotFoundError();

      expect(error.code).toBe("BROWSER_NOT_FOUND");
      expect(error.name).toBe("EnhancedBrowserNotFoundError");
      expect(error.severity).toBe("high");
    });
  });

  describe("EnhancedLLMTimeoutError", () => {
    it("should create with custom message", () => {
      const error = new EnhancedLLMTimeoutError("Custom message");

      expect(error.code).toBe("LLM_TIMEOUT");
      expect(error.message).toBe("Custom message");
      expect(error.name).toBe("EnhancedLLMTimeoutError");
    });

    it("should use default message when not provided", () => {
      const error = new EnhancedLLMTimeoutError();

      expect(error.message).toBe("LLM request timeout");
    });
  });

  describe("EnhancedNavigationError", () => {
    it("should create with url and reason", () => {
      const error = new EnhancedNavigationError(
        "https://example.com",
        "timeout",
      );

      expect(error.code).toBe("NAVIGATION_ERROR");
      expect(error.message).toContain("https://example.com");
      expect(error.message).toContain("timeout");
      expect(error.url).toBe("https://example.com");
      expect(error.reason).toBe("timeout");
    });
  });

  describe("EnhancedActionFailedError", () => {
    it("should create with action and reason", () => {
      const error = new EnhancedActionFailedError(
        "click",
        "element not visible",
      );

      expect(error.code).toBe("ACTION_FAILED");
      expect(error.message).toContain("click");
      expect(error.message).toContain("element not visible");
      expect(error.action).toBe("click");
      expect(error.reason).toBe("element not visible");
    });
  });

  describe("EnhancedConnectionTimeoutError", () => {
    it("should create with endpoint and timeout", () => {
      const error = new EnhancedConnectionTimeoutError(
        "ws://localhost:9222",
        30000,
      );

      expect(error.code).toBe("CONNECTION_TIMEOUT");
      expect(error.message).toContain("30000ms");
      expect(error.endpoint).toBe("ws://localhost:9222");
      expect(error.timeout).toBe(30000);
    });
  });

  describe("EnhancedRateLimitError", () => {
    it("should create with custom message", () => {
      const error = new EnhancedRateLimitError("Too many requests", {
        retryAfter: 60,
      });

      expect(error.code).toBe("RATE_LIMIT");
      expect(error.message).toBe("Too many requests");
      expect(error.retryAfter).toBe(60);
    });

    it("should accept retryAfter in options", () => {
      const error = new EnhancedRateLimitError("Rate limited", {
        retryAfter: 120,
      });
      expect(error.retryAfter).toBe(120);
    });
  });

  describe("enhanceError", () => {
    it("should return same error if already enhanced", () => {
      const original = new EnhancedAutomationError("CODE", "message");
      const enhanced = enhanceError(original);

      expect(enhanced).toBe(original);
    });

    it("should create enhanced error with suggestions", () => {
      const original = new Error("Original error");
      original.code = "BROWSER_NOT_FOUND";

      getSuggestionsForError.mockReturnValue({
        suggestions: ["fix1"],
        docs: "docs/test.html",
        severity: "high",
      });

      const enhanced = enhanceError(original);

      expect(enhanced.code).toBe("BROWSER_NOT_FOUND");
      expect(enhanced.message).toBe("Original error");
      expect(enhanced.suggestions).toEqual(["fix1"]);
      expect(enhanced.cause).toBe(original);
    });

    it("should create generic enhanced error when no suggestions", () => {
      const original = new Error("Original error");
      getSuggestionsForError.mockReturnValue(null);

      const enhanced = enhanceError(original);

      expect(enhanced.code).toBe("Error");
      expect(enhanced.message).toBe("Original error");
    });

    it("should use error.name when code not available", () => {
      const original = new Error("Original error");
      original.name = "CustomError";
      getSuggestionsForError.mockReturnValue(null);

      const enhanced = enhanceError(original);

      expect(enhanced.code).toBe("CustomError");
    });

    it("should merge metadata from options", () => {
      const original = new Error("Original error");
      original.code = "TEST";
      getSuggestionsForError.mockReturnValue({
        suggestions: ["fix1"],
        docs: "docs/test.html",
        severity: "high",
      });

      const enhanced = enhanceError(original, {
        metadata: { extra: "value" },
      });

      expect(enhanced.metadata.extra).toBe("value");
    });
  });

  describe("withEnhancedError", () => {
    it("should return result on success", async () => {
      const fn = vi.fn().mockResolvedValue("success");
      const result = await withEnhancedError(fn, "test");

      expect(result).toBe("success");
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should throw enhanced error on failure", async () => {
      const originalError = new Error("Original");
      const fn = vi.fn().mockRejectedValue(originalError);

      await expect(withEnhancedError(fn, "test")).rejects.toThrow(
        EnhancedAutomationError,
      );
    });

    it("should include context in metadata", async () => {
      const originalError = new Error("Original");
      const fn = vi.fn().mockRejectedValue(originalError);

      try {
        await withEnhancedError(fn, "myContext");
      } catch (enhanced) {
        expect(enhanced.metadata.context).toBe("myContext");
      }
    });
  });

  describe("Error inheritance", () => {
    it("EnhancedElementNotFoundError should extend EnhancedAutomationError", () => {
      const error = new EnhancedElementNotFoundError(".btn");
      expect(error).toBeInstanceOf(EnhancedAutomationError);
    });

    it("EnhancedContextNotInitializedError should extend EnhancedAutomationError", () => {
      const error = new EnhancedContextNotInitializedError();
      expect(error).toBeInstanceOf(EnhancedAutomationError);
    });

    it("EnhancedBrowserNotFoundError should extend EnhancedAutomationError", () => {
      const error = new EnhancedBrowserNotFoundError();
      expect(error).toBeInstanceOf(EnhancedAutomationError);
    });
  });

  describe("Edge cases", () => {
    it("should handle empty metadata", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        metadata: {},
      });

      const str = error.toString();
      expect(str).not.toContain("undefined");
    });

    it("should handle null suggestions", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: null,
      });

      const str = error.toString();
      expect(str).not.toContain("null");
    });

    it("should handle missing docs", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        suggestions: ["fix1"],
      });

      const str = error.toString();
      expect(str).not.toContain("undefined");
    });

    it("should handle null cause in toJSON", () => {
      const error = new EnhancedAutomationError("CODE", "message", {
        cause: null,
      });

      const json = error.toJSON();
      expect(json.cause).toBeNull();
    });
  });
});
