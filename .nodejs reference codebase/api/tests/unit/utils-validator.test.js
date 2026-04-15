/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

describe("api/utils/validator.js", () => {
  let validator;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/utils/validator.js");
    validator = module.default;
  });

  describe("validator object", () => {
    it("should be defined", () => {
      expect(validator).toBeDefined();
    });

    it("should have validatePayload method", () => {
      expect(typeof validator.validatePayload).toBe("function");
    });

    it("should have validateApiResponse method", () => {
      expect(typeof validator.validateApiResponse).toBe("function");
    });

    it("should have validateBrowserConnection method", () => {
      expect(typeof validator.validateBrowserConnection).toBe("function");
    });

    it("should have validateTaskExecution method", () => {
      expect(typeof validator.validateTaskExecution).toBe("function");
    });
  });

  describe("validatePayload", () => {
    it("should validate valid payload", () => {
      const result = validator.validatePayload(
        { name: "test" },
        { name: { type: "string", required: true } },
      );
      expect(result.isValid).toBe(true);
    });

    it("should reject missing required field", () => {
      const result = validator.validatePayload(
        {},
        { name: { type: "string", required: true } },
      );
      expect(result.isValid).toBe(false);
    });

    it("should reject invalid type", () => {
      const result = validator.validatePayload(
        { count: "abc" },
        { count: { type: "number" } },
      );
      expect(result.isValid).toBe(false);
    });
  });

  describe("validateBrowserConnection", () => {
    it("should validate valid ws endpoint", () => {
      const result = validator.validateBrowserConnection(
        "ws://localhost:9222/devtools/browser/abc123",
      );
      expect(result.isValid).toBe(true);
    });

    it("should reject invalid endpoint", () => {
      const result = validator.validateBrowserConnection("invalid");
      expect(result.isValid).toBe(false);
    });

    it("should reject empty endpoint", () => {
      const result = validator.validateBrowserConnection("");
      expect(result.isValid).toBe(false);
    });
  });

  describe("validateApiResponse", () => {
    it("should validate successful response for known apiType", () => {
      const result = validator.validateApiResponse(
        { code: 0, data: [] },
        "roxybrowser",
      );
      expect(result.isValid).toBe(true);
    });

    it("should return valid for unknown apiType", () => {
      const result = validator.validateApiResponse(
        { success: false, error: "fail" },
        "unknown",
      );
      expect(result.isValid).toBe(true);
    });
  });
});
