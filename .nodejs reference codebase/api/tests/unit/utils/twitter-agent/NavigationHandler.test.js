/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { NavigationHandler } from "@api/twitter/twitter-agent/NavigationHandler.js";

vi.mock("@api/index.js", () => ({
  api: {
    goto: vi.fn().mockResolvedValue(undefined),
    wait: vi.fn().mockResolvedValue(undefined),
    waitForURL: vi.fn().mockResolvedValue(undefined),
    visible: vi.fn().mockResolvedValue(true),
  },
}));

vi.mock("@api/twitter/twitter-agent/BaseHandler.js", () => ({
  BaseHandler: class MockBaseHandler {
    constructor(agent) {
      this.agent = agent;
      this.page = agent.page;
      this.config = agent.config;
      this.logger = agent.logger;
      this.human = agent.human;
      this.ghost = agent.ghost;
      this.mathUtils = agent.mathUtils;
      this.safeHumanClick = vi.fn().mockResolvedValue(undefined);
    }

    log(msg) {
      this.logger.info(msg);
    }
  },
}));

vi.mock("@api/behaviors/scroll-helper.js", () => ({
  scrollRandom: vi.fn().mockResolvedValue(),
}));

import { api } from "@api/index.js";
import { scrollRandom } from "@api/behaviors/scroll-helper.js";

describe("NavigationHandler", () => {
  let handler;
  let mockAgent;
  let mockPage;
  let mockLogger;
  let mockHuman;
  let mockGhost;
  let mockMathUtils;

  beforeEach(() => {
    vi.clearAllMocks();

    // Re-apply api mock implementations after clearAllMocks
    api.goto.mockResolvedValue(undefined);
    api.wait.mockResolvedValue(undefined);
    api.waitForURL.mockResolvedValue(undefined);
    api.visible.mockResolvedValue(true);
    api.exists = vi.fn().mockResolvedValue(true);

    mockMathUtils = {
      roll: vi.fn().mockReturnValue(false),
      randomInRange: vi
        .fn()
        .mockImplementation((min, max) => Math.floor((min + max) / 2)),
      random: vi.fn().mockReturnValue(0.5),
    };

    mockPage = {
      waitForTimeout: vi.fn().mockResolvedValue(undefined),
      goto: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
      waitForURL: vi.fn().mockResolvedValue(undefined),
      waitForSelector: vi.fn().mockResolvedValue(undefined),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(false),
          count: vi.fn().mockResolvedValue(0),
          textContent: vi.fn().mockResolvedValue(""),
          getAttribute: vi.fn().mockResolvedValue(null),
          click: vi.fn().mockResolvedValue(undefined),
          evaluate: vi.fn().mockResolvedValue(undefined),
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          nth: vi.fn().mockReturnValue({
            isVisible: vi.fn().mockResolvedValue(false),
            textContent: vi.fn().mockResolvedValue(""),
            getAttribute: vi.fn().mockResolvedValue(null),
            click: vi.fn().mockResolvedValue(undefined),
          }),
        }),
      }),
      $$eval: vi.fn(),
      evaluate: vi.fn().mockResolvedValue(undefined),
    };

    mockLogger = {
      info: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
    };

    mockHuman = {
      think: vi.fn().mockResolvedValue(),
      recoverFromError: vi.fn().mockResolvedValue(),
    };

    mockGhost = {
      click: vi.fn().mockResolvedValue({ success: true }),
      move: vi.fn().mockResolvedValue(),
    };

    mockAgent = {
      page: mockPage,
      config: {},
      logger: mockLogger,
      state: {},
      human: mockHuman,
      ghost: mockGhost,
      mathUtils: mockMathUtils,
    };

    handler = new NavigationHandler(mockAgent);

    // Spy on safeHumanClick
    vi.spyOn(handler, "safeHumanClick").mockResolvedValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("navigateHome", () => {
    beforeEach(() => {
      api.goto.mockReset();
      api.goto.mockResolvedValue(undefined);
      api.wait.mockReset();
      api.wait.mockResolvedValue(undefined);
      api.waitForURL.mockReset();
      api.waitForURL.mockResolvedValue(undefined);
    });

    it("should navigate via direct URL when 10% chance hits", async () => {
      mockMathUtils.roll.mockReturnValue(true);
      mockPage.waitForSelector.mockResolvedValue(true);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            nth: vi.fn().mockReturnValue({
              isVisible: vi.fn().mockResolvedValue(true),
              textContent: vi.fn().mockResolvedValue("For you"),
              getAttribute: vi.fn().mockResolvedValue("true"),
            }),
          }),
        }),
      });

      await handler.navigateHome();

      expect(api.goto).toHaveBeenCalledWith("https://x.com/home");
    });

    it("should fallback to click when direct URL fails", async () => {
      mockMathUtils.roll.mockReturnValue(true);
      api.goto.mockRejectedValueOnce(new Error("Network error"));
      mockPage.waitForURL.mockResolvedValue(undefined);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(true),
        }),
      });

      await handler.navigateHome();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Direct URL failed"),
      );
    });

    it("should navigate via Home Icon click (80% chance)", async () => {
      mockMathUtils.roll.mockReturnValue(false);
      mockMathUtils.random.mockReturnValue(0.5);
      mockMathUtils.randomInRange.mockReturnValue(1000);
      api.visible.mockResolvedValue(true);

      const homeBtn = {
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(true),
        }),
      };

      mockPage.locator
        .mockImplementationOnce((selector) => {
          if (selector.includes("Home")) return homeBtn;
          return {
            first: vi
              .fn()
              .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
          };
        })
        .mockImplementationOnce((selector) => {
          return {
            first: vi
              .fn()
              .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
          };
        })
        .mockReturnValue({
          first: vi.fn().mockReturnValue({
            locator: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(1),
              nth: vi.fn().mockReturnValue({
                isVisible: vi.fn().mockResolvedValue(true),
                textContent: vi.fn().mockResolvedValue("For you"),
                getAttribute: vi.fn().mockResolvedValue("true"),
              }),
            }),
          }),
        });

      mockPage.waitForURL.mockResolvedValue(undefined);
      mockPage.waitForSelector.mockResolvedValue(true);

      await handler.navigateHome();

      expect(handler.safeHumanClick).toHaveBeenCalledTimes(1);
      expect(api.waitForURL).toHaveBeenCalledWith("**/home**", {
        timeout: 5000,
      });
    });

    it("should navigate via X Logo click when Math.random >= 0.8", async () => {
      mockMathUtils.roll.mockReturnValue(false);
      mockMathUtils.random.mockReturnValue(0.9);
      mockMathUtils.randomInRange.mockReturnValue(1000);
      api.visible.mockResolvedValue(true);

      mockPage.locator
        .mockImplementationOnce((selector) => {
          if (selector.includes("Home")) {
            return {
              first: vi.fn().mockReturnValue({
                isVisible: vi.fn().mockResolvedValue(false),
              }),
            };
          }
          if (selector.includes('aria-label="X"')) {
            return {
              first: vi.fn().mockReturnValue({
                isVisible: vi.fn().mockResolvedValue(true),
              }),
            };
          }
          return {
            first: vi
              .fn()
              .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
          };
        })
        .mockImplementationOnce(() => {
          return {
            first: vi
              .fn()
              .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(true) }),
          };
        })
        .mockReturnValue({
          first: vi.fn().mockReturnValue({
            locator: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(1),
              nth: vi.fn().mockReturnValue({
                isVisible: vi.fn().mockResolvedValue(true),
                textContent: vi.fn().mockResolvedValue("For you"),
                getAttribute: vi.fn().mockResolvedValue("true"),
              }),
            }),
          }),
        });

      mockPage.waitForURL.mockResolvedValue(undefined);
      mockPage.waitForSelector.mockResolvedValue(true);

      await handler.navigateHome();

      expect(handler.safeHumanClick).toHaveBeenCalledTimes(1);
    });

    it("should catch and handle locator errors", async () => {
      mockMathUtils.roll.mockReturnValue(false);
      mockMathUtils.random.mockReturnValue(0.5);

      mockPage.locator.mockImplementation(() => {
        throw new Error("Locator exploded");
      });

      await handler.navigateHome();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Interaction failed"),
      );
      expect(api.goto).toHaveBeenCalledWith("https://x.com/home");
    });

    it("should not call safeHumanClick when target is not visible", async () => {
      mockMathUtils.roll.mockReturnValue(false);
      mockMathUtils.random.mockReturnValue(0.5);
      mockMathUtils.randomInRange.mockReturnValue(1000);

      // Make api.visible return false for the target
      api.visible.mockResolvedValue(false);

      mockPage.locator
        .mockImplementationOnce(() => ({
          first: vi
            .fn()
            .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
        }))
        .mockImplementationOnce(() => ({
          first: vi
            .fn()
            .mockReturnValue({ isVisible: vi.fn().mockResolvedValue(false) }),
        }))
        .mockReturnValue({
          first: vi.fn().mockReturnValue({
            locator: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(1),
              nth: vi.fn().mockReturnValue({
                isVisible: vi.fn().mockResolvedValue(true),
                textContent: vi.fn().mockResolvedValue("For you"),
                getAttribute: vi.fn().mockResolvedValue("true"),
              }),
            }),
          }),
        });

      mockPage.waitForSelector.mockResolvedValue(true);

      await handler.navigateHome();

      expect(handler.safeHumanClick).not.toHaveBeenCalled();
      expect(api.goto).toHaveBeenCalledWith("https://x.com/home");
    });
  });

  describe("ensureForYouTab", () => {
    it("should select For you tab via text match", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      const mockTab = {
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("For you"),
        getAttribute: vi.fn().mockResolvedValue("false"),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(2),
            nth: vi.fn().mockReturnValue(mockTab),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(handler.safeHumanClick).toHaveBeenCalled();
    });

    it("should not click if already selected", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      const mockTab = {
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("For you"),
        getAttribute: vi.fn().mockResolvedValue("true"),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(2),
            nth: vi.fn().mockReturnValue(mockTab),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(handler.safeHumanClick).not.toHaveBeenCalled();
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("already selected"),
      );
    });

    it("should fallback to index 0 when text not found", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      const mockTab = {
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("Following"),
        getAttribute: vi.fn().mockResolvedValue("false"),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(2),
            nth: vi.fn().mockReturnValue(mockTab),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Fallback to index"),
      );
      expect(handler.safeHumanClick).toHaveBeenCalled();
    });

    it("should return early if tablist not found", async () => {
      mockPage.waitForSelector.mockRejectedValue(new Error("Timeout"));

      await handler.ensureForYouTab();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Tablist not found"),
      );
    });

    it("should fallback to native click when safeHumanClick fails", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      const mockTab = {
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("For you"),
        getAttribute: vi.fn().mockResolvedValue("false"),
        click: vi.fn().mockResolvedValue(undefined),
      };

      handler.safeHumanClick = vi
        .fn()
        .mockRejectedValue(new Error("Click failed"));

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(2),
            nth: vi.fn().mockReturnValue(mockTab),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Ghost click failed"),
      );
      expect(mockTab.click).toHaveBeenCalled();
      vi.restoreAllMocks();
    });

    it("should not click if target tab is not visible", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      // Make api.visible return false for the tab
      api.visible.mockResolvedValue(false);

      const mockTab = {
        isVisible: vi.fn().mockResolvedValue(false),
        textContent: vi.fn().mockResolvedValue("For you"),
        getAttribute: vi.fn().mockResolvedValue("false"),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(2),
            nth: vi.fn().mockReturnValue(mockTab),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(handler.safeHumanClick).not.toHaveBeenCalled();
    });

    it("should handle error when no tabs found", async () => {
      mockPage.waitForSelector.mockResolvedValue(true);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          locator: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
            nth: vi.fn().mockReturnValue(null),
          }),
        }),
      });

      await handler.ensureForYouTab();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("not be found via text or index"),
      );
    });

    it("should handle errors gracefully", async () => {
      mockPage.locator.mockImplementation(() => {
        throw new Error("Tablist exploded");
      });

      await handler.ensureForYouTab();

      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Failed to ensure timeline tab"),
      );
    });
  });

  describe("checkAndClickShowPostsButton", () => {
    it("should click button when found", async () => {
      api.wait.mockResolvedValue(undefined);

      const mockBtn = {
        count: vi.fn().mockResolvedValue(1),
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("Show 5 posts"),
        evaluate: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 200, width: 120, height: 40 }),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue(mockBtn),
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(true);
      expect(handler.safeHumanClick).toHaveBeenCalled();
      expect(scrollRandom).toHaveBeenCalled();
    });

    it("should return false when button not found", async () => {
      api.wait.mockResolvedValue(undefined);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          isVisible: vi.fn().mockResolvedValue(false),
        }),
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(false);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining('No "Show X posts" button found'),
      );
    });

    it("should skip button if text does not match pattern", async () => {
      api.wait.mockResolvedValue(undefined);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(1),
          isVisible: vi.fn().mockResolvedValue(true),
          textContent: vi.fn().mockResolvedValue("Show random stuff"),
        }),
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(false);
    });

    it("should handle error during execution", async () => {
      api.wait.mockRejectedValue(new Error("Wait failed"));

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(false);
      expect(mockLogger.info).toHaveBeenCalledWith(
        expect.stringContaining("Error checking"),
      );
    });

    it("should handle null boundingBox", async () => {
      api.wait.mockResolvedValue(undefined);

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(1),
          isVisible: vi.fn().mockResolvedValue(true),
          textContent: vi.fn().mockResolvedValue("Show 5 posts"),
          evaluate: vi.fn().mockResolvedValue(undefined),
          boundingBox: vi.fn().mockResolvedValue(null),
        }),
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(true);
      expect(mockGhost.move).not.toHaveBeenCalled();
    });

    it("should handle errors from locator gracefully", async () => {
      api.wait.mockResolvedValue(undefined);

      mockPage.locator.mockImplementation(() => {
        throw new Error("Selector error");
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(false);
    });

    it("should try multiple selectors until one works", async () => {
      api.wait.mockResolvedValue(undefined);

      const mockBtn = {
        count: vi.fn().mockResolvedValueOnce(0).mockResolvedValueOnce(1),
        isVisible: vi.fn().mockResolvedValue(true),
        textContent: vi.fn().mockResolvedValue("Show 10 posts"),
        evaluate: vi.fn().mockResolvedValue(undefined),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 200, width: 120, height: 40 }),
      };

      mockPage.locator.mockReturnValue({
        first: vi.fn().mockReturnValue(mockBtn),
      });

      const result = await handler.checkAndClickShowPostsButton();

      expect(result).toBe(true);
    });
  });
});
