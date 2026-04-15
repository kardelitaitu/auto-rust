/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi } from "vitest";
import * as errors from "@api/core/errors.js";

describe("api/core/errors.js", () => {
  describe("AutomationError", () => {
    it("should create with default code", () => {
      const error = new errors.AutomationError("Test error");
      expect(error.message).toBe("Test error");
      expect(error.code).toBe("AUTOMATION_ERROR");
      expect(error.name).toBe("AutomationError");
    });

    it("should create with custom code", () => {
      const error = new errors.AutomationError("Test error", "CUSTOM_CODE");
      expect(error.code).toBe("CUSTOM_CODE");
    });

    it("should have stack trace", () => {
      const error = new errors.AutomationError("Test error");
      expect(error.stack).toBeDefined();
      expect(error.stack).toContain("AutomationError");
    });
  });

  describe("SessionError", () => {
    it("should create session error", () => {
      const error = new errors.SessionError("Session failed");
      expect(error.name).toBe("SessionError");
      expect(error.code).toBe("SESSION_ERROR");
    });

    it("should create SessionDisconnectedError", () => {
      const error = new errors.SessionDisconnectedError();
      expect(error.name).toBe("SessionDisconnectedError");
      expect(error.code).toBe("SESSION_DISCONNECTED");
      expect(error.message).toContain("disconnected");
    });

    it("should create SessionNotFoundError", () => {
      const error = new errors.SessionNotFoundError("session-123");
      expect(error.name).toBe("SessionNotFoundError");
      expect(error.code).toBe("SESSION_NOT_FOUND");
      expect(error.message).toContain("session-123");
    });

    it("should create SessionTimeoutError", () => {
      const error = new errors.SessionTimeoutError("Custom timeout");
      expect(error.name).toBe("SessionTimeoutError");
      expect(error.code).toBe("SESSION_TIMEOUT");
      expect(error.message).toBe("Custom timeout");
    });
  });

  describe("ContextError", () => {
    it("should create context error", () => {
      const error = new errors.ContextError("Context failed");
      expect(error.name).toBe("ContextError");
      expect(error.code).toBe("CONTEXT_ERROR");
    });

    it("should create ContextNotInitializedError", () => {
      const error = new errors.ContextNotInitializedError();
      expect(error.name).toBe("ContextNotInitializedError");
      expect(error.code).toBe("CONTEXT_NOT_INITIALIZED");
      expect(error.message).toContain("api.withPage");
    });

    it("should create PageClosedError", () => {
      const error = new errors.PageClosedError();
      expect(error.name).toBe("PageClosedError");
      expect(error.code).toBe("PAGE_CLOSED");
      expect(error.message).toContain("closed");
    });
  });

  describe("ElementError", () => {
    it("should create element error", () => {
      const error = new errors.ElementError("Element failed");
      expect(error.name).toBe("ElementError");
      expect(error.code).toBe("ELEMENT_ERROR");
    });

    it("should create ElementNotFoundError", () => {
      const error = new errors.ElementNotFoundError('[data-testid="test"]');
      expect(error.name).toBe("ElementNotFoundError");
      expect(error.code).toBe("ELEMENT_NOT_FOUND");
      expect(error.message).toContain('[data-testid="test"]');
    });

    it("should create ElementDetachedError", () => {
      const error = new errors.ElementDetachedError("button");
      expect(error.name).toBe("ElementDetachedError");
      expect(error.code).toBe("ELEMENT_DETACHED");
      expect(error.message).toContain("button");
    });

    it("should create ElementObscuredError", () => {
      const error = new errors.ElementObscuredError("div");
      expect(error.name).toBe("ElementObscuredError");
      expect(error.code).toBe("ELEMENT_OBSCURED");
      expect(error.message).toContain("div");
    });

    it("should create ElementTimeoutError", () => {
      const error = new errors.ElementTimeoutError(
        '[data-testid="test"]',
        5000,
      );
      expect(error.name).toBe("ElementTimeoutError");
      expect(error.code).toBe("ELEMENT_TIMEOUT");
      expect(error.message).toContain("5000ms");
    });
  });

  describe("ActionError", () => {
    it("should create action error", () => {
      const error = new errors.ActionError("Action failed");
      expect(error.name).toBe("ActionError");
      expect(error.code).toBe("ACTION_ERROR");
    });

    it("should create ActionFailedError", () => {
      const error = new errors.ActionFailedError("click", "element not found");
      expect(error.name).toBe("ActionFailedError");
      expect(error.code).toBe("ACTION_FAILED");
      expect(error.action).toBe("click");
      expect(error.message).toContain("click");
    });

    it("should create NavigationError", () => {
      const error = new errors.NavigationError(
        "https://example.com",
        "timeout",
      );
      expect(error.name).toBe("NavigationError");
      expect(error.code).toBe("NAVIGATION_ERROR");
      expect(error.url).toBe("https://example.com");
    });

    it("should create TaskTimeoutError", () => {
      const error = new errors.TaskTimeoutError("myTask", 10000);
      expect(error.name).toBe("TaskTimeoutError");
      expect(error.code).toBe("TASK_TIMEOUT");
      expect(error.taskName).toBe("myTask");
      expect(error.timeout).toBe(10000);
    });
  });

  describe("ConfigError", () => {
    it("should create config error", () => {
      const error = new errors.ConfigError("Config failed");
      expect(error.name).toBe("ConfigError");
      expect(error.code).toBe("CONFIG_ERROR");
    });

    it("should create ConfigNotFoundError", () => {
      const error = new errors.ConfigNotFoundError("api.key");
      expect(error.name).toBe("ConfigNotFoundError");
      expect(error.code).toBe("CONFIG_NOT_FOUND");
      expect(error.message).toContain("api.key");
    });
  });

  describe("LLMError", () => {
    it("should create LLM error", () => {
      const error = new errors.LLMError("LLM failed");
      expect(error.name).toBe("LLMError");
      expect(error.code).toBe("LLM_ERROR");
    });

    it("should create LLMTimeoutError", () => {
      const error = new errors.LLMTimeoutError("Custom timeout");
      expect(error.name).toBe("LLMTimeoutError");
      expect(error.code).toBe("LLM_TIMEOUT");
    });

    it("should create LLMRateLimitError", () => {
      const error = new errors.LLMRateLimitError("Custom rate limit");
      expect(error.name).toBe("LLMRateLimitError");
      expect(error.code).toBe("LLM_RATE_LIMIT");
    });

    it("should create LLMCircuitOpenError", () => {
      const error = new errors.LLMCircuitOpenError("gpt-4", 30000);
      expect(error.name).toBe("LLMCircuitOpenError");
      expect(error.code).toBe("LLM_CIRCUIT_OPEN");
      expect(error.modelId).toBe("gpt-4");
      expect(error.retryAfter).toBe(30000);
      expect(error.message).toContain("30s");
    });
  });

  describe("ValidationError", () => {
    it("should create validation error", () => {
      const error = new errors.ValidationError(
        "VALIDATION_ERROR",
        "Validation failed",
      );
      expect(error.name).toBe("ValidationError");
      expect(error.code).toBe("VALIDATION_ERROR");
    });
  });

  describe("isErrorCode", () => {
    it("should match by code", () => {
      const error = new errors.ElementNotFoundError('[data-testid="test"]');
      expect(errors.isErrorCode(error, "ELEMENT_NOT_FOUND")).toBe(true);
    });

    it("should match by name", () => {
      const error = new errors.ElementNotFoundError('[data-testid="test"]');
      expect(errors.isErrorCode(error, "ElementNotFoundError")).toBe(true);
    });

    it("should return false for non-matching code", () => {
      const error = new errors.ElementNotFoundError('[data-testid="test"]');
      expect(errors.isErrorCode(error, "OTHER_CODE")).toBe(false);
    });

    it("should return false for null error", () => {
      expect(errors.isErrorCode(null, "CODE")).toBe(false);
    });

    it("should return false for non-error", () => {
      expect(errors.isErrorCode("string", "CODE")).toBe(false);
    });
  });

  describe("withErrorHandling", () => {
    it("should return successful result", async () => {
      const fn = vi.fn().mockResolvedValue("success");
      const result = await errors.withErrorHandling(fn, "test");
      expect(result).toBe("success");
    });

    it("should pass through AutomationError", async () => {
      const automationError = new errors.AutomationError("Auto error");
      const fn = vi.fn().mockRejectedValue(automationError);

      await expect(errors.withErrorHandling(fn, "test")).rejects.toThrow(
        errors.AutomationError,
      );
    });

    it("should wrap other errors in ActionError", async () => {
      const fn = vi.fn().mockRejectedValue(new Error("Regular error"));

      await expect(
        errors.withErrorHandling(fn, "test operation"),
      ).rejects.toThrow(errors.ActionError);
      await expect(
        errors.withErrorHandling(fn, "test operation"),
      ).rejects.toThrow("Error during test operation");
    });
  });

  describe("default export", () => {
    it("should export all error classes", () => {
      expect(errors.default.AutomationError).toBe(errors.AutomationError);
      expect(errors.default.SessionError).toBe(errors.SessionError);
      expect(errors.default.ValidationError).toBe(errors.ValidationError);
      expect(errors.default.isErrorCode).toBe(errors.isErrorCode);
      expect(errors.default.withErrorHandling).toBe(errors.withErrorHandling);
    });
  });
});
