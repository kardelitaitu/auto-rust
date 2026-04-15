/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/responseValidator.js
 * @module tests/unit/agent/responseValidator.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("api/agent/responseValidator.js", () => {
  let responseValidator;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/responseValidator.js");
    responseValidator = module.responseValidator || module.default;
  });

  describe("Constructor", () => {
    it("should initialize with valid actions", () => {
      expect(responseValidator.validActions).toContain("click");
      expect(responseValidator.validActions).toContain("type");
      expect(responseValidator.validActions).toContain("wait");
      expect(responseValidator.validActions).toContain("done");
    });

    it("should initialize with required params for actions", () => {
      expect(responseValidator.requiredParams.click).toContain("selector");
      expect(responseValidator.requiredParams.type).toContain("selector");
      expect(responseValidator.requiredParams.type).toContain("value");
      expect(responseValidator.requiredParams.clickAt).toContain("x");
      expect(responseValidator.requiredParams.clickAt).toContain("y");
    });

    it("should initialize with selector patterns", () => {
      expect(responseValidator.selectorPatterns.css).toBeDefined();
      expect(responseValidator.selectorPatterns.role).toBeDefined();
      expect(responseValidator.selectorPatterns.text).toBeDefined();
      expect(responseValidator.selectorPatterns.xpath).toBeDefined();
    });
  });

  describe("validate()", () => {
    it("should return invalid for null response", () => {
      const result = responseValidator.validate(null);
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Response is not an object");
    });

    it("should return invalid for undefined response", () => {
      const result = responseValidator.validate(undefined);
      expect(result.valid).toBe(false);
    });

    it("should return invalid for non-object response", () => {
      const result = responseValidator.validate("string");
      expect(result.valid).toBe(false);
    });

    it("should validate array of actions", () => {
      const result = responseValidator.validate([
        { action: "click", selector: "#btn" },
        { action: "type", selector: "#input", value: "text" },
      ]);
      expect(result.valid).toBe(true);
    });

    it("should return invalid for empty array", () => {
      const result = responseValidator.validate([]);
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Empty action array");
    });

    it("should validate single valid action", () => {
      const result = responseValidator.validate({
        action: "click",
        selector: "#btn",
        rationale: "Click button",
      });
      expect(result.valid).toBe(true);
    });

    it("should detect missing action field", () => {
      const result = responseValidator.validate({
        selector: "#btn",
      });
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Missing 'action' field");
    });

    it("should detect invalid action type", () => {
      const result = responseValidator.validate({
        action: "invalidAction",
      });
      expect(result.valid).toBe(false);
      expect(result.errors[0]).toContain("Invalid action type");
    });

    it("should detect missing required parameters", () => {
      const result = responseValidator.validate({
        action: "click",
        rationale: "Click without selector",
      });
      expect(result.valid).toBe(false);
      expect(
        result.errors.some((e) => e.includes("Missing required parameter")),
      ).toBe(true);
    });

    it("should warn about missing rationale", () => {
      const result = responseValidator.validate({
        action: "click",
        selector: "#btn",
      });
      expect(result.valid).toBe(true);
      expect(result.warnings.some((w) => w.includes("rationale"))).toBe(true);
    });
  });

  describe("_validateArray()", () => {
    it("should validate multiple actions", () => {
      const result = responseValidator._validateArray([
        { action: "click", selector: "#btn", rationale: "click" },
        { action: "wait", value: "1000", rationale: "wait" },
      ]);
      expect(result.valid).toBe(true);
      expect(result.errors.length).toBe(0);
    });

    it("should aggregate errors from multiple actions", () => {
      const result = responseValidator._validateArray([
        { action: "click" },
        { action: "type", selector: "#input" },
      ]);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });

  describe("_validateSingle()", () => {
    it("should validate complete action", () => {
      const result = responseValidator._validateSingle({
        action: "click",
        selector: "#btn",
        rationale: "Click button",
      });
      expect(result.valid).toBe(true);
    });

    it("should check x coordinate is number", () => {
      const result = responseValidator._validateSingle({
        action: "clickAt",
        x: "100",
        y: 200,
      });
      expect(result.valid).toBe(false);
      expect(
        result.errors.some((e) => e.includes("'x' must be a number")),
      ).toBe(true);
    });

    it("should check y coordinate is number", () => {
      const result = responseValidator._validateSingle({
        action: "clickAt",
        x: 100,
        y: "200",
      });
      expect(result.valid).toBe(false);
      expect(
        result.errors.some((e) => e.includes("'y' must be a number")),
      ).toBe(true);
    });
  });

  describe("_validateSelector()", () => {
    it("should detect placeholder selectors", () => {
      const result = responseValidator._validateSelector("...");
      expect(result.valid).toBe(false);
      expect(result.reason).toContain("placeholder");
    });

    it("should detect non-string selector", () => {
      const result = responseValidator._validateSelector(123);
      expect(result.valid).toBe(false);
    });

    it("should validate CSS selector", () => {
      const result = responseValidator._validateSelector("#myId");
      expect(result.valid).toBe(true);
    });

    it("should validate role selector", () => {
      const result = responseValidator._validateSelector("role=button");
      expect(result.valid).toBe(true);
    });

    it("should validate text selector", () => {
      const result = responseValidator._validateSelector("text=Click me");
      expect(result.valid).toBe(true);
    });

    it("should validate xpath selector", () => {
      const result = responseValidator._validateSelector('//div[@id="test"]');
      expect(result.valid).toBe(true);
    });

    it("should accept unknown selector formats", () => {
      const result = responseValidator._validateSelector(
        "complex:selector:format",
      );
      expect(result.valid).toBe(true);
      expect(result.type).toBe("unknown");
    });
  });

  describe("autoCorrect()", () => {
    it("should return non-object as-is", () => {
      expect(responseValidator.autoCorrect(null)).toBeNull();
      expect(responseValidator.autoCorrect("string")).toBe("string");
    });

    it("should correct single action", () => {
      const result = responseValidator.autoCorrect({
        action: "clic",
        selector: "#btn",
        value: "text",
      });
      expect(result.action).toBe("click");
    });

    it("should correct array of actions", () => {
      const result = responseValidator.autoCorrect([
        { action: "clck", selector: "#btn" },
        { action: "tye", selector: "#input", value: 100 },
      ]);
      expect(result[0].action).toBe("click");
      expect(result[1].action).toBe("type");
      expect(result[1].value).toBe("100");
    });

    it("should convert coordinates to numbers", () => {
      const result = responseValidator.autoCorrect({
        action: "clickAt",
        x: "100",
        y: "200",
      });
      expect(typeof result.x).toBe("number");
      expect(typeof result.y).toBe("number");
    });

    it("should convert type action value to string", () => {
      const result = responseValidator.autoCorrect({
        action: "type",
        selector: "#input",
        value: 123,
      });
      expect(result.value).toBe("123");
    });

    it("should convert wait action value to string", () => {
      const result = responseValidator.autoCorrect({
        action: "wait",
        value: 1000,
      });
      expect(result.value).toBe("1000");
    });

    it("should add # prefix to bare selectors", () => {
      const result = responseValidator.autoCorrect({
        action: "click",
        selector: "myButton",
      });
      expect(result.selector).toBe("#myButton");
    });
  });

  describe("_correctActionType()", () => {
    it("should correct common typos", () => {
      expect(responseValidator._correctActionType("clic")).toBe("click");
      expect(responseValidator._correctActionType("tye")).toBe("type");
      expect(responseValidator._correctActionType("wiat")).toBe("wait");
      expect(responseValidator._correctActionType("don")).toBe("done");
      expect(responseValidator._correctActionType("scrol")).toBe("scroll");
    });

    it("should return unknown action unchanged", () => {
      expect(responseValidator._correctActionType("unknownAction")).toBe(
        "unknownAction",
      );
    });

    it("should be case insensitive", () => {
      expect(responseValidator._correctActionType("CLIC")).toBe("click");
      expect(responseValidator._correctActionType("TyE")).toBe("type");
    });
  });

  describe("_correctSelector()", () => {
    it("should trim whitespace", () => {
      expect(responseValidator._correctSelector("  #btn  ")).toBe("#btn");
    });

    it("should add # prefix to bare identifiers", () => {
      expect(responseValidator._correctSelector("myId")).toBe("#myId");
    });

    it("should not add # to selectors that already have prefix", () => {
      expect(responseValidator._correctSelector("#existing")).toBe("#existing");
      expect(responseValidator._correctSelector(".existing")).toBe(".existing");
    });
  });

  describe("getSummary()", () => {
    it("should return valid summary for valid result", () => {
      const summary = responseValidator.getSummary({
        valid: true,
        warnings: [],
      });
      expect(summary).toContain("Valid");
    });

    it("should include warning count", () => {
      const summary = responseValidator.getSummary({
        valid: true,
        warnings: ["w1", "w2"],
      });
      expect(summary).toContain("2 warning(s)");
    });

    it("should return invalid summary for invalid result", () => {
      const summary = responseValidator.getSummary({
        valid: false,
        errors: ["e1"],
        warnings: ["w1"],
      });
      expect(summary).toContain("Invalid");
      expect(summary).toContain("1 error(s)");
    });
  });

  describe("Edge Cases", () => {
    it("should handle deeply nested array validation", () => {
      const actions = Array(5)
        .fill(null)
        .map((_, i) => ({
          action: "click",
          selector: `#btn${i}`,
          rationale: `Click button ${i}`,
        }));
      const result = responseValidator.validate(actions);
      expect(result.valid).toBe(true);
    });

    it("should handle action with extra properties", () => {
      const result = responseValidator.validate({
        action: "click",
        selector: "#btn",
        extraProp: "value",
        rationale: "Click",
      });
      expect(result.valid).toBe(true);
    });

    it("should handle wait value as number", () => {
      const result = responseValidator.validate({
        action: "wait",
        value: 1000,
        rationale: "Wait",
      });
      expect(result.valid).toBe(true);
    });
  });
});
