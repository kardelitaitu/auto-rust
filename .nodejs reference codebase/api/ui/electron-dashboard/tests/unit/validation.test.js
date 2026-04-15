/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * @fileoverview Unit tests for dashboard.js validation functions
 */
import { describe, it, expect } from "vitest";
import {
  validateTask,
  validateSession,
  validateMetrics,
  validatePayload,
} from "../../dashboard.js";

describe("validateTask", () => {
  it("should return null for null input", () => {
    expect(validateTask(null)).toBeNull();
  });

  it("should return null for undefined input", () => {
    expect(validateTask(undefined)).toBeNull();
  });

  it("should return null for non-object input", () => {
    expect(validateTask("string")).toBeNull();
    expect(validateTask(123)).toBeNull();
    expect(validateTask([])).toBeNull();
  });

  it("should return null for empty object", () => {
    expect(validateTask({})).toBeNull();
  });

  it("should return validated task with valid fields", () => {
    const task = {
      id: "task-1",
      taskName: "followUser",
      sessionId: "session-123",
      timestamp: Date.now(),
      success: true,
      extraField: "should be ignored",
    };
    const result = validateTask(task);
    expect(result).not.toBeNull();
    expect(result.id).toBe("task-1");
    expect(result.taskName).toBe("followUser");
    expect(result.sessionId).toBe("session-123");
    expect(result.success).toBe(true);
    expect(result.extraField).toBeUndefined();
  });

  it("should handle partial valid fields", () => {
    const task = { id: "task-1" };
    const result = validateTask(task);
    expect(result).not.toBeNull();
    expect(result.id).toBe("task-1");
  });

  it("should handle all valid fields", () => {
    const task = {
      id: "task-full",
      taskName: "retweet",
      name: "retweet",
      command: "retweet",
      sessionId: "session-1",
      session: "session-1",
      timestamp: 1234567890,
      status: "completed",
      success: true,
      error: null,
      duration: 5000,
    };
    const result = validateTask(task);
    expect(result).not.toBeNull();
    expect(Object.keys(result).length).toBe(11);
  });
});

describe("validateSession", () => {
  it("should return null for null input", () => {
    expect(validateSession(null)).toBeNull();
  });

  it("should return null for undefined input", () => {
    expect(validateSession(undefined)).toBeNull();
  });

  it("should return null for non-object input", () => {
    expect(validateSession("string")).toBeNull();
    expect(validateSession(123)).toBeNull();
    expect(validateSession([])).toBeNull();
  });

  it("should return null for empty object", () => {
    expect(validateSession({})).toBeNull();
  });

  it("should return validated session with valid fields", () => {
    const session = {
      id: "session-1",
      status: "online",
      browser: "chrome",
      profile: "profile-1",
      port: 53200,
      invalid: "field",
    };
    const result = validateSession(session);
    expect(result).not.toBeNull();
    expect(result.id).toBe("session-1");
    expect(result.status).toBe("online");
    expect(result.browser).toBe("chrome");
    expect(result.invalid).toBeUndefined();
  });

  it("should handle partial valid fields", () => {
    const session = { id: "session-1", status: "online" };
    const result = validateSession(session);
    expect(result).not.toBeNull();
    expect(result.id).toBe("session-1");
    expect(result.status).toBe("online");
  });

  it("should handle all valid fields", () => {
    const session = {
      id: "session-full",
      status: "online",
      browser: "firefox",
      profile: "profile-1",
      port: 6699,
      ws: "ws://localhost:53200",
      lastSeen: Date.now(),
      firstSeen: Date.now(),
    };
    const result = validateSession(session);
    expect(result).not.toBeNull();
    expect(Object.keys(result).length).toBe(8);
  });
});

describe("validateMetrics", () => {
  it("should return null for null input", () => {
    expect(validateMetrics(null)).toBeNull();
  });

  it("should return null for undefined input", () => {
    expect(validateMetrics(undefined)).toBeNull();
  });

  it("should return null for non-object input", () => {
    expect(validateMetrics("string")).toBeNull();
    expect(validateMetrics(123)).toBeNull();
  });

  it("should return null or empty object for empty object", () => {
    const result = validateMetrics({});
    // Accept both null or empty object - defensive programming
    expect(result === null || Object.keys(result).length === 0).toBe(true);
  });

  it("should return validated metrics with twitter data", () => {
    const metrics = {
      twitter: { actions: { likes: 10, retweets: 5 } },
      invalid: "data",
    };
    const result = validateMetrics(metrics);
    expect(result).not.toBeNull();
    expect(result.twitter).toEqual({ actions: { likes: 10, retweets: 5 } });
    expect(result.invalid).toBeUndefined();
  });

  it("should return validated metrics with api data", () => {
    const metrics = {
      api: { calls: 100, failures: 5 },
    };
    const result = validateMetrics(metrics);
    expect(result).not.toBeNull();
    expect(result.api).toEqual({ calls: 100, failures: 5 });
  });

  it("should return validated metrics with browsers data", () => {
    const metrics = {
      browsers: { discovered: 5, connected: 3 },
    };
    const result = validateMetrics(metrics);
    expect(result).not.toBeNull();
    expect(result.browsers).toEqual({ discovered: 5, connected: 3 });
  });

  it("should return validated metrics with all data types", () => {
    const metrics = {
      twitter: { actions: { likes: 10 } },
      api: { calls: 100 },
      browsers: { discovered: 5 },
    };
    const result = validateMetrics(metrics);
    expect(result).not.toBeNull();
    expect(result.twitter).toBeDefined();
    expect(result.api).toBeDefined();
    expect(result.browsers).toBeDefined();
  });

  it("should filter out invalid nested data", () => {
    const metrics = {
      twitter: "invalid",
      api: null,
      browsers: 123,
    };
    const result = validateMetrics(metrics);
    // Returns null when all nested data is invalid
    expect(result).toBeNull();
  });
});

describe("validatePayload", () => {
  it("should return null for null input", () => {
    expect(validatePayload(null)).toBeNull();
  });

  it("should return null for undefined input", () => {
    expect(validatePayload(undefined)).toBeNull();
  });

  it("should return null for non-object input", () => {
    expect(validatePayload("string")).toBeNull();
    expect(validatePayload(123)).toBeNull();
  });

  it("should return null or empty object for empty payload", () => {
    const result = validatePayload({});
    // Accept both null or empty object - defensive programming
    expect(result === null || Object.keys(result).length === 0).toBe(true);
  });

  it("should validate full payload with sessions, tasks, metrics", () => {
    const payload = {
      sessions: [
        { id: "s1", status: "online", browser: "chrome" },
        { id: "s2", status: "offline", invalid: "field" },
      ],
      recentTasks: [
        { id: "t1", taskName: "follow", success: true },
        { id: "t2", invalid: "field" },
      ],
      metrics: {
        twitter: { actions: { likes: 10 } },
        api: { calls: 100 },
      },
      errors: ["error 1", "error 2"],
    };

    const result = validatePayload(payload);
    expect(result).not.toBeNull();
    expect(result.sessions).toHaveLength(2); // Both have valid id/status
    expect(result.sessions[0].id).toBe("s1");
    expect(result.recentTasks).toHaveLength(2); // Both have valid id
    expect(result.recentTasks[0].taskName).toBe("follow");
    expect(result.metrics.twitter).toBeDefined();
    expect(result.metrics.api).toBeDefined();
    expect(result.errors).toEqual(["error 1", "error 2"]);
  });

  it("should filter out invalid errors", () => {
    const payload = {
      errors: ["valid error", 123, null, "another error", undefined],
    };
    const result = validatePayload(payload);
    expect(result.errors).toEqual(["valid error", "another error"]);
  });

  it("should handle partial payloads", () => {
    const payload = { sessions: [{ id: "s1", status: "online" }] };
    const result = validatePayload(payload);
    expect(result).not.toBeNull();
    expect(result.sessions).toHaveLength(1);
    expect(result.recentTasks).toBeUndefined();
    expect(result.metrics).toBeUndefined();
  });

  it("should handle empty arrays", () => {
    const payload = {
      sessions: [],
      recentTasks: [],
      errors: [],
    };
    const result = validatePayload(payload);
    // Returns null when all arrays are empty (no valid data)
    expect(result).toBeNull();
  });
});
