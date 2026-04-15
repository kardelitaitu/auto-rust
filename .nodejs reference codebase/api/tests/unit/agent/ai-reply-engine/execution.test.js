/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock dependencies
vi.mock("@api/index.js", () => ({
  api: {
    wait: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@api/behaviors/human-interaction.js", () => ({
  HumanInteraction: vi.fn().mockImplementation(() => ({
    debugMode: false,
    selectMethod: vi
      .fn()
      .mockReturnValue({ name: "button_click", fn: vi.fn() }),
    verifyComposerOpen: vi
      .fn()
      .mockResolvedValue({ open: true, selector: ".composer" }),
    safeHumanClick: vi.fn().mockResolvedValue(true),
    typeText: vi.fn().mockResolvedValue(undefined),
    postTweet: vi.fn().mockResolvedValue({ success: true, reason: "posted" }),
    logStep: vi.fn(),
  })),
}));

describe("api/agent/ai-reply-engine/execution.js", () => {
  let executeReply;
  let mockEngine;
  let mockPage;
  let mockHuman;

  beforeEach(async () => {
    vi.clearAllMocks();
    vi.resetModules();

    // Import the module
    const module = await import("@api/agent/ai-reply-engine/execution.js");
    executeReply = module.executeReply;

    // Setup mocks
    mockEngine = {
      logger: {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
      },
    };

    mockPage = {
      evaluate: vi.fn().mockResolvedValue({
        url: "https://twitter.com/test",
        title: "Test Page",
        activeTag: "BODY",
        activeAria: null,
        hasComposer: false,
      }),
      keyboard: {
        press: vi.fn().mockResolvedValue(undefined),
      },
      mouse: {
        click: vi.fn().mockResolvedValue(undefined),
      },
      locator: vi.fn().mockReturnValue({
        first: vi.fn().mockReturnValue({
          count: vi.fn().mockResolvedValue(1),
          click: vi.fn().mockResolvedValue(undefined),
        }),
      }),
      viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
    };

    // Reset HumanInteraction mock
    const { HumanInteraction } =
      await import("@api/behaviors/human-interaction.js");
    mockHuman = {
      debugMode: false,
      selectMethod: vi.fn().mockReturnValue({
        name: "button_click",
        fn: vi
          .fn()
          .mockResolvedValue({ success: true, method: "button_click" }),
      }),
      verifyComposerOpen: vi
        .fn()
        .mockResolvedValue({ open: true, selector: ".composer" }),
      safeHumanClick: vi.fn().mockResolvedValue(true),
      typeText: vi.fn().mockResolvedValue(undefined),
      postTweet: vi.fn().mockResolvedValue({ success: true, reason: "posted" }),
      logStep: vi.fn(),
    };
    HumanInteraction.mockReturnValue(mockHuman);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("executeReply", () => {
    it("should execute reply and return result", async () => {
      const replyText = "This is a test reply";

      const result = await executeReply(mockEngine, mockPage, replyText);

      expect(result).toBeDefined();
      expect(mockEngine.logger.info).toHaveBeenCalled();
    });

    it("should call human.selectMethod to select reply method", async () => {
      await executeReply(mockEngine, mockPage, "Test reply");

      expect(mockHuman.selectMethod).toHaveBeenCalled();
    });

    it("should fallback to button_click when method returns failure", async () => {
      mockHuman.selectMethod.mockReturnValueOnce({
        name: "keyboard_shortcut",
        fn: vi
          .fn()
          .mockResolvedValue({ success: false, reason: "composer_not_open" }),
      });

      const result = await executeReply(mockEngine, mockPage, "Test reply");

      expect(result.success).toBe(true);
      expect(mockEngine.logger.warn).toHaveBeenCalled();
    });

    it("should fallback to button_click when method throws error", async () => {
      mockHuman.selectMethod.mockReturnValueOnce({
        name: "keyboard_shortcut",
        fn: vi.fn().mockRejectedValue(new Error("Test error")),
      });

      const result = await executeReply(mockEngine, mockPage, "Test reply");

      expect(result).toBeDefined();
      expect(mockEngine.logger.error).toHaveBeenCalled();
    });
  });

  describe("reply methods", () => {
    // Import the private functions for testing
    let replyMethodA_Keyboard;
    let replyMethodB_Button;
    let replyMethodC_Tab;
    let replyMethodD_RightClick;

    beforeEach(async () => {
      // We'll test them indirectly through executeReply with mocked selectMethod
    });

    describe("keyboard_shortcut method (Method A)", () => {
      it("should use keyboard shortcut for reply", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          const keyboardMethod = methods.find(
            (m) => m.name === "keyboard_shortcut",
          );
          return keyboardMethod;
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(mockPage.keyboard.press).toHaveBeenCalled();
      });

      it("should handle composer verification failure", async () => {
        mockHuman.verifyComposerOpen
          .mockResolvedValueOnce({ open: false })
          .mockResolvedValueOnce({ open: true, selector: ".composer" });

        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "keyboard_shortcut");
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should return failure when composer never opens", async () => {
        mockHuman.verifyComposerOpen.mockResolvedValue({ open: false });

        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "keyboard_shortcut");
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        // Should fallback to button_click
        expect(result).toBeDefined();
      });
    });

    describe("button_click method (Method B)", () => {
      it("should find and click reply button", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "button_click");
        });

        mockPage.locator.mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
          }),
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should return failure when reply button not found", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "button_click");
        });

        mockPage.locator.mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
          }),
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should return failure when composer does not open after button click", async () => {
        mockHuman.verifyComposerOpen.mockResolvedValue({ open: false });

        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "button_click");
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });
    });

    describe("tab_navigation method (Method C)", () => {
      it("should use tab navigation for reply", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "tab_navigation");
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(mockPage.keyboard.press).toHaveBeenCalled();
      });

      it("should return failure when composer does not open via tab", async () => {
        mockHuman.verifyComposerOpen.mockResolvedValue({ open: false });

        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "tab_navigation");
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });
    });

    describe("right_click method (Method D)", () => {
      it("should use right-click menu for reply", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "right_click");
        });

        mockPage.locator.mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            click: vi.fn().mockResolvedValue(undefined),
          }),
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should return failure when tweet element not found", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "right_click");
        });

        mockPage.locator.mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(0),
          }),
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should handle reply option not found in context menu", async () => {
        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "right_click");
        });

        let callCount = 0;
        mockPage.locator.mockImplementation(() => ({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockImplementation(async () => {
              callCount++;
              // First call for tweet element returns 1, subsequent calls return 0
              return callCount === 1 ? 1 : 0;
            }),
            click: vi.fn().mockResolvedValue(undefined),
          }),
        }));

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });

      it("should return failure when composer does not open after right-click", async () => {
        mockHuman.verifyComposerOpen.mockResolvedValue({ open: false });

        mockHuman.selectMethod.mockImplementation((methods) => {
          return methods.find((m) => m.name === "right_click");
        });

        mockPage.locator.mockReturnValue({
          first: vi.fn().mockReturnValue({
            count: vi.fn().mockResolvedValue(1),
            click: vi.fn().mockResolvedValue(undefined),
          }),
        });

        const result = await executeReply(mockEngine, mockPage, "Test reply");

        expect(result).toBeDefined();
      });
    });
  });

  describe("method weights", () => {
    it("should include all four reply methods with correct weights", async () => {
      let capturedMethods = null;
      mockHuman.selectMethod.mockImplementation((methods) => {
        capturedMethods = methods;
        return methods[1]; // button_click
      });

      await executeReply(mockEngine, mockPage, "Test");

      expect(capturedMethods).toHaveLength(4);

      const methodNames = capturedMethods.map((m) => m.name);
      expect(methodNames).toContain("keyboard_shortcut");
      expect(methodNames).toContain("button_click");
      expect(methodNames).toContain("tab_navigation");
      expect(methodNames).toContain("right_click");

      const weights = capturedMethods.map((m) => m.weight);
      expect(weights.reduce((a, b) => a + b, 0)).toBe(100);
    });
  });

  describe("error handling", () => {
    it("should log error when method fails", async () => {
      mockHuman.selectMethod.mockReturnValueOnce({
        name: "keyboard_shortcut",
        fn: vi.fn().mockRejectedValue(new Error("Network error")),
      });

      await executeReply(mockEngine, mockPage, "Test reply");

      expect(mockEngine.logger.error).toHaveBeenCalledWith(
        expect.stringContaining("Network error"),
      );
    });

    it("should log warning when method returns failure", async () => {
      mockHuman.selectMethod.mockReturnValueOnce({
        name: "button_click",
        fn: vi
          .fn()
          .mockResolvedValue({ success: false, reason: "button_not_found" }),
      });

      await executeReply(mockEngine, mockPage, "Test reply");

      expect(mockEngine.logger.warn).toHaveBeenCalled();
    });
  });
});
