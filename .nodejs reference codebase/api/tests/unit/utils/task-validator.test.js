/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  validateTaskPayload,
  registerTaskSchema,
  getTaskSchema,
  listTaskSchemas,
  FIELD_TYPES,
  TASK_SCHEMAS,
} from "@api/utils/task-validator.js";

describe("task-validator", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("FIELD_TYPES", () => {
    it("should validate string type", () => {
      expect(FIELD_TYPES.string("hello")).toBe(true);
      expect(FIELD_TYPES.string(123)).toBe(false);
      expect(FIELD_TYPES.string(null)).toBe(false);
    });

    it("should validate number type", () => {
      expect(FIELD_TYPES.number(123)).toBe(true);
      expect(FIELD_TYPES.number(0)).toBe(true);
      expect(FIELD_TYPES.number(NaN)).toBe(false);
      expect(FIELD_TYPES.number("123")).toBe(false);
    });

    it("should validate boolean type", () => {
      expect(FIELD_TYPES.boolean(true)).toBe(true);
      expect(FIELD_TYPES.boolean(false)).toBe(true);
      expect(FIELD_TYPES.boolean(1)).toBe(false);
    });

    it("should validate object type", () => {
      expect(FIELD_TYPES.object({})).toBe(true);
      expect(FIELD_TYPES.object({ a: 1 })).toBe(true);
      expect(FIELD_TYPES.object(null)).toBe(false);
      expect(FIELD_TYPES.object([])).toBe(true);
    });

    it("should validate array type", () => {
      expect(FIELD_TYPES.array([])).toBe(true);
      expect(FIELD_TYPES.array([1, 2, 3])).toBe(true);
      expect(FIELD_TYPES.array("123")).toBe(false);
    });

    it("should validate url type", () => {
      expect(FIELD_TYPES.url("https://example.com")).toBe(true);
      expect(FIELD_TYPES.url("http://test.com/path")).toBe(true);
      expect(FIELD_TYPES.url("not-a-url")).toBe(false);
      expect(FIELD_TYPES.url(123)).toBe(false);
    });

    it("should validate function type", () => {
      expect(FIELD_TYPES.function(() => {})).toBe(true);
      expect(FIELD_TYPES.function(function () {})).toBe(true);
      expect(FIELD_TYPES.function("fn")).toBe(false);
    });
  });

  describe("TASK_SCHEMAS", () => {
    it("should have pageview schema", () => {
      expect(TASK_SCHEMAS.pageview).toBeDefined();
      expect(TASK_SCHEMAS.pageview.optional).toHaveProperty("url");
    });

    it("should have twitterFollow schema", () => {
      expect(TASK_SCHEMAS.twitterFollow).toBeDefined();
      expect(TASK_SCHEMAS.twitterFollow.validate).toBeDefined();
    });

    it("should have like schema", () => {
      expect(TASK_SCHEMAS.like).toBeDefined();
    });

    it("should have retweet schema", () => {
      expect(TASK_SCHEMAS.retweet).toBeDefined();
    });

    it("should have reply schema", () => {
      expect(TASK_SCHEMAS.reply).toBeDefined();
    });
  });

  describe("validateTaskPayload", () => {
    it("should return valid for unknown task", () => {
      const result = validateTaskPayload("unknownTask", {});
      expect(result.isValid).toBe(true);
      expect(result.errors).toEqual([]);
      expect(result.warnings).toContain("Unknown task: unknownTask");
    });

    describe("pageview validation", () => {
      it("should pass with valid url", () => {
        const result = validateTaskPayload("pageview", { url: "https://example.com" });
        expect(result.isValid).toBe(true);
        expect(result.errors).toEqual([]);
      });

      it("should pass without url (loads from file)", () => {
        const result = validateTaskPayload("pageview", {});
        expect(result.isValid).toBe(true);
      });

      it("should fail with invalid url", () => {
        const result = validateTaskPayload("pageview", { url: "not-a-url" });
        expect(result.isValid).toBe(false);
        expect(result.errors[0]).toContain("url");
      });

      it("should accept timeout as number", () => {
        const result = validateTaskPayload("pageview", { url: "https://example.com", timeout: 5000 });
        expect(result.isValid).toBe(true);
      });
    });

    describe("twitterFollow validation", () => {
      it("should pass with valid targetUrl", () => {
        const result = validateTaskPayload("twitterFollow", { targetUrl: "https://x.com/user" });
        expect(result.isValid).toBe(true);
      });

      it("should pass with valid url", () => {
        const result = validateTaskPayload("twitterFollow", { url: "https://x.com/user" });
        expect(result.isValid).toBe(true);
      });

      it("should pass without url (uses default)", () => {
        const result = validateTaskPayload("twitterFollow", {});
        expect(result.isValid).toBe(true);
      });

      it("should fail with invalid targetUrl", () => {
        const result = validateTaskPayload("twitterFollow", { targetUrl: "invalid" });
        expect(result.isValid).toBe(false);
      });

      it("should fail with invalid taskTimeoutMs", () => {
        const result = validateTaskPayload("twitterFollow", { targetUrl: "https://x.com/user", taskTimeoutMs: 500 });
        expect(result.isValid).toBe(false);
        expect(result.errors[0]).toContain("taskTimeoutMs");
      });

      it("should accept valid taskTimeoutMs", () => {
        const result = validateTaskPayload("twitterFollow", { taskTimeoutMs: 5000 });
        expect(result.isValid).toBe(true);
      });
    });

    describe("like validation", () => {
      it("should pass with valid tweetUrl", () => {
        const result = validateTaskPayload("like", { tweetUrl: "https://x.com/status/123" });
        expect(result.isValid).toBe(true);
      });

      it("should fail without url", () => {
        const result = validateTaskPayload("like", {});
        expect(result.isValid).toBe(false);
        expect(result.errors).toContain("tweetUrl or url is required");
      });

      it("should fail with invalid url", () => {
        const result = validateTaskPayload("like", { tweetUrl: "not-a-tweet" });
        expect(result.isValid).toBe(false);
      });
    });

    describe("retweet validation", () => {
      it("should pass with valid tweetUrl", () => {
        const result = validateTaskPayload("retweet", { tweetUrl: "https://x.com/status/123" });
        expect(result.isValid).toBe(true);
      });

      it("should fail without url", () => {
        const result = validateTaskPayload("retweet", {});
        expect(result.isValid).toBe(false);
      });
    });

    describe("reply validation", () => {
      it("should pass with valid url and message", () => {
        const result = validateTaskPayload("reply", {
          tweetUrl: "https://x.com/status/123",
          message: "Hello!"
        });
        expect(result.isValid).toBe(true);
      });

      it("should fail without url", () => {
        const result = validateTaskPayload("reply", { message: "Hello!" });
        expect(result.isValid).toBe(false);
      });

      it("should fail with non-string message", () => {
        const result = validateTaskPayload("reply", {
          tweetUrl: "https://x.com/status/123",
          message: 123
        });
        expect(result.isValid).toBe(false);
        expect(result.errors[0]).toContain("message");
      });
    });

    describe("required fields", () => {
      it("should add custom schema with required field", () => {
        registerTaskSchema("customTask", {
          required: { name: "string" },
          optional: { timeout: "number" }
        });

        const result = validateTaskPayload("customTask", {});
        expect(result.isValid).toBe(false);
        expect(result.errors).toContain("Required field missing: name");
      });

      it("should validate required field type", () => {
        registerTaskSchema("typedTask", {
          required: { count: "number" }
        });

        const result = validateTaskPayload("typedTask", { count: "five" });
        expect(result.isValid).toBe(false);
        expect(result.errors).toContain("Field count must be of type number");
      });
    });

    describe("optional fields", () => {
      it("should pass optional fields", () => {
        const result = validateTaskPayload("pageview", { url: "https://example.com", timeout: 5000 });
        expect(result.isValid).toBe(true);
      });

      it("should validate optional field type", () => {
        const result = validateTaskPayload("pageview", { url: "https://example.com", timeout: "five" });
        expect(result.isValid).toBe(false);
        expect(result.errors).toContain("Optional field timeout must be of type number");
      });
    });
  });

  describe("registerTaskSchema", () => {
    it("should register new schema", () => {
      const newSchema = {
        required: { url: "url" },
        optional: { retries: "number" },
        validate: (payload) => {
          if (!payload.url) return ["url required"];
          return [];
        }
      };

      registerTaskSchema("newTask", newSchema);
      expect(getTaskSchema("newTask")).toEqual(newSchema);
    });

    it("should overwrite existing schema", () => {
      const originalSchema = getTaskSchema("pageview");
      const newSchema = { required: {} };
      registerTaskSchema("pageview", newSchema);
      expect(getTaskSchema("pageview")).toEqual(newSchema);
      registerTaskSchema("pageview", originalSchema);
    });
  });

  describe("getTaskSchema", () => {
    it("should return existing schema", () => {
      const schema = getTaskSchema("pageview");
      expect(schema).toBeDefined();
    });

    it("should return undefined for unknown task", () => {
      expect(getTaskSchema("nonexistent")).toBeUndefined();
    });
  });

  describe("listTaskSchemas", () => {
    it("should list all registered schemas", () => {
      const schemas = listTaskSchemas();
      expect(schemas).toContain("pageview");
      expect(schemas).toContain("twitterFollow");
      expect(schemas).toContain("like");
      expect(schemas).toContain("retweet");
      expect(schemas).toContain("reply");
    });
  });
});
