/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/error-suggestions.js
 * @module tests/unit/error-suggestions.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import errorSuggestions from "@api/utils/error-suggestions.js";

const {
  getSuggestions,
  getSuggestionsForError,
  formatSuggestions,
  addSuggestion,
  getKnownErrorCodes,
  ERROR_SUGGESTIONS,
} = errorSuggestions;

describe("utils/error-suggestions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    delete ERROR_SUGGESTIONS["CUSTOM_TEST_CODE"];
  });

  describe("getSuggestions", () => {
    it("should return suggestion for known error code", () => {
      const suggestion = getSuggestions("BROWSER_NOT_FOUND");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("No browsers discovered");
      expect(suggestion.suggestions).toBeInstanceOf(Array);
      expect(suggestion.suggestions.length).toBeGreaterThan(0);
      expect(suggestion.docs).toBeDefined();
      expect(suggestion.severity).toBe("high");
    });

    it("should return suggestion for CONNECTION_TIMEOUT", () => {
      const suggestion = getSuggestions("CONNECTION_TIMEOUT");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Connection timeout");
      expect(suggestion.severity).toBe("high");
    });

    it("should return suggestion for SESSION_DISCONNECTED", () => {
      const suggestion = getSuggestions("SESSION_DISCONNECTED");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Session disconnected");
    });

    it("should return suggestion for ELEMENT_NOT_FOUND", () => {
      const suggestion = getSuggestions("ELEMENT_NOT_FOUND");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Element not found");
      expect(suggestion.severity).toBe("medium");
    });

    it("should return suggestion for LLM_TIMEOUT", () => {
      const suggestion = getSuggestions("LLM_TIMEOUT");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("LLM request timeout");
      expect(suggestion.severity).toBe("high");
    });

    it("should return suggestion for LLM_CIRCUIT_OPEN", () => {
      const suggestion = getSuggestions("LLM_CIRCUIT_OPEN");

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Circuit breaker open");
    });

    it("should return suggestion for HTTP errors", () => {
      const suggestion = getSuggestions("HTTP_401");
      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Unauthorized");
      expect(suggestion.severity).toBe("high");

      const suggestion403 = getSuggestions("HTTP_403");
      expect(suggestion403.message).toBe("Forbidden");

      const suggestion404 = getSuggestions("HTTP_404");
      expect(suggestion404.message).toBe("Not found");

      const suggestion429 = getSuggestions("HTTP_429");
      expect(suggestion429.message).toBe("Too many requests");

      const suggestion500 = getSuggestions("HTTP_500");
      expect(suggestion500.message).toBe("Internal server error");

      const suggestion503 = getSuggestions("HTTP_503");
      expect(suggestion503.message).toBe("Service unavailable");
    });

    it("should return null for unknown error code", () => {
      const suggestion = getSuggestions("UNKNOWN_ERROR_CODE");
      expect(suggestion).toBeNull();
    });

    it("should return null for null input", () => {
      const suggestion = getSuggestions(null);
      expect(suggestion).toBeNull();
    });

    it("should return null for undefined input", () => {
      const suggestion = getSuggestions(undefined);
      expect(suggestion).toBeNull();
    });
  });

  describe("getSuggestionsForError", () => {
    it("should get suggestions from error with code property", () => {
      const error = { code: "BROWSER_NOT_FOUND", message: "test" };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("No browsers discovered");
    });

    it("should get suggestions from error with name property", () => {
      const error = { name: "ELEMENT_NOT_FOUND", message: "test" };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Element not found");
    });

    it("should prioritize code over name", () => {
      const error = {
        code: "BROWSER_NOT_FOUND",
        name: "ELEMENT_NOT_FOUND",
        message: "test",
      };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion.message).toBe("No browsers discovered");
    });

    it("should get suggestions from HTTP status when code exists", () => {
      const error = { code: "HTTP_404", status: 404, message: "Not found" };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Not found");
    });

    it("should return null when no code, name, or status", () => {
      const error = { message: "test error" };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).toBeNull();
    });

    it("should return null for null error", () => {
      const suggestion = getSuggestionsForError(null);
      expect(suggestion).toBeNull();
    });

    it("should return null for undefined error", () => {
      const suggestion = getSuggestionsForError(undefined);
      expect(suggestion).toBeNull();
    });

    it("should handle error with empty code", () => {
      const error = { code: "", name: "BROWSER_NOT_FOUND" };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).not.toBeNull();
    });

    it("should handle error object with additional properties", () => {
      const error = {
        code: "LLM_TIMEOUT",
        message: "timeout",
        details: { model: "llama2" },
      };
      const suggestion = getSuggestionsForError(error);

      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("LLM request timeout");
    });
  });

  describe("formatSuggestions", () => {
    it("should format suggestion with all fields", () => {
      const suggestion = {
        message: "Test error",
        suggestions: ["fix1", "fix2", "fix3"],
        docs: "docs/test.html",
        severity: "high",
      };

      const formatted = formatSuggestions(suggestion);

      expect(formatted).toContain("Issue: Test error");
      expect(formatted).toContain("Suggestions:");
      expect(formatted).toContain("1. fix1");
      expect(formatted).toContain("2. fix2");
      expect(formatted).toContain("3. fix3");
      expect(formatted).toContain("Documentation: docs/test.html");
      expect(formatted).toContain("Severity: high");
    });

    it("should format suggestion with single suggestion", () => {
      const suggestion = {
        message: "Test error",
        suggestions: ["only fix"],
        docs: "docs/test.html",
        severity: "low",
      };

      const formatted = formatSuggestions(suggestion);

      expect(formatted).toContain("1. only fix");
      expect(formatted).toContain("Severity: low");
    });

    it("should return default message for null input", () => {
      const formatted = formatSuggestions(null);
      expect(formatted).toBe("No suggestions available");
    });

    it("should return default message for undefined input", () => {
      const formatted = formatSuggestions(undefined);
      expect(formatted).toBe("No suggestions available");
    });

    it("should handle suggestion without docs", () => {
      const suggestion = {
        message: "Test error",
        suggestions: ["fix1"],
      };

      const formatted = formatSuggestions(suggestion);

      expect(formatted).toContain("Issue: Test error");
      expect(formatted).toContain("Documentation: undefined");
    });

    it("should handle suggestion without severity", () => {
      const suggestion = {
        message: "Test error",
        suggestions: ["fix1"],
        docs: "docs/test.html",
      };

      const formatted = formatSuggestions(suggestion);

      expect(formatted).toContain("Severity: undefined");
    });
  });

  describe("addSuggestion", () => {
    it("should add custom suggestion", () => {
      const customSuggestion = {
        message: "Custom error",
        suggestions: ["custom fix"],
        docs: "docs/custom.html",
        severity: "medium",
      };

      addSuggestion("CUSTOM_TEST_CODE", customSuggestion);

      const retrieved = getSuggestions("CUSTOM_TEST_CODE");
      expect(retrieved).toEqual(customSuggestion);
    });

    it("should override existing suggestion", () => {
      const original = getSuggestions("BROWSER_NOT_FOUND");
      const override = {
        message: "Custom message",
        suggestions: ["custom"],
        docs: "docs/custom.html",
        severity: "low",
      };

      addSuggestion("BROWSER_NOT_FOUND", override);

      const retrieved = getSuggestions("BROWSER_NOT_FOUND");
      expect(retrieved.message).toBe("Custom message");
    });
  });

  describe("getKnownErrorCodes", () => {
    it("should return array of error codes", () => {
      const codes = getKnownErrorCodes();

      expect(Array.isArray(codes)).toBe(true);
      expect(codes.length).toBeGreaterThan(0);
    });

    it("should include common error codes", () => {
      const codes = getKnownErrorCodes();

      expect(codes).toContain("BROWSER_NOT_FOUND");
      expect(codes).toContain("CONNECTION_TIMEOUT");
      expect(codes).toContain("ELEMENT_NOT_FOUND");
      expect(codes).toContain("LLM_TIMEOUT");
      expect(codes).toContain("HTTP_401");
      expect(codes).toContain("HTTP_404");
      expect(codes).toContain("HTTP_500");
    });

    it("should not include added custom codes after initial load", () => {
      const codesBefore = getKnownErrorCodes().length;

      addSuggestion("NEW_CODE", {
        message: "New",
        suggestions: ["fix"],
        docs: "",
        severity: "low",
      });

      const codesAfter = getKnownErrorCodes();
      expect(codesAfter.length).toBe(codesBefore + 1);
      expect(codesAfter).toContain("NEW_CODE");
    });
  });

  describe("ERROR_SUGGESTIONS", () => {
    it("should have suggestions for browser errors", () => {
      expect(ERROR_SUGGESTIONS.BROWSER_NOT_FOUND).toBeDefined();
      expect(ERROR_SUGGESTIONS.CONNECTION_TIMEOUT).toBeDefined();
      expect(ERROR_SUGGESTIONS.SESSION_DISCONNECTED).toBeDefined();
    });

    it("should have suggestions for element errors", () => {
      expect(ERROR_SUGGESTIONS.ELEMENT_NOT_FOUND).toBeDefined();
      expect(ERROR_SUGGESTIONS.ELEMENT_TIMEOUT).toBeDefined();
      expect(ERROR_SUGGESTIONS.ELEMENT_OBSCURED).toBeDefined();
      expect(ERROR_SUGGESTIONS.ELEMENT_DETACHED).toBeDefined();
    });

    it("should have suggestions for LLM errors", () => {
      expect(ERROR_SUGGESTIONS.LLM_TIMEOUT).toBeDefined();
      expect(ERROR_SUGGESTIONS.LLM_RATE_LIMIT).toBeDefined();
      expect(ERROR_SUGGESTIONS.LLM_CIRCUIT_OPEN).toBeDefined();
    });

    it("should have suggestions for HTTP errors", () => {
      expect(ERROR_SUGGESTIONS.HTTP_401).toBeDefined();
      expect(ERROR_SUGGESTIONS.HTTP_403).toBeDefined();
      expect(ERROR_SUGGESTIONS.HTTP_404).toBeDefined();
      expect(ERROR_SUGGESTIONS.HTTP_429).toBeDefined();
      expect(ERROR_SUGGESTIONS.HTTP_500).toBeDefined();
      expect(ERROR_SUGGESTIONS.HTTP_503).toBeDefined();
    });

    it("should have all required fields for each suggestion", () => {
      const keys = Object.keys(ERROR_SUGGESTIONS);

      keys.forEach((key) => {
        const suggestion = ERROR_SUGGESTIONS[key];
        expect(typeof suggestion.message).toBe("string");
        expect(Array.isArray(suggestion.suggestions)).toBe(true);
        expect(suggestion.suggestions.length).toBeGreaterThan(0);
        expect(typeof suggestion.docs).toBe("string");
        expect(["high", "medium", "low"]).toContain(suggestion.severity);
      });
    });
  });

  describe("Edge cases", () => {
    it("should handle empty error object", () => {
      const suggestion = getSuggestionsForError({});
      expect(suggestion).toBeNull();
    });

    it("should handle error with status but no code/name", () => {
      const suggestion = getSuggestionsForError({ status: 500 });
      expect(suggestion).toBeNull();
    });

    it("should handle error with both status and code", () => {
      const suggestion = getSuggestionsForError({
        code: "HTTP_500",
        status: 500,
      });
      expect(suggestion).not.toBeNull();
      expect(suggestion.message).toBe("Internal server error");
    });

    it("should handle case-sensitive error codes", () => {
      const suggestion = getSuggestions("browser_not_found");
      expect(suggestion).toBeNull();

      const suggestionUpper = getSuggestions("BROWSER_NOT_FOUND");
      expect(suggestionUpper).not.toBeNull();
    });
  });
});
