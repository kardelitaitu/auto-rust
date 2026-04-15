/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  wait,
  waitWithAbort,
  waitFor,
  waitVisible,
  waitHidden,
  waitForLoadState,
  waitForURL,
} from "@api/interactions/wait.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  getCursor: vi.fn(),
  checkSession: vi.fn(),
}));

const mockLocator = {
  waitFor: vi.fn(),
  isVisible: vi.fn(),
};

vi.mock("@api/utils/locator.js", () => ({
  getLocator: vi.fn(() => ({
    ...mockLocator,
    first: () => mockLocator,
  })),
}));

import { getPage, getCursor, checkSession } from "@api/core/context.js";
import { getLocator } from "@api/utils/locator.js";

describe("api/interactions/wait.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockLocator.waitFor.mockResolvedValue(undefined);
    mockLocator.isVisible.mockResolvedValue(true);

    mockPage = {
      waitForSelector: vi.fn().mockResolvedValue(),
      waitForLoadState: vi.fn().mockResolvedValue(),
      waitForURL: vi.fn().mockResolvedValue(),
      locator: vi.fn().mockReturnValue(mockLocator),
      url: vi.fn().mockReturnValue("http://test.com"),
    };

    getPage.mockReturnValue(mockPage);
    getCursor.mockReturnValue({});
    checkSession.mockReturnValue(undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  describe("wait", () => {
    it("should wait for specified duration with jitter", async () => {
      vi.useFakeTimers();
      const promise = wait(100);

      // Should not resolve immediately
      await Promise.resolve();

      // Fast forward time using async version to properly handle promise resolution
      await vi.advanceTimersByTimeAsync(150); // 100 + max jitter 15 = 115

      await promise;
      vi.useRealTimers();
    });
  });

  describe("waitFor", () => {
    it("should wait for attached selector", async () => {
      await waitFor("#selector", { timeout: 5000 });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "attached",
        timeout: 5000,
      });
    });

    it("should use default timeout", async () => {
      await waitFor("#selector");
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "attached",
        timeout: 10000,
      });
    });

    it("should wait with timeout option", async () => {
      await waitFor("#selector", { timeout: 3000 });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "attached",
        timeout: 3000,
      });
    });

    it("should waitFor with state visible", async () => {
      await waitFor("#selector", { timeout: 5000, state: "visible" });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "visible",
        timeout: 5000,
      });
    });

    it("should waitFor with state hidden", async () => {
      await waitFor("#selector", { timeout: 5000, state: "hidden" });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "hidden",
        timeout: 5000,
      });
    });

    it("should waitFor with immediate return when element already in desired state", async () => {
      mockLocator.waitFor.mockResolvedValueOnce(undefined);
      await waitFor("#selector", { timeout: 5000, state: "attached" });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "attached",
        timeout: 5000,
      });
    });

    it("should throw on waitFor timeout", async () => {
      mockLocator.waitFor.mockRejectedValueOnce(new Error("Timeout"));
      await expect(waitFor("#selector", { timeout: 100 })).rejects.toThrow(
        "Timeout",
      );
    });
  });

  describe("waitVisible", () => {
    it("should wait for visible selector", async () => {
      await waitVisible("#selector");
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "visible",
        timeout: 10000,
      });
    });

    it("should use custom timeout for waitVisible", async () => {
      await waitVisible("#selector", { timeout: 5000 });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "visible",
        timeout: 5000,
      });
    });
  });

  describe("waitHidden", () => {
    it("should wait for hidden selector", async () => {
      await waitHidden("#selector");
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "hidden",
        timeout: 10000,
      });
    });

    it("should use custom timeout for waitHidden", async () => {
      await waitHidden("#selector", { timeout: 5000 });
      expect(mockLocator.waitFor).toHaveBeenCalledWith({
        state: "hidden",
        timeout: 5000,
      });
    });
  });

  describe("waitForURL", () => {
    it("should wait for URL", async () => {
      await waitForURL("http://example.com");
      expect(mockPage.waitForURL).toHaveBeenCalledWith("http://example.com", {
        timeout: 10000,
        waitUntil: undefined,
      });
    });

    it("should wait for URL with waitUntil option", async () => {
      await waitForURL("http://example.com", { waitUntil: "networkidle" });
      expect(mockPage.waitForURL).toHaveBeenCalledWith("http://example.com", {
        timeout: 10000,
        waitUntil: "networkidle",
      });
    });

    it("should wait for URL with custom timeout", async () => {
      await waitForURL("http://example.com", { timeout: 5000 });
      expect(mockPage.waitForURL).toHaveBeenCalledWith("http://example.com", {
        timeout: 5000,
        waitUntil: undefined,
      });
    });

    it("should throw on waitForURL timeout", async () => {
      mockPage.waitForURL.mockRejectedValueOnce(new Error("URL timeout"));
      await expect(waitForURL("http://example.com")).rejects.toThrow(
        "URL timeout",
      );
    });
  });

  describe("waitForLoadState", () => {
    it("should wait for load state", async () => {
      await waitForLoadState("networkidle");
      expect(mockPage.waitForLoadState).toHaveBeenCalledWith("networkidle", {
        timeout: 10000,
      });
    });

    it("should wait for load state with custom timeout", async () => {
      await waitForLoadState("domcontentloaded", { timeout: 5000 });
      expect(mockPage.waitForLoadState).toHaveBeenCalledWith(
        "domcontentloaded",
        {
          timeout: 5000,
        },
      );
    });

    it("should throw on load state error", async () => {
      mockPage.waitForLoadState.mockRejectedValueOnce(new Error("Load failed"));
      await expect(waitForLoadState("networkidle")).rejects.toThrow(
        "Load failed",
      );
    });
  });

  describe("waitWithAbort", () => {
    it("should wait for specified duration with jitter", async () => {
      vi.useFakeTimers();
      const promise = waitWithAbort(100);

      // Should not resolve immediately
      await Promise.resolve();

      // Fast forward time using async version to properly handle promise resolution
      await vi.advanceTimersByTimeAsync(150); // 100 + max jitter 15 = 115

      await promise;
      vi.useRealTimers();
    });

    it("should reject when signal is already aborted", async () => {
      const controller = new AbortController();
      controller.abort();

      await expect(waitWithAbort(100, controller.signal)).rejects.toThrow(
        "This operation was aborted",
      );
    });

    it("should reject when signal is aborted during wait", async () => {
      vi.useFakeTimers();
      const controller = new AbortController();

      // Start waiting and immediately attach rejection handler
      const waitPromise = waitWithAbort(1000, controller.signal);

      // Create a promise that will be rejected when abort happens
      const rejectionPromise = expect(waitPromise).rejects.toThrow(
        "This operation was aborted",
      );

      // Abort the signal
      controller.abort();
      await vi.advanceTimersByTimeAsync(0);

      // Wait for the rejection to be handled
      await rejectionPromise;

      vi.useRealTimers();
    });

    it("should work without signal parameter", async () => {
      vi.useFakeTimers();
      const promise = waitWithAbort(100);

      await vi.advanceTimersByTimeAsync(150);
      await promise;
      vi.useRealTimers();
    });
  });
});
