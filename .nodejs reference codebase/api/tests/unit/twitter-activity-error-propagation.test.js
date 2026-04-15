/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Tests for error propagation improvements in twitter activity
 * @module tests/unit/twitter-activity-error-propagation.test
 */

import { describe, it, expect, vi } from "vitest";

describe("Error Propagation Improvements", () => {
  describe("makeApiExecutor Error Handling", () => {
    // Simulate the enhanced error handling pattern
    const createEnhancedExecutor = (apiFn) => {
      return async (context = {}) => {
        try {
          const result = await apiFn(context);
          return { success: true, data: result };
        } catch (error) {
          return {
            success: false,
            reason: "exception",
            data: {
              error: error.message,
              errorType: error.constructor?.name || "Error",
              stack: error.stack,
              code: error.code,
            },
          };
        }
      };
    };

    it("should preserve error type and stack trace", async () => {
      const apiFn = vi.fn().mockRejectedValue(new Error("API failed"));
      const executor = createEnhancedExecutor(apiFn);

      const result = await executor({});

      expect(result.success).toBe(false);
      expect(result.reason).toBe("exception");
      expect(result.data.error).toBe("API failed");
      expect(result.data.errorType).toBe("Error");
      expect(result.data.stack).toBeDefined();
    });

    it("should handle custom error types", async () => {
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
      const executor = createEnhancedExecutor(apiFn);

      const result = await executor({});

      expect(result.data.errorType).toBe("CustomError");
      expect(result.data.code).toBe("CUSTOM_CODE");
    });

    it("should handle errors without constructor name", async () => {
      const error = new Error("Test error");
      delete error.constructor; // Simulate edge case

      const apiFn = vi.fn().mockRejectedValue(error);
      const executor = createEnhancedExecutor(apiFn);

      const result = await executor({});

      expect(result.data.errorType).toBe("Error");
    });

    it("should preserve stack trace for debugging", async () => {
      const apiFn = vi.fn().mockRejectedValue(new Error("Stack trace test"));
      const executor = createEnhancedExecutor(apiFn);

      const result = await executor({});

      expect(result.data.stack).toContain("Stack trace test");
    });
  });

  describe("Session Success Tracking Pattern", () => {
    // Simulate the session success tracking pattern
    const simulateSession = async (runSession, logger) => {
      let sessionSuccess = false; // eslint-disable-line no-useless-assignment
      try {
        await runSession();
        sessionSuccess = true;
        logger.info("Session completed successfully");
      } catch (sessionError) {
        sessionSuccess = false;
        logger.warn(`Session error: ${sessionError.message}`);
        // Could attempt recovery here
      }

      if (sessionSuccess) {
        logger.info("Session completed successfully");
      } else {
        logger.warn("Session completed with errors");
      }

      return sessionSuccess;
    };

    it("should log success on successful session", async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };
      const runSession = vi.fn().mockResolvedValue(undefined);

      const success = await simulateSession(runSession, logger);

      expect(success).toBe(true);
      expect(logger.info).toHaveBeenCalledWith(
        "Session completed successfully",
      );
      expect(logger.warn).not.toHaveBeenCalled();
    });

    it("should log error on failed session", async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };
      const runSession = vi.fn().mockRejectedValue(new Error("Session failed"));

      const success = await simulateSession(runSession, logger);

      expect(success).toBe(false);
      expect(logger.warn).toHaveBeenCalledWith("Session error: Session failed");
      expect(logger.warn).toHaveBeenCalledWith("Session completed with errors");
      expect(logger.info).not.toHaveBeenCalledWith(
        "Session completed successfully",
      );
    });

    it("should differentiate between success and failure logs", async () => {
      const logger = { info: vi.fn(), warn: vi.fn() };

      // Successful run
      await simulateSession(vi.fn().mockResolvedValue(), logger);
      expect(logger.info).toHaveBeenCalledWith(
        "Session completed successfully",
      );

      logger.info.mockClear();
      logger.warn.mockClear();

      // Failed run
      await simulateSession(
        vi.fn().mockRejectedValue(new Error("Fail")),
        logger,
      );
      expect(logger.warn).toHaveBeenCalledWith("Session completed with errors");
      expect(logger.info).not.toHaveBeenCalled();
    });
  });
});
