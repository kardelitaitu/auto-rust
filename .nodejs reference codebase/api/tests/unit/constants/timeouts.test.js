/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from "vitest";
import {
  SESSION_TIMEOUTS,
  importSessionTimeouts,
} from "@api/constants/session-timeouts.js";
import {
  ALL_TIMEOUTS,
  importAllTimeouts,
  TWITTER_TIMEOUTS,
  importTimeouts,
} from "@api/constants/timeouts.js";

describe("constants/session-timeouts.js", () => {
  describe("SESSION_TIMEOUTS", () => {
    it("should export an object with timeout constants", () => {
      expect(SESSION_TIMEOUTS).toBeDefined();
      expect(typeof SESSION_TIMEOUTS).toBe("object");
    });

    it("should have SESSION_TIMEOUT_MS set to 30 minutes", () => {
      expect(SESSION_TIMEOUTS.SESSION_TIMEOUT_MS).toBe(30 * 60 * 1000);
    });

    it("should have CLEANUP_INTERVAL_MS set to 5 minutes", () => {
      expect(SESSION_TIMEOUTS.CLEANUP_INTERVAL_MS).toBe(5 * 60 * 1000);
    });

    it("should have WORKER_WAIT_TIMEOUT_MS set to 30 seconds", () => {
      expect(SESSION_TIMEOUTS.WORKER_WAIT_TIMEOUT_MS).toBe(30000);
    });

    it("should have STUCK_WORKER_THRESHOLD_MS set to 10 minutes", () => {
      expect(SESSION_TIMEOUTS.STUCK_WORKER_THRESHOLD_MS).toBe(600000);
    });

    it("should have PAGE_CLOSE_TIMEOUT_MS set to 5 seconds", () => {
      expect(SESSION_TIMEOUTS.PAGE_CLOSE_TIMEOUT_MS).toBe(5000);
    });

    it("should have HEALTH_CHECK_INTERVAL_MS set to 30 seconds", () => {
      expect(SESSION_TIMEOUTS.HEALTH_CHECK_INTERVAL_MS).toBe(30000);
    });

    it("should have all numeric values", () => {
      Object.values(SESSION_TIMEOUTS).forEach((value) => {
        expect(typeof value).toBe("number");
      });
    });

    it("should have all positive values", () => {
      Object.values(SESSION_TIMEOUTS).forEach((value) => {
        expect(value).toBeGreaterThan(0);
      });
    });
  });

  describe("importSessionTimeouts", () => {
    it("should return default timeouts when called with no arguments", () => {
      const result = importSessionTimeouts();
      expect(result).toEqual(SESSION_TIMEOUTS);
    });

    it("should return default timeouts when called with empty object", () => {
      const result = importSessionTimeouts({});
      expect(result).toEqual(SESSION_TIMEOUTS);
    });

    it("should return default timeouts when settings.timeouts is undefined", () => {
      const result = importSessionTimeouts({ other: "value" });
      expect(result).toEqual(SESSION_TIMEOUTS);
    });

    it("should return default timeouts when settings.timeouts.session is undefined", () => {
      const result = importSessionTimeouts({ timeouts: {} });
      expect(result).toEqual(SESSION_TIMEOUTS);
    });

    it("should override SESSION_TIMEOUT_MS when provided", () => {
      const customValue = 60 * 60 * 1000; // 1 hour
      const result = importSessionTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customValue,
          },
        },
      });
      expect(result.SESSION_TIMEOUT_MS).toBe(customValue);
    });

    it("should override multiple values when provided", () => {
      const customSession = 60 * 60 * 1000;
      const customCleanup = 10 * 60 * 1000;
      const result = importSessionTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customSession,
            CLEANUP_INTERVAL_MS: customCleanup,
          },
        },
      });
      expect(result.SESSION_TIMEOUT_MS).toBe(customSession);
      expect(result.CLEANUP_INTERVAL_MS).toBe(customCleanup);
    });

    it("should preserve default values when only some are overridden", () => {
      const customValue = 60 * 60 * 1000;
      const result = importSessionTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customValue,
          },
        },
      });
      expect(result.SESSION_TIMEOUT_MS).toBe(customValue);
      expect(result.CLEANUP_INTERVAL_MS).toBe(
        SESSION_TIMEOUTS.CLEANUP_INTERVAL_MS,
      );
      expect(result.WORKER_WAIT_TIMEOUT_MS).toBe(
        SESSION_TIMEOUTS.WORKER_WAIT_TIMEOUT_MS,
      );
    });
  });
});

describe("constants/timeouts.js", () => {
  describe("Re-exports", () => {
    it("should re-export SESSION_TIMEOUTS", () => {
      // AllTimeouts should have session property
      expect(ALL_TIMEOUTS.session).toBeDefined();
      expect(ALL_TIMEOUTS.session).toEqual(SESSION_TIMEOUTS);
    });

    it("should re-export TWITTER_TIMEOUTS", () => {
      // AllTimeouts should have twitter property
      expect(ALL_TIMEOUTS.twitter).toBeDefined();
    });

    it("should re-export importSessionTimeouts", () => {
      // Function should be importable
      expect(typeof importSessionTimeouts).toBe("function");
    });

    it("should re-export importTimeouts", () => {
      // Function should be importable
      expect(typeof importTimeouts).toBe("function");
    });
  });

  describe("ALL_TIMEOUTS", () => {
    it("should be an object with session and twitter properties", () => {
      expect(ALL_TIMEOUTS).toBeDefined();
      expect(ALL_TIMEOUTS.session).toBeDefined();
      expect(ALL_TIMEOUTS.twitter).toBeDefined();
    });

    it("should have session property equal to SESSION_TIMEOUTS", () => {
      expect(ALL_TIMEOUTS.session).toEqual(SESSION_TIMEOUTS);
    });
  });

  describe("importAllTimeouts", () => {
    it("should return all default timeouts when called with no arguments", () => {
      const result = importAllTimeouts();
      expect(result.session).toEqual(SESSION_TIMEOUTS);
      expect(result.twitter).toBeDefined();
    });

    it("should return all default timeouts when called with empty object", () => {
      const result = importAllTimeouts({});
      expect(result.session).toEqual(SESSION_TIMEOUTS);
    });

    it("should override session timeouts when provided", () => {
      const customValue = 60 * 60 * 1000;
      const result = importAllTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customValue,
          },
        },
      });
      expect(result.session.SESSION_TIMEOUT_MS).toBe(customValue);
    });

    it("should override twitter timeouts when provided", () => {
      const customValue = 120000;
      const result = importAllTimeouts({
        timeouts: {
          twitter: {
            TYPING_DELAY_MS: customValue,
          },
        },
      });
      expect(result.twitter.TYPING_DELAY_MS).toBe(customValue);
    });

    it("should override both session and twitter timeouts when provided", () => {
      const customSession = 60 * 60 * 1000;
      const customTwitter = 120000;
      const result = importAllTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customSession,
          },
          twitter: {
            TYPING_DELAY_MS: customTwitter,
          },
        },
      });
      expect(result.session.SESSION_TIMEOUT_MS).toBe(customSession);
      expect(result.twitter.TYPING_DELAY_MS).toBe(customTwitter);
    });

    it("should preserve defaults when only some session values are overridden", () => {
      const customValue = 60 * 60 * 1000;
      const result = importAllTimeouts({
        timeouts: {
          session: {
            SESSION_TIMEOUT_MS: customValue,
          },
        },
      });
      expect(result.session.SESSION_TIMEOUT_MS).toBe(customValue);
      expect(result.session.CLEANUP_INTERVAL_MS).toBe(
        SESSION_TIMEOUTS.CLEANUP_INTERVAL_MS,
      );
    });

    it("should return object with correct structure", () => {
      const result = importAllTimeouts();
      expect(result).toHaveProperty("session");
      expect(result).toHaveProperty("twitter");
      expect(typeof result.session).toBe("object");
      expect(typeof result.twitter).toBe("object");
    });
  });
});
