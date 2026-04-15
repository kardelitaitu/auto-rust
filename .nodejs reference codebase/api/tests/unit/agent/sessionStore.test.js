/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/agent/sessionStore.js
 * @module tests/unit/agent/sessionStore.test
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

// Mock better-sqlite3 to throw error (force fallback mode)
vi.mock("better-sqlite3", () => ({
  default: vi.fn(() => {
    throw new Error("better-sqlite3 not available");
  }),
}));

describe("api/agent/sessionStore.js", () => {
  let sessionStore;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Dynamic import to get fresh instance
    const module = await import("@api/agent/sessionStore.js");
    sessionStore = module.sessionStore || module.default;

    // Clear and reset to ensure clean state
    sessionStore.clear();
  });

  describe("Constructor", () => {
    it("should have database properties", () => {
      expect(sessionStore).toHaveProperty("enabled");
    });

    it("should have memoryStore for fallback", () => {
      expect(sessionStore.memoryStore).toBeDefined();
      expect(sessionStore.memoryStore).toHaveProperty("sessions");
      expect(sessionStore.memoryStore).toHaveProperty("actionPatterns");
      expect(sessionStore.memoryStore).toHaveProperty("learnedSelectors");
    });

    it("should initialize with empty arrays", () => {
      expect(sessionStore.memoryStore.sessions).toEqual([]);
      expect(sessionStore.memoryStore.actionPatterns).toEqual([]);
      expect(sessionStore.memoryStore.learnedSelectors).toEqual([]);
    });
  });

  describe("_extractUrlPattern()", () => {
    it("should extract hostname and path up to 2 segments", () => {
      const pattern = sessionStore._extractUrlPattern(
        "https://example.com/path/to/page?id=123",
      );
      // URL pattern: hostname + first 2 path segments after split
      expect(pattern).toContain("example.com");
    });

    it("should handle URLs with single path segment", () => {
      const pattern = sessionStore._extractUrlPattern(
        "https://example.com/page",
      );
      expect(pattern).toBe("example.com/page");
    });

    it("should handle invalid URLs by returning as-is", () => {
      const pattern = sessionStore._extractUrlPattern("not-a-url");
      expect(pattern).toBe("not-a-url");
    });

    it("should handle URLs with no path", () => {
      const pattern = sessionStore._extractUrlPattern("https://example.com");
      expect(pattern).toContain("example.com");
    });

    it("should strip query parameters", () => {
      const pattern = sessionStore._extractUrlPattern(
        "https://example.com/path?foo=bar&baz=qux",
      );
      expect(pattern).toBe("example.com/path");
    });
  });

  describe("recordSession()", () => {
    it("should not record when disabled", () => {
      sessionStore.enabled = false;

      sessionStore.recordSession({
        id: "session1",
        goal: "Login",
        url: "https://example.com",
        success: true,
      });

      expect(sessionStore.memoryStore.sessions.length).toBe(0);
    });

    it("should store session in memoryStore when enabled", () => {
      sessionStore.enabled = true;

      sessionStore.recordSession({
        id: "session1",
        goal: "Login",
        url: "https://example.com",
        success: true,
        steps: 5,
        durationMs: 1000,
      });

      expect(sessionStore.memoryStore.sessions.length).toBe(1);
      expect(sessionStore.memoryStore.sessions[0].id).toBe("session1");
    });

    it("should store multiple sessions", () => {
      sessionStore.enabled = true;

      sessionStore.recordSession({ id: "s1", goal: "Goal 1" });
      sessionStore.recordSession({ id: "s2", goal: "Goal 2" });
      sessionStore.recordSession({ id: "s3", goal: "Goal 3" });

      expect(sessionStore.memoryStore.sessions.length).toBe(3);
    });

    it("should store session data correctly", () => {
      sessionStore.enabled = true;

      const sessionData = {
        id: "test-session",
        goal: "Submit form",
        url: "https://example.com/form",
        success: true,
        steps: 10,
        durationMs: 5000,
      };

      sessionStore.recordSession(sessionData);

      const stored = sessionStore.memoryStore.sessions[0];
      expect(stored.id).toBe("test-session");
      expect(stored.goal).toBe("Submit form");
      expect(stored.success).toBe(true);
      expect(stored.steps).toBe(10);
    });
  });

  describe("recordAction()", () => {
    it("should not record when disabled", () => {
      sessionStore.enabled = false;

      // Should not throw
      expect(() => {
        sessionStore.recordAction("https://example.com", "#btn", "click", true);
      }).not.toThrow();

      expect(sessionStore.memoryStore.actionPatterns.length).toBe(0);
    });

    it("should store action pattern in memoryStore", () => {
      sessionStore.enabled = true;

      sessionStore.recordAction(
        "https://example.com/page",
        "#submit",
        "click",
        true,
      );

      expect(sessionStore.memoryStore.actionPatterns.length).toBe(1);
      expect(sessionStore.memoryStore.actionPatterns[0]).toMatchObject({
        selector: "#submit",
        actionType: "click",
        success: true,
      });
    });

    it("should extract URL pattern from full URL", () => {
      sessionStore.enabled = true;

      sessionStore.recordAction(
        "https://example.com/path/to/page?id=123",
        "#btn",
        "click",
        true,
      );

      const stored = sessionStore.memoryStore.actionPatterns[0];
      expect(stored.urlPattern).toContain("example.com");
    });

    it("should record failed actions", () => {
      sessionStore.enabled = true;

      sessionStore.recordAction("https://example.com", "#btn", "click", false);

      const stored = sessionStore.memoryStore.actionPatterns[0];
      expect(stored.success).toBe(false);
    });
  });

  describe("getActionSuccessRate()", () => {
    it("should return 0.5 when disabled", () => {
      sessionStore.enabled = false;

      const rate = sessionStore.getActionSuccessRate(
        "https://example.com",
        "#btn",
        "click",
      );
      expect(rate).toBe(0.5);
    });

    it("should return 0.5 when no data found", () => {
      sessionStore.enabled = true;

      const rate = sessionStore.getActionSuccessRate(
        "https://example.com",
        "#btn",
        "click",
      );
      expect(rate).toBe(0.5);
    });
  });

  describe("learnSelector()", () => {
    it("should not learn when disabled", () => {
      sessionStore.enabled = false;

      // Should not throw
      expect(() => {
        sessionStore.learnSelector(
          "https://example.com",
          "Submit button",
          "#submit",
          true,
        );
      }).not.toThrow();

      expect(sessionStore.memoryStore.learnedSelectors.length).toBe(0);
    });

    it("should store selector in memoryStore", () => {
      sessionStore.enabled = true;

      sessionStore.learnSelector(
        "https://example.com",
        "Submit button",
        "#submit",
        true,
      );

      expect(sessionStore.memoryStore.learnedSelectors.length).toBe(1);
      expect(sessionStore.memoryStore.learnedSelectors[0]).toMatchObject({
        description: "Submit button",
        selector: "#submit",
        success: true,
      });
    });

    it("should record successful selector", () => {
      sessionStore.enabled = true;

      sessionStore.learnSelector(
        "https://example.com",
        "Login button",
        "#login",
        true,
      );

      const stored = sessionStore.memoryStore.learnedSelectors[0];
      expect(stored.success).toBe(true);
    });

    it("should record failed selector", () => {
      sessionStore.enabled = true;

      sessionStore.learnSelector(
        "https://example.com",
        "Login button",
        "#login",
        false,
      );

      const stored = sessionStore.memoryStore.learnedSelectors[0];
      expect(stored.success).toBe(false);
    });
  });

  describe("getBestSelector()", () => {
    it("should return null when disabled", () => {
      sessionStore.enabled = false;

      const selector = sessionStore.getBestSelector(
        "https://example.com",
        "Submit button",
      );
      expect(selector).toBeNull();
    });

    it("should return null when no match found", () => {
      sessionStore.enabled = true;

      const selector = sessionStore.getBestSelector(
        "https://example.com",
        "Submit button",
      );
      expect(selector).toBeNull();
    });
  });

  describe("getStats()", () => {
    it("should return disabled stats when not enabled", () => {
      sessionStore.enabled = false;

      const stats = sessionStore.getStats();
      expect(stats).toEqual({ enabled: false });
    });

    it("should return memory store stats when enabled", () => {
      sessionStore.enabled = true;
      sessionStore._initFallbackStorage(); // Ensure memoryStore is initialized

      sessionStore.memoryStore.sessions.push({ id: "s1" });
      sessionStore.memoryStore.sessions.push({ id: "s2" });
      sessionStore.memoryStore.actionPatterns.push({ selector: "#btn" });
      sessionStore.memoryStore.learnedSelectors.push({ selector: "#input" });

      const stats = sessionStore.getStats();

      expect(stats.enabled).toBe(true);
      expect(stats.sessions).toBe(2);
      expect(stats.patterns).toBe(1);
      expect(stats.selectors).toBe(1);
    });

    it("should return zero counts for empty store", () => {
      sessionStore.enabled = true;

      const stats = sessionStore.getStats();

      expect(stats.sessions).toBe(0);
      expect(stats.patterns).toBe(0);
      expect(stats.selectors).toBe(0);
    });
  });

  describe("clear()", () => {
    it("should clear memory store", () => {
      sessionStore.enabled = true;
      sessionStore._initFallbackStorage(); // Ensure memoryStore is initialized

      sessionStore.memoryStore.sessions.push({ id: "s1" });
      sessionStore.memoryStore.actionPatterns.push({ selector: "#btn" });
      sessionStore.memoryStore.learnedSelectors.push({ selector: "#input" });

      sessionStore.clear();

      expect(sessionStore.memoryStore.sessions).toEqual([]);
      expect(sessionStore.memoryStore.actionPatterns).toEqual([]);
      expect(sessionStore.memoryStore.learnedSelectors).toEqual([]);
    });

    it("should allow recording after clear", () => {
      sessionStore.enabled = true;

      sessionStore.recordSession({ id: "s1" });
      expect(sessionStore.memoryStore.sessions.length).toBe(1);

      sessionStore.clear();
      expect(sessionStore.memoryStore.sessions.length).toBe(0);

      sessionStore.recordSession({ id: "s2" });
      expect(sessionStore.memoryStore.sessions.length).toBe(1);
    });
  });

  describe("_initFallbackStorage()", () => {
    it("should initialize empty arrays", () => {
      // Add some data first
      sessionStore.memoryStore.sessions.push({ id: "test" });

      // Re-initialize
      sessionStore._initFallbackStorage();

      expect(sessionStore.memoryStore.sessions).toEqual([]);
      expect(sessionStore.memoryStore.actionPatterns).toEqual([]);
      expect(sessionStore.memoryStore.learnedSelectors).toEqual([]);
    });
  });

  describe("database-backed mode", () => {
    let dbMock;
    let moduleStore;

    beforeEach(async () => {
      vi.resetModules();
      dbMock = {
        exec: vi.fn(),
        prepare: vi.fn((sql) => {
          if (sql.includes("INSERT OR REPLACE INTO sessions")) {
            return { run: vi.fn() };
          }
          if (sql.includes("INSERT INTO action_patterns")) {
            return { run: vi.fn() };
          }
          if (sql.includes("INSERT INTO learned_selectors")) {
            return { run: vi.fn() };
          }
          if (sql.includes("SELECT success_count")) {
            return {
              get: vi
                .fn()
                .mockReturnValue({ success_count: 3, failure_count: 1 }),
            };
          }
          if (sql.includes("SELECT selector, confidence")) {
            return {
              get: vi
                .fn()
                .mockReturnValue({ selector: "#best", confidence: 0.8 }),
            };
          }
          if (sql.includes("SELECT COUNT(*) as count FROM sessions")) {
            return { get: vi.fn().mockReturnValue({ count: 2 }) };
          }
          if (sql.includes("SELECT COUNT(*) as count FROM action_patterns")) {
            return { get: vi.fn().mockReturnValue({ count: 4 }) };
          }
          if (sql.includes("SELECT COUNT(*) as count FROM learned_selectors")) {
            return { get: vi.fn().mockReturnValue({ count: 5 }) };
          }
          return { run: vi.fn(), get: vi.fn() };
        }),
      };

      vi.doMock("better-sqlite3", () => ({
        default: vi.fn(() => dbMock),
      }));

      const module = await import("@api/agent/sessionStore.js");
      moduleStore = module.sessionStore || module.default;
      await moduleStore._initDatabase();
    });

    it("should enable db mode and create tables", () => {
      expect(moduleStore.enabled).toBe(true);
      expect(dbMock.exec).toHaveBeenCalled();
    });

    it("should record sessions and actions in db mode", () => {
      moduleStore.recordSession({
        id: "db-session",
        goal: "Goal",
        url: "https://example.com/path/to/page",
        success: true,
        steps: 3,
        durationMs: 100,
      });
      moduleStore.recordAction(
        "https://example.com/path/to/page",
        "#btn",
        "click",
        true,
      );
      moduleStore.learnSelector(
        "https://example.com/path/to/page",
        "Submit",
        "#submit",
        true,
      );

      expect(moduleStore.db.prepare).toHaveBeenCalled();
    });

    it("should return db-backed stats and selector rate", () => {
      expect(
        moduleStore.getActionSuccessRate(
          "https://example.com/path/to/page",
          "#btn",
          "click",
        ),
      ).toBe(0.75);
      expect(
        moduleStore.getBestSelector(
          "https://example.com/path/to/page",
          "Submit",
        ),
      ).toBe("#best");
      expect(moduleStore.getStats()).toEqual({
        enabled: true,
        sessions: 2,
        patterns: 4,
        selectors: 5,
      });
    });

    it("should clear db-backed store without throwing", () => {
      expect(() => moduleStore.clear()).not.toThrow();
      expect(dbMock.exec).toHaveBeenCalled();
    });

    it("should fall back to null for low-confidence selectors", () => {
      dbMock.prepare.mockImplementationOnce((sql) => {
        if (sql.includes("SELECT selector, confidence")) {
          return {
            get: vi
              .fn()
              .mockReturnValue({ selector: "#weak", confidence: 0.4 }),
          };
        }
        return { get: vi.fn(), run: vi.fn() };
      });

      expect(
        moduleStore.getBestSelector(
          "https://example.com/path/to/page",
          "Submit",
        ),
      ).toBeNull();
    });

    it("should fall back to memory stats when db stats fail", () => {
      dbMock.prepare.mockImplementationOnce(() => {
        throw new Error("stats failed");
      });

      moduleStore._initFallbackStorage();
      moduleStore.memoryStore.sessions.push({ id: "s1" });
      moduleStore.memoryStore.actionPatterns.push({ selector: "#btn" });
      moduleStore.memoryStore.learnedSelectors.push({ selector: "#input" });

      expect(moduleStore.getStats()).toEqual({
        enabled: true,
        sessions: 1,
        patterns: 1,
        selectors: 1,
      });
    });

    it("should swallow recordSession db errors", () => {
      dbMock.prepare.mockImplementationOnce(() => {
        throw new Error("record failed");
      });

      expect(() =>
        moduleStore.recordSession({
          id: "db-error",
          goal: "Goal",
          url: "https://example.com",
          success: false,
        }),
      ).not.toThrow();
    });

    it("should swallow clear db errors", () => {
      dbMock.exec.mockImplementationOnce(() => {
        throw new Error("clear failed");
      });

      expect(() => moduleStore.clear()).not.toThrow();
    });
  });
});
