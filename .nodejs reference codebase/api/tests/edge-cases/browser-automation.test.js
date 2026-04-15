/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Edge Case Tests: Browser Automation Scenarios
 *
 * Tests for handling browser automation edge cases:
 * - Element not found scenarios
 * - Stale element references
 * - Page navigation races
 * - Frame switching issues
 * - Browser crash handling
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("Edge Cases: Browser Automation", () => {
  describe("Element Not Found Scenarios", () => {
    it("should handle element not found gracefully", async () => {
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          first: vi.fn().mockReturnThis(),
          isVisible: vi.fn().mockResolvedValue(false),
        }),
      };

      const element = mockPage.locator('[data-testid="missing-element"]');
      const count = await element.count();

      expect(count).toBe(0);
    });

    it("should handle timeout waiting for element", async () => {
      const waitForSelector = vi
        .fn()
        .mockRejectedValue(
          new Error(
            'TimeoutError: Waiting for selector [data-testid="btn"] failed: Timeout 5000ms exceeded',
          ),
        );

      await expect(
        waitForSelector('[data-testid="btn"]', { timeout: 5000 }),
      ).rejects.toThrow("Timeout");
    });

    it("should implement retry with different selectors", async () => {
      const selectors = [
        '[data-testid="button-primary"]',
        '[data-testid="button-secondary"]',
        'button:has-text("Submit")',
        ".submit-btn",
      ];

      let foundSelector = null;
      const mockLocator = vi.fn().mockImplementation((selector) => {
        const exists = selector.includes("secondary");
        if (exists) foundSelector = selector;
        return { count: vi.fn().mockResolvedValue(exists ? 1 : 0) };
      });

      for (const selector of selectors) {
        const el = mockLocator(selector);
        const count = await el.count();
        if (count > 0) {
          foundSelector = selector;
          break;
        }
      }

      expect(foundSelector).toBe('[data-testid="button-secondary"]');
    });

    it("should handle dynamic element appearance", async () => {
      let callCount = 0;
      const mockLocator = vi.fn().mockImplementation(() => {
        callCount++;
        return {
          isVisible: vi.fn().mockResolvedValue(callCount >= 3),
          count: vi.fn().mockResolvedValue(callCount >= 3 ? 1 : 0),
        };
      });

      // Simulate polling for element
      const maxAttempts = 5;
      let elementFound = false;

      for (let i = 0; i < maxAttempts; i++) {
        const el = mockLocator('[data-testid="dynamic"]');
        if ((await el.count()) > 0) {
          elementFound = true;
          break;
        }
      }

      expect(elementFound).toBe(true);
    });
  });

  describe("Stale Element References", () => {
    it("should detect stale element", async () => {
      const staleError = new Error(
        "Error: Element has become stale and is no longer attached to the DOM",
      );

      const mockElement = {
        click: vi.fn().mockRejectedValue(staleError),
      };

      await expect(mockElement.click()).rejects.toThrow("stale");
    });

    it("should implement stale element recovery", async () => {
      let attempts = 0;
      const mockPage = {
        locator: vi.fn().mockImplementation(() => ({
          click: vi.fn().mockImplementation(() => {
            attempts++;
            if (attempts < 3) {
              throw new Error("Element is stale");
            }
            return Promise.resolve();
          }),
        })),
      };

      const robustClick = async (selector, maxAttempts = 3) => {
        for (let i = 0; i < maxAttempts; i++) {
          try {
            const element = mockPage.locator(selector);
            await element.click();
            return true;
          } catch (error) {
            if (error.message.includes("stale") && i < maxAttempts - 1) {
              continue; // Re-fetch element
            }
            throw error;
          }
        }
        return false;
      };

      const result = await robustClick('[data-testid="btn"]');
      expect(result).toBe(true);
      expect(attempts).toBe(3);
    });

    it("should handle detached frame elements", async () => {
      const mockFrame = {
        isDetached: vi.fn().mockReturnValue(true),
        locator: vi.fn(),
      };

      expect(mockFrame.isDetached()).toBe(true);

      // Should not attempt to use detached frame
      if (mockFrame.isDetached()) {
        expect(mockFrame.locator).not.toHaveBeenCalled();
      }
    });
  });

  describe("Page Navigation Races", () => {
    it("should handle navigation during click", async () => {
      const navigationError = new Error(
        "Navigation interrupted by another navigation",
      );

      const mockClick = vi.fn().mockRejectedValue(navigationError);

      await expect(mockClick()).rejects.toThrow("Navigation interrupted");
    });

    it("should wait for page load after navigation", async () => {
      const mockPage = {
        goto: vi.fn().mockResolvedValue(undefined),
        waitForLoadState: vi.fn().mockResolvedValue(undefined),
      };

      const safeNavigate = async (url) => {
        await mockPage.goto(url);
        await mockPage.waitForLoadState("domcontentloaded");
      };

      await safeNavigate("https://example.com");

      expect(mockPage.goto).toHaveBeenCalledWith("https://example.com");
      expect(mockPage.waitForLoadState).toHaveBeenCalledWith(
        "domcontentloaded",
      );
    });

    it("should handle redirect chains", async () => {
      const redirectChain = [];
      const mockResponse = {
        request: vi.fn().mockReturnValue({
          redirectedFrom: vi.fn().mockReturnValue({
            url: () => "https://original.com",
          }),
        }),
        url: () => "https://final.com",
      };

      // Simulate building redirect chain
      redirectChain.push(mockResponse.request().redirectedFrom().url());
      redirectChain.push(mockResponse.url());

      expect(redirectChain).toEqual([
        "https://original.com",
        "https://final.com",
      ]);
    });

    it("should handle SPA navigation without page reload", async () => {
      let url = "https://example.com/page1";
      let loadCount = 0;

      const mockPage = {
        url: vi.fn().mockImplementation(() => url),
        waitForNavigation: vi.fn().mockImplementation(async () => {
          // In SPA, URL changes without full page load
          url = "https://example.com/page2";
        }),
      };

      const navigateSPA = async (newUrl) => {
        // SPAs don't trigger full navigation
        url = newUrl;
        loadCount++;
      };

      await navigateSPA("https://example.com/new-page");
      expect(mockPage.url()).toBe("https://example.com/new-page");
    });
  });

  describe("Frame and Context Issues", () => {
    it("should handle iframe switching", async () => {
      const mockPage = {
        frame: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            click: vi.fn().mockResolvedValue(undefined),
          }),
        }),
        mainFrame: vi.fn().mockReturnThis(),
      };

      // Switch to iframe
      const iframe = mockPage.frame("iframe#content");
      expect(iframe).toBeDefined();

      // Perform action in iframe
      await iframe.locator("button").click();
      expect(iframe.locator).toHaveBeenCalledWith("button");
    });

    it("should handle missing iframe", async () => {
      const mockPage = {
        frame: vi.fn().mockReturnValue(null),
      };

      const iframe = mockPage.frame("iframe#missing");
      expect(iframe).toBeNull();
    });

    it("should handle multiple browser contexts", () => {
      const contexts = [];

      const createContext = () => {
        const context = {
          id: `context-${contexts.length}`,
          pages: [],
          close: vi.fn(),
        };
        contexts.push(context);
        return context;
      };

      const ctx1 = createContext();
      const ctx2 = createContext();

      expect(contexts.length).toBe(2);
      expect(ctx1.id).not.toBe(ctx2.id);
    });

    it("should handle popup windows", async () => {
      const mockPage = {
        context: vi.fn().mockReturnValue({
          pages: vi
            .fn()
            .mockReturnValue([
              { url: () => "https://main.com" },
              { url: () => "https://popup.com" },
            ]),
        }),
      };

      const pages = mockPage.context().pages();
      expect(pages.length).toBe(2);
    });
  });

  describe("Browser Crash Handling", () => {
    it("should detect browser crash", async () => {
      const mockBrowser = {
        isConnected: vi.fn().mockReturnValue(false),
        contexts: vi.fn().mockRejectedValue(new Error("Browser disconnected")),
      };

      expect(mockBrowser.isConnected()).toBe(false);
    });

    it("should handle page crash", async () => {
      const crashHandler = vi.fn();
      const mockPage = {
        on: vi.fn().mockImplementation((event, handler) => {
          if (event === "crash") {
            crashHandler.mockImplementation(handler);
          }
        }),
        isClosed: vi.fn().mockReturnValue(false),
      };

      mockPage.on("crash", () => {
        mockPage.isClosed.mockReturnValue(true);
      });

      // Simulate crash
      crashHandler();

      expect(mockPage.isClosed()).toBe(true);
    });

    it("should implement browser reconnection", async () => {
      let connected = true;
      let reconnectionAttempts = 0;

      const browser = {
        connect: vi.fn().mockImplementation(() => {
          reconnectionAttempts++;
          if (reconnectionAttempts < 3) {
            throw new Error("Connection failed");
          }
          connected = true;
          return { connected: true };
        }),
        disconnect: vi.fn().mockImplementation(() => {
          connected = false;
        }),
      };

      // Simulate disconnection
      browser.disconnect();
      expect(connected).toBe(false);

      // Reconnect with retry
      let result;
      for (let i = 0; i < 5; i++) {
        try {
          result = await browser.connect();
          break;
        } catch {
          if (i === 4) throw new Error("Max reconnection attempts");
        }
      }

      expect(result.connected).toBe(true);
      expect(reconnectionAttempts).toBe(3);
    });
  });

  describe("Viewport and Rendering Issues", () => {
    it("should handle element outside viewport", async () => {
      const mockElement = {
        boundingBox: vi.fn().mockResolvedValue(null),
        scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
      };

      const box = await mockElement.boundingBox();
      expect(box).toBeNull();

      // Should scroll to make element visible
      await mockElement.scrollIntoViewIfNeeded();
      expect(mockElement.scrollIntoViewIfNeeded).toHaveBeenCalled();
    });

    it("should handle element covered by overlay", async () => {
      const mockPage = {
        evaluate: vi.fn().mockResolvedValue(true),
      };

      const isElementCovered = async (element) => {
        return await mockPage.evaluate((el) => {
          const rect = el.getBoundingClientRect();
          const centerX = rect.left + rect.width / 2;
          const centerY = rect.top + rect.height / 2;
          const topElement = document.elementFromPoint(centerX, centerY);
          return topElement && !el.contains(topElement);
        }, element);
      };

      const mockElement = { id: "test-element" };
      const covered = await isElementCovered(mockElement);

      expect(mockPage.evaluate).toHaveBeenCalled();
      expect(covered).toBe(true);
    });

    it("should handle window resize during automation", async () => {
      let viewport = { width: 1920, height: 1080 };
      const resizeHandlers = [];

      const mockPage = {
        setViewportSize: vi.fn().mockImplementation((size) => {
          viewport = size;
          resizeHandlers.forEach((handler) => handler(viewport));
        }),
        on: vi.fn().mockImplementation((event, handler) => {
          if (event === "resize") {
            resizeHandlers.push(handler);
          }
        }),
      };

      // Simulate resize
      mockPage.on("resize", (vp) => {
        viewport = vp;
      });

      mockPage.setViewportSize({ width: 800, height: 600 });
      expect(viewport.width).toBe(800);
    });

    it("should handle headless mode detection differences", () => {
      const mockNavigator = {
        userAgent:
          "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 HeadlessChrome",
        webdriver: true,
      };

      const isHeadless =
        mockNavigator.userAgent.includes("Headless") ||
        mockNavigator.webdriver === true;

      expect(isHeadless).toBe(true);
    });
  });

  describe("Timeout Handling", () => {
    it("should implement configurable timeouts", () => {
      const defaultTimeouts = {
        navigation: 30000,
        selector: 10000,
        action: 5000,
        wait: 30000,
      };

      const customTimeouts = {
        ...defaultTimeouts,
        navigation: 60000,
      };

      expect(customTimeouts.navigation).toBe(60000);
      expect(customTimeouts.selector).toBe(10000);
    });

    it("should handle timeout with abort signal", async () => {
      const abortController = new AbortController();

      const timeoutId = setTimeout(() => {
        abortController.abort();
      }, 1000);

      const longOperation = new Promise((resolve, reject) => {
        abortController.signal.addEventListener("abort", () => {
          reject(new Error("Operation aborted due to timeout"));
        });

        // Use fake timers instead of actual 5s wait
        setTimeout(resolve, 5000);
      });

      vi.useFakeTimers();

      // Fire the timeout immediately
      vi.advanceTimersByTime(5000);

      await expect(longOperation).rejects.toThrow("aborted");
      clearTimeout(timeoutId);

      vi.useRealTimers();
    });

    it("should implement timeout per operation type", () => {
      const timeoutManager = {
        timeouts: {
          click: 5000,
          type: 10000,
          navigation: 30000,
          waitForSelector: 10000,
        },

        getTimeout(operation) {
          return this.timeouts[operation] || 10000;
        },

        setTimeout(operation, timeout) {
          this.timeouts[operation] = timeout;
        },
      };

      expect(timeoutManager.getTimeout("click")).toBe(5000);
      expect(timeoutManager.getTimeout("unknown")).toBe(10000);

      timeoutManager.setTimeout("custom", 25000);
      expect(timeoutManager.getTimeout("custom")).toBe(25000);
    });
  });
});
