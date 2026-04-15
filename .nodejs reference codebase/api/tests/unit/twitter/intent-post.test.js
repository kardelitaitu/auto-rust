/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { post } from "@api/twitter/intent-post.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    success: vi.fn(),
  })),
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue(undefined),
  waitForLoadState: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/interactions/navigation.js", () => ({
  back: vi.fn().mockResolvedValue(undefined),
  goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@api/interactions/queries.js", () => ({
  visible: vi.fn().mockResolvedValue(true),
}));

vi.mock("@api/utils/math.js", () => ({
  mathUtils: {
    randomInRange: vi.fn().mockReturnValue(2000),
  },
}));

describe("intent-post.js", () => {
  let mockPage;
  let mockGetPage;
  let mockGoto;
  let mockBack;
  let mockClick;
  let mockVisible;
  let mockWait;
  let mockWaitForLoadState;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    const { getPage } = await import("@api/core/context.js");
    mockGetPage = getPage;

    const { goto, back } = await import("@api/interactions/navigation.js");
    mockGoto = goto;
    mockBack = back;

    const { click } = await import("@api/interactions/actions.js");
    mockClick = click;

    const { visible } = await import("@api/interactions/queries.js");
    mockVisible = visible;

    const { wait, waitForLoadState } =
      await import("@api/interactions/wait.js");
    mockWait = wait;
    mockWaitForLoadState = waitForLoadState;

    mockPage = { url: vi.fn().mockReturnValue("https://example.com") };
    mockGetPage.mockReturnValue(mockPage);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("post", () => {
    it("should be a function", () => {
      expect(typeof post).toBe("function");
    });

    it("should return missing_parameters when text is empty", async () => {
      const result = await post("");
      expect(result).toEqual({ success: false, reason: "missing_parameters" });
    });

    it("should return missing_parameters when text is null", async () => {
      const result = await post(null);
      expect(result).toEqual({ success: false, reason: "missing_parameters" });
    });

    it("should return missing_parameters when text is undefined", async () => {
      const result = await post(undefined);
      expect(result).toEqual({ success: false, reason: "missing_parameters" });
    });

    it("should navigate to intent URL with encoded text", async () => {
      mockVisible.mockResolvedValue(true);

      const resultPromise = post("Hello world!");

      // Fast-forward past the wait calls
      await vi.runAllTimersAsync();

      const result = await resultPromise;
      expect(mockGoto).toHaveBeenCalled();
    });

    it("should return success when tweet button is clicked", async () => {
      mockVisible.mockResolvedValue(true);

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      const result = await resultPromise;
      expect(mockClick).toHaveBeenCalled();
      expect(result.success).toBe(true);
    });

    it("should return tweet_button_not_found when button not visible", async () => {
      mockVisible.mockResolvedValue(false);

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      const result = await resultPromise;
      expect(mockClick).not.toHaveBeenCalled();
      expect(result.success).toBe(false);
      expect(result.reason).toBe("tweet_button_not_found");
    });

    it("should call back() in finally block after navigation", async () => {
      mockVisible.mockResolvedValue(true);

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      expect(mockBack).toHaveBeenCalled();
    });

    it("should return timeout when operation takes too long", async () => {
      mockVisible.mockImplementation(() => new Promise(() => {})); // Never resolves

      const resultPromise = post("Test tweet");

      // Advance 20 seconds to trigger timeout
      await vi.advanceTimersByTimeAsync(20000);

      const result = await resultPromise;
      expect(result.success).toBe(false);
      expect(result.reason).toBe("timeout");
    });

    it("should return unhandled_error on exception", async () => {
      mockGetPage.mockImplementation(() => {
        throw new Error("Page error");
      });

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      const result = await resultPromise;
      expect(result.success).toBe(false);
      expect(result.reason).toBe("unhandled_error");
    });

    it("should call wait with random delay after load", async () => {
      mockVisible.mockResolvedValue(true);
      const { mathUtils } = await import("@api/utils/math.js");

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      expect(mockWait).toHaveBeenCalled();
      expect(mathUtils.randomInRange).toHaveBeenCalled();
    });

    it("should wait for load state after navigation", async () => {
      mockVisible.mockResolvedValue(true);

      const resultPromise = post("Test tweet");

      await vi.runAllTimersAsync();

      expect(mockWaitForLoadState).toHaveBeenCalledWith("domcontentloaded");
    });

    it("should handle special characters in text", async () => {
      mockVisible.mockResolvedValue(true);

      const resultPromise = post("Hello & goodbye <world>");

      await vi.runAllTimersAsync();

      expect(mockGoto).toHaveBeenCalled();
    });
  });
});
