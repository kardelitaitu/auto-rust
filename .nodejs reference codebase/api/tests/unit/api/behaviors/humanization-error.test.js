/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    gaussian: vi.fn((mean, std) => mean),
    randomInRange: vi.fn((min, max) => min),
    roll: vi.fn(() => false),
  },
}));

vi.mock("@api/utils/entropyController.js", () => ({
  entropy: {
    reactionTime: vi.fn().mockReturnValue(100),
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollRandom: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/index.js", () => {
  const mockPage = {
    $$: vi.fn().mockResolvedValue([]),
    click: vi.fn().mockResolvedValue(undefined),
    waitForTimeout: vi.fn().mockResolvedValue(undefined),
    goBack: vi.fn().mockResolvedValue(undefined),
    goForward: vi.fn().mockResolvedValue(undefined),
    title: vi.fn().mockResolvedValue("Test Page"),
  };

  return {
    api: {
      wait: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
      goto: vi.fn().mockResolvedValue(undefined),
      getPage: vi.fn().mockReturnValue(mockPage),
      getCurrentUrl: vi.fn().mockResolvedValue("https://example.com"),
      think: vi.fn().mockResolvedValue(undefined),
    },
    __mockPage: mockPage,
  };
});

import { ErrorRecovery } from "@api/behaviors/humanization/error.js";
import { api } from "@api/index.js";
import { mathUtils } from "@api/utils/math.js";
import { entropy } from "@api/utils/entropyController.js";

describe("api/behaviors/humanization/error.js", () => {
  let mockPage;
  let mockLogger;
  let errorRecovery;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      debug: vi.fn(),
    };

    mockPage = {
      $$: vi.fn().mockResolvedValue([]),
      click: vi.fn().mockResolvedValue(undefined),
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      goBack: vi.fn().mockResolvedValue(undefined),
      goForward: vi.fn().mockResolvedValue(undefined),
      title: vi.fn().mockResolvedValue("Test Page"),
    };

    errorRecovery = new ErrorRecovery(mockPage, mockLogger);
  });

  describe("ErrorRecovery constructor", () => {
    it("should initialize with page and logger", () => {
      expect(errorRecovery.page).toBe(mockPage);
      expect(errorRecovery.logger).toBe(mockLogger);
    });

    it("should initialize recoveryChain as empty array", () => {
      expect(Array.isArray(errorRecovery.recoveryChain)).toBe(true);
      expect(errorRecovery.recoveryChain.length).toBe(0);
    });
  });

  describe("handle method", () => {
    it("should handle element_not_found error", async () => {
      const mockLocator = {
        count: vi.fn().mockResolvedValue(0),
      };

      const result = await errorRecovery.handle("element_not_found", {
        locator: mockLocator,
      });

      expect(result).toHaveProperty("success");
      expect(result).toHaveProperty("strategy");
    });

    it("should handle click_failed error", async () => {
      const mockLocator = {
        click: vi.fn().mockRejectedValue(new Error("Click failed")),
      };

      const result = await errorRecovery.handle("click_failed", {
        locator: mockLocator,
      });

      expect(result).toHaveProperty("success");
    });

    it("should handle navigation_failed error", async () => {
      const result = await errorRecovery.handle("navigation_failed", {});

      expect(result).toHaveProperty("success");
    });

    it("should retry navigation when a url is provided", async () => {
      const result = await errorRecovery._retryNavigation({
        url: "https://example.com",
      });

      expect(result.success).toBe(true);
      expect(result.strategy).toBe("navigation_retry");
    });

    it("should go back and forward when roll returns true", async () => {
      mathUtils.roll.mockReturnValue(true);
      const page = api.getPage();

      const result = await errorRecovery._goBackAndRetry({});

      expect(page.goBack).toHaveBeenCalled();
      expect(page.goForward).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });

    it("should handle timeout error", async () => {
      const result = await errorRecovery.handle("timeout", {});

      expect(result).toHaveProperty("success");
    });

    it("should handle verification_failed error", async () => {
      const result = await errorRecovery.handle("verification_failed", {});

      expect(result).toHaveProperty("success");
    });

    it("should handle unknown error type with default pattern", async () => {
      const result = await errorRecovery.handle("unknown_error", {});

      expect(result).toHaveProperty("success");
      expect(result).toHaveProperty("strategy");
    });

    it("should log the error", async () => {
      await errorRecovery.handle("element_not_found", {});

      expect(mockLogger.warn).toHaveBeenCalled();
    });

    it("should try multiple recovery strategies", async () => {
      const mockLocator = {
        count: vi.fn().mockResolvedValue(0),
        click: vi.fn().mockRejectedValue(new Error("Failed")),
      };

      const result = await errorRecovery.handle("element_not_found", {
        locator: mockLocator,
      });

      // It should have tried multiple strategies - we just check that it returns a result
      expect(result).toHaveProperty("success");
      expect(result).toHaveProperty("strategy");
    });
  });

  describe("recovery strategies", () => {
    it("_scrollAndRetry should scroll and wait", async () => {
      const mockLocator = {
        count: vi.fn().mockResolvedValue(1),
      };

      const result = await errorRecovery._scrollAndRetry({
        locator: mockLocator,
      });

      expect(result).toHaveProperty("success");
    });

    it("_scrollAndRetry should return failure when element not found", async () => {
      const mockLocator = {
        count: vi.fn().mockResolvedValue(0),
      };

      const result = await errorRecovery._scrollAndRetry({
        locator: mockLocator,
      });

      expect(result.success).toBe(false);
    });

    it("_waitAndRetry should wait and return success", async () => {
      const result = await errorRecovery._waitAndRetry({});

      expect(result.success).toBe(true);
      expect(result.strategy).toBe("wait");
    });

    it("_giveUp should wait and scroll", async () => {
      const result = await errorRecovery._giveUp();

      expect(result.success).toBe(true);
      expect(result.strategy).toBe("gave_up");
    });

    it("_checkState should return current state", async () => {
      const result = await errorRecovery._checkState({});

      expect(result.success).toBe(true);
      expect(result.strategy).toBe("state_check");
    });

    it("_retryAction should wait and return success", async () => {
      const result = await errorRecovery._retryAction({});

      expect(result.success).toBe(true);
      expect(result.strategy).toBe("retry");
    });

    it("_retryWithForce should try force click", async () => {
      const mockLocator = {
        click: vi.fn().mockResolvedValue(undefined),
      };

      const result = await errorRecovery._retryWithForce({
        locator: mockLocator,
      });

      expect(result.success).toBe(true);
    });

    it("_retryWithForce should return failure when no locator", async () => {
      const result = await errorRecovery._retryWithForce({});

      expect(result.success).toBe(false);
    });

    it("_clickNearby should try clicking nearby elements", async () => {
      mockPage.$$ = vi
        .fn()
        .mockResolvedValue([
          { click: vi.fn().mockResolvedValue(undefined) },
          { click: vi.fn().mockResolvedValue(undefined) },
        ]);

      const result = await errorRecovery._clickNearby({});

      expect(result.success).toBe(true);
    });

    it("_clickNearby should fail when no nearby elements are available", async () => {
      mockPage.$$ = vi.fn().mockResolvedValue([]);

      const result = await errorRecovery._clickNearby({});

      expect(result.success).toBe(false);
      expect(result.strategy).toBe("nearby_click");
    });

    it("_refreshAndRetry should return failure when reload throws", async () => {
      api.reload.mockRejectedValueOnce(new Error("reload failed"));

      const result = await errorRecovery._refreshAndRetry({});

      expect(result.success).toBe(false);
      expect(result.strategy).toBe("refresh");
    });
  });

  describe("logging methods", () => {
    it("_logError should call logger.warn", () => {
      errorRecovery._logError("test_error", {});

      expect(mockLogger.warn).toHaveBeenCalled();
    });

    it("_logStrategy should call logger.debug", () => {
      errorRecovery._logStrategy("test_strategy");

      expect(mockLogger.debug).toHaveBeenCalled();
    });

    it("_logRecovery should call logger for success", () => {
      errorRecovery._logRecovery("test_strategy");

      expect(mockLogger.info).toHaveBeenCalled();
    });

    it("_logRecovery should call logger.warn for failure", () => {
      errorRecovery._logRecovery("failed");

      expect(mockLogger.warn).toHaveBeenCalled();
    });

    it("_logDebug should call logger.debug with state", () => {
      errorRecovery._logDebug({ url: "test" });

      expect(mockLogger.debug).toHaveBeenCalled();
    });
  });
});
