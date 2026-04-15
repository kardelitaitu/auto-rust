/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { vi, describe, it, expect, beforeEach } from "vitest";

import { api } from "@api/index.js";
import ErrorRecovery from "@api/behaviors/humanization/error.js";
import { mathUtils } from "@api/utils/math.js";
import * as scrollHelper from "@api/behaviors/scroll-helper.js";

vi.mock("@api/index.js", () => ({
  api: {
    setPage: vi.fn(() => undefined),
    getPage: vi.fn(),
    wait: vi.fn().mockResolvedValue(undefined),
    think: vi.fn().mockResolvedValue(undefined),
    getPersona: vi
      .fn()
      .mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
      toTop: vi.fn().mockResolvedValue(undefined),
      back: vi.fn().mockResolvedValue(undefined),
      read: vi.fn().mockResolvedValue(undefined),
      focus: vi.fn().mockResolvedValue(undefined),
    }),
    visible: vi.fn().mockImplementation(async (el) => {
      if (el && typeof el.isVisible === "function") return await el.isVisible();
      if (el && typeof el.count === "function") return (await el.count()) > 0;
      return true;
    }),
    exists: vi.fn().mockImplementation(async (el) => {
      if (el && typeof el.count === "function") return (await el.count()) > 0;
      return el !== null;
    }),
    getCurrentUrl: vi.fn().mockResolvedValue("https://x.com/home"),
    goto: vi.fn().mockResolvedValue(undefined),
    reload: vi.fn().mockResolvedValue(undefined),
    eval: vi.fn().mockResolvedValue("mock result"),
    text: vi.fn().mockResolvedValue("mock text"),
    click: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn(),
    roll: vi.fn(),
    gaussian: vi.fn(),
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollRandom: vi.fn().mockResolvedValue(undefined),
}));

describe("ErrorRecovery", () => {
  let errorRecovery;
  let mockPage;
  let mockLogger;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
      goBack: vi.fn().mockResolvedValue(undefined),
      goForward: vi.fn().mockResolvedValue(undefined),
      goto: vi.fn().mockResolvedValue(undefined),
      url: vi.fn().mockResolvedValue("https://example.com"),
      title: vi.fn().mockResolvedValue("Example"),
      $$: vi.fn().mockResolvedValue([]),
      $: vi.fn().mockResolvedValue(null),
      isClosed: vi.fn().mockReturnValue(false),
      context: vi.fn().mockReturnValue({
        browser: vi
          .fn()
          .mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
      }),
    };

    api.getPage.mockReturnValue(mockPage);
    api.setPage.mockReturnValue(undefined);

    mathUtils.randomInRange.mockImplementation((min) => min);
    mathUtils.roll.mockReturnValue(true);
    mathUtils.gaussian.mockImplementation((m) => m);

    mockLogger = {
      log: vi.fn(),
      error: vi.fn(),
      warn: vi.fn(),
      debug: vi.fn(),
      info: vi.fn(),
    };

    errorRecovery = new ErrorRecovery(mockPage, mockLogger);
  });

  describe("handle", () => {
    it("should handle element_not_found with scroll strategy", async () => {
      const mockLocator = { count: vi.fn().mockResolvedValue(1) };
      const result = await errorRecovery.handle("element_not_found", {
        locator: mockLocator,
      });
      expect(result.success).toBe(true);
      expect(result.strategy).toBe("scroll");
    });

    it("should give up if all fail", async () => {
      scrollHelper.scrollRandom.mockRejectedValueOnce(new Error("fail"));
      api.reload.mockRejectedValueOnce(new Error("fail"));

      const result = await errorRecovery.handle("element_not_found", {
        locator: { count: vi.fn().mockResolvedValue(0) },
      });

      expect(result.strategy).toBe("gave_up");
    });

    it("should handle click_failed with click nearby", async () => {
      const mockEl = { click: vi.fn().mockResolvedValue(undefined) };
      mockPage.$$ = vi.fn().mockResolvedValue([mockEl, mockEl]);
      const result = await errorRecovery.handle("click_failed");
      expect(result.success).toBe(true);
      expect(result.strategy).toBe("nearby_click");
    });

    it("should handle timeout with wait strategy", async () => {
      const result = await errorRecovery.handle("timeout");
      expect(result.success).toBe(true);
      expect(result.strategy).toBe("wait");
    });

    it("should handle navigation failed with retry navigation", async () => {
      const result = await errorRecovery.handle("navigation_failed", {
        url: "https://x.com",
      });
      expect(api.goto).toHaveBeenCalledWith(
        "https://x.com",
        expect.any(Object),
      );
      expect(result.success).toBe(true);
      expect(result.strategy).toBe("navigation_retry");
    });
  });
});
