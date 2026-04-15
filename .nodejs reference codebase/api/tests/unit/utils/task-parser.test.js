/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for task-parser.js
 */

import { describe, it, expect } from "vitest";
import {
  parseTaskArgs,
  formatUrl,
  taskExists,
  getTasksDir,
  formatParseResult,
  TASK_SEPARATOR,
} from "@api/utils/task-parser.js";

describe("task-parser", () => {
  describe("TASK_SEPARATOR", () => {
    it('should be "then"', () => {
      expect(TASK_SEPARATOR).toBe("then");
    });
  });

  describe("formatUrl", () => {
    it("should return empty string as-is", () => {
      expect(formatUrl("")).toBe("");
    });

    it("should return null/undefined as-is", () => {
      expect(formatUrl(null)).toBeNull();
      expect(formatUrl(undefined)).toBeUndefined();
    });

    it("should keep http:// URLs unchanged", () => {
      expect(formatUrl("http://example.com")).toBe("http://example.com");
    });

    it("should keep https:// URLs unchanged", () => {
      expect(formatUrl("https://example.com")).toBe("https://example.com");
    });

    it("should add https:// to URLs with domain", () => {
      expect(formatUrl("example.com")).toBe("https://example.com");
      expect(formatUrl("www.example.com")).toBe("https://www.example.com");
      expect(formatUrl("sub.domain.example.com")).toBe(
        "https://sub.domain.example.com",
      );
    });

    it("should add https:// to localhost", () => {
      expect(formatUrl("localhost")).toBe("https://localhost");
      expect(formatUrl("localhost:3000")).toBe("https://localhost:3000");
    });

    it("should trim whitespace", () => {
      expect(formatUrl("  example.com  ")).toBe("https://example.com");
    });

    it("should not modify paths without dots", () => {
      expect(formatUrl("follow")).toBe("follow");
      expect(formatUrl("retweet")).toBe("retweet");
    });

    it("should preserve query strings", () => {
      expect(formatUrl("example.com?foo=bar")).toBe(
        "https://example.com?foo=bar",
      );
      expect(formatUrl("example.com/path?query=value")).toBe(
        "https://example.com/path?query=value",
      );
    });

    it("should preserve fragments", () => {
      expect(formatUrl("example.com#section")).toBe(
        "https://example.com#section",
      );
    });

    it("should handle complex URLs", () => {
      expect(formatUrl("www.x.com/username")).toBe(
        "https://www.x.com/username",
      );
      expect(formatUrl("facebook.com/photo?fbid=123")).toBe(
        "https://facebook.com/photo?fbid=123",
      );
    });
  });

  describe("getTasksDir", () => {
    it("should return absolute path to tasks directory", () => {
      const tasksDir = getTasksDir();
      expect(tasksDir).toBeDefined();
      expect(tasksDir.endsWith("tasks") || tasksDir.endsWith("tasks\\")).toBe(
        true,
      );
      expect(tasksDir.length > 5).toBe(true);
    });
  });

  describe("taskExists", () => {
    it("should return true for existing task files", () => {
      expect(taskExists("twitterFollow")).toBe(true);
      expect(taskExists("followback")).toBe(true);
      expect(taskExists("pageview")).toBe(true);
    });

    it("should return false for non-existent tasks", () => {
      expect(taskExists("nonexistent-task")).toBe(false);
      expect(taskExists("totally-fake")).toBe(false);
    });

    it("should handle .js extension", () => {
      expect(taskExists("pageview.js")).toBe(true);
    });
  });

  describe("parseTaskArgs", () => {
    describe("empty input", () => {
      it("should return empty groups for empty array", () => {
        const result = parseTaskArgs([]);
        expect(result.groups).toEqual([]);
        expect(result.taskCount).toBe(0);
      });

      it("should return empty groups for null input", () => {
        const result = parseTaskArgs(null);
        expect(result.groups).toEqual([]);
        expect(result.taskCount).toBe(0);
      });

      it("should return empty groups for undefined input", () => {
        const result = parseTaskArgs(undefined);
        expect(result.groups).toEqual([]);
        expect(result.taskCount).toBe(0);
      });
    });

    describe("single task", () => {
      it("should parse simple task with URL", () => {
        const result = parseTaskArgs(["follow=x.com"]);
        expect(result.groups).toHaveLength(1);
        expect(result.groups[0]).toHaveLength(1);
        expect(result.groups[0][0]).toEqual({
          name: "follow",
          payload: { url: "https://x.com" },
        });
        expect(result.taskCount).toBe(1);
      });

      it("should parse task with https:// URL", () => {
        const result = parseTaskArgs(["follow=https://x.com/user"]);
        expect(result.groups[0][0].payload.url).toBe("https://x.com/user");
      });

      it("should parse task with numeric value", () => {
        const result = parseTaskArgs(["followback=5"]);
        expect(result.groups[0][0].payload).toEqual({ value: 5 });
      });

      it("should parse task without value (positional)", () => {
        const result = parseTaskArgs(["cookiebot"]);
        expect(result.groups[0][0].name).toBe("cookiebot");
        expect(result.groups[0][0].payload).toEqual({});
      });

      it("should strip .js extension from task name", () => {
        const result = parseTaskArgs(["follow.js=x.com"]);
        expect(result.groups[0][0].name).toBe("follow");
      });

      it("should handle quoted values", () => {
        const result = parseTaskArgs(['task="value with spaces"']);
        expect(result.groups[0][0].payload.url).toBe("value with spaces");
      });
    });

    describe("multiple tasks in group", () => {
      it("should parse multiple tasks with shorthand (same key)", () => {
        const result = parseTaskArgs(["follow=x.com", "follow=y.com"]);
        expect(result.groups).toHaveLength(1);
        expect(result.groups[0]).toHaveLength(2);
        expect(result.groups[0][0]).toEqual({
          name: "follow",
          payload: { url: "https://x.com" },
        });
        expect(result.groups[0][1]).toEqual({
          name: "follow",
          payload: { url: "https://y.com" },
        });
        expect(result.taskCount).toBe(2);
      });

      it("should add different key as parameter to current task (not auto-formatted)", () => {
        const result = parseTaskArgs(["follow=x.com", "retweet=y.com"]);
        expect(result.groups).toHaveLength(1);
        expect(result.groups[0]).toHaveLength(1);
        expect(result.groups[0][0].name).toBe("follow");
        expect(result.groups[0][0].payload).toEqual({
          url: "https://x.com",
          retweet: "y.com",
        });
      });

      it("should handle shorthand (same key) by creating new task", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "retweet=y.com",
          "follow=z.com",
        ]);
        expect(result.groups[0]).toHaveLength(2);
        expect(result.groups[0][0].name).toBe("follow");
        expect(result.groups[0][0].payload.url).toBe("https://x.com");
        expect(result.groups[0][1].name).toBe("follow");
        expect(result.groups[0][1].payload.url).toBe("https://z.com");
      });
    });

    describe("task groups with then separator", () => {
      it("should split by then separator", () => {
        const result = parseTaskArgs(["follow=x.com", "then", "retweet=y.com"]);
        expect(result.groups).toHaveLength(2);
        expect(result.groups[0]).toHaveLength(1);
        expect(result.groups[1]).toHaveLength(1);
        expect(result.groups[0][0].name).toBe("follow");
        expect(result.groups[1][0].name).toBe("retweet");
        expect(result.taskCount).toBe(2);
      });

      it("should handle multiple groups", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "then",
          "retweet=y.com",
          "then",
          "pageview=z.com",
        ]);
        expect(result.groups).toHaveLength(3);
        expect(result.taskCount).toBe(3);
      });

      it("should handle empty groups (consecutive then)", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "then",
          "then",
          "retweet=y.com",
        ]);
        expect(result.groups).toHaveLength(2);
        expect(result.taskCount).toBe(2);
      });

      it("should be case-insensitive for then", () => {
        const result1 = parseTaskArgs([
          "follow=x.com",
          "THEN",
          "retweet=y.com",
        ]);
        const result2 = parseTaskArgs([
          "follow=x.com",
          "Then",
          "retweet=y.com",
        ]);

        expect(result1.groups).toHaveLength(2);
        expect(result2.groups).toHaveLength(2);
      });
    });

    describe("complex command parsing", () => {
      it("should parse the example command: follow=x.com follow=y.com then retweet=z.com cookiebot pageview=facebook.com/user", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "follow=y.com",
          "then",
          "retweet=z.com",
          "cookiebot",
          "pageview=facebook.com/user",
        ]);

        expect(result.groups).toHaveLength(2);
        expect(result.taskCount).toBe(4);

        expect(result.groups[0]).toHaveLength(2);
        expect(result.groups[0][0].name).toBe("follow");
        expect(result.groups[0][0].payload.url).toBe("https://x.com");
        expect(result.groups[0][1].name).toBe("follow");
        expect(result.groups[0][1].payload.url).toBe("https://y.com");

        expect(result.groups[1]).toHaveLength(2);
        expect(result.groups[1][0].name).toBe("retweet");
        expect(result.groups[1][1].name).toBe("cookiebot");
        expect(result.groups[1][1].payload).toEqual({
          pageview: "facebook.com/user",
        });
      });

      it("should handle URL with complex path", () => {
        const result = parseTaskArgs(["follow=www.x.com/username/followers"]);
        expect(result.groups[0][0].payload.url).toBe(
          "https://www.x.com/username/followers",
        );
      });

      it("should handle URL with query params", () => {
        const result = parseTaskArgs(["pageview=example.com?q=test&page=1"]);
        expect(result.groups[0][0].payload.url).toBe(
          "https://example.com?q=test&page=1",
        );
      });
    });

    describe("task parameters", () => {
      it("should add parameters to current task", () => {
        const result = parseTaskArgs(["follow=x.com", "action=like"]);
        expect(result.groups[0][0]).toEqual({
          name: "follow",
          payload: { url: "https://x.com", action: "like" },
        });
      });

      it("should add multiple parameters", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "action=like",
          "count=5",
        ]);
        expect(result.groups[0][0].payload).toEqual({
          url: "https://x.com",
          action: "like",
          count: "5",
        });
      });

      it("should treat non-url values as url parameters", () => {
        const result = parseTaskArgs(["task=somevalue", "param=other"]);
        expect(result.groups[0][0].name).toBe("task");
        expect(result.groups[0][0].payload.url).toBe("somevalue");
        expect(result.groups[0][0].payload.param).toBe("other");
        expect(result.taskCount).toBe(1);
      });

      it("should auto-format url parameter", () => {
        const result = parseTaskArgs(["follow=x.com", "url=y.com"]);
        expect(result.groups[0][0].payload.url).toBe("https://y.com");
      });
    });

    describe("edge cases", () => {
      it("should handle equal sign in value", () => {
        const result = parseTaskArgs(["task=value=with=equals"]);
        expect(result.groups[0][0].payload.url).toBe("value=with=equals");
      });

      it("should handle empty value", () => {
        const result = parseTaskArgs(["task="]);
        expect(result.groups[0][0].payload.url).toBe("");
      });

      it("should create new task for key without equals, subsequent key=value adds as parameter", () => {
        const result = parseTaskArgs([
          "follow=x.com",
          "invalid",
          "retweet=y.com",
        ]);
        expect(result.groups[0]).toHaveLength(2);
        expect(result.groups[0][0].name).toBe("follow");
        expect(result.groups[0][0].payload).toEqual({ url: "https://x.com" });
        expect(result.groups[0][1].name).toBe("invalid");
        expect(result.groups[0][1].payload).toEqual({ retweet: "y.com" });
      });

      it("should ignore empty strings", () => {
        const result = parseTaskArgs(["follow=x.com", "", "retweet=y.com"]);
        expect(result.groups[0]).toHaveLength(2);
      });
    });
  });

  describe("formatParseResult", () => {
    it("should format empty result", () => {
      expect(formatParseResult({ groups: [], taskCount: 0 })).toBe("No tasks");
    });

    it("should format single task", () => {
      const result = {
        groups: [[{ name: "follow", payload: {} }]],
        taskCount: 1,
      };
      expect(formatParseResult(result)).toBe("1 task(s) [follow]");
    });

    it("should format multiple tasks", () => {
      const result = {
        groups: [
          [
            { name: "follow", payload: {} },
            { name: "retweet", payload: {} },
          ],
        ],
        taskCount: 2,
      };
      expect(formatParseResult(result)).toBe("2 task(s) [follow, retweet]");
    });

    it("should format multiple groups", () => {
      const result = {
        groups: [
          [{ name: "follow", payload: {} }],
          [{ name: "retweet", payload: {} }],
        ],
        taskCount: 2,
      };
      expect(formatParseResult(result)).toBe(
        "2 task(s) [Group 1: follow | Group 2: retweet]",
      );
    });
  });
});
