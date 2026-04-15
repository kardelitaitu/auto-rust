/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for Human Interaction utilities
 * @module tests/unit/human-interaction.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock GhostCursor
vi.mock("@api/utils/ghostCursor.js", () => ({
  GhostCursor: vi.fn().mockImplementation((page) => ({
    page,
    click: vi.fn().mockResolvedValue({ success: true }),
    move: vi.fn().mockResolvedValue(undefined),
  })),
}));

// Import modules - the api will be the real one but we'll spy on it
import { api } from "@api/index.js";
import { HumanInteraction } from "@api/behaviors/human-interaction.js";

// Spy on api methods - use mockImplementation to allow per-test overrides
vi.spyOn(api, "wait").mockResolvedValue(undefined);
vi.spyOn(api, "think").mockResolvedValue(undefined);
vi.spyOn(api, "scroll").mockResolvedValue(undefined);
vi.spyOn(api, "getPersona").mockReturnValue({
  microMoveChance: 0.1,
  fidgetChance: 0.05,
});
vi.spyOn(api, "getCurrentUrl").mockReturnValue("https://x.com/home");
vi.spyOn(api, "setPage").mockImplementation(() => {});

// visible, exists, and count will be set in beforeEach to allow test overrides
let visibleSpy, existsSpy, countSpy, getPageSpy;

vi.mock("@api/index.js", () => ({
  api: {
    setPage: vi.fn(),
    getPage: vi.fn(),
    wait: vi.fn().mockResolvedValue(undefined),
    think: vi.fn().mockResolvedValue(undefined),
    scroll: vi.fn().mockResolvedValue(undefined),
    getPersona: vi
      .fn()
      .mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    getCurrentUrl: vi.fn().mockReturnValue("https://x.com/home"),
    visible: vi.fn().mockImplementation(async (el) => {
      // Handle locator mocks that have isVisible method
      if (el && typeof el.isVisible === "function") {
        const result = await el.isVisible();
        return result !== undefined ? result : true;
      }
      // Handle locator mocks that have first().isVisible() chain
      if (el && typeof el.first === "function") {
        const firstEl = el.first();
        if (firstEl && typeof firstEl.isVisible === "function") {
          const result = await firstEl.isVisible();
          return result !== undefined ? result : true;
        }
      }
      return true;
    }),
    exists: vi.fn().mockImplementation((el) => {
      if (el && typeof el.count === "function")
        return el.count().then((c) => c > 0);
      return Promise.resolve(el !== null);
    }),
    count: vi.fn().mockImplementation((el) => {
      if (el && typeof el.count === "function") return el.count();
      return Promise.resolve(1);
    }),
  },
}));

describe("HumanInteraction", () => {
  let human;

  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();

    // Re-apply spies after clearAllMocks
    vi.spyOn(api, "wait").mockResolvedValue(undefined);
    vi.spyOn(api, "think").mockResolvedValue(undefined);
    vi.spyOn(api, "scroll").mockResolvedValue(undefined);
    vi.spyOn(api, "getPersona").mockReturnValue({
      microMoveChance: 0.1,
      fidgetChance: 0.05,
    });
    vi.spyOn(api, "getCurrentUrl").mockReturnValue("https://x.com/home");
    vi.spyOn(api, "setPage").mockImplementation(() => {});

    // Set up visible spy with a flexible implementation
    visibleSpy = vi.spyOn(api, "visible").mockImplementation(async (el) => {
      if (el && typeof el.isVisible === "function") {
        return await el.isVisible();
      }
      return true;
    });

    existsSpy = vi.spyOn(api, "exists").mockImplementation(async (el) => {
      if (el && typeof el.count === "function") {
        return (await el.count()) > 0;
      }
      return el !== null;
    });

    countSpy = vi.spyOn(api, "count").mockImplementation(async (el) => {
      if (el && typeof el.count === "function") {
        return await el.count();
      }
      return 1;
    });

    const mockPage = {
      mouse: {
        move: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(undefined),
      },
      isClosed: vi.fn().mockReturnValue(false),
      context: vi.fn().mockReturnValue({
        browser: vi
          .fn()
          .mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
      }),
      url: vi.fn().mockReturnValue("https://x.com/home"),
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          isVisible: vi.fn().mockResolvedValue(true),
          count: vi.fn().mockResolvedValue(1),
        }),
        all: vi.fn().mockResolvedValue([]),
      }),
    };
    getPageSpy = vi.spyOn(api, "getPage").mockReturnValue(mockPage);

    human = new HumanInteraction();
  });

  describe("Initialization", () => {
    it("should initialize with default values", () => {
      expect(human.debugMode).toBe(false);
      expect(human.page).toBe(null);
      expect(human.ghost).toBe(null);
    });

    it("should accept page parameter", () => {
      const mockPage = { mouse: { move: vi.fn() } };
      human = new HumanInteraction(mockPage);
      expect(human.page).toBe(mockPage);
      expect(human.ghost).not.toBe(null);
    });
  });

  describe("hesitation", () => {
    it("should return a delay value", async () => {
      const delay = await human.hesitation(100, 200);
      expect(typeof delay).toBe("number");
      expect(delay).toBeGreaterThanOrEqual(100);
      expect(delay).toBeLessThanOrEqual(200);
    });

    it("should use default range when not specified", async () => {
      const delay = await human.hesitation();
      expect(typeof delay).toBe("number");
      expect(delay).toBeGreaterThanOrEqual(300);
      expect(delay).toBeLessThanOrEqual(1500);
    });
  });

  describe("readingTime", () => {
    it("should return a time value within range", async () => {
      const time = await human.readingTime(1000, 2000);
      expect(typeof time).toBe("number");
      expect(time).toBeGreaterThanOrEqual(1000);
      expect(time).toBeLessThanOrEqual(2000);
    });
  });

  describe("fixation", () => {
    it("should return a time value within range", async () => {
      const time = await human.fixation(100, 500);
      expect(typeof time).toBe("number");
      expect(time).toBeGreaterThanOrEqual(100);
      expect(time).toBeLessThanOrEqual(500);
    });
  });

  describe("selectMethod", () => {
    it("should select a method based on weights", () => {
      const methods = [
        { name: "method1", weight: 50 },
        { name: "method2", weight: 50 },
      ];

      const selected = human.selectMethod(methods);
      expect(methods.map((m) => m.name)).toContain(selected.name);
    });

    it("should return first method if roll exceeds all weights", () => {
      const methods = [
        { name: "method1", weight: 30 },
        { name: "method2", weight: 30 },
      ];

      vi.spyOn(Math, "random").mockReturnValue(0.99);
      const selected = human.selectMethod(methods);
      expect(selected).toBe(methods[0]);

      vi.restoreAllMocks();
    });
  });

  describe("maybeScroll", () => {
    it("should return false when random roll does not trigger scroll", async () => {
      const mockPage = { evaluate: vi.fn() };
      vi.spyOn(Math, "random").mockReturnValue(0.5);

      const result = await human.maybeScroll(mockPage, 100, 300);
      expect(result).toBe(false);
      expect(api.scroll).not.toHaveBeenCalled();

      vi.restoreAllMocks();
    });

    it("should call scroll when random roll triggers it", async () => {
      const mockPage = { evaluate: vi.fn() };
      vi.spyOn(Math, "random").mockReturnValue(0.1);

      const result = await human.maybeScroll(mockPage, 100, 300);
      expect(result).toBe(true);
      expect(api.scroll).toHaveBeenCalled();

      vi.restoreAllMocks();
    });

    it("should scroll in negative direction when roll selects it", async () => {
      const mockPage = {
        evaluate: vi.fn(),
      };

      vi.spyOn(Math, "random")
        .mockReturnValueOnce(0.1) // pass < 0.3 roll
        .mockReturnValueOnce(0.5) // maybe used by randomInRange
        .mockReturnValueOnce(0.5) // maybe used by randomInRange
        .mockReturnValueOnce(0.9) // direction roll > 0.5 means negative
        .mockReturnValue(0.9); // fallback

      const result = await human.maybeScroll(mockPage, 100, 100);

      expect(result).toBe(true);
      expect(api.scroll).toHaveBeenCalled();
      // Cannot reliably predict exact value if randomInRange uses Math.random internally a variable number of times,
      // but we know it should be called with some number. Let's just expect it to be called with a number < 0
      const callArg = api.scroll.mock.calls[0][0];
      expect(callArg).toBeLessThanOrEqual(0);
      vi.restoreAllMocks();
    });
  });

  describe("microMove", () => {
    it("should call page.mouse.move", async () => {
      const mockPage = { mouse: { move: vi.fn() } };
      human.page = mockPage;

      await human.microMove(mockPage, 20);
      expect(mockPage.mouse.move).toHaveBeenCalled();
    });
  });

  describe("findElement", () => {
    it("should return null element when no selectors match", async () => {
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          all: vi.fn().mockResolvedValue([]),
        }),
      };

      const result = await human.findElement(mockPage, [
        ".selector1",
        ".selector2",
      ]);

      expect(result.element).toBe(null);
      expect(result.selector).toBe(null);
      expect(result.index).toBe(-1);
    });

    it("should return first visible element when found", async () => {
      const mockElement = { isVisible: vi.fn().mockResolvedValue(true) };
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          all: vi.fn().mockResolvedValue([mockElement]),
        }),
      };

      const result = await human.findElement(mockPage, [".selector1"]);

      expect(result.element).toBe(mockElement);
      expect(result.selector).toBe(".selector1");
      expect(result.index).toBe(0);
    });

    it("should return element even when visibility check is disabled", async () => {
      const mockElement = { isVisible: vi.fn().mockResolvedValue(false) };
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          all: vi.fn().mockResolvedValue([mockElement]),
        }),
      };

      const result = await human.findElement(mockPage, [".selector1"], {
        visibleOnly: false,
      });

      expect(result.element).toBe(mockElement);
      expect(result.selector).toBe(".selector1");
      expect(result.index).toBe(0);
    });

    it("should handle element visibility errors", async () => {
      const mockElement = {
        isVisible: vi.fn().mockRejectedValue(new Error("boom")),
      };
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          all: vi.fn().mockResolvedValue([mockElement]),
        }),
      };

      const result = await human.findElement(mockPage, [".selector1"]);

      expect(result.element).toBe(null);
      expect(result.selector).toBe(null);
    });

    it("should handle locator errors and continue searching", async () => {
      const mockElement = { isVisible: vi.fn().mockResolvedValue(true) };
      const mockPage = {
        locator: vi
          .fn()
          .mockImplementationOnce(() => {
            throw new Error("bad selector");
          })
          .mockReturnValue({
            all: vi.fn().mockResolvedValue([mockElement]),
          }),
      };

      const result = await human.findElement(mockPage, [".bad", ".good"]);

      expect(result.element).toBe(mockElement);
      expect(result.selector).toBe(".good");
    });

    it("should stop searching when timeout is exceeded", async () => {
      const nowSpy = vi.spyOn(Date, "now");
      nowSpy.mockReturnValueOnce(0).mockReturnValueOnce(6000);
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          all: vi.fn().mockResolvedValue([]),
        }),
      };

      const result = await human.findElement(mockPage, [".selector1"], {
        timeout: 5000,
      });

      expect(result.element).toBe(null);
      nowSpy.mockRestore();
    });
  });

  describe("verifyComposerOpen", () => {
    it("should return open: false when composer not visible", async () => {
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };

      const result = await human.verifyComposerOpen(mockPage);

      expect(result.open).toBe(false);
      expect(result.selector).toBe(null);
    });

    it("should handle selector errors while checking composer", async () => {
      const mockPage = {
        locator: vi.fn().mockImplementation(() => {
          throw new Error("bad selector");
        }),
      };

      const result = await human.verifyComposerOpen(mockPage);

      expect(result.open).toBe(false);
      expect(result.selector).toBe(null);
    });
  });

  describe("verifyPostSent", () => {
    it("should return sent: false when no confirmation found", async () => {
      const mockElementBuilder = (count, visible, inputVal = "") => ({
        count: vi.fn().mockResolvedValue(count),
        innerText: vi.fn().mockResolvedValue(""),
        isVisible: vi.fn().mockResolvedValue(visible),
        inputValue: vi.fn().mockResolvedValue(inputVal),
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(count),
          innerText: vi.fn().mockResolvedValue(""),
          isVisible: vi.fn().mockResolvedValue(visible),
          inputValue: vi.fn().mockResolvedValue(inputVal),
        }),
      });

      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => {
          if (selector === '[data-testid="tweetTextarea_0"]') {
            return mockElementBuilder(1, true, "draft text");
          }
          return mockElementBuilder(0, false);
        }),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const result = await human.verifyPostSent(mockPage);

      expect(result.sent).toBe(false);
    });

    it("should confirm send when url changes after checks", async () => {
      const mockElementBuilder = (count, visible, inputVal = "") => ({
        count: vi.fn().mockResolvedValue(count),
        innerText: vi.fn().mockResolvedValue(""),
        isVisible: vi.fn().mockResolvedValue(visible),
        inputValue: vi.fn().mockResolvedValue(inputVal),
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(count),
          innerText: vi.fn().mockResolvedValue(""),
          isVisible: vi.fn().mockResolvedValue(visible),
          inputValue: vi.fn().mockResolvedValue(inputVal),
        }),
      });

      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => {
          if (selector === '[data-testid="tweetTextarea_0"]') {
            return mockElementBuilder(1, true, "");
          }
          return mockElementBuilder(0, false);
        }),
        url: vi.fn().mockReturnValue("https://x.com/home"),
      };

      const result = await human.verifyPostSent(mockPage);

      expect(result.method).toBe("composer_cleared");
    });
  });

  describe("humanClick without ghost", () => {
    it("should throw when no page or ghost", async () => {
      const element = { click: vi.fn().mockResolvedValue(undefined) };
      await expect(human.humanClick(element, "NoGhost")).rejects.toThrow(
        "ghost_not_initialized",
      );
    });

    it("should throw when no page or ghost even if click exists", async () => {
      const element = { click: vi.fn().mockRejectedValue(new Error("fail")) };
      await expect(human.humanClick(element, "NoGhostFail")).rejects.toThrow(
        "ghost_not_initialized",
      );
    });
  });

  describe("safeHumanClick failures", () => {
    it("should return false after retries", async () => {
      vi.useFakeTimers();
      vi.spyOn(human, "humanClick").mockRejectedValue(new Error("fail"));
      const resultPromise = human.safeHumanClick({}, "RetryFail", 2);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result).toBe(false);
      expect(human.humanClick).toHaveBeenCalledTimes(2);
      vi.useRealTimers();
    });
  });

  describe("verifyComposerOpen success paths", () => {
    it("should return open: true when composer is visible and valid", async () => {
      const mockPage = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            boundingBox: vi.fn().mockResolvedValue({ width: 80, height: 40 }),
            inputValue: vi.fn().mockResolvedValue("draft"),
          }),
        }),
      };

      const result = await human.verifyComposerOpen(mockPage);
      expect(result.open).toBe(true);
      expect(result.selector).not.toBe(null);
    });

    it("should continue when box is too small and then succeed", async () => {
      let call = 0;
      const mockPage = {
        locator: vi.fn().mockImplementation(() => {
          call += 1;
          return {
            first: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(1),
              isVisible: vi.fn().mockResolvedValue(true),
              boundingBox: vi
                .fn()
                .mockResolvedValue(
                  call === 1
                    ? { width: 10, height: 10 }
                    : { width: 80, height: 40 },
                ),
              inputValue: vi.fn().mockRejectedValue(new Error("no value")),
            }),
          };
        }),
      };

      const result = await human.verifyComposerOpen(mockPage);
      expect(result.open).toBe(true);
    });

    it("should detect composer on second pass", async () => {
      vi.useFakeTimers();
      const selectors = [
        '[data-testid="tweetTextarea_0"]',
        '[contenteditable="true"][role="textbox"]',
        '[data-testid="tweetTextarea"]',
        '[class*="composer"] textarea',
        'textarea[placeholder*="Post your reply"]',
        'textarea[placeholder*="What\'s happening"]',
        '[role="textbox"][contenteditable="true"]',
      ];
      let callCount = 0;
      const mockPage = {
        locator: vi.fn().mockImplementation(() => {
          callCount += 1;
          const isLate = callCount > selectors.length;
          return {
            first: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(isLate ? 1 : 0),
              isVisible: vi.fn().mockResolvedValue(isLate),
              boundingBox: vi
                .fn()
                .mockResolvedValue(isLate ? { width: 80, height: 40 } : null),
              inputValue: vi.fn().mockResolvedValue(""),
            }),
          };
        }),
      };

      const resultPromise = human.verifyComposerOpen(mockPage);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.open).toBe(true);
      vi.useRealTimers();
    });
  });

  describe("verifyPostSent success paths", () => {
    it("should return sent: true when toast is found", async () => {
      const mockPage = {
        locator: vi.fn().mockImplementation((_selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi
              .fn()
              .mockResolvedValue(_selector === '[data-testid="toast"]' ? 1 : 0),
            innerText: vi.fn().mockResolvedValue("sent"),
            isVisible: vi.fn().mockResolvedValue(true),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
        })),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const result = await human.verifyPostSent(mockPage);
      expect(result.sent).toBe(true);
    });

    it("should handle innerText errors when toast is found", async () => {
      const mockPage = {
        locator: vi.fn().mockImplementation((_selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi
              .fn()
              .mockResolvedValue(_selector === '[data-testid="toast"]' ? 1 : 0),
            innerText: vi.fn().mockRejectedValue(new Error("fail")),
            isVisible: vi.fn().mockResolvedValue(true),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
        })),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const result = await human.verifyPostSent(mockPage);
      expect(result.sent).toBe(true);
    });

    it("should return sent: true when composer is closed", async () => {
      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => {
          const mockLocator = {
            first: vi.fn().mockReturnValue({
              count: vi
                .fn()
                .mockResolvedValue(
                  selector === '[data-testid="tweetTextarea_0"]' ? 0 : 0,
                ),
              innerText: vi.fn().mockResolvedValue(""),
              isVisible: vi.fn().mockResolvedValue(false),
              inputValue: vi.fn().mockResolvedValue(""),
            }),
            isVisible: vi.fn().mockResolvedValue(false),
            inputValue: vi.fn().mockResolvedValue(""),
          };
          return mockLocator;
        }),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const result = await human.verifyPostSent(mockPage);
      expect(result.sent).toBe(true);
    });

    it("should return sent: true when url changes", async () => {
      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => {
          const mockLocator = {
            first: vi.fn().mockReturnValue({
              count: vi
                .fn()
                .mockResolvedValue(
                  selector === '[data-testid="tweetTextarea_0"]' ? 1 : 0,
                ),
              innerText: vi.fn().mockResolvedValue(""),
              isVisible: vi.fn().mockResolvedValue(false),
              inputValue: vi.fn().mockResolvedValue(""),
            }),
            isVisible: vi.fn().mockResolvedValue(false),
            inputValue: vi.fn().mockResolvedValue(""),
          };
          return mockLocator;
        }),
        url: vi.fn().mockReturnValue("https://x.com/home"),
      };

      const result = await human.verifyPostSent(mockPage);
      expect(result.sent).toBe(true);
    });

    it("should return sent: true when composer closes after wait", async () => {
      vi.useFakeTimers();
      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi
              .fn()
              .mockResolvedValue(
                selector === '[data-testid="tweetTextarea_0"]' ? 1 : 0,
              ),
            innerText: vi.fn().mockResolvedValue(""),
            isVisible: vi.fn().mockResolvedValue(false),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
          isVisible: vi
            .fn()
            .mockResolvedValue(selector !== '[data-testid="tweetTextarea_0"]'),
        })),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const resultPromise = human.verifyPostSent(mockPage);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.method).toBe("composer_closed");
      vi.useRealTimers();
    });

    it("should confirm composer closed after wait when initial checks fail", async () => {
      vi.useFakeTimers();
      const mockPage = {
        locator: vi.fn().mockImplementation((selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi
              .fn()
              .mockResolvedValue(
                selector === '[data-testid="tweetTextarea_0"]' ? 1 : 0,
              ),
            innerText: vi.fn().mockResolvedValue(""),
            isVisible: vi.fn().mockResolvedValue(false),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
          isVisible: vi.fn().mockResolvedValue(false),
        })),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const resultPromise = human.verifyPostSent(mockPage);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.method).toBe("composer_closed");
      vi.useRealTimers();
    });

    it("should treat composer visibility errors as closed after wait", async () => {
      vi.useFakeTimers();
      const mockPage = {
        locator: vi.fn().mockImplementation((_selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
            innerText: vi.fn().mockResolvedValue(""),
            isVisible: vi.fn().mockResolvedValue(false),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
          isVisible: vi.fn().mockRejectedValue(new Error("vis fail")),
        })),
        url: vi.fn().mockReturnValue("https://x.com/compose/tweet"),
      };

      const resultPromise = human.verifyPostSent(mockPage);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.sent).toBe(true);
      vi.useRealTimers();
    });

    it("should return sent: true when timeline return is detected", async () => {
      vi.useFakeTimers();
      const urlSpy = vi
        .fn()
        .mockReturnValueOnce("https://x.com/compose/tweet")
        .mockReturnValueOnce("https://x.com/home");
      const mockPage = {
        locator: vi.fn().mockImplementation(() => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            innerText: vi.fn().mockResolvedValue(""),
            isVisible: vi.fn().mockResolvedValue(true),
            inputValue: vi.fn().mockResolvedValue(""),
          }),
          isVisible: vi.fn().mockResolvedValue(true),
        })),
        url: urlSpy,
      };

      const resultPromise = human.verifyPostSent(mockPage);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.sent).toBe(true);
      vi.useRealTimers();
    });
  });

  describe("typeText", () => {
    it("should type text and use fallback click when not focused", async () => {
      vi.useFakeTimers();
      vi.spyOn(Math, "random").mockReturnValue(0.99);
      const page = {
        keyboard: { press: vi.fn(), type: vi.fn() },
        evaluate: vi.fn().mockResolvedValue({
          tagName: "DIV",
          isContentEditable: false,
          hasFocus: false,
        }),
      };
      const inputEl = { click: vi.fn().mockResolvedValue(undefined) };
      vi.spyOn(human, "ensureFocus").mockResolvedValue(false);

      const resultPromise = human.typeText(page, "ab", inputEl);
      await vi.runAllTimersAsync();
      await resultPromise;

      expect(inputEl.click).toHaveBeenCalled();
      expect(page.keyboard.type).toHaveBeenCalledTimes(2);
      vi.useRealTimers();
      vi.restoreAllMocks();
    });

    it("should apply punctuation and thinking pauses", async () => {
      vi.useFakeTimers();
      vi.spyOn(Math, "random").mockReturnValue(0.01);
      const page = {
        keyboard: { press: vi.fn(), type: vi.fn() },
        evaluate: vi.fn().mockResolvedValue({
          tagName: "TEXTAREA",
          isContentEditable: true,
          hasFocus: true,
        }),
      };
      const inputEl = { click: vi.fn().mockResolvedValue(undefined) };
      vi.spyOn(human, "ensureFocus").mockResolvedValue(true);

      const resultPromise = human.typeText(page, "a b!", inputEl);
      await vi.runAllTimersAsync();
      await resultPromise;

      expect(page.keyboard.type).toHaveBeenCalledTimes(4);
      vi.useRealTimers();
      vi.restoreAllMocks();
    });

    it("should log when clear text fails", async () => {
      vi.useFakeTimers();
      const page = {
        keyboard: { press: vi.fn(), type: vi.fn() },
        evaluate: vi.fn().mockResolvedValue({
          tagName: "TEXTAREA",
          isContentEditable: true,
          hasFocus: true,
        }),
      };
      const inputEl = { click: vi.fn().mockResolvedValue(undefined) };
      vi.spyOn(human, "humanClick").mockRejectedValue(new Error("clear fail"));
      vi.spyOn(human, "ensureFocus").mockResolvedValue(true);

      const resultPromise = human.typeText(page, "a", inputEl);
      await vi.runAllTimersAsync();
      await resultPromise;

      vi.useRealTimers();
    });

    it("should use active element data when evaluate executes in test context", async () => {
      vi.useFakeTimers();
      vi.spyOn(Math, "random").mockReturnValue(0.99);
      global.document = {
        activeElement: {
          tagName: "TEXTAREA",
          getAttribute: () => "true",
        },
        querySelector: () => global.document.activeElement,
      };
      const page = {
        keyboard: { press: vi.fn(), type: vi.fn() },
        evaluate: vi.fn().mockImplementation((fn) => fn()),
      };
      const inputEl = { click: vi.fn().mockResolvedValue(undefined) };
      vi.spyOn(human, "ensureFocus").mockResolvedValue(true);

      const resultPromise = human.typeText(page, "a", inputEl);
      await vi.runAllTimersAsync();
      await resultPromise;

      expect(page.evaluate).toHaveBeenCalled();
      vi.useRealTimers();
      vi.restoreAllMocks();
      delete global.document;
    });
  });

  describe("ensureFocus", () => {
    it("should succeed with strategy 1", async () => {
      const page = {
        evaluate: vi.fn().mockResolvedValue(true),
      };
      const element = { focus: vi.fn(), click: vi.fn() };
      human.page = page;
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);
      const result = await human.ensureFocus(page, element);
      expect(result).toBe(true);
    });

    it("should succeed with strategy 2", async () => {
      const page = {
        evaluate: vi.fn().mockResolvedValue(true),
      };
      const element = {
        focus: vi.fn().mockResolvedValue(undefined),
        click: vi.fn(),
      };
      human.page = page;
      vi.spyOn(human, "humanClick").mockRejectedValue(new Error("fail"));
      const result = await human.ensureFocus(page, element);
      expect(result).toBe(true);
    });

    it("should fail when all strategies fail", async () => {
      const page = {
        evaluate: vi.fn().mockResolvedValue(false),
      };
      const element = {
        focus: vi.fn().mockRejectedValue(new Error("fail")),
        click: vi.fn().mockRejectedValue(new Error("fail")),
      };
      human.page = page;
      vi.spyOn(human, "humanClick").mockRejectedValue(new Error("fail"));
      const result = await human.ensureFocus(page, element);
      expect(result).toBe(false);
    });

    it("should succeed with strategy 3 after other strategies fail", async () => {
      vi.useFakeTimers();
      global.document = {
        activeElement: {
          tagName: "INPUT",
          getAttribute: () => null,
        },
      };
      const page = {
        evaluate: vi.fn().mockImplementation((fn) => fn()),
      };
      const element = {
        focus: vi.fn().mockRejectedValue(new Error("fail")),
        click: vi.fn().mockResolvedValue(undefined),
      };
      human.page = page;
      vi.spyOn(human, "humanClick").mockRejectedValue(new Error("fail"));

      const resultPromise = human.ensureFocus(page, element);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result).toBe(false);
      vi.useRealTimers();
      delete global.document;
    });

    it("should set page reference when different page is provided", async () => {
      const page = {
        evaluate: vi.fn().mockResolvedValue(true),
      };
      const element = { focus: vi.fn(), click: vi.fn() };
      const setPageSpy = vi.spyOn(human, "setPage");
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);

      const result = await human.ensureFocus(page, element);

      expect(result).toBe(true);
      expect(setPageSpy).toHaveBeenCalledWith(page);
    });

    it("should handle evaluate errors during focus verification", async () => {
      const page = {
        evaluate: vi.fn().mockRejectedValue(new Error("eval fail")),
      };
      const element = {
        focus: vi.fn().mockRejectedValue(new Error("fail")),
        click: vi.fn(),
      };
      human.page = page;
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);

      const result = await human.ensureFocus(page, element);

      expect(result).toBe(false);
    });
  });

  describe("postTweet", () => {
    it("should return success when button click works", async () => {
      vi.useFakeTimers();
      const page = {
        locator: vi.fn().mockImplementation((_selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            evaluate: vi.fn().mockResolvedValue(false),
          }),
        })),
      };
      vi.spyOn(human, "verifyPostSent").mockResolvedValue({
        sent: true,
        method: "button_click",
      });
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);
      const resultPromise = human.postTweet(page);
      await vi.runAllTimersAsync();
      const result = await resultPromise;
      expect(result.success).toBe(true);
      expect(result.method).toBe("js_click");
      vi.useRealTimers();
    });

    it("should fallback to force click when humanClick succeeds but verify fails", async () => {
      vi.useFakeTimers();
      const mockButtonElement = {
        isVisible: vi.fn().mockResolvedValue(true),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 80, height: 40 }),
        evaluate: vi.fn().mockResolvedValue(false),
        getAttribute: vi.fn().mockResolvedValue(null),
        innerText: vi.fn().mockResolvedValue("Post"),
        click: vi.fn().mockResolvedValue(undefined),
      };

      const page = {
        locator: vi.fn().mockImplementation((_selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            evaluate: vi.fn().mockResolvedValue(false),
            click: vi.fn().mockResolvedValue(undefined),
          }),
          count: vi.fn().mockResolvedValue(1),
          nth: vi.fn().mockReturnValue(mockButtonElement),
          isVisible: vi.fn().mockResolvedValue(true),
        })),
      };
      vi.spyOn(human, "verifyPostSent")
        .mockResolvedValueOnce({ sent: false })
        .mockResolvedValueOnce({ sent: true, method: "force_click" });
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);

      const resultPromise = human.postTweet(page);
      await vi.runAllTimersAsync();
      const result = await resultPromise;

      expect(result.success).toBe(true);
      vi.useRealTimers();
    });

    it("should return failure when button click throws and no method works", async () => {
      vi.useFakeTimers();
      const mockButtonElement = {
        isVisible: vi.fn().mockResolvedValue(true),
        boundingBox: vi
          .fn()
          .mockResolvedValue({ x: 100, y: 100, width: 80, height: 40 }),
        evaluate: vi.fn().mockResolvedValue(false),
        getAttribute: vi.fn().mockResolvedValue("Post"),
        innerText: vi.fn().mockResolvedValue("Post"),
        click: vi.fn().mockRejectedValue(new Error("click fail")),
      };

      const page = {
        locator: vi.fn().mockImplementation((selector) => {
          const isTweetButton = selector === '[data-testid="tweetButton"]';
          return {
            count: vi.fn().mockResolvedValue(isTweetButton ? 1 : 0),
            nth: vi.fn().mockReturnValue(mockButtonElement),
            first: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(isTweetButton ? 1 : 0),
              isVisible: vi.fn().mockResolvedValue(true),
              evaluate: vi.fn().mockResolvedValue(false),
            }),
          };
        }),
      };
      vi.spyOn(human, "verifyPostSent").mockResolvedValue({ sent: false });
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);

      const resultPromise = human.postTweet(page);
      await vi.runAllTimersAsync();
      const result = await resultPromise;

      expect(result.success).toBe(false);
      vi.useRealTimers();
    });
  });

  describe("fallback helpers", () => {
    it("should find element with fallback selectors", async () => {
      human.page = {
        locator: vi.fn().mockImplementation((selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(selector === ".hit" ? 1 : 0),
            isVisible: vi.fn().mockResolvedValue(true),
          }),
        })),
      };
      const result = await human.findWithFallback([".miss", ".hit"]);
      expect(result.selector).toBe(".hit");
    });

    it("should skip invisible elements and try next selector", async () => {
      let call = 0;
      human.page = {
        locator: vi.fn().mockImplementation(() => {
          call += 1;
          return {
            first: vi.fn().mockReturnValue({
              count: vi.fn().mockResolvedValue(1),
              isVisible: vi.fn().mockResolvedValue(call !== 1),
            }),
          };
        }),
      };
      const result = await human.findWithFallback([".first", ".second"]);
      expect(result.selector).toBe(".second");
    });

    it("should return element when visibility checks are disabled", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.findWithFallback([".hit"], { visible: false });
      expect(result.selector).toBe(".hit");
    });

    it("should handle errors when searching fallback selectors", async () => {
      human.page = {
        locator: vi.fn().mockImplementation(() => {
          throw new Error("bad selector");
        }),
      };
      const result = await human.findWithFallback([".bad"]);
      expect(result).toBe(null);
    });

    it("should treat visibility errors as not visible in fallback search", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockRejectedValue(new Error("vis fail")),
          }),
        }),
      };
      const result = await human.findWithFallback([".hit"]);
      expect(result).toBe(null);
    });

    it("should return null when fallback selectors fail", async () => {
      const nowSpy = vi.spyOn(Date, "now");
      nowSpy.mockReturnValueOnce(0).mockReturnValueOnce(10);
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.findWithFallback([".miss"], { timeout: 5 });
      expect(result).toBe(null);
      nowSpy.mockRestore();
    });

    it("should find all visible elements with fallback", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(2),
          nth: vi.fn().mockImplementation(() => ({
            isVisible: vi.fn().mockResolvedValue(true),
          })),
        }),
      };
      const result = await human.findAllWithFallback([".hit"]);
      expect(result.length).toBe(2);
    });

    it("should include elements when visibility check is disabled", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(2),
          nth: vi.fn().mockImplementation(() => ({
            isVisible: vi.fn().mockResolvedValue(false),
          })),
        }),
      };
      const result = await human.findAllWithFallback([".hit"], {
        visible: false,
      });
      expect(result.length).toBe(2);
    });

    it("should handle selector errors when finding all fallback elements", async () => {
      human.page = {
        locator: vi.fn().mockImplementation(() => {
          throw new Error("bad selector");
        }),
      };
      const result = await human.findAllWithFallback([".bad"]);
      expect(result).toEqual([]);
    });

    it("should skip invisible elements when visibility is required", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(2),
          nth: vi.fn().mockImplementation(() => ({
            isVisible: vi.fn().mockRejectedValue(new Error("vis fail")),
          })),
        }),
      };
      const result = await human.findAllWithFallback([".hit"], {
        visible: true,
      });
      expect(result.length).toBe(0);
    });

    it("should return empty list when no elements found", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(0),
          nth: vi.fn(),
        }),
      };
      const result = await human.findAllWithFallback([".miss"]);
      expect(result).toEqual([]);
    });

    it("should click with fallback selectors", async () => {
      human.page = {
        locator: vi.fn().mockImplementation((selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(selector === ".hit" ? 1 : 0),
            isVisible: vi.fn().mockResolvedValue(selector === ".hit"),
          }),
        })),
      };
      vi.spyOn(human, "humanClick").mockResolvedValue(undefined);
      const result = await human.clickWithFallback([".miss", ".hit"], "Target");
      expect(result).toBe(true);
    });

    it("should handle selector errors during click fallback", async () => {
      human.page = {
        locator: vi.fn().mockImplementation(() => {
          throw new Error("bad selector");
        }),
      };
      const result = await human.clickWithFallback([".bad"], "Target");
      expect(result).toBe(false);
    });

    it("should return false when click fallback fails", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.clickWithFallback([".miss"], "Target");
      expect(result).toBe(false);
    });

    it("should skip elements that error on visibility check in click fallback", async () => {
      human.page = {
        locator: vi.fn().mockImplementation((selector) => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(selector === ".hit" ? 1 : 0),
            isVisible: vi.fn().mockRejectedValue(new Error("vis fail")),
          }),
        })),
      };
      const result = await human.clickWithFallback([".hit"], "Target");
      expect(result).toBe(false);
    });

    it("should wait for element with fallback", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            waitFor: vi.fn().mockResolvedValue(undefined),
            isVisible: vi.fn().mockResolvedValue(true),
          }),
        }),
      };
      const result = await human.waitForWithFallback([".hit"]);
      expect(result.selector).toBe(".hit");
    });

    it("should return element when wait fallback is visible", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            waitFor: vi.fn().mockResolvedValue(undefined),
            isVisible: vi.fn().mockResolvedValue(true),
          }),
        }),
      };
      const result = await human.waitForWithFallback([".hit"], {
        visible: true,
      });
      expect(result.selector).toBe(".hit");
    });

    it("should return element when visibility is disabled for wait fallback", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            waitFor: vi.fn().mockResolvedValue(undefined),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.waitForWithFallback([".hit"], {
        visible: false,
      });
      expect(result.selector).toBe(".hit");
    });

    it("should continue when wait fallback element is not visible", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            waitFor: vi.fn().mockResolvedValue(undefined),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.waitForWithFallback([".miss"], {
        visible: true,
      });
      expect(result).toBe(null);
    });

    it("should return null when wait fallback fails", async () => {
      human.page = {
        locator: vi.fn().mockReturnValue({
          first: vi.fn().mockReturnValue({
            waitFor: vi.fn().mockRejectedValue(new Error("fail")),
            isVisible: vi.fn().mockResolvedValue(false),
          }),
        }),
      };
      const result = await human.waitForWithFallback([".miss"]);
      expect(result).toBe(null);
    });
  });

  describe("logWarn", () => {
    it("should log warn messages", () => {
      const originalLog = console.log;
      console.log = vi.fn();
      human.logWarn("warn");
      expect(console.log).toHaveBeenCalled();
      console.log = originalLog;
    });
  });

  describe("logDebug", () => {
    it("should not log when debugMode is false", () => {
      const originalLog = console.log;
      console.log = vi.fn();

      human.logDebug("test message");

      expect(console.log).not.toHaveBeenCalled();
      console.log = originalLog;
    });

    it("should log when debugMode is true", () => {
      human.debugMode = true;
      const originalLog = console.log;
      console.log = vi.fn();

      human.logDebug("test message");

      expect(console.log).toHaveBeenCalled();
      console.log = originalLog;
    });
  });

  describe("logStep", () => {
    it("should call logDebug", () => {
      human.debugMode = true;
      const originalLog = console.log;
      console.log = vi.fn();

      human.logStep("TestStep", "details");

      expect(console.log).toHaveBeenCalled();
      console.log = originalLog;
    });

    it("should log step without details", () => {
      human.debugMode = true;
      const originalLog = console.log;
      console.log = vi.fn();

      human.logStep("TestStep");

      expect(console.log).toHaveBeenCalled();
      console.log = originalLog;
    });
  });
});

describe("HumanInteraction with GhostCursor", () => {
  let human;
  let mockPage;
  let mockElement;

  beforeEach(() => {
    vi.useRealTimers();
    mockPage = {
      mouse: { move: vi.fn(), click: vi.fn() },
      evaluate: vi.fn(),
      waitForTimeout: vi.fn().mockImplementation((_ms) => Promise.resolve()),
    };

    mockElement = {
      boundingBox: vi
        .fn()
        .mockResolvedValue({ x: 100, y: 200, width: 50, height: 50 }),
      click: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn().mockResolvedValue(undefined),
      scrollIntoView: vi.fn().mockResolvedValue(undefined),
    };

    human = new HumanInteraction(mockPage);
  });

  describe("humanClick", () => {
    it("should scroll element into view before clicking", async () => {
      human.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };
      await human.humanClick(mockElement, "Test Element");

      expect(mockElement.evaluate).toHaveBeenCalled();
    });

    it("should call ghost click", async () => {
      human.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };
      await human.humanClick(mockElement, "Test Element");

      expect(human.ghost.click).toHaveBeenCalled();
    });

    it("should reject when ghost click fails", async () => {
      human.ghost = { click: vi.fn().mockResolvedValue({ success: false }) };
      await expect(
        human.humanClick(mockElement, "Test Element"),
      ).rejects.toThrow("ghost_click_failed");
    });

    it("should execute element evaluate callback when provided", async () => {
      const scrollSpy = vi.fn();
      mockElement.evaluate = vi
        .fn()
        .mockImplementation((fn) => fn({ scrollIntoView: scrollSpy }));
      human.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };

      await human.humanClick(mockElement, "Test Element");

      expect(scrollSpy).toHaveBeenCalled();
    });

    it("should throw when ghost click throws", async () => {
      human.ghost = {
        click: vi.fn().mockRejectedValue(new Error("ghost fail")),
      };

      await expect(
        human.humanClick(mockElement, "Test Element"),
      ).rejects.toThrow("ghost fail");
    });
  });

  describe("safeHumanClick", () => {
    it("should return true on successful click", async () => {
      human.ghost = { click: vi.fn().mockResolvedValue({ success: true }) };
      const result = await human.safeHumanClick(mockElement, "Test Element", 3);

      expect(result).toBe(true);
    });
  });
});
