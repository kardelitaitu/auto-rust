/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { EventEmitter } from "events";
import {
  APIEvents,
  apiEvents,
  getAvailableHooks,
  getHookDescription,
  createHookWrapper,
  withErrorHook,
} from "@api/core/events.js";

describe("APIEvents", () => {
  let events;

  beforeEach(() => {
    events = new APIEvents();
  });

  describe("basics", () => {
    it("should be an instance of EventEmitter", () => {
      expect(events).toBeInstanceOf(EventEmitter);
    });

    it("should have custom max listeners", () => {
      expect(events.getMaxListeners()).toBe(50);
    });
  });

  describe("getAvailableHooks", () => {
    it("should return all hook keys", () => {
      const hooks = getAvailableHooks();
      expect(hooks).toContain("before:init");
      expect(hooks).toContain("on:error");
    });
  });

  describe("getHookDescription", () => {
    it("should return description for valid hook", () => {
      expect(getHookDescription("on:error")).toBe("Called on any error");
    });

    it("should return undefined for invalid hook", () => {
      expect(getHookDescription("non:existent")).toBeUndefined();
    });
  });

  describe("emitAsync", () => {
    it("should call all handlers and return results", async () => {
      events.on("test", async (val) => val + 1);
      events.on("test", async (val) => val + 2);

      const results = await events.emitAsync("test", 10);
      expect(results).toEqual([11, 12]);
    });

    it("should handle rejected promises and return null", async () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      events.on("test", async () => {
        throw new Error("fail");
      });
      events.on("test", async () => "success");

      const results = await events.emitAsync("test");
      expect(results).toEqual([null, "success"]);
      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });

    it("should handle rejected promises without message", async () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      events.on("test", async () => {
        throw "no message";
      });

      const results = await events.emitAsync("test");
      expect(results).toEqual([null]);
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.any(String),
        expect.stringContaining("no message"),
      );
      consoleSpy.mockRestore();
    });
  });

  describe("emitStrict", () => {
    it("should return results if all succeed", async () => {
      events.on("test", async () => "ok");
      const results = await events.emitStrict("test");
      expect(results).toEqual(["ok"]);
    });

    it("should throw AggregateError if any handler fails", async () => {
      events.on("test", async () => "ok");
      events.on("test", async () => {
        throw new Error("fail");
      });

      await expect(events.emitStrict("test")).rejects.toThrow(AggregateError);
    });
  });

  describe("emitSafe", () => {
    it("should emit on next tick", async () => {
      let called = false;
      events.on("test", () => {
        called = true;
      });

      events.emitSafe("test");
      expect(called).toBe(false);

      await new Promise((resolve) => process.nextTick(resolve));
      expect(called).toBe(true);
    });
  });

  describe("emitSync", () => {
    it("should call handlers synchronously", () => {
      events.on("test", (val) => val * 2);
      const results = events.emitSync("test", 5);
      expect(results).toEqual([10]);
    });

    it("should catch errors and return null", () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      events.on("test", () => {
        throw new Error("sync fail");
      });

      const results = events.emitSync("test");
      expect(results).toEqual([null]);
      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe("createHookWrapper", () => {
    it("should emit before and after events", async () => {
      const spy = vi.spyOn(apiEvents, "emitSafe");
      const fn = vi.fn().mockResolvedValue("result");
      const wrapped = createHookWrapper("test", fn);

      const res = await wrapped("arg1");

      expect(res).toBe("result");
      expect(fn).toHaveBeenCalledWith("arg1");
      expect(spy).toHaveBeenCalledWith("before:test", "arg1");
      expect(spy).toHaveBeenCalledWith("after:test", "arg1", "result");

      spy.mockRestore();
    });

    it("should obey emitBefore/emitAfter options", async () => {
      const spy = vi.spyOn(apiEvents, "emitSafe");
      const wrapped = createHookWrapper("test", async () => {}, {
        emitBefore: false,
        emitAfter: false,
      });

      await wrapped();
      expect(spy).not.toHaveBeenCalled();
      spy.mockRestore();
    });

    it("should emit on:action:error on failure", async () => {
      const spy = vi.spyOn(apiEvents, "emitSafe");
      const err = new Error("action fail");
      const wrapped = createHookWrapper("test", async () => {
        throw err;
      });

      await expect(wrapped("arg")).rejects.toThrow(err);
      expect(spy).toHaveBeenCalledWith("on:action:error", {
        action: "test",
        error: err,
        args: ["arg"],
      });
      spy.mockRestore();
    });
  });

  describe("withErrorHook", () => {
    it("should return result if successful", async () => {
      const res = await withErrorHook("ctx", async () => "good");
      expect(res).toBe("good");
    });

    it("should emit on:error and on:detection on failure", async () => {
      const spy = vi.spyOn(apiEvents, "emitSafe");
      const err = new Error("boom");

      await expect(
        withErrorHook("ctx", async () => {
          throw err;
        }),
      ).rejects.toThrow(err);

      expect(spy).toHaveBeenCalledWith("on:error", {
        context: "ctx",
        error: err,
      });
      expect(spy).toHaveBeenCalledWith("on:detection", {
        type: "error",
        details: "boom",
      });

      spy.mockRestore();
    });
  });
});
