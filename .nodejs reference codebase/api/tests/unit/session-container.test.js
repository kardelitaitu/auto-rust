/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Unit tests for SessionContainer
 * @module tests/unit/session-container.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  }),
}));

vi.mock("@api/core/errors.js", () => ({
  SessionClosedError: class SessionClosedError extends Error {
    constructor(sessionId) {
      super(`Session ${sessionId} is closed`);
      this.name = "SessionClosedError";
    }
  },
  SessionDisconnectedError: class SessionDisconnectedError extends Error {
    constructor() {
      super("Session disconnected");
      this.name = "SessionDisconnectedError";
    }
  },
}));

import SessionContainer from "@api/core/session-container.js";

describe("SessionContainer", () => {
  let container;
  let mockPage;

  beforeEach(() => {
    mockPage = {
      isClosed: vi.fn().mockReturnValue(false),
      close: vi.fn().mockResolvedValue(undefined),
    };
    container = new SessionContainer("test-session-1", mockPage);
  });

  describe("constructor", () => {
    it("should create container with sessionId and page", () => {
      expect(container.sessionId).toBe("test-session-1");
      expect(container.page).toBe(mockPage);
      expect(container.isClosed()).toBe(false);
      expect(container.intervals).toBeDefined();
      expect(container.locks).toBeDefined();
      expect(container.state).toBeDefined();
    });
  });

  describe("verify", () => {
    it("should not throw when session is active", () => {
      expect(() => container.verify()).not.toThrow();
    });

    it("should throw when session is closed", async () => {
      await container.close();
      expect(() => container.verify()).toThrow("closed");
    });

    it("should throw when page is disconnected", () => {
      mockPage.isClosed.mockReturnValue(true);
      expect(() => container.verify()).toThrow("disconnected");
    });
  });

  describe("isConnected", () => {
    it("should return true when page is not closed", () => {
      expect(container.isConnected()).toBe(true);
    });

    it("should return false when page is closed", () => {
      mockPage.isClosed.mockReturnValue(true);
      expect(container.isConnected()).toBe(false);
    });
  });

  describe("uptime", () => {
    it("should return uptime in milliseconds", () => {
      const uptime = container.uptime();
      expect(typeof uptime).toBe("number");
      expect(uptime).toBeGreaterThanOrEqual(0);
    });
  });

  describe("acquireLock", () => {
    it("should acquire and release lock", async () => {
      const lock = await container.acquireLock("resource1");
      expect(container.locks.isLocked("test-session-1:resource1")).toBe(true);
      await lock.release();
      expect(container.locks.isLocked("test-session-1:resource1")).toBe(false);
    });
  });

  describe("setInterval/clearInterval", () => {
    it("should set and clear intervals", () => {
      const fn = vi.fn();
      container.setInterval("test-interval", fn, 100);
      expect(container.intervals.has("test-interval")).toBe(true);

      container.clearInterval("test-interval");
      expect(container.intervals.has("test-interval")).toBe(false);
    });

    it("should throw when session is closed", async () => {
      await container.close();
      expect(() => container.setInterval("test", () => {}, 100)).toThrow(
        "closed",
      );
    });
  });

  describe("setState/getState", () => {
    it("should store and retrieve state", () => {
      container.setState("name", "test-value");
      expect(container.getState("name")).toBe("test-value");
    });

    it("should throw when session is closed", async () => {
      await container.close();
      expect(() => container.setState("key", "value")).toThrow("closed");
      expect(() => container.getState("key")).toThrow("closed");
    });
  });

  describe("close", () => {
    it("should close container and cleanup resources", async () => {
      container.setState("test", "value");
      container.setInterval("test-interval", () => {}, 100);

      await container.close();

      expect(container.isClosed()).toBe(true);
      expect(container.intervals.size()).toBe(0);
      expect(mockPage.close).toHaveBeenCalled();
    });

    it("should not throw when already closed", async () => {
      await container.close();
      await expect(container.close()).resolves.not.toThrow();
    });

    it("should handle page close errors gracefully", async () => {
      mockPage.close.mockRejectedValue(new Error("Browser closed"));
      await container.close();
      expect(container.isClosed()).toBe(true);
    });
  });

  describe("getStatus", () => {
    it("should return complete status", () => {
      const status = container.getStatus();
      expect(status.sessionId).toBe("test-session-1");
      expect(status.closed).toBe(false);
      expect(status.uptime).toBeGreaterThanOrEqual(0);
      expect(status.intervalsCount).toBe(0);
    });

    it("should reflect closed status", async () => {
      await container.close();
      const status = container.getStatus();
      expect(status.closed).toBe(true);
    });
  });
});
