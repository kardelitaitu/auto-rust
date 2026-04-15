/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  withPage,
  isSessionActive,
  checkSession,
  getStore,
  getPage,
  getCursor,
  getEvents,
  getPlugins,
  evalPage,
  getInterval,
  setSessionInterval,
  clearSessionInterval,
  clearContext,
} from "@api/core/context.js";
import { loggerContext } from "@api/core/logger.js";

vi.mock("@api/tests/core/logger.js", () => ({
  loggerContext: {
    getStore: vi.fn(),
    run: vi.fn((ctx, fn) => fn()),
  },
}));

vi.mock("@api/tests/utils/ghostCursor.js", () => ({
  GhostCursor: vi.fn(),
}));

vi.mock("@api/tests/core/context-state.js", () => ({
  getDefaultState: vi.fn().mockReturnValue({}),
  setContextStore: vi.fn(),
}));

vi.mock("@api/tests/core/events.js", () => ({
  APIEvents: vi.fn(),
}));

vi.mock("@api/tests/core/plugins/manager.js", () => ({
  PluginManager: vi.fn(),
}));

describe("api/core/context.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    clearContext();

    mockPage = {
      isClosed: vi.fn().mockReturnValue(false),
      on: vi.fn(),
      context: vi.fn().mockReturnValue({
        browser: vi.fn().mockReturnValue({
          isConnected: vi.fn().mockReturnValue(true),
        }),
      }),
      evaluate: vi.fn().mockResolvedValue("result"),
    };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("withPage / getPage", () => {
    it("should set and get page via withPage", async () => {
      await withPage(mockPage, async () => {
        expect(getPage()).toBe(mockPage);
      });
    });

    it("should throw if page is invalid", async () => {
      await expect(withPage(null, async () => {})).rejects.toThrow(
        "withPage requires a valid Playwright page instance",
      );
    });

    it("should throw if page is closed", async () => {
      mockPage.isClosed.mockReturnValue(true);
      await expect(
        withPage(mockPage, async () => {
          getPage();
        }),
      ).rejects.toThrow();
    });
  });

  describe("isSessionActive", () => {
    it("should return true if session is active", async () => {
      await withPage(mockPage, async () => {
        expect(isSessionActive()).toBe(true);
      });
    });

    it("should return false if no page", async () => {
      clearContext();
      expect(isSessionActive()).toBe(false);
    });

    it("should return false if page closed", async () => {
      mockPage.isClosed.mockReturnValue(true);
      await withPage(mockPage, async () => {
        expect(isSessionActive()).toBe(false);
      });
    });

    it("should return false when browser is disconnected", async () => {
      await withPage(mockPage, async () => {
        mockPage.context.mockReturnValue({
          browser: vi.fn().mockReturnValue({
            isConnected: vi.fn().mockReturnValue(false),
          }),
        });
        expect(isSessionActive()).toBe(false);
      });
    });
  });

  describe("checkSession", () => {
    it("should throw if no context", () => {
      clearContext();
      expect(() => checkSession()).toThrow("API context not initialized");
    });

    it("should throw when page is closed", async () => {
      mockPage.isClosed.mockReturnValue(true);
      await withPage(mockPage, async () => {
        expect(() => checkSession()).toThrow("Page has been closed");
      });
    });

    it("should throw when browser connection is lost", async () => {
      await withPage(mockPage, async () => {
        mockPage.context.mockReturnValue({
          browser: vi.fn().mockReturnValue({
            isConnected: vi.fn().mockReturnValue(false),
          }),
        });
        expect(() => checkSession()).toThrow("Session has been disconnected");
      });
    });
  });

  describe("withPage", () => {
    it("should execute function in context", async () => {
      const result = await withPage(mockPage, async () => {
        expect(getPage()).toBe(mockPage);
        return "success";
      });
      expect(result).toBe("success");
    });

    it("should pass through sessionId from existing logger context", async () => {
      const getStoreSpy = vi.spyOn(loggerContext, "getStore");
      const runSpy = vi
        .spyOn(loggerContext, "run")
        .mockImplementation(async (ctx, fn) => {
          getStoreSpy.mockReturnValue(ctx);
          return fn();
        });

      getStoreSpy.mockReturnValue({
        sessionId: "existing-session-123",
        traceId: "existing-trace-456",
      });

      const result = await withPage(mockPage, async () => {
        const ctx = loggerContext.getStore();
        return ctx.sessionId;
      });
      expect(result).toBe("existing-session-123");

      getStoreSpy.mockRestore();
      runSpy.mockRestore();
    });

    it("should generate new sessionId when no logger context exists", async () => {
      const getStoreSpy = vi.spyOn(loggerContext, "getStore");
      const runSpy = vi
        .spyOn(loggerContext, "run")
        .mockImplementation(async (ctx, fn) => {
          getStoreSpy.mockReturnValue(ctx);
          return fn();
        });

      getStoreSpy.mockReturnValue(null);

      let capturedSessionId;
      const result = await withPage(mockPage, async () => {
        capturedSessionId = loggerContext.getStore()?.sessionId;
        return capturedSessionId;
      });
      expect(capturedSessionId).toMatch(/^session-[a-f0-9]+$/);

      getStoreSpy.mockRestore();
      runSpy.mockRestore();
    });

    it("should handle errors thrown inside the callback", async () => {
      const getStoreSpy = vi.spyOn(loggerContext, "getStore");
      const runSpy = vi
        .spyOn(loggerContext, "run")
        .mockImplementation(async (ctx, fn) => fn());

      getStoreSpy.mockReturnValue(null);

      await expect(
        withPage(mockPage, async () => {
          throw new Error("Test error");
        }),
      ).rejects.toThrow("Test error");

      getStoreSpy.mockRestore();
      runSpy.mockRestore();
    });

    it("should throw if page is invalid", async () => {
      await expect(withPage(null, async () => {})).rejects.toThrow(
        "withPage requires a valid Playwright page instance",
      );
    });
  });

  describe("clearContext", () => {
    it("should clear the context", async () => {
      await withPage(mockPage, async () => {
        expect(getPage()).toBe(mockPage);
        clearContext();
        expect(isSessionActive()).toBe(false);
      });
    });

    it("should clear session intervals before tearing down the context", async () => {
      const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");

      await withPage(mockPage, async () => {
        const store = getStore();
        store.intervals.set("poll-a", 101);
        store.intervals.set("poll-b", 202);

        clearContext();

        expect(clearIntervalSpy).toHaveBeenCalledWith(101);
        expect(clearIntervalSpy).toHaveBeenCalledWith(202);
        expect(store.intervals.size).toBe(0);
      });

      clearIntervalSpy.mockRestore();
    });
  });

  describe("getPage", () => {
    it("should throw with proper message when no context", () => {
      clearContext();
      expect(() => getPage()).toThrow("API context not initialized");
    });
  });

  describe("getCursor", () => {
    it("should return cursor", async () => {
      await withPage(mockPage, async () => {
        const cursor = getCursor();
        expect(cursor).toBeDefined();
      });
    });
  });

  describe("getEvents", () => {
    it("should return events", async () => {
      await withPage(mockPage, async () => {
        const events = getEvents();
        expect(events).toBeDefined();
      });
    });
  });

  describe("getPlugins", () => {
    it("should return plugin manager", async () => {
      await withPage(mockPage, async () => {
        const plugins = getPlugins();
        expect(plugins).toBeDefined();
      });
    });
  });

  describe("evalPage", () => {
    it("should evaluate on page", async () => {
      await withPage(mockPage, async () => {
        const result = await evalPage((value) => value.toUpperCase(), "test");
        expect(result).toBe("result");
        expect(mockPage.evaluate).toHaveBeenCalledWith(
          expect.any(Function),
          "test",
        );
      });
    });
  });

  describe("session intervals", () => {
    it("should return null for an unknown interval without an active store", () => {
      clearContext();
      expect(getInterval("missing")).toBeNull();
    });

    it("should return the active interval id when one is registered", async () => {
      await withPage(mockPage, async () => {
        const store = getStore();
        store.intervals.set("heartbeat", 321);
        expect(getInterval("heartbeat")).toBe(321);
      });
    });

    it("should not create an interval when no store exists", () => {
      clearContext();
      expect(setSessionInterval("heartbeat", vi.fn(), 50)).toBeUndefined();
    });

    it("should clear a previously registered interval before replacing it", async () => {
      const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
      const fn = vi.fn();

      await withPage(mockPage, async () => {
        const store = getStore();
        store.intervals.set("heartbeat", 111);

        const nextId = setSessionInterval("heartbeat", fn, 50);

        expect(clearIntervalSpy).toHaveBeenCalledWith(111);
        expect(store.intervals.get("heartbeat")).toBe(nextId);
      });

      clearIntervalSpy.mockRestore();
    });

    it("should execute the interval callback while the page is alive", async () => {
      const fn = vi.fn();

      await withPage(mockPage, async () => {
        setSessionInterval("heartbeat", fn, 50);
        await vi.advanceTimersByTimeAsync(50);
        expect(fn).toHaveBeenCalledTimes(1);
      });
    });

    it("should auto-clean up the interval when the page is closed", async () => {
      const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
      const fn = vi.fn();

      await withPage(mockPage, async () => {
        mockPage.isClosed.mockReturnValue(true);

        setSessionInterval("heartbeat", fn, 50);
        await vi.advanceTimersByTimeAsync(50);

        expect(fn).not.toHaveBeenCalled();
        expect(clearIntervalSpy).toHaveBeenCalled();
        expect(getInterval("heartbeat")).toBeNull();
      });

      clearIntervalSpy.mockRestore();
    });

    it("should stop the interval when the callback throws a session-related error", async () => {
      const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
      const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

      await withPage(mockPage, async () => {
        setSessionInterval(
          "heartbeat",
          async () => {
            throw new Error("SessionDisconnectedError: closed");
          },
          50,
        );

        await vi.advanceTimersByTimeAsync(50);

        expect(clearIntervalSpy).toHaveBeenCalled();
        expect(getInterval("heartbeat")).toBeNull();
      });

      debugSpy.mockRestore();
      clearIntervalSpy.mockRestore();
    });

    it("should clear a registered interval on explicit clear", async () => {
      const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");

      await withPage(mockPage, async () => {
        const store = getStore();
        store.intervals.set("heartbeat", 444);

        clearSessionInterval("heartbeat");

        expect(clearIntervalSpy).toHaveBeenCalledWith(444);
        expect(getInterval("heartbeat")).toBeNull();
      });

      clearIntervalSpy.mockRestore();
    });

    it("should ignore clear requests without an active store", () => {
      clearContext();
      expect(() => clearSessionInterval("heartbeat")).not.toThrow();
    });
  });
});
