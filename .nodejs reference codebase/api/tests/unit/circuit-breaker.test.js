/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for core/circuit-breaker.js
 * @module tests/unit/circuit-breaker.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import CircuitBreaker, {
  BrowserCircuitBreaker,
  CircuitOpenError,
} from "@api/core/circuit-breaker.js";

// Mock logger
vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("core/circuit-breaker", () => {
  let breaker;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    breaker = new CircuitBreaker({
      failureThreshold: 20, // 20%
      successThreshold: 2,
      halfOpenTime: 1000,
      monitoringWindow: 5000,
      minSamples: 3,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with defaults", () => {
      const cb = new CircuitBreaker();
      expect(cb.failureThreshold).toBe(50);
      expect(cb.halfOpenTime).toBe(30000);
      expect(cb.minSamples).toBe(5);
    });
  });

  describe("execute", () => {
    it("should execute successful function", async () => {
      const fn = vi.fn().mockResolvedValue("ok");
      const result = await breaker.execute("m1", fn);

      expect(result).toBe("ok");
      expect(fn).toHaveBeenCalledTimes(1);
      expect(breaker.getHealth("m1").status).toBe("CLOSED");
    });

    it("should record failures and trip to OPEN after minSamples", async () => {
      const error = new Error("fail");
      const fn = vi.fn().mockRejectedValue(error);

      // 1st failure (minSamples: 3)
      await expect(breaker.execute("m1", fn)).rejects.toThrow("fail");
      expect(breaker.getHealth("m1").status).toBe("CLOSED");

      // 2nd failure
      await expect(breaker.execute("m1", fn)).rejects.toThrow("fail");
      expect(breaker.getHealth("m1").status).toBe("CLOSED");

      // 3rd failure (minSamples reached, failureRate 100% >= 20%)
      await expect(breaker.execute("m1", fn)).rejects.toThrow("fail");
      expect(breaker.getHealth("m1").status).toBe("OPEN");
    });

    it("should reject calls while OPEN and transition to HALF_OPEN", async () => {
      breaker.forceOpen("m1");

      // Should be OPEN
      await expect(breaker.execute("m1", async () => "ok")).rejects.toThrow(
        /Circuit breaker OPEN/,
      );

      // Advance time past halfOpenTime
      const b = breaker.getBreaker("m1");
      b.nextAttempt = Date.now() + 1000;
      vi.advanceTimersByTime(1100);

      // Now it should transition to HALF_OPEN upon execution
      const result = await breaker.execute("m1", async () => "ok");
      expect(result).toBe("ok");
      expect(breaker.getHealth("m1").status).toBe("HALF_OPEN");
    });

    it("should close after successThreshold in HALF_OPEN", async () => {
      const b = breaker.getBreaker("m1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() - 100;

      // First success in HALF_OPEN
      await breaker.execute("m1", async () => "ok");
      expect(breaker.getHealth("m1").status).toBe("HALF_OPEN");

      // Second success in HALF_OPEN (threshold 2)
      await breaker.execute("m1", async () => "ok");
      expect(breaker.getHealth("m1").status).toBe("CLOSED");
    });

    it("should trip back to OPEN if failure occurs in HALF_OPEN", async () => {
      const b = breaker.getBreaker("m1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() - 100;

      const error = new Error("half-fail");
      await expect(
        breaker.execute("m1", async () => {
          throw error;
        }),
      ).rejects.toThrow("half-fail");

      expect(breaker.getHealth("m1").status).toBe("OPEN");
    });
  });

  describe("_isOpen and Internal logic", () => {
    it("should internal _isOpen handle states properly", () => {
      const b = breaker.getBreaker("m1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() + 1000;
      expect(breaker._isOpen(b)).toBe(true);

      vi.advanceTimersByTime(2000);
      expect(breaker._isOpen(b)).toBe(false);

      b.state = "CLOSED";
      expect(breaker._isOpen(b)).toBe(false);
    });
  });

  describe("Monitoring Window & History", () => {
    it("should cleanup history old entries", async () => {
      const b = breaker.getBreaker("m1");
      b.history.push({ time: Date.now() - 10000, type: "failure" });

      await breaker.execute("m1", async () => "ok");
      const health = breaker.getHealth("m1");
      expect(health.recentOperations).toBe(1);
    });

    it("should cap history at 100 entries", async () => {
      const b = breaker.getBreaker("m1");
      for (let i = 0; i < 150; i++) {
        b.history.push({ time: Date.now(), type: "success" });
      }

      await breaker.execute("m1", async () => "ok");
      expect(b.history.length).toBe(100);
    });
  });

  describe("Health & Status", () => {
    it("should return health for unknown model", () => {
      const health = breaker.getHealth("unknown");
      expect(health.status).toBe("unknown");
    });

    it("should return all status", async () => {
      await breaker.execute("m1", async () => "ok");
      breaker.forceOpen("m2");

      const status = breaker.getAllStatus();
      expect(status.m1.state).toBe("CLOSED");
      expect(status.m2.state).toBe("OPEN");
    });
  });

  describe("Manual Control", () => {
    it("should reset specific breaker", async () => {
      breaker.forceOpen("m1");
      breaker.reset("m1");
      expect(breaker.getHealth("m1").status).toBe("CLOSED");
    });

    it("should reset all breakers", async () => {
      breaker.forceOpen("m1");
      breaker.forceOpen("m2");
      breaker.resetAll();
      expect(breaker.breakers.size).toBe(0);
    });

    it("should force close breaker", async () => {
      breaker.forceOpen("m1");
      breaker.forceClose("m1");
      expect(breaker.getHealth("m1").status).toBe("CLOSED");
    });

    it("should force open breaker", () => {
      breaker.forceOpen("m1");
      const health = breaker.getHealth("m1");
      expect(health.status).toBe("OPEN");
      expect(health.nextAttempt).toBeGreaterThan(Date.now());
    });

    it("should create new breaker when forceOpen called on non-existent", () => {
      breaker.forceOpen("newModel");
      expect(breaker.breakers.has("newModel")).toBe(true);
    });

    it("should create new breaker when forceClose called on non-existent", () => {
      breaker.forceClose("newModel");
      const health = breaker.getHealth("newModel");
      expect(health.status).toBe("CLOSED");
    });
  });

  describe("Edge Cases", () => {
    it("should handle multiple models independently", async () => {
      const fn1 = vi.fn().mockRejectedValue(new Error("fail"));
      const fn2 = vi.fn().mockResolvedValue("ok");

      // Fail model1 multiple times
      await expect(breaker.execute("model1", fn1)).rejects.toThrow();
      await expect(breaker.execute("model1", fn1)).rejects.toThrow();
      await expect(breaker.execute("model1", fn1)).rejects.toThrow();

      // model2 should still work
      const result = await breaker.execute("model2", fn2);
      expect(result).toBe("ok");

      expect(breaker.getHealth("model1").status).toBe("OPEN");
      expect(breaker.getHealth("model2").status).toBe("CLOSED");
    });

    it("should calculate failure rate correctly with mixed history", async () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now(), type: "success" },
        { time: Date.now(), type: "success" },
        { time: Date.now(), type: "failure" },
        { time: Date.now(), type: "failure" },
      ];

      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(50);
    });

    it("should return 0 failure rate when history is empty", () => {
      const b = breaker.getBreaker("m1");
      b.history = [];

      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(0);
    });

    it("should return 0 failure rate when below minSamples", () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now(), type: "failure" },
        { time: Date.now(), type: "failure" },
      ];

      // minSamples is 3, so should return 0
      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(0);
    });

    it("should calculate failure rate only for recent history", () => {
      const b = breaker.getBreaker("m1");
      const now = Date.now();

      // 3 recent failures (meets minSamples=3)
      // 3 old successes (should be ignored)
      b.history = [
        { time: now, type: "failure" },
        { time: now, type: "failure" },
        { time: now, type: "failure" },
        { time: now - 600000, type: "success" },
        { time: now - 600000, type: "success" },
        { time: now - 600000, type: "success" },
      ];

      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(100);
    });
  });

  describe("CircuitOpenError", () => {
    it("should have correct error code and properties", async () => {
      // Test through actual execution
      breaker.forceOpen("testModel");

      try {
        await breaker.execute("testModel", async () => "ok");
      } catch (error) {
        expect(error.code).toBe("CIRCUIT_OPEN");
        expect(error.modelId).toBe("testModel");
        expect(error.message).toContain("testModel");
        expect(error.message).toContain("s");
      }
    });
  });

  describe("Coverage Gap Tests", () => {
    it("should return 0 failure rate with empty history", () => {
      const b = breaker.getBreaker("m1");
      b.history = [];
      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(0);
    });

    it("should return 0 failure rate when history length < minSamples", () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now(), type: "failure" },
        { time: Date.now(), type: "failure" },
      ];
      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(0);
    });

    it("should return correct failure rate with mixed history", () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now(), type: "success" },
        { time: Date.now(), type: "failure" },
        { time: Date.now(), type: "failure" },
        { time: Date.now(), type: "failure" },
      ];
      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(75);
    });

    it("should calculate failure rate only within monitoring window", () => {
      const b = breaker.getBreaker("m1");
      const now = Date.now();
      b.history = [
        { time: now, type: "failure" },
        { time: now, type: "failure" },
        { time: now, type: "failure" },
        { time: now - 100000, type: "success" },
        { time: now - 100000, type: "success" },
      ];
      const rate = breaker._calculateFailureRate(b);
      expect(rate).toBe(100);
    });

    it("should transition from CLOSED to OPEN when failure threshold met", async () => {
      const error = new Error("fail");
      const fn = vi.fn().mockRejectedValue(error);

      for (let i = 0; i < 2; i++) {
        try {
          await breaker.execute("test", fn);
        } catch {
          /* intentional no-op */
        }
      }
      expect(breaker.getHealth("test").status).toBe("CLOSED");

      await expect(breaker.execute("test", fn)).rejects.toThrow();
      expect(breaker.getHealth("test").status).toBe("OPEN");
    });

    it("should transition from OPEN to HALF_OPEN when nextAttempt time passed", async () => {
      breaker.forceOpen("m1");
      const b = breaker.getBreaker("m1");
      b.nextAttempt = Date.now() - 1;

      await breaker.execute("m1", async () => "success");
      expect(breaker.getHealth("m1").status).toBe("HALF_OPEN");
    });

    it("should stay in OPEN when nextAttempt not yet passed", async () => {
      breaker.forceOpen("m1");
      const b = breaker.getBreaker("m1");
      b.nextAttempt = Date.now() + 60000;

      await expect(breaker.execute("m1", async () => "ok")).rejects.toThrow(
        "Circuit breaker OPEN",
      );
      expect(breaker.getHealth("m1").status).toBe("OPEN");
    });

    it("should transition from HALF_OPEN to CLOSED after successThreshold", async () => {
      const b = breaker.getBreaker("m1");
      b.state = "HALF_OPEN";
      b.successes = 0;

      await breaker.execute("m1", async () => "ok");
      expect(b.state).toBe("HALF_OPEN");
      expect(b.successes).toBe(1);

      await breaker.execute("m1", async () => "ok");
      expect(b.state).toBe("CLOSED");
    });

    it("should transition from HALF_OPEN back to OPEN on failure", async () => {
      const b = breaker.getBreaker("m1");
      b.state = "HALF_OPEN";

      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }

      expect(b.state).toBe("OPEN");
      expect(b.nextAttempt).toBeGreaterThan(Date.now());
    });

    it("should reset failures and successes when closing from HALF_OPEN", async () => {
      const b = breaker.getBreaker("m1");
      b.state = "HALF_OPEN";
      b.failures = 10;
      b.successes = 0;

      await breaker.execute("m1", async () => "ok");
      expect(b.state).toBe("HALF_OPEN");
      expect(b.successes).toBe(1);

      await breaker.execute("m1", async () => "ok");
      expect(b.state).toBe("CLOSED");
      expect(b.failures).toBe(0);
      expect(b.successes).toBe(0);
    });

    it("should update lastSuccess on successful execution", async () => {
      vi.setSystemTime(1000);
      await breaker.execute("m1", async () => "ok");

      const health = breaker.getHealth("m1");
      expect(health.lastSuccess).toBe(1000);
    });

    it("should update lastFailure on failed execution", async () => {
      vi.setSystemTime(2000);
      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }

      const health = breaker.getHealth("m1");
      expect(health.lastFailure).toBe(2000);
    });

    it("should cleanup history after execution", async () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now() - 10000, type: "failure" },
        { time: Date.now() - 10000, type: "success" },
      ];

      await breaker.execute("m1", async () => "ok");
      expect(b.history.length).toBeLessThanOrEqual(3);
    });

    it("should cap history at 100 entries after cleanup", async () => {
      const b = breaker.getBreaker("m1");
      for (let i = 0; i < 120; i++) {
        b.history.push({ time: Date.now(), type: "success" });
      }

      await breaker.execute("m1", async () => "ok");
      expect(b.history.length).toBe(100);
    });

    it("should get breaker creates new breaker if not exists", () => {
      const result = breaker.getBreaker("newModel");
      expect(result).toBeDefined();
      expect(result.state).toBe("CLOSED");
      expect(breaker.breakers.has("newModel")).toBe(true);
    });

    it("should get breaker returns existing breaker", () => {
      const b1 = breaker.getBreaker("m1");
      const b2 = breaker.getBreaker("m1");
      expect(b1).toBe(b2);
    });

    it("should reset non-existent breaker does nothing", () => {
      expect(() => breaker.reset("nonExistent")).not.toThrow();
    });

    it("should getAllStatus returns empty object when no breakers", () => {
      const status = breaker.getAllStatus();
      expect(status).toEqual({});
    });

    it("should getAllStatus returns failureRate as string with percent", async () => {
      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }
      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }
      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }

      const status = breaker.getAllStatus();
      expect(status.m1.failureRate).toContain("%");
    });

    it("should getHealth returns nextAttempt for OPEN breaker", () => {
      breaker.forceOpen("m1");
      const health = breaker.getHealth("m1");
      expect(health.nextAttempt).toBeGreaterThan(Date.now());
    });

    it("should getHealth returns nextAttempt as null for CLOSED breaker", async () => {
      await breaker.execute("m1", async () => "ok");
      const health = breaker.getHealth("m1");
      expect(health.nextAttempt).toBeNull();
    });

    it("should execute passes through result of function", async () => {
      const obj = { key: "value" };
      const result = await breaker.execute("m1", async () => obj);
      expect(result).toBe(obj);
    });

    it("should execute re-throws error after recording", async () => {
      const error = new Error("test error");
      const fn = vi.fn().mockRejectedValue(error);

      await expect(breaker.execute("m1", fn)).rejects.toThrow("test error");
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should forceOpen sets nextAttempt correctly", () => {
      vi.setSystemTime(5000);
      breaker.forceOpen("m1");
      const b = breaker.getBreaker("m1");
      expect(b.nextAttempt).toBe(5000 + breaker.halfOpenTime);
    });

    it("should forceClose resets failures and successes", () => {
      const b = breaker.getBreaker("m1");
      b.failures = 100;
      b.successes = 50;

      breaker.forceClose("m1");

      expect(b.failures).toBe(0);
      expect(b.successes).toBe(0);
      expect(b.state).toBe("CLOSED");
    });

    it("should _recordSuccess increments successes", async () => {
      const b = breaker.getBreaker("m1");
      breaker._recordSuccess(b, "m1");
      expect(b.successes).toBe(1);
    });

    it("should _recordFailure increments failures", () => {
      const b = breaker.getBreaker("m1");
      breaker._recordFailure(b, "m1", new Error("fail"));
      expect(b.failures).toBe(1);
    });

    it("should _recordFailure adds error message to history", () => {
      const b = breaker.getBreaker("m1");
      breaker._recordFailure(b, "m1", new Error("test error"));
      expect(b.history[b.history.length - 1].error).toBe("test error");
    });

    it("should _cleanupHistory removes old entries", () => {
      const b = breaker.getBreaker("m1");
      b.history = [
        { time: Date.now() - 100000, type: "success" },
        { time: Date.now(), type: "failure" },
      ];

      breaker._cleanupHistory(b);
      expect(b.history.length).toBe(1);
      expect(b.history[0].type).toBe("failure");
    });

    it("should handle rapid success/failure sequences", async () => {
      for (let i = 0; i < 10; i++) {
        try {
          await breaker.execute("m1", async () => "ok");
        } catch {
          /* intentional no-op */
        }
        try {
          await breaker.execute("m1", async () => {
            throw new Error("fail");
          });
        } catch {
          /* intentional no-op */
        }
      }
      const health = breaker.getHealth("m1");
      expect(health.recentOperations).toBeLessThanOrEqual(20);
    });

    it("should handle model with no history in getHealth", () => {
      breaker.getBreaker("m1");
      const health = breaker.getHealth("m1");
      expect(health.status).toBe("CLOSED");
      expect(health.failureRate).toBe(0);
      expect(health.recentOperations).toBe(0);
    });

    it("should handle concurrent execute calls for same model", async () => {
      const promises = Array(5)
        .fill(null)
        .map(() => breaker.execute("m1", async () => "ok"));
      const results = await Promise.allSettled(promises);
      const fulfilled = results.filter((r) => r.status === "fulfilled");
      expect(fulfilled.length).toBe(5);
    });

    it("should record success history entry", async () => {
      const b = breaker.getBreaker("m1");
      const before = b.history.length;

      await breaker.execute("m1", async () => "ok");

      expect(b.history.length).toBe(before + 1);
      expect(b.history[b.history.length - 1].type).toBe("success");
    });

    it("should record failure history entry", async () => {
      const b = breaker.getBreaker("m1");
      const before = b.history.length;

      try {
        await breaker.execute("m1", async () => {
          throw new Error("fail");
        });
      } catch {
        /* intentional no-op */
      }

      expect(b.history.length).toBe(before + 1);
      expect(b.history[b.history.length - 1].type).toBe("failure");
    });
  });

  describe("getState, getAllStates, getStats", () => {
    it("should getState return null for unknown model", () => {
      const state = breaker.getState("unknown");
      expect(state).toBeNull();
    });

    it("should getState return correct state", async () => {
      const b = breaker.getBreaker("m1::default");
      b.state = "OPEN";
      b.failures = 5;
      b.successes = 2;

      const state = breaker.getState("m1");
      expect(state.state).toBe("OPEN");
      expect(state.failures).toBe(5);
      expect(state.successes).toBe(2);
    });

    it("should getAllStates return all breaker states", async () => {
      await breaker.execute("m1", async () => "ok");
      breaker.forceOpen("m2");
      breaker.forceOpen("m3");

      const states = breaker.getAllStates();
      expect(Object.keys(states)).toHaveLength(3);
      expect(states.m1.state).toBe("CLOSED");
      expect(states.m2.state).toBe("OPEN");
      expect(states.m3.state).toBe("OPEN");
    });

    it("should getAllStates return empty object when no breakers", () => {
      const states = breaker.getAllStates();
      expect(states).toEqual({});
    });

    it("should getStats return correct counts", async () => {
      await breaker.execute("closed", async () => "ok");
      breaker.forceOpen("open");
      const b = breaker.getBreaker("halfOpen");
      b.state = "HALF_OPEN";

      const stats = breaker.getStats();
      expect(stats.total).toBe(3);
      expect(stats.closed).toBe(1);
      expect(stats.open).toBe(1);
      expect(stats.halfOpen).toBe(1);
    });

    it("should getStats return zeros when no breakers", () => {
      const stats = breaker.getStats();
      expect(stats.total).toBe(0);
      expect(stats.closed).toBe(0);
      expect(stats.open).toBe(0);
      expect(stats.halfOpen).toBe(0);
    });
  });
});

describe("BrowserCircuitBreaker", () => {
  let browserBreaker;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    browserBreaker = new BrowserCircuitBreaker({
      failureThreshold: 3,
      successThreshold: 2,
      halfOpenTime: 1000,
      enabled: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Constructor", () => {
    it("should initialize with defaults", () => {
      const cb = new BrowserCircuitBreaker();
      expect(cb.failureThreshold).toBe(5);
      expect(cb.successThreshold).toBe(3);
      expect(cb.halfOpenTime).toBe(30000);
      expect(cb.enabled).toBe(true);
    });

    it("should accept custom options", () => {
      const cb = new BrowserCircuitBreaker({
        failureThreshold: 10,
        successThreshold: 5,
        halfOpenTime: 5000,
        enabled: false,
      });
      expect(cb.failureThreshold).toBe(10);
      expect(cb.successThreshold).toBe(5);
      expect(cb.halfOpenTime).toBe(5000);
      expect(cb.enabled).toBe(false);
    });
  });

  describe("check", () => {
    it("should return allowed when disabled", () => {
      browserBreaker.enabled = false;
      const result = browserBreaker.check("profile1");
      expect(result.allowed).toBe(true);
      expect(result.state).toBe("disabled");
    });

    it("should return allowed for closed breaker", () => {
      const result = browserBreaker.check("profile1");
      expect(result.allowed).toBe(true);
      expect(result.state).toBe("closed");
    });

    it("should return not allowed for open breaker", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() + 60000;
      const result = browserBreaker.check("profile1");
      expect(result.allowed).toBe(false);
      expect(result.state).toBe("open");
      expect(result.retryAfter).toBeDefined();
    });

    it("should transition to half-open when nextAttempt passed", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() - 100;

      const result = browserBreaker.check("profile1");
      expect(result.allowed).toBe(true);
      expect(result.state).toBe("half-open");
    });

    it("should return allowed for half-open breaker", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "HALF_OPEN";

      const result = browserBreaker.check("profile1");
      expect(result.allowed).toBe(true);
      expect(result.state).toBe("half-open");
    });
  });

  describe("recordSuccess", () => {
    it("should not record when disabled", () => {
      browserBreaker.enabled = false;
      browserBreaker.recordSuccess("profile1");
      expect(browserBreaker.breakers.has("profile1")).toBe(false);
    });

    it("should decrement failures in closed state", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.failures = 2;

      browserBreaker.recordSuccess("profile1");
      expect(b.failures).toBe(1);
    });

    it("should increment successes in half-open state", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "HALF_OPEN";

      browserBreaker.recordSuccess("profile1");
      expect(b.successes).toBe(1);
    });

    it("should close after successThreshold in half-open", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "HALF_OPEN";

      browserBreaker.recordSuccess("profile1");
      expect(b.state).toBe("HALF_OPEN");

      browserBreaker.recordSuccess("profile1");
      expect(b.state).toBe("CLOSED");
      expect(b.failures).toBe(0);
      expect(b.successes).toBe(0);
    });
  });

  describe("recordFailure", () => {
    it("should not record when disabled", () => {
      browserBreaker.enabled = false;
      browserBreaker.recordFailure("profile1");
      expect(browserBreaker.breakers.has("profile1")).toBe(false);
    });

    it("should increment failures in closed state", () => {
      browserBreaker.recordFailure("profile1");
      const b = browserBreaker.breakers.get("profile1");
      expect(b.failures).toBe(1);
    });

    it("should not open until threshold reached in closed state", () => {
      browserBreaker.recordFailure("profile1");
      browserBreaker.recordFailure("profile1");

      let b = browserBreaker.breakers.get("profile1");
      expect(b.state).toBe("CLOSED");

      browserBreaker.recordFailure("profile1");
      b = browserBreaker.breakers.get("profile1");
      expect(b.state).toBe("OPEN");
      expect(b.nextAttempt).toBeDefined();
    });

    it("should reopen from half-open on failure", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "HALF_OPEN";

      browserBreaker.recordFailure("profile1");
      expect(b.state).toBe("OPEN");
      expect(b.nextAttempt).toBeDefined();
    });
  });

  describe("execute", () => {
    it("should execute successful function", async () => {
      const fn = vi.fn().mockResolvedValue("ok");
      const result = await browserBreaker.execute("profile1", fn);
      expect(result).toBe("ok");
      expect(fn).toHaveBeenCalledTimes(1);
    });

    it("should throw CircuitOpenError when open", async () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "OPEN";
      b.nextAttempt = Date.now() + 60000;

      await expect(
        browserBreaker.execute("profile1", async () => "ok"),
      ).rejects.toThrow(CircuitOpenError);
    });

    it("should record failure and throw on error", async () => {
      const fn = vi.fn().mockRejectedValue(new Error("fail"));

      await expect(browserBreaker.execute("profile1", fn)).rejects.toThrow(
        "fail",
      );

      const b = browserBreaker.breakers.get("profile1");
      expect(b.failures).toBe(1);
    });

    it("should record success on success", async () => {
      const fn = vi.fn().mockResolvedValue("ok");
      await browserBreaker.execute("profile1", fn);

      const b = browserBreaker.breakers.get("profile1");
      expect(b.failures).toBe(0);
    });
  });

  describe("getState", () => {
    it("should return null for unknown profile", () => {
      const state = browserBreaker.getState("unknown");
      expect(state).toBeNull();
    });

    it("should return state for existing profile", async () => {
      await browserBreaker.execute("profile1", async () => "ok");

      const state = browserBreaker.getState("profile1");
      expect(state.state).toBe("CLOSED");
      expect(state.failures).toBe(0);
      expect(state.successes).toBeDefined();
    });
  });

  describe("getAllStates", () => {
    it("should return empty object when no breakers", () => {
      const states = browserBreaker.getAllStates();
      expect(states).toEqual({});
    });

    it("should return all breaker states", async () => {
      await browserBreaker.execute("p1", async () => "ok");
      const b2 = browserBreaker._getOrCreate("p2");
      b2.state = "OPEN";

      const states = browserBreaker.getAllStates();
      expect(Object.keys(states)).toHaveLength(2);
      expect(states.p1.state).toBe("CLOSED");
      expect(states.p2.state).toBe("OPEN");
    });
  });

  describe("reset", () => {
    it("should reset specific breaker", () => {
      const b = browserBreaker._getOrCreate("profile1");
      b.state = "OPEN";
      browserBreaker.reset("profile1");

      const state = browserBreaker.getState("profile1");
      expect(state.state).toBe("CLOSED");
    });

    it("should do nothing for non-existent breaker", () => {
      expect(() => browserBreaker.reset("unknown")).not.toThrow();
    });
  });

  describe("resetAll", () => {
    it("should clear all breakers", () => {
      browserBreaker._getOrCreate("p1").state = "OPEN";
      browserBreaker._getOrCreate("p2").state = "OPEN";
      browserBreaker.resetAll();

      expect(browserBreaker.breakers.size).toBe(0);
    });
  });

  describe("getStats", () => {
    it("should return zeros when no breakers", () => {
      const stats = browserBreaker.getStats();
      expect(stats.total).toBe(0);
      expect(stats.open).toBe(0);
      expect(stats.halfOpen).toBe(0);
      expect(stats.closed).toBe(0);
    });

    it("should return correct counts", async () => {
      await browserBreaker.execute("closed", async () => "ok");
      const b = browserBreaker._getOrCreate("open");
      b.state = "OPEN";
      const bHalf = browserBreaker._getOrCreate("halfOpen");
      bHalf.state = "HALF_OPEN";

      const stats = browserBreaker.getStats();
      expect(stats.total).toBe(3);
      expect(stats.closed).toBe(1);
      expect(stats.open).toBe(1);
      expect(stats.halfOpen).toBe(1);
    });
  });

  describe("_getOrCreate", () => {
    it("should create new breaker if not exists", () => {
      const breaker = browserBreaker._getOrCreate("newProfile");
      expect(breaker).toBeDefined();
      expect(breaker.state).toBe("CLOSED");
    });

    it("should return existing breaker", () => {
      const b1 = browserBreaker._getOrCreate("profile1");
      const b2 = browserBreaker._getOrCreate("profile1");
      expect(b1).toBe(b2);
    });
  });

  describe("_createBreaker", () => {
    it("should create breaker with correct defaults", () => {
      const breaker = browserBreaker._createBreaker();
      expect(breaker.state).toBe("CLOSED");
      expect(breaker.failures).toBe(0);
      expect(breaker.successes).toBe(0);
      expect(breaker.lastFailure).toBeNull();
      expect(breaker.nextAttempt).toBeNull();
    });
  });

  describe("Edge Cases", () => {
    it("should handle multiple profiles independently", async () => {
      const failFn = vi.fn().mockRejectedValue(new Error("fail"));
      const okFn = vi.fn().mockResolvedValue("ok");

      browserBreaker.recordFailure("p1");
      browserBreaker.recordFailure("p1");
      browserBreaker.recordFailure("p1");
      browserBreaker.recordFailure("p1");

      const result = await browserBreaker.execute("p2", okFn);
      expect(result).toBe("ok");

      const p1State = browserBreaker.getState("p1");
      const p2State = browserBreaker.getState("p2");
      expect(p1State.state).toBe("OPEN");
      expect(p2State.state).toBe("CLOSED");
    });

    it("should track failures correctly", () => {
      browserBreaker.recordFailure("p1");
      browserBreaker.recordFailure("p1");

      const b = browserBreaker.breakers.get("p1");
      expect(b.failures).toBe(2);
    });

    it("should update lastFailure on recordFailure", () => {
      vi.setSystemTime(5000);
      browserBreaker.recordFailure("p1");

      const b = browserBreaker.breakers.get("p1");
      expect(b.lastFailure).toBe(5000);
    });
  });
});
