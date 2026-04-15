/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import SessionManager from "@api/core/sessionManager.js";
import { SimpleSemaphore } from "@api/core/sessionManager.js";

// Mocks
const mockSqlite = {
  pragma: vi.fn(),
  exec: vi.fn(),
  prepare: vi.fn().mockReturnValue({
    run: vi.fn(),
    all: vi.fn().mockReturnValue([]),
    get: vi.fn(),
  }),
  transaction: vi.fn((fn) => fn),
  close: vi.fn(),
};

vi.mock("better-sqlite3", () => ({
  default: vi.fn(() => mockSqlite),
}));

vi.mock("@api/utils/configLoader.js", () => ({
  getTimeoutValue: vi.fn().mockResolvedValue({}),
  getSettings: vi.fn().mockResolvedValue({ concurrencyPerBrowser: 1 }),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/metrics.js", () => ({
  default: {
    increment: vi.fn(),
    decrement: vi.fn(),
    gauge: vi.fn(),
    timing: vi.fn(),
    recordSessionEvent: vi.fn(),
  },
}));

import { getTimeoutValue } from "@api/utils/configLoader.js";

describe("SessionManager", () => {
  let manager;

  beforeEach(async () => {
    vi.clearAllMocks();
    getTimeoutValue.mockResolvedValue({});
    manager = new SessionManager();
    await new Promise((resolve) => setTimeout(resolve, 50));
  });

  afterEach(async () => {
    if (manager) {
      manager.stopCleanupTimer();
    }
  });

  it("should initialize with default values", () => {
    expect(manager.sessions).toEqual([]);
    expect(manager.concurrencyPerBrowser).toBe(1);
  });

  it("should add a session", () => {
    const browser = {
      close: vi.fn(),
      contexts: () => [],
      isConnected: vi.fn().mockReturnValue(true),
    };
    const id = manager.addSession(browser, "test-profile");
    expect(id).toBe("test-profile");
    expect(manager.sessions.length).toBe(1);
  });

  it("should remove a session", async () => {
    const browser = {
      close: vi.fn(),
      contexts: () => [],
      isConnected: vi.fn().mockReturnValue(true),
    };
    const id = manager.addSession(browser, "test-profile");
    await manager.removeSession(id);
    expect(manager.sessions.length).toBe(0);
  });

  it("should acquire and release a worker", async () => {
    const browser = {
      close: vi.fn(),
      contexts: () => [],
      isConnected: vi.fn().mockReturnValue(true),
      newContext: vi.fn().mockResolvedValue({
        newPage: vi.fn().mockResolvedValue({ on: vi.fn(), close: vi.fn() }),
      }),
    };
    const id = manager.addSession(browser, "test-profile");
    const worker = await manager.acquireWorker(id);
    expect(worker).toBeDefined();
    expect(worker.status).toBe("busy");

    await manager.releaseWorker(id, worker.id);
    expect(worker.status).toBe("idle");
  });

  it("should handle session timeout", async () => {
    const browser = {
      close: vi.fn(),
      contexts: () => [],
      isConnected: vi.fn().mockReturnValue(true),
    };
    manager.sessionTimeoutMs = 10;
    const id = manager.addSession(browser, "test-profile");

    const session = manager.sessions[0];
    session.lastActivity = Date.now() - 100;

    const removedCount = await manager.cleanupTimedOutSessions();
    expect(removedCount).toBe(1);
    expect(manager.sessions.length).toBe(0);
  });

  describe("SimpleSemaphore", () => {
    it("should acquire and release permit", async () => {
      const sem = new SimpleSemaphore(1);
      const p1 = await sem.acquire();
      expect(p1).toBe(true);

      let p2Acquired = false;
      sem.acquire().then(() => {
        p2Acquired = true;
      });

      expect(p2Acquired).toBe(false);
      sem.release();

      await new Promise((r) => setTimeout(r, 10));
      expect(p2Acquired).toBe(true);
    });

    it("should timeout if permit not available", async () => {
      const sem = new SimpleSemaphore(1);
      await sem.acquire();
      const p2 = await sem.acquire(10);
      expect(p2).toBe(false);
    });
  });

  describe("Session map operations", () => {
    it("should track sessions in map", () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      manager.addSession(browser, "session1");
      expect(manager.sessionsMap.has("session1")).toBe(true);
    });

    it("should get session by id", () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      manager.addSession(browser, "session2");
      const session = manager.getSession("session2");
      expect(session).toBeDefined();
      expect(session.id).toBe("session2");
    });

    it("should return all sessions", () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      manager.addSession(browser, "s3");
      manager.addSession(browser, "s4");
      const all = manager.getAllSessions();
      expect(all.length).toBe(2);
    });
  });

  describe("Worker occupancy tracking", () => {
    it("should track worker occupancy after acquire", async () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      const id = manager.addSession(browser, "worker-test");
      const worker = await manager.acquireWorker(id);

      const occupancyKey = `${id}:${worker.id}`;
      const occupancy = manager.workerOccupancy.get(occupancyKey);
      expect(occupancy).toBeDefined();
      expect(occupancy.startTime).toBeDefined();
    });
  });

  describe("Page management", () => {
    it("should handle null page release", async () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      const id = manager.addSession(browser, "null-page-test");

      await manager.releasePage(id, null);
    });
  });

  describe("Active/idle session counts", () => {
    it("should count active sessions", () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      manager.addSession(browser, "count1");
      manager.addSession(browser, "count2");
      expect(manager.activeSessionsCount).toBe(2);
    });

    it("should count idle workers per session", async () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      const id = manager.addSession(browser, "idle-test");
      await manager.acquireWorker(id);
      expect(manager.idleSessionsCount).toBe(0);
    });
  });

  describe("Health check operations", () => {
    it("should stop cleanup timer", () => {
      manager.stopCleanupTimer();
      expect(manager.cleanupInterval).toBeNull();
    });
  });

  describe("Advanced functionality", () => {
    it("should shutdown all sessions", async () => {
      const browser = {
        close: vi.fn(),
        contexts: () => [],
        isConnected: vi.fn().mockReturnValue(true),
      };
      manager.addSession(browser, "s1");
      manager.addSession(browser, "s2");

      await manager.shutdown();
      expect(browser.close).toHaveBeenCalledTimes(2);
      expect(manager.sessions.length).toBe(0);
    });

    it("should recover sessions from database", async () => {
      const mockAll = vi
        .fn()
        .mockReturnValueOnce([
          {
            id: "db-session",
            browserInfo: "{}",
            wsEndpoint: "ws://",
            workers: "[]",
            createdAt: Date.now(),
            lastActivity: Date.now(),
          },
        ])
        .mockReturnValueOnce([{ key: "nextSessionId", value: "5" }]);

      manager.db = {
        prepare: vi.fn().mockReturnValue({
          all: mockAll,
        }),
      };

      const state = await manager.loadSessionState();
      expect(state).not.toBeNull();
      expect(state.rows.length).toBe(1);
      expect(state.rows[0].id).toBe("db-session");
      expect(state.meta.nextSessionId).toBe("5");
    });
  });
});
