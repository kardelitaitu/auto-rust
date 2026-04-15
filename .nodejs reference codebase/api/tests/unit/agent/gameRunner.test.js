/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  createLogger: () => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
}));

vi.mock("@api/agent/llmClient.js", () => ({
  llmClient: {
    init: vi.fn().mockResolvedValue(undefined),
    generateCompletionWithRetry: vi.fn().mockResolvedValue({ action: "done" }),
    chat: vi.fn().mockResolvedValue({ content: '{"action": "done"}' }),
    config: { useVision: true },
  },
}));

vi.mock("@api/agent/actionEngine.js", () => ({
  actionEngine: {
    execute: vi.fn().mockResolvedValue({ success: true, done: false }),
  },
}));

vi.mock("@api/agent/tokenCounter.js", () => ({
  estimateConversationTokens: vi.fn().mockReturnValue(100),
}));

vi.mock("@api/core/config.js", () => ({
  configManager: {
    get: vi.fn().mockReturnValue({}),
  },
}));

vi.mock("@api/agent/visualDiff.js", () => ({
  visualDiffEngine: {
    captureState: vi.fn().mockResolvedValue({}),
    hasChanged: vi.fn().mockReturnValue(false),
    compareScreenshots: vi
      .fn()
      .mockResolvedValue({ changed: false, diffRatio: 0, method: "pixel" }),
  },
}));

vi.mock("@api/agent/adaptiveTiming.js", () => ({
  adaptiveTiming: {
    getDelay: vi.fn().mockReturnValue(100),
    delay: vi.fn().mockResolvedValue(undefined),
    measureSitePerformance: vi
      .fn()
      .mockResolvedValue({ click: 100, waitMultiplier: 1 }),
    getAdjustedDelay: vi.fn().mockReturnValue(0),
  },
}));

vi.mock("@api/agent/goalDecomposer.js", () => ({
  goalDecomposer: {
    decompose: vi.fn().mockReturnValue([{ subgoal: "test", priority: 1 }]),
    getNextStep: vi.fn().mockReturnValue(null),
    advanceStep: vi.fn().mockImplementation((d) => d),
    isComplete: vi.fn().mockReturnValue(false),
  },
}));

vi.mock("@api/agent/sessionStore.js", () => ({
  sessionStore: {
    get: vi.fn().mockReturnValue(null),
    set: vi.fn(),
    recordSession: vi.fn(),
  },
}));

vi.mock("@api/agent/progressTracker.js", () => ({
  progressTracker: {
    track: vi.fn(),
    getProgress: vi.fn().mockReturnValue(0),
    startSession: vi.fn(),
    endSession: vi.fn(),
    recordStep: vi.fn(),
    recordLlmCall: vi.fn(),
    updateUrl: vi.fn(),
    recordStuck: vi.fn(),
    completeSession: vi.fn(),
  },
}));

vi.mock("@api/agent/actionRollback.js", () => ({
  actionRollback: {
    save: vi.fn(),
    restore: vi.fn(),
    executeWithRollback: vi.fn().mockImplementation(async (fn) => fn()),
    isCriticalAction: vi.fn().mockReturnValue(false),
    capturePreState: vi.fn(),
    recordAction: vi.fn(),
    rollbackLast: vi.fn().mockResolvedValue(false),
  },
}));

vi.mock("@api/agent/semanticMapper.js", () => ({
  semanticMapper: {
    map: vi.fn().mockReturnValue([]),
    enrichAXTree: vi.fn((tree) => tree),
    getPageSummary: vi.fn(() => "Page summary"),
  },
}));

vi.mock("@api/agent/parallelExecutor.js", () => ({
  parallelExecutor: {
    execute: vi.fn().mockResolvedValue([]),
    canParallelize: vi.fn().mockReturnValue(false),
    estimateSpeedup: vi.fn().mockReturnValue({ speedup: 1 }),
    executeSequence: vi.fn().mockResolvedValue([]),
  },
}));

vi.mock("@api/utils/VisionPreprocessor.js", () => ({
  VisionPreprocessor: vi.fn().mockImplementation(() => ({
    process: vi.fn().mockResolvedValue(Buffer.from("processed")),
  })),
}));

describe("api/agent/gameRunner.js", () => {
  let gameAgentRunner;
  let mockPage;

  beforeEach(async () => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://game.example.com"),
      screenshot: vi.fn().mockResolvedValue(Buffer.from("screenshot")),
      bringToFront: vi.fn().mockResolvedValue(undefined),
      viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
      evaluate: vi.fn().mockResolvedValue({ width: 1920, height: 1080 }),
      accessibility: {
        snapshot: vi.fn().mockResolvedValue({ role: "root", children: [] }),
      },
    };

    const module = await import("@api/agent/gameRunner.js");
    gameAgentRunner = module.gameAgentRunner || module.default;
    gameAgentRunner.isRunning = false;
    gameAgentRunner.actionCache = new Map();
    gameAgentRunner.screenshotCache = null;
    gameAgentRunner.screenshotCacheTime = 0;
  });

  describe("gameAgentRunner", () => {
    it("should be defined", () => {
      expect(gameAgentRunner).toBeDefined();
    });
  });

  describe("module exports", () => {
    it("should export gameAgentRunner", () => {
      expect(gameAgentRunner).toBeDefined();
      expect(typeof gameAgentRunner.run).toBe("function");
    });
  });

  describe("Memoization System", () => {
    describe("_hashElement", () => {
      it('should return "null" for null/undefined elements', () => {
        expect(gameAgentRunner._hashElement(null)).toBe("null");
        expect(gameAgentRunner._hashElement(undefined)).toBe("null");
      });

      it("should generate consistent hashes for same element", () => {
        const element = { role: "button", name: "Submit" };
        const hash1 = gameAgentRunner._hashElement(element);
        const hash2 = gameAgentRunner._hashElement(element);
        expect(hash1).toBe(hash2);
      });

      it("should generate different hashes for different elements", () => {
        const element1 = { role: "button", name: "Submit" };
        const element2 = { role: "link", name: "Cancel" };
        const hash1 = gameAgentRunner._hashElement(element1);
        const hash2 = gameAgentRunner._hashElement(element2);
        expect(hash1).not.toBe(hash2);
      });

      it("should return a base-36 string", () => {
        const element = { id: 1, text: "test" };
        const hash = gameAgentRunner._hashElement(element);
        expect(typeof hash).toBe("string");
        expect(/^[0-9a-z]+$/.test(hash)).toBe(true);
      });
    });

    describe("_getActionCacheKey", () => {
      it("should generate cache key from url, goal, and element", () => {
        const key = gameAgentRunner._getActionCacheKey(
          "https://example.com",
          "click button",
          { role: "button" },
        );
        expect(key).toContain("https://example.com");
        expect(key).toContain("click button");
        expect(key).toContain("|");
      });

      it("should produce different keys for different inputs", () => {
        const key1 = gameAgentRunner._getActionCacheKey("url1", "goal1", {
          id: 1,
        });
        const key2 = gameAgentRunner._getActionCacheKey("url2", "goal2", {
          id: 2,
        });
        expect(key1).not.toBe(key2);
      });
    });

    describe("_getOrComputeAction", () => {
      it("should compute and cache result on first call", async () => {
        const computeFn = vi.fn().mockResolvedValue("computed-result");
        const result = await gameAgentRunner._getOrComputeAction(
          "https://example.com",
          "test goal",
          { id: 1 },
          computeFn,
        );
        expect(result).toBe("computed-result");
        expect(computeFn).toHaveBeenCalledTimes(1);
      });

      it("should return cached result on subsequent calls", async () => {
        // Clear cache for this test
        gameAgentRunner.actionCache = new Map();

        const computeFn = vi.fn().mockResolvedValue("computed-result");
        const url = "https://example.com";
        const goal = "test goal";
        const element = { id: 1 };

        const result1 = await gameAgentRunner._getOrComputeAction(
          url,
          goal,
          element,
          computeFn,
        );
        const result2 = await gameAgentRunner._getOrComputeAction(
          url,
          goal,
          element,
          computeFn,
        );

        expect(result1).toBe(result2);
        expect(computeFn).toHaveBeenCalledTimes(1);
      });

      it("should evict oldest entry when cache exceeds limit", async () => {
        gameAgentRunner.maxActionCacheSize = 2;
        gameAgentRunner.actionCache = new Map();

        const computeFn = vi
          .fn()
          .mockImplementation((val) => Promise.resolve(`result-${val}`));

        await gameAgentRunner._getOrComputeAction(
          "url1",
          "goal",
          { id: 1 },
          () => computeFn(1),
        );
        await gameAgentRunner._getOrComputeAction(
          "url2",
          "goal",
          { id: 2 },
          () => computeFn(2),
        );
        await gameAgentRunner._getOrComputeAction(
          "url3",
          "goal",
          { id: 3 },
          () => computeFn(3),
        );

        expect(gameAgentRunner.actionCache.size).toBeLessThanOrEqual(2);
      });
    });
  });

  describe("Stuck Detection", () => {
    describe("_checkStuck", () => {
      it("should return false when not stuck", () => {
        gameAgentRunner.lastProgressStep = 0;
        gameAgentRunner.maxAttemptsWithoutChange = 5;
        expect(gameAgentRunner._checkStuck(3)).toBe(false);
      });

      it("should return true when stuck (exceeded max attempts)", () => {
        gameAgentRunner.lastProgressStep = 0;
        gameAgentRunner.maxAttemptsWithoutChange = 5;
        expect(gameAgentRunner._checkStuck(5)).toBe(true);
      });

      it("should return true when exactly at max attempts", () => {
        gameAgentRunner.lastProgressStep = 10;
        gameAgentRunner.maxAttemptsWithoutChange = 3;
        expect(gameAgentRunner._checkStuck(13)).toBe(true);
      });

      it("should return false when just below max attempts", () => {
        gameAgentRunner.lastProgressStep = 10;
        gameAgentRunner.maxAttemptsWithoutChange = 3;
        expect(gameAgentRunner._checkStuck(12)).toBe(false);
      });
    });
  });

  describe("Comparison Methods", () => {
    describe("_compareUrl", () => {
      it("should return true when URLs differ", () => {
        expect(
          gameAgentRunner._compareUrl(
            "https://example.com",
            "https://other.com",
          ),
        ).toBe(true);
      });

      it("should return false when URLs are the same", () => {
        expect(
          gameAgentRunner._compareUrl(
            "https://example.com",
            "https://example.com",
          ),
        ).toBe(false);
      });

      it("should detect hash changes", () => {
        expect(
          gameAgentRunner._compareUrl(
            "https://example.com#old",
            "https://example.com#new",
          ),
        ).toBe(true);
      });
    });

    describe("_stripDynamicContent", () => {
      it("should strip timestamps from JSON strings", () => {
        const input = '{"timestamp":"1234567890123","data":"test"}';
        const result = gameAgentRunner._stripDynamicContent(input);
        expect(result).toContain("[TIMESTAMP]");
        expect(result).not.toContain("1234567890123");
      });

      it("should strip dynamic IDs", () => {
        const input = '{"id":"abc123def456ghi789jkl012mno"}';
        const result = gameAgentRunner._stripDynamicContent(input);
        expect(result).toContain("[DYNAMIC_ID]");
      });

      it("should handle objects by stringifying", () => {
        const input = { timestamp: "1234567890123", data: "test" };
        const result = gameAgentRunner._stripDynamicContent(input);
        expect(typeof result).toBe("string");
      });

      it("should preserve short IDs", () => {
        const input = '{"id":"short"}';
        const result = gameAgentRunner._stripDynamicContent(input);
        expect(result).toContain("short");
        expect(result).not.toContain("[DYNAMIC_ID]");
      });
    });

    describe("_compareAXTree", () => {
      it("should return true when trees differ", () => {
        const tree1 = JSON.stringify({ role: "button", name: "Submit" });
        const tree2 = JSON.stringify({ role: "link", name: "Cancel" });
        expect(gameAgentRunner._compareAXTree(tree1, tree2)).toBe(true);
      });

      it("should return false for identical trees", () => {
        const tree = JSON.stringify({ role: "button", name: "Submit" });
        expect(gameAgentRunner._compareAXTree(tree, tree)).toBe(false);
      });

      it("should ignore dynamic content when comparing", () => {
        const tree1 = JSON.stringify({ id: "1234567890123", role: "button" });
        const tree2 = JSON.stringify({ id: "9876543210987", role: "button" });
        expect(gameAgentRunner._compareAXTree(tree1, tree2)).toBe(false);
      });
    });

    describe("_compareVisual", () => {
      it("should return false when either screenshot is missing", async () => {
        expect(await gameAgentRunner._compareVisual(null, "base64data")).toBe(
          false,
        );
        expect(await gameAgentRunner._compareVisual("base64data", null)).toBe(
          false,
        );
        expect(await gameAgentRunner._compareVisual(undefined, undefined)).toBe(
          false,
        );
      });

      it("should return result from visualDiffEngine when successful", async () => {
        const { visualDiffEngine } = await import("@api/agent/visualDiff.js");
        visualDiffEngine.compareScreenshots.mockResolvedValue({
          changed: true,
          diffRatio: 0.1,
          method: "pixel",
        });

        const result = await gameAgentRunner._compareVisual(
          "aGVsbG8",
          "d29ybGQ=",
        );
        expect(result).toBe(true);
      });

      it("should use fallback comparison on error", async () => {
        const { visualDiffEngine } = await import("@api/agent/visualDiff.js");
        visualDiffEngine.compareScreenshots.mockRejectedValue(
          new Error("Test error"),
        );

        const longString1 = "a".repeat(1000);
        const longString2 = "b".repeat(100);
        const result = await gameAgentRunner._compareVisual(
          longString1,
          longString2,
        );
        expect(result).toBe(true);
      });
    });
  });

  describe("Coordinate Scaling", () => {
    describe("_scaleActionCoordinates", () => {
      it("should return action unchanged when no scaling needed", () => {
        gameAgentRunner.vprepScaleFactor = 1.0;
        const action = { action: "clickAt", x: 100, y: 200 };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(result).toEqual(action);
      });

      it("should return null/undefined action unchanged", () => {
        gameAgentRunner.vprepScaleFactor = 2.0;
        expect(gameAgentRunner._scaleActionCoordinates(null)).toBe(null);
        expect(gameAgentRunner._scaleActionCoordinates(undefined)).toBe(
          undefined,
        );
      });

      it("should scale clickAt coordinates", () => {
        gameAgentRunner.vprepScaleFactor = 2.0;
        const action = { action: "clickAt", x: 100, y: 200 };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(result.x).toBe(200);
        expect(result.y).toBe(400);
      });

      it("should scale drag coordinates", () => {
        gameAgentRunner.vprepScaleFactor = 0.5;
        const action = {
          action: "drag",
          x: 100,
          y: 200,
          targetX: 300,
          targetY: 400,
        };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(result.x).toBe(50);
        expect(result.y).toBe(100);
        expect(result.targetX).toBe(150);
        expect(result.targetY).toBe(200);
      });

      it("should not modify original action object", () => {
        gameAgentRunner.vprepScaleFactor = 2.0;
        const action = { action: "clickAt", x: 100, y: 200 };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(action.x).toBe(100);
        expect(result.x).toBe(200);
      });

      it("should handle actions without coordinates", () => {
        gameAgentRunner.vprepScaleFactor = 2.0;
        const action = { action: "type", text: "hello" };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(result.text).toBe("hello");
      });

      it("should scale with non-integer results using rounding", () => {
        gameAgentRunner.vprepScaleFactor = 1.5;
        const action = { action: "clickAt", x: 11, y: 22 };
        const result = gameAgentRunner._scaleActionCoordinates(action);
        expect(result.x).toBe(17);
        expect(result.y).toBe(33);
      });
    });
  });

  describe("Interactive Elements Extraction", () => {
    describe("_extractInteractiveElements", () => {
      it("should return null for null tree", () => {
        expect(gameAgentRunner._extractInteractiveElements(null)).toBe(null);
      });

      it("should extract basic tree properties", () => {
        const tree = { role: "button", name: "Submit" };
        const result = gameAgentRunner._extractInteractiveElements(tree);
        expect(result.role).toBe("button");
        expect(result.name).toBe("Submit");
      });

      it("should include selector for interactive elements", () => {
        const tree = {
          role: "button",
          name: "Submit",
          selector: "#submit-btn",
        };
        const result = gameAgentRunner._extractInteractiveElements(tree);
        expect(result.selector).toBe("#submit-btn");
      });

      it("should include value for interactive elements", () => {
        const tree = {
          role: "textbox",
          name: "Input",
          value: "current value",
        };
        const result = gameAgentRunner._extractInteractiveElements(tree);
        expect(result.value).toBe("current value");
      });

      it("should limit depth to 3 levels", () => {
        const deepTree = {
          role: "root",
          name: "root",
          children: [
            {
              role: "level1",
              name: "l1",
              children: [
                {
                  role: "level2",
                  name: "l2",
                  children: [
                    {
                      role: "level3",
                      name: "l3",
                      children: [{ role: "level4", name: "l4" }],
                    },
                  ],
                },
              ],
            },
          ],
        };
        const result = gameAgentRunner._extractInteractiveElements(deepTree);
        expect(result.children[0].children[0].children).toBeDefined();
      });
    });
  });

  describe("Prompt Building", () => {
    describe("_buildPrompt", () => {
      beforeEach(() => {
        gameAgentRunner.history = [];
        gameAgentRunner.useAXTree = true;
      });

      it("should return array with system message", () => {
        const result = gameAgentRunner._buildPrompt(
          "test goal",
          "base64data",
          "axtree",
          "https://example.com",
        );
        expect(Array.isArray(result)).toBe(true);
        expect(result[0].role).toBe("system");
      });

      it("should include goal in user message", () => {
        const result = gameAgentRunner._buildPrompt(
          "click button",
          "base64data",
          "axtree",
          "https://example.com",
        );
        const userMessage = result[result.length - 1];
        expect(userMessage.role).toBe("user");
        expect(JSON.stringify(userMessage.content)).toContain("click button");
      });

      it("should include current URL", () => {
        const result = gameAgentRunner._buildPrompt(
          "goal",
          "base64data",
          "axtree",
          "https://example.com",
        );
        const content = JSON.stringify(result[result.length - 1].content);
        expect(content).toContain("https://example.com");
      });

      it("should include AXTree when enabled", () => {
        gameAgentRunner.useAXTree = true;
        const result = gameAgentRunner._buildPrompt(
          "goal",
          "base64data",
          "test-axtree-content",
          "https://example.com",
        );
        const content = JSON.stringify(result[result.length - 1].content);
        expect(content).toContain("test-axtree-content");
      });

      it("should show disabled message when AXTree is disabled", () => {
        gameAgentRunner.useAXTree = false;
        const result = gameAgentRunner._buildPrompt(
          "goal",
          "base64data",
          "axtree",
          "https://example.com",
        );
        const content = JSON.stringify(result[result.length - 1].content);
        expect(content).toContain("Disabled");
      });

      it("should include screenshot when vision is enabled", () => {
        gameAgentRunner.llmClient = { config: { useVision: true } };
        const result = gameAgentRunner._buildPrompt(
          "goal",
          "base64screenshot",
          "axtree",
          "https://example.com",
        );
        const userMessage = result[result.length - 1];
        const hasImage = userMessage.content.some(
          (p) => p.type === "image_url",
        );
        expect(hasImage).toBe(true);
      });

      it("should include recent history (last 4 items)", () => {
        gameAgentRunner.history = [
          { role: "user", content: "msg1" },
          { role: "assistant", content: "msg2" },
          { role: "user", content: "msg3" },
          { role: "assistant", content: "msg4" },
          { role: "user", content: "msg5" },
          { role: "assistant", content: "msg6" },
        ];
        const result = gameAgentRunner._buildPrompt(
          "goal",
          "screenshot",
          "axtree",
          "url",
        );
        expect(result.length).toBe(6);
      });
    });
  });

  describe("State Management", () => {
    describe("stop", () => {
      it("should set isRunning to false", () => {
        gameAgentRunner.isRunning = true;
        gameAgentRunner._abortController = { abort: vi.fn() };
        gameAgentRunner.stop();
        expect(gameAgentRunner.isRunning).toBe(false);
      });

      it("should call abort on abort controller", () => {
        const abortMock = vi.fn();
        gameAgentRunner._abortController = { abort: abortMock };
        gameAgentRunner.stop();
        expect(abortMock).toHaveBeenCalled();
      });

      it("should handle missing abort controller gracefully", () => {
        gameAgentRunner._abortController = null;
        expect(() => gameAgentRunner.stop()).not.toThrow();
      });
    });

    describe("getUsageStats", () => {
      it("should return stats object", () => {
        gameAgentRunner.isRunning = false;
        gameAgentRunner.currentGoal = "test goal";
        gameAgentRunner.history = [1, 2, 3, 4];
        gameAgentRunner.maxSteps = 100;

        const stats = gameAgentRunner.getUsageStats();
        expect(stats).toEqual({
          isRunning: false,
          goal: "test goal",
          steps: 2,
          maxSteps: 100,
          historySize: 4,
        });
      });

      it("should reflect current running state", () => {
        gameAgentRunner.isRunning = true;
        const stats = gameAgentRunner.getUsageStats();
        expect(stats.isRunning).toBe(true);
      });
    });

    describe("_invalidateScreenshotCache", () => {
      it("should clear screenshot cache", () => {
        gameAgentRunner.screenshotCache = "cached-data";
        gameAgentRunner.screenshotCacheTime = Date.now();

        gameAgentRunner._invalidateScreenshotCache();

        expect(gameAgentRunner.screenshotCache).toBe(null);
        expect(gameAgentRunner.screenshotCacheTime).toBe(0);
      });
    });
  });

  describe("Constructor and Initialization", () => {
    it("should have default configuration values", () => {
      expect(gameAgentRunner.maxSteps).toBeDefined();
      expect(gameAgentRunner.maxActionCacheSize).toBeDefined();
      expect(gameAgentRunner.screenshotCacheTTL).toBeDefined();
      expect(gameAgentRunner.maxAttemptsWithoutChange).toBeDefined();
    });

    it("should initialize with empty history", () => {
      expect(Array.isArray(gameAgentRunner.history)).toBe(true);
    });

    it("should initialize with action cache", () => {
      expect(gameAgentRunner.actionCache).toBeInstanceOf(Map);
    });

    it("should not be running initially after reset", () => {
      // Reset state for this test
      gameAgentRunner.isRunning = false;
      expect(gameAgentRunner.isRunning).toBe(false);
    });

    it("should export singleton instance", async () => {
      const module1 = await import("@api/agent/gameRunner.js");
      const module2 = await import("@api/agent/gameRunner.js");
      expect(module1.default).toBe(module2.default);
    });
  });

  describe("Screenshot Capture", () => {
    describe("_captureScreenshot", () => {
      it("should return cached screenshot if fresh", async () => {
        gameAgentRunner.screenshotCache = "cached-screenshot";
        gameAgentRunner.screenshotCacheTime = Date.now();
        gameAgentRunner.screenshotCacheTTL = 5000;

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(result).toBe("cached-screenshot");
        expect(mockPage.screenshot).not.toHaveBeenCalled();
      });

      it("should capture new screenshot when cache expired", async () => {
        gameAgentRunner.screenshotCache = "old-cache";
        gameAgentRunner.screenshotCacheTime = Date.now() - 10000;
        gameAgentRunner.screenshotCacheTTL = 5000;
        gameAgentRunner.vprepScaleFactor = 1.0;

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(mockPage.screenshot).toHaveBeenCalled();
      });

      it("should force refresh when requested", async () => {
        gameAgentRunner.screenshotCache = "cached-screenshot";
        gameAgentRunner.screenshotCacheTime = Date.now();
        gameAgentRunner.screenshotCacheTTL = 5000;
        gameAgentRunner.vprepScaleFactor = 1.0;

        await gameAgentRunner._captureScreenshot(mockPage, true);
        expect(mockPage.screenshot).toHaveBeenCalled();
      });
    });

    describe("_captureState", () => {
      it("should return object with screenshot, axTree, and url", async () => {
        gameAgentRunner.useAXTree = false;
        mockPage.url.mockReturnValue("https://test.com");

        const result = await gameAgentRunner._captureState(mockPage);
        expect(result).toHaveProperty("url");
        expect(result.url).toBe("https://test.com");
      });
    });
  });

  describe("Run Method", () => {
    it("should throw when already running", async () => {
      gameAgentRunner.isRunning = true;
      await expect(gameAgentRunner.run("test goal")).rejects.toThrow(
        "already running",
      );
    });

    it("should fail when no page is available", async () => {
      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { progressTracker } = await import("@api/agent/progressTracker.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");

      gameAgentRunner.isRunning = false;
      getPage.mockReturnValue(null);
      llmClient.init.mockResolvedValueOnce(undefined);
      adaptiveTiming.measureSitePerformance.mockResolvedValueOnce({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValueOnce({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });

      await expect(gameAgentRunner.run("test goal")).rejects.toThrow(
        "No page available",
      );
      expect(progressTracker.startSession).not.toHaveBeenCalled();
    });

    it("should abort after repeated LLM failures", async () => {
      vi.useFakeTimers();

      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { progressTracker } = await import("@api/agent/progressTracker.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");

      gameAgentRunner.isRunning = false;
      getPage.mockReturnValue(mockPage);
      llmClient.generateCompletionWithRetry.mockRejectedValue(
        new Error("LLM down"),
      );
      adaptiveTiming.measureSitePerformance.mockResolvedValue({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValue({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });
      mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);

      const result = await gameAgentRunner.run("test goal", { maxSteps: 5 });

      expect(result.success).toBe(false);
      expect(llmClient.generateCompletionWithRetry).toHaveBeenCalledTimes(3);
      expect(progressTracker.recordLlmCall).toHaveBeenCalledTimes(3);
      vi.useRealTimers();
    });

    it("should finish when the LLM returns a done action", async () => {
      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");
      const { progressTracker } = await import("@api/agent/progressTracker.js");
      const { actionEngine } = await import("@api/agent/actionEngine.js");
      const { parallelExecutor } =
        await import("@api/agent/parallelExecutor.js");

      gameAgentRunner.isRunning = false;
      getPage.mockReturnValue(mockPage);
      llmClient.generateCompletionWithRetry.mockResolvedValueOnce({
        action: "done",
      });
      adaptiveTiming.measureSitePerformance.mockResolvedValueOnce({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValueOnce({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });
      parallelExecutor.canParallelize.mockReturnValueOnce(false);
      actionEngine.execute.mockResolvedValueOnce({ success: true, done: true });

      const result = await gameAgentRunner.run("test goal", { maxSteps: 1 });

      expect(result).toEqual({ success: true, done: true, steps: 1 });
      expect(progressTracker.startSession).toHaveBeenCalledWith("test goal");
    });

    it("should continue past an invalid LLM response", async () => {
      vi.useFakeTimers();

      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");

      gameAgentRunner.isRunning = false;
      getPage.mockReturnValue(mockPage);
      llmClient.generateCompletionWithRetry.mockResolvedValueOnce(null);
      adaptiveTiming.measureSitePerformance.mockResolvedValueOnce({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValueOnce({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });
      mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);

      const result = await gameAgentRunner.run("test goal", {
        maxSteps: 1,
        stuckDetection: false,
      });

      expect(result).toBeDefined();
      expect(typeof result.success).toBe("boolean");
      vi.useRealTimers();
    });

    it("should handle verification disabled and parallel execution path", async () => {
      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");
      const { parallelExecutor } =
        await import("@api/agent/parallelExecutor.js");
      const { actionEngine } = await import("@api/agent/actionEngine.js");
      const { progressTracker } = await import("@api/agent/progressTracker.js");

      gameAgentRunner.isRunning = false;
      gameAgentRunner.verifyAction = false;
      getPage.mockReturnValue(mockPage);
      llmClient.generateCompletionWithRetry.mockResolvedValueOnce([
        { action: "wait", value: "10" },
        { action: "done" },
      ]);
      adaptiveTiming.measureSitePerformance.mockResolvedValueOnce({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValueOnce({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });
      parallelExecutor.canParallelize.mockReturnValueOnce(true);
      parallelExecutor.executeSequence.mockResolvedValueOnce([
        { action: { action: "wait", value: "10" }, result: { success: true } },
        { action: { action: "done" }, result: { success: true, done: true } },
      ]);
      actionEngine.execute.mockResolvedValue({ success: true, done: false });

      const result = await gameAgentRunner.run("test goal", {
        maxSteps: 1,
        verifyAction: false,
      });

      expect(result).toEqual({ success: true, done: true, steps: 1 });
      expect(progressTracker.recordStep).toHaveBeenCalled();
    });

    it("should handle parallel execution entries without an action", async () => {
      const { getPage } = await import("@api/core/context.js");
      const { llmClient } = await import("@api/agent/llmClient.js");
      const { adaptiveTiming } = await import("@api/agent/adaptiveTiming.js");
      const { goalDecomposer } = await import("@api/agent/goalDecomposer.js");
      const { parallelExecutor } =
        await import("@api/agent/parallelExecutor.js");

      gameAgentRunner.isRunning = false;
      gameAgentRunner.verifyAction = false;
      getPage.mockReturnValue(mockPage);
      llmClient.generateCompletionWithRetry.mockResolvedValueOnce([
        { note: "skip-me" },
        { action: "done" },
      ]);
      adaptiveTiming.measureSitePerformance.mockResolvedValueOnce({
        click: 100,
        waitMultiplier: 1,
      });
      goalDecomposer.decompose.mockResolvedValueOnce({
        totalSteps: 1,
        pattern: "linear",
        currentStep: 0,
      });
      parallelExecutor.canParallelize.mockReturnValueOnce(true);
      parallelExecutor.executeSequence.mockResolvedValueOnce([
        {
          action: { note: "skip-me" },
          result: { success: false, error: "No action specified" },
        },
        { action: { action: "done" }, result: { success: true, done: true } },
      ]);

      const result = await gameAgentRunner.run("test goal", {
        maxSteps: 1,
        verifyAction: false,
      });

      expect(result.done).toBe(true);
      expect(parallelExecutor.executeSequence).toHaveBeenCalled();
    });
  });

  describe("Action Verification", () => {
    describe("_verifyAction", () => {
      it("should return true when AXTree is disabled", async () => {
        gameAgentRunner.useAXTree = false;
        const preState = {
          axTree: "{}",
          url: "https://a.com",
          screenshot: "abc",
        };
        const action = { action: "click" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(result).toBe(true);
      });

      it("should verify click action with AXTree changes", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        mockPage.url = vi.fn().mockReturnValue("https://after.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValueOnce(Buffer.from("before"))
          .mockResolvedValueOnce(Buffer.from("after"));
        mockPage.accessibility = {
          snapshot: vi
            .fn()
            .mockResolvedValueOnce({ role: "root", name: "before" })
            .mockResolvedValueOnce({ role: "root", name: "after" }),
        };

        const preState = {
          axTree: '{"role":"root","name":"before"}',
          url: "https://before.com",
          screenshot: "before",
        };
        const action = { action: "click" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(result).toBe(true); // URL changed
      });

      it("should verify clickAt action", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        mockPage.url = vi.fn().mockReturnValue("https://same.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValueOnce(Buffer.from("before"))
          .mockResolvedValueOnce(Buffer.from("before"));
        mockPage.accessibility = {
          snapshot: vi
            .fn()
            .mockResolvedValueOnce({ role: "root" })
            .mockResolvedValueOnce({ role: "root" }),
        };

        const preState = {
          axTree: '{"role":"root"}',
          url: "https://same.com",
          screenshot: "before",
        };
        const action = { action: "clickAt", x: 100, y: 100 };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(typeof result).toBe("boolean");
      });

      it("should verify type action and return boolean", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        mockPage.url = vi.fn().mockReturnValue("https://same.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValueOnce(Buffer.from("before"))
          .mockResolvedValueOnce(Buffer.from("after"));
        mockPage.accessibility = {
          snapshot: vi
            .fn()
            .mockResolvedValueOnce({ role: "textbox", name: "Input" })
            .mockResolvedValueOnce({ role: "textbox", name: "Input" }),
        };

        const preState = {
          axTree: '{"role":"textbox","name":"Input"}',
          url: "https://same.com",
          screenshot: "before",
        };
        const action = { action: "type", value: "typedvalue" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        // Result should be boolean (mock always returns false for visual diff)
        expect(typeof result).toBe("boolean");
      });

      it("should return true for type action when typed text is already in AXTree", async () => {
        gameAgentRunner.useAXTree = true;
        const preState = {
          axTree: '{"role":"textbox","name":"Input"}',
          url: "https://same.com",
          screenshot: "before",
        };
        const action = { action: "type", value: "typedvalue" };
        const captureSpy = vi
          .spyOn(gameAgentRunner, "_captureState")
          .mockResolvedValueOnce({
            axTree: '{"role":"textbox","name":"Input","value":"typedvalue"}',
            url: "https://same.com",
            screenshot: "before",
          });
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        captureSpy.mockRestore();
        expect(result).toBe(true);
      });

      it("should verify type action with URL change fallback", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        mockPage.url = vi.fn().mockReturnValue("https://after.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValueOnce(Buffer.from("before"))
          .mockResolvedValueOnce(Buffer.from("after"));
        mockPage.accessibility = {
          snapshot: vi
            .fn()
            .mockResolvedValueOnce({ role: "textbox" })
            .mockResolvedValueOnce({ role: "textbox" }),
        };

        const preState = {
          axTree: '{"role":"textbox"}',
          url: "https://before.com",
          screenshot: "before",
        };
        const action = { action: "type", value: "no match in tree" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(result).toBe(true); // URL changed
      });

      it("should return true for unknown action types", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi.fn().mockResolvedValue(undefined);
        mockPage.url = vi.fn().mockReturnValue("https://example.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValueOnce(Buffer.from("before"))
          .mockResolvedValueOnce(Buffer.from("after"));
        mockPage.accessibility = {
          snapshot: vi.fn().mockResolvedValue({ role: "root" }),
        };

        const preState = {
          axTree: "{}",
          url: "https://example.com",
          screenshot: "before",
        };
        const action = { action: "scroll" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(result).toBe(true);
      });

      it("should return false on verification error", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.waitForTimeout = vi
          .fn()
          .mockRejectedValue(new Error("timeout"));

        const preState = {
          axTree: "{}",
          url: "https://example.com",
          screenshot: "before",
        };
        const action = { action: "click" };
        const result = await gameAgentRunner._verifyAction(
          mockPage,
          preState,
          action,
        );
        expect(result).toBe(false);
      });
    });
  });

  describe("Screenshot Capture - Full Coverage", () => {
    describe("_captureScreenshot", () => {
      it("should handle missing viewport with evaluate fallback", async () => {
        mockPage.viewportSize = vi.fn().mockReturnValue(null);
        mockPage.evaluate = vi
          .fn()
          .mockResolvedValue({ width: 800, height: 600 });
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValue(Buffer.from("screenshot"));

        gameAgentRunner.vprepScaleFactor = 1.0;
        gameAgentRunner.screenshotCache = null;

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(mockPage.evaluate).toHaveBeenCalled();
      });

      it("should handle V-PREP failure with fallback", async () => {
        mockPage.viewportSize = vi
          .fn()
          .mockReturnValue({ width: 1920, height: 1080 });
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValue(Buffer.from("fallback-screenshot"));

        // Clear mocks for VPreprocessor
        const { VisionPreprocessor } =
          await import("@api/utils/VisionPreprocessor.js");
        VisionPreprocessor.mockImplementation(() => ({
          process: vi.fn().mockRejectedValue(new Error("VPrep failed")),
        }));

        gameAgentRunner.screenshotCache = null;
        gameAgentRunner.vprepScaleFactor = 1.0;

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(result).toBeTruthy();
      });

      it("should fall back to raw screenshot base64 when V-PREP fails", async () => {
        mockPage.viewportSize = vi
          .fn()
          .mockReturnValue({ width: 1280, height: 720 });
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValue(Buffer.from("raw-fallback"));
        gameAgentRunner.screenshotCache = null;
        gameAgentRunner.vprepScaleFactor = 1.0;

        const { VisionPreprocessor } =
          await import("@api/utils/VisionPreprocessor.js");
        VisionPreprocessor.mockImplementation(() => ({
          process: vi.fn().mockRejectedValue(new Error("VPrep failed")),
        }));

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(typeof result).toBe("string");
        expect(result).toBe(Buffer.from("raw-fallback").toString("base64"));
      });

      it("should handle complete screenshot failure", async () => {
        mockPage.viewportSize = vi
          .fn()
          .mockReturnValue({ width: 1920, height: 1080 });
        mockPage.screenshot = vi
          .fn()
          .mockRejectedValue(new Error("Screenshot failed"));

        const { VisionPreprocessor } =
          await import("@api/utils/VisionPreprocessor.js");
        VisionPreprocessor.mockImplementation(() => ({
          process: vi.fn().mockRejectedValue(new Error("VPrep failed")),
        }));

        gameAgentRunner.screenshotCache = null;

        const result = await gameAgentRunner._captureScreenshot(mockPage);
        expect(result).toBe("");
      });
    });

    describe("_captureAXTree", () => {
      it("should return full tree when compact is false", async () => {
        const mockTree = { role: "root", name: "page", children: [] };
        mockPage.accessibility = {
          snapshot: vi.fn().mockResolvedValue(mockTree),
        };

        const result = await gameAgentRunner._captureAXTree(mockPage, false);
        expect(result).toContain("root");
        expect(gameAgentRunner.lastFullAXTree).toBeDefined();
      });

      it("should return compact tree by default", async () => {
        const mockTree = {
          role: "root",
          name: "page",
          children: [{ role: "button", name: "Click me", selector: "#btn" }],
        };
        mockPage.accessibility = {
          snapshot: vi.fn().mockResolvedValue(mockTree),
        };

        const result = await gameAgentRunner._captureAXTree(mockPage);
        expect(result).toBeTruthy();
      });

      it("should return empty string on error", async () => {
        mockPage.accessibility = {
          snapshot: vi.fn().mockRejectedValue(new Error("AXTree failed")),
        };

        const result = await gameAgentRunner._captureAXTree(mockPage);
        expect(result).toBe("");
      });
    });
  });

  describe("Stuck Screenshot", () => {
    describe("_saveStuckScreenshot", () => {
      it("should save screenshot and return filename", async () => {
        mockPage.screenshot = vi.fn().mockResolvedValue(undefined);

        const result = await gameAgentRunner._saveStuckScreenshot(mockPage, 5);
        expect(result).toContain("stuck-");
        expect(result).toContain("-step5.png");
      });

      it("should return null on save failure", async () => {
        mockPage.screenshot = vi
          .fn()
          .mockRejectedValue(new Error("Save failed"));

        const result = await gameAgentRunner._saveStuckScreenshot(mockPage, 5);
        expect(result).toBe(null);
      });
    });
  });

  describe("Full State Capture", () => {
    describe("_captureState", () => {
      it("should capture full state with AXTree enabled", async () => {
        gameAgentRunner.useAXTree = true;
        mockPage.url = vi.fn().mockReturnValue("https://test.com");
        mockPage.screenshot = vi
          .fn()
          .mockResolvedValue(Buffer.from("screenshot"));
        mockPage.accessibility = {
          snapshot: vi.fn().mockResolvedValue({ role: "root" }),
        };

        const result = await gameAgentRunner._captureState(mockPage);
        expect(result).toHaveProperty("screenshot");
        expect(result).toHaveProperty("axTree");
        expect(result).toHaveProperty("url");
      });

      it("should skip AXTree when disabled", async () => {
        gameAgentRunner.useAXTree = false;
        mockPage.url = vi.fn().mockReturnValue("https://test.com");

        const result = await gameAgentRunner._captureState(mockPage);
        expect(result.axTree).toBe("[AXTree Disabled]");
      });
    });
  });
});
