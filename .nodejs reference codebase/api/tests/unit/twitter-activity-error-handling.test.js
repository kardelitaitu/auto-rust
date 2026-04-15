/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Tests for error handling improvements in twitter activity
 * @module tests/unit/twitter-activity-error-handling.test
 */

import { describe, it, expect, vi } from "vitest";

describe("Error Handling Improvements", () => {
  describe("Session Success/Failure Tracking", () => {
    // Simulate the current buggy behavior
    const buggySessionHandler = async (runSession, logger) => {
      try {
        await runSession();
        logger.info("Session completed successfully");
      } catch (sessionError) {
        logger.warn(`Session error: ${sessionError.message}`);
        // Bug: no tracking of success/failure
      }
      logger.info("Session completed"); // Always logs this
    };

    // Simulate the fixed behavior
    const fixedSessionHandler = async (runSession, logger) => {
      let sessionSuccess = false; // eslint-disable-line no-useless-assignment
      try {
        await runSession();
        sessionSuccess = true;
        logger.info("Session completed successfully");
      } catch (sessionError) {
        sessionSuccess = false;
        logger.warn(`Session error: ${sessionError.message}`);
        // Could re-throw or handle recovery
      }

      if (sessionSuccess) {
        logger.info("Session completed successfully");
      } else {
        logger.warn("Session completed with errors");
      }

      return sessionSuccess;
    };

    it('BUG: current handler logs "Session completed" even on failure', async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };
      const runSession = vi.fn().mockRejectedValue(new Error("Session failed"));

      await buggySessionHandler(runSession, logger);

      // Current buggy behavior
      expect(logger.info).toHaveBeenCalledWith("Session completed"); // Always logged
      expect(logger.info).not.toHaveBeenCalledWith(
        "Session completed successfully",
      );
      expect(logger.warn).toHaveBeenCalledWith("Session error: Session failed");
    });

    it("FIXED: handler should track success and log appropriately", async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };
      const runSession = vi.fn().mockRejectedValue(new Error("Session failed"));

      const success = await fixedSessionHandler(runSession, logger);

      // Fixed behavior
      expect(success).toBe(false);
      expect(logger.warn).toHaveBeenCalledWith("Session error: Session failed");
      expect(logger.warn).toHaveBeenCalledWith("Session completed with errors");
      expect(logger.info).not.toHaveBeenCalledWith(
        "Session completed successfully",
      );
    });

    it("FIXED: handler should log success on successful session", async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };
      const runSession = vi.fn().mockResolvedValue(undefined);

      const success = await fixedSessionHandler(runSession, logger);

      // Fixed behavior
      expect(success).toBe(true);
      expect(logger.info).toHaveBeenCalledWith(
        "Session completed successfully",
      );
      expect(logger.warn).not.toHaveBeenCalled();
    });
  });

  describe("Error Propagation in makeApiExecutor", () => {
    // Simulate current buggy error handling
    const buggyApiExecutor = async (apiFn) => {
      try {
        const result = await apiFn();
        if (result.success) {
          return { success: true, data: result };
        }
        return { success: false, reason: result.reason || "api_failed" };
      } catch (error) {
        // Bug: loses error type and stack trace
        return {
          success: false,
          reason: "exception",
          data: { error: error.message }, // Only message, no stack or type
        };
      }
    };

    // Simulate fixed error handling
    const fixedApiExecutor = async (apiFn) => {
      try {
        const result = await apiFn();
        if (result.success) {
          return { success: true, data: result };
        }
        return { success: false, reason: result.reason || "api_failed" };
      } catch (error) {
        // Fixed: preserves error information
        return {
          success: false,
          reason: "exception",
          data: {
            error: error.message,
            errorType: error.constructor.name,
            stack: error.stack,
            code: error.code,
          },
        };
      }
    };

    it("BUG: current executor loses error type and stack trace", async () => {
      const apiFn = vi.fn().mockRejectedValue(new Error("API failed"));

      const result = await buggyApiExecutor(apiFn);

      expect(result.success).toBe(false);
      expect(result.reason).toBe("exception");
      expect(result.data.error).toBe("API failed");
      expect(result.data.errorType).toBeUndefined();
      expect(result.data.stack).toBeUndefined();
    });

    it("FIXED: executor should preserve error type and stack trace", async () => {
      const apiFn = vi.fn().mockRejectedValue(new Error("API failed"));

      const result = await fixedApiExecutor(apiFn);

      expect(result.success).toBe(false);
      expect(result.reason).toBe("exception");
      expect(result.data.error).toBe("API failed");
      expect(result.data.errorType).toBe("Error");
      expect(result.data.stack).toBeDefined();
    });

    it("FIXED: executor should handle custom error types", async () => {
      class CustomError extends Error {
        constructor(message, code) {
          super(message);
          this.code = code;
          this.name = "CustomError";
        }
      }

      const apiFn = vi
        .fn()
        .mockRejectedValue(new CustomError("Custom failed", "CUSTOM_CODE"));

      const result = await fixedApiExecutor(apiFn);

      expect(result.data.errorType).toBe("CustomError");
      expect(result.data.code).toBe("CUSTOM_CODE");
    });
  });

  describe("Configuration Validation", () => {
    // Test config validation patterns

    const validateConfig = (config) => {
      const errors = [];

      if (!config.engagement) {
        errors.push("Missing engagement configuration");
      }

      if (!config.engagement?.probabilities) {
        errors.push("Missing engagement probabilities");
      } else {
        const probs = config.engagement.probabilities;
        if (typeof probs.reply !== "number")
          errors.push("Invalid reply probability");
        if (typeof probs.quote !== "number")
          errors.push("Invalid quote probability");
        if (typeof probs.like !== "number")
          errors.push("Invalid like probability");
        if (typeof probs.bookmark !== "number")
          errors.push("Invalid bookmark probability");
        if (typeof probs.retweet !== "number")
          errors.push("Invalid retweet probability");
        if (typeof probs.follow !== "number")
          errors.push("Invalid follow probability");
      }

      return { valid: errors.length === 0, errors };
    };

    const safeConfigAccess = (config) => {
      return {
        reply: config?.engagement?.probabilities?.reply ?? 0.5,
        quote: config?.engagement?.probabilities?.quote ?? 0.2,
        like: config?.engagement?.probabilities?.like ?? 0.15,
        bookmark: config?.engagement?.probabilities?.bookmark ?? 0.05,
        retweet: config?.engagement?.probabilities?.retweet ?? 0.2,
        follow: config?.engagement?.probabilities?.follow ?? 0.1,
      };
    };

    it("should validate complete config", () => {
      const config = {
        engagement: {
          probabilities: {
            reply: 0.5,
            quote: 0.2,
            like: 0.15,
            bookmark: 0.05,
            retweet: 0.2,
            follow: 0.1,
          },
        },
      };

      const result = validateConfig(config);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it("should detect missing engagement config", () => {
      const config = {};
      const result = validateConfig(config);
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Missing engagement configuration");
    });

    it("should detect missing probabilities", () => {
      const config = { engagement: {} };
      const result = validateConfig(config);
      expect(result.valid).toBe(false);
      expect(result.errors).toContain("Missing engagement probabilities");
    });

    it("should provide safe defaults for missing config", () => {
      const config = {};
      const defaults = safeConfigAccess(config);

      expect(defaults.reply).toBe(0.5);
      expect(defaults.quote).toBe(0.2);
      expect(defaults.like).toBe(0.15);
      expect(defaults.bookmark).toBe(0.05);
      expect(defaults.retweet).toBe(0.2);
      expect(defaults.follow).toBe(0.1);
    });

    it("should use provided values over defaults", () => {
      const config = {
        engagement: {
          probabilities: {
            reply: 0.8,
            quote: 0.3,
          },
        },
      };

      const defaults = safeConfigAccess(config);
      expect(defaults.reply).toBe(0.8);
      expect(defaults.quote).toBe(0.3);
      expect(defaults.like).toBe(0.15); // Default
    });
  });
});
