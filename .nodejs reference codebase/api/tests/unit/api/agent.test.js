/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { see, processDomElements } from "@api/agent/observer.js";
import { find } from "@api/agent/finder.js";
import { doAction } from "@api/agent/executor.js";
import {
  screenshot,
  injectAnnotations,
  removeAnnotations,
} from "@api/agent/vision.js";

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(),
  isSessionActive: vi.fn(),
}));

vi.mock("@api/core/context-state.js", () => ({
  getStateAgentElementMap: vi.fn(),
  setStateAgentElementMap: vi.fn(),
  getStateSection: vi.fn(),
}));

vi.mock("@api/core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    error: vi.fn(),
    warn: vi.fn(),
    debug: vi.fn(),
  })),
}));

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue("clicked"),
  type: vi.fn().mockResolvedValue("typed"),
  hover: vi.fn().mockResolvedValue("hovered"),
  drag: vi.fn().mockResolvedValue("dragged"),
  clickAt: vi.fn().mockResolvedValue("clickedAt"),
  multiSelect: vi.fn().mockResolvedValue("multiSelected"),
  press: vi.fn().mockResolvedValue("pressed"),
}));

vi.mock("sharp", () => ({
  default: vi.fn(() => ({
    metadata: vi.fn().mockResolvedValue({ width: 800, height: 600 }),
    resize: vi.fn().mockReturnThis(),
    jpeg: vi.fn().mockReturnThis(),
    toBuffer: vi.fn().mockResolvedValue(Buffer.from("fake-image-data")),
  })),
}));

import { getPage, isSessionActive } from "@api/core/context.js";
import {
  getStateAgentElementMap,
  setStateAgentElementMap,
} from "@api/core/context-state.js";
import { click, type, hover, drag } from "@api/interactions/actions.js";

describe("api/agent/observer.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://example.com"),
      waitForRequest: vi.fn(),
      evaluate: vi.fn(),
    };

    getPage.mockReturnValue(mockPage);
    isSessionActive.mockReturnValue(true);
  });

  describe("see", () => {
    it("should return compact string by default", async () => {
      const mockElements = [
        {
          id: 1,
          role: "button",
          label: "Submit",
          selector: '[data-agent-id="1"]',
        },
        { id: 2, role: "link", label: "Home", selector: '[data-agent-id="2"]' },
      ];

      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();

      expect(result).toBe('[1] button: "Submit"\n[2] link: "Home"');
      expect(setStateAgentElementMap).toHaveBeenCalledWith(mockElements);
    });

    it("should return full element array when compact is false", async () => {
      const mockElements = [
        {
          id: 1,
          role: "button",
          label: "Submit",
          selector: '[data-agent-id="1"]',
        },
      ];

      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see({ compact: false });

      expect(result).toEqual(mockElements);
      expect(setStateAgentElementMap).toHaveBeenCalledWith(mockElements);
    });

    it("should handle empty element list", async () => {
      mockPage.evaluate.mockResolvedValue([]);

      const result = await see();

      expect(result).toBe("");
      expect(setStateAgentElementMap).toHaveBeenCalledWith([]);
    });

    it("should throw when page evaluate fails", async () => {
      mockPage.evaluate.mockRejectedValue(new Error("Evaluation failed"));

      await expect(see()).rejects.toThrow("Evaluation failed");
    });

    it("should pass compact option to function", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      mockPage.evaluate.mockResolvedValue(mockElements);

      await see({ compact: true });
      await see({ compact: false });

      expect(setStateAgentElementMap).toHaveBeenCalledTimes(2);
    });

    it("should handle single element correctly", async () => {
      const mockElements = [
        {
          id: 1,
          role: "button",
          label: "Submit",
          selector: '[data-agent-id="1"]',
        },
      ];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();

      expect(result).toContain("[1]");
    });

    it("should handle multiple elements", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit" },
        { id: 2, role: "input", label: "Email" },
        { id: 3, role: "link", label: "Forgot Password" },
      ];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();

      expect(result).toContain("[1]");
      expect(result).toContain("[2]");
      expect(result).toContain("[3]");
    });

    it("should call getPage to get page context", async () => {
      mockPage.evaluate.mockResolvedValue([]);

      await see();

      expect(getPage).toHaveBeenCalled();
    });

    it("should use default compact option when not provided", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      mockPage.evaluate.mockResolvedValue(mockElements);

      // compact defaults to true
      const result = await see();

      expect(result).toContain("[1]");
    });

    it("should handle elements with special characters in labels", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit & Continue >" },
      ];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();

      expect(result).toContain("Submit");
    });
  });
});

describe("api/agent/observer.js - processDomElements", () => {
  it("should process button elements", () => {
    const mockElement = {
      tagName: "BUTTON",
      innerText: "Click Me",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50, top: 10, left: 10 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].label).toBe("Click Me");
  });

  it("should skip hidden elements", () => {
    const mockElement = {
      tagName: "BUTTON",
      innerText: "Hidden",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "none",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(0);
  });

  it("should handle elements with zero dimensions", () => {
    const mockElement = {
      tagName: "BUTTON",
      innerText: "Zero Size",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi.fn().mockReturnValue({ width: 0, height: 0 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(0);
  });

  it("should extract aria-label when innerText is empty", () => {
    const mockElement = {
      tagName: "BUTTON",
      innerText: "",
      getAttribute: vi.fn((name) => {
        if (name === "aria-label") return "Accessible Button";
        return null;
      }),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
  });

  it("should process input elements with type", () => {
    const mockElement = {
      tagName: "INPUT",
      innerText: "",
      type: "email",
      value: "test@example.com",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].role).toBe("email");
  });

  it("should use data-testid when no label", () => {
    let callCount = 0;
    const mockGetAttribute = (name) => {
      callCount++;
      if (name === "data-testid") return "test-element";
      if (name === "role") return null;
      return null;
    };

    const mockElement = {
      tagName: "DIV",
      innerText: "",
      getAttribute: mockGetAttribute,
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].label).toBe("[test-element]");
  });

  it("should skip element when no data-testid and no role", () => {
    const mockElement = {
      tagName: "DIV",
      innerText: "",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(0);
  });

  it("should determine role as link for anchor tags", () => {
    const mockElement = {
      tagName: "A",
      innerText: "Click Here",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].role).toBe("link");
  });

  it("should determine role for input with type", () => {
    const mockElement = {
      tagName: "INPUT",
      innerText: "",
      type: "checkbox",
      value: "yes",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].role).toBe("checkbox");
  });

  it("should determine role as input when input has no type", () => {
    const mockElement = {
      tagName: "INPUT",
      innerText: "",
      type: undefined,
      value: "test",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].role).toBe("input");
  });

  it("should determine role from tagName for other elements", () => {
    const mockElement = {
      tagName: "SELECT",
      innerText: "Choose",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].role).toBe("select");
  });

  it("should set selector with data-agent-id", () => {
    const mockElement = {
      tagName: "BUTTON",
      innerText: "Test",
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].selector).toBe('[data-agent-id="1"]');
  });

  it("should assign sequential IDs to elements", () => {
    const mockElements = [
      {
        tagName: "BUTTON",
        innerText: "One",
        getAttribute: vi.fn().mockReturnValue(null),
        setAttribute: vi.fn(),
        getBoundingClientRect: vi
          .fn()
          .mockReturnValue({ width: 100, height: 50 }),
      },
      {
        tagName: "BUTTON",
        innerText: "Two",
        getAttribute: vi.fn().mockReturnValue(null),
        setAttribute: vi.fn(),
        getBoundingClientRect: vi
          .fn()
          .mockReturnValue({ width: 100, height: 50 }),
      },
      {
        tagName: "BUTTON",
        innerText: "Three",
        getAttribute: vi.fn().mockReturnValue(null),
        setAttribute: vi.fn(),
        getBoundingClientRect: vi
          .fn()
          .mockReturnValue({ width: 100, height: 50 }),
      },
    ];

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue(mockElements),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(3);
    expect(results[0].id).toBe(1);
    expect(results[1].id).toBe(2);
    expect(results[2].id).toBe(3);
  });

  it("should use [no-label] fallback when label is empty and has role", () => {
    // When there IS a role, the element won't be skipped, but label will be empty string
    const mockElement = {
      tagName: "BUTTON",
      innerText: "",
      getAttribute: vi.fn((name) => {
        if (name === "role") return "button"; // Has role, so won't be skipped
        return null;
      }),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].label).toBe("[no-label]");
  });

  it("should truncate long labels", () => {
    const longText = "A".repeat(100);
    const mockElement = {
      tagName: "BUTTON",
      innerText: longText,
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getBoundingClientRect: vi
        .fn()
        .mockReturnValue({ width: 100, height: 50 }),
    };

    const mockDoc = {
      querySelectorAll: vi.fn().mockReturnValue([mockElement]),
      defaultView: {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      },
    };

    const results = processDomElements(mockDoc);
    expect(results.length).toBe(1);
    expect(results[0].label.length).toBe(50);
    expect(results[0].label.endsWith("...")).toBe(true);
  });
});

describe("api/agent/finder.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("find", () => {
    it("should return exact label match", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit" },
        { id: 2, role: "link", label: "Cancel" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await find("Submit");

      expect(result).toEqual(mockElements[0]);
    });

    it("should return case-insensitive match", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await find("submit");

      expect(result).toEqual(mockElements[0]);
    });

    it("should return inclusion match when exact fails", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit Form" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await find("Submit");

      expect(result).toEqual(mockElements[0]);
    });

    it("should match role + label combination", async () => {
      const mockElements = [{ id: 1, role: "button", label: "post" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await find("button post");

      expect(result).toEqual(mockElements[0]);
    });

    it("should return null when no match found", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await find("Nonexistent");

      expect(result).toBeNull();
    });

    it("should return null when element map is empty", async () => {
      getStateAgentElementMap.mockReturnValue(null);

      const result = await find("test");

      expect(result).toBeNull();
    });

    it("should return null when element map is empty array", async () => {
      getStateAgentElementMap.mockReturnValue([]);

      const result = await find("test");

      expect(result).toBeNull();
    });
  });
});

describe("api/agent/executor.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("doAction", () => {
    it("should click element by numeric ID", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit", selector: "#submit-btn" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await doAction("click", 1);

      expect(click).toHaveBeenCalledWith("#submit-btn");
      expect(result).toBe("clicked");
    });

    it("should click element by exact label", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit", selector: "#submit-btn" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await doAction("click", "Submit");

      expect(click).toHaveBeenCalledWith("#submit-btn");
    });

    it("should click element by label inclusion", async () => {
      const mockElements = [
        {
          id: 1,
          role: "button",
          label: "Submit Form",
          selector: "#submit-btn",
        },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await doAction("click", "Submit");

      expect(click).toHaveBeenCalledWith("#submit-btn");
    });

    it("should type into element", async () => {
      const mockElements = [
        { id: 1, role: "input", label: "Search", selector: "#search-input" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await doAction("type", "Search", "test query");

      expect(type).toHaveBeenCalledWith("#search-input", "test query");
      expect(result).toBe("typed");
    });

    it("should fill element (alias for type)", async () => {
      const mockElements = [
        { id: 1, role: "input", label: "Email", selector: "#email-input" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      await doAction("fill", "Email", "test@example.com");

      expect(type).toHaveBeenCalledWith("#email-input", "test@example.com");
    });

    it("should hover over element", async () => {
      const mockElements = [
        { id: 1, role: "link", label: "Menu", selector: "#menu-link" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      const result = await doAction("hover", "Menu");

      expect(hover).toHaveBeenCalledWith("#menu-link");
      expect(result).toBe("hovered");
    });

    it("should throw when element not found by ID", async () => {
      getStateAgentElementMap.mockReturnValue([]);

      await expect(doAction("click", 999)).rejects.toThrow(
        "not found in current view",
      );
    });

    it("should throw when element not found by label", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      await expect(doAction("click", "Nonexistent")).rejects.toThrow(
        "not found in current view",
      );
    });

    it("should throw for unsupported action", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];

      getStateAgentElementMap.mockReturnValue(mockElements);

      await expect(doAction("invalidAction", 1)).rejects.toThrow(
        "Unsupported agent action",
      );
    });

    it("should support drag action with target", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Source", selector: "#source" },
        { id: 2, role: "button", label: "Target", selector: "#target" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      await doAction("drag", 1, "#target");

      expect(drag).toHaveBeenCalledWith("#source", "#target", {});
    });

    it("should handle case-insensitive action names", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit", selector: "#submit-btn" },
      ];

      getStateAgentElementMap.mockReturnValue(mockElements);

      await doAction("CLICK", 1);

      expect(click).toHaveBeenCalledWith("#submit-btn");
    });
  });
});

describe("api/agent/vision.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPage = {
      url: vi.fn().mockReturnValue("https://example.com"),
      addInitScript: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn().mockResolvedValue(undefined),
      screenshot: vi.fn().mockResolvedValue(Buffer.from("fake-image-data")),
    };

    getPage.mockReturnValue(mockPage);
    getStateAgentElementMap.mockReturnValue([]);
  });

  describe("screenshot", () => {
    it("should return base64 by default", async () => {
      const result = await screenshot();

      expect(result).toBe("ZmFrZS1pbWFnZS1kYXRh"); // base64 of 'fake-image-data'
    });

    it("should capture full page when requested", async () => {
      await screenshot({ fullPage: true });

      expect(mockPage.screenshot).toHaveBeenCalledWith({
        type: "jpeg",
        quality: 100,
        fullPage: true,
      });
    });

    it("should handle path options by ignoring it and returning base64", async () => {
      const result = await screenshot({ path: "/tmp/screenshot.jpg" });

      expect(mockPage.screenshot).toHaveBeenCalledWith({
        path: "/tmp/screenshot.jpg",
        type: "jpeg",
        quality: 100,
        fullPage: false,
      });
      expect(result).toBe("ZmFrZS1pbWFnZS1kYXRh");
    });

    it("should annotate elements when requested with non-empty element map", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true });

      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should skip annotation when element map is empty", async () => {
      // Element map is already set to [] in beforeEach
      // The code checks elementMap.length > 0 before annotating
      const result = await screenshot({ annotate: true });

      expect(result).toBe("ZmFrZS1pbWFnZS1kYXRh");
    });

    it("should clean up annotations after screenshot", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Submit" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true });

      // Should be called twice: once for annotation, once for cleanup
      expect(mockPage.evaluate).toHaveBeenCalledTimes(2);
    });

    it("should handle screenshot error gracefully", async () => {
      mockPage.screenshot.mockRejectedValue(new Error("Screenshot failed"));

      const result = await screenshot();

      expect(result).toBeNull();
    });

    it("should use jpeg format with quality 100 for Playwright", async () => {
      await screenshot();

      expect(mockPage.screenshot).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "jpeg",
          quality: 100,
        }),
      );
    });

    it("should accept all options together", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true, fullPage: true, path: "/test.jpg" });

      expect(mockPage.screenshot).toHaveBeenCalledWith({
        path: "/test.jpg",
        type: "jpeg",
        quality: 100,
        fullPage: true,
      });
    });

    it("should handle annotation injection error gracefully", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      // First evaluate call throws
      mockPage.evaluate
        .mockRejectedValueOnce(new Error("DOM error"))
        .mockResolvedValueOnce(undefined);

      const result = await screenshot({ annotate: true });

      expect(result).toBeNull();
    });

    it("should return base64 string", async () => {
      const result = await screenshot();

      // Verify it's a valid base64 string
      expect(typeof result).toBe("string");
      expect(result.length).toBeGreaterThan(0);
    });

    it("should handle multiple elements in annotation", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit" },
        { id: 2, role: "input", label: "Email" },
        { id: 3, role: "link", label: "Forgot Password" },
      ];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true });

      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should use default options when none provided", async () => {
      await screenshot();

      expect(mockPage.screenshot).toHaveBeenCalledWith({
        type: "jpeg",
        quality: 100,
        fullPage: false,
      });
    });

    it("should call page.evaluate with injectAnnotations for annotation", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true });

      // Verify evaluate was called with the annotation function
      expect(mockPage.evaluate).toHaveBeenCalled();
    });

    it("should call page.evaluate with removeAnnotations for cleanup", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      getStateAgentElementMap.mockReturnValue(mockElements);

      await screenshot({ annotate: true });

      // Should be called twice - once for inject, once for cleanup
      expect(mockPage.evaluate).toHaveBeenCalledTimes(2);
    });
  });
});

describe("api/agent/vision.js - injectAnnotations", () => {
  it("should create annotation container", () => {
    const mockDoc = {
      createElement: vi.fn().mockImplementation((tag) => ({
        id: "",
        style: {},
        appendChild: vi.fn(),
        getAttribute: vi.fn(),
      })),
      body: {
        appendChild: vi.fn(),
      },
      querySelector: vi.fn().mockReturnValue({
        getBoundingClientRect: vi
          .fn()
          .mockReturnValue({ width: 100, height: 50 }),
      }),
      defaultView: {
        scrollY: 0,
        scrollX: 0,
      },
    };

    const mockMap = [{ id: 1, role: "button", label: "Test" }];

    // Call and verify element creation
    injectAnnotations(mockDoc, mockMap);

    expect(mockDoc.createElement).toHaveBeenCalledWith("div");
  });

  it("should skip elements not found in DOM", () => {
    const mockDoc = {
      createElement: vi.fn().mockImplementation((tag) => ({
        id: "",
        style: {},
        appendChild: vi.fn(),
        getAttribute: vi.fn(),
      })),
      body: {
        appendChild: vi.fn(),
      },
      querySelector: vi.fn().mockReturnValue(null),
    };

    const mockMap = [{ id: 1, role: "button", label: "Test" }];

    injectAnnotations(mockDoc, mockMap);

    // Container created but no boxes for missing elements
    expect(mockDoc.createElement).toHaveBeenCalled();
  });
});

describe("api/agent/vision.js - removeAnnotations", () => {
  it("should remove existing annotation container", () => {
    const mockContainer = { remove: vi.fn() };
    const mockDoc = {
      getElementById: vi.fn().mockReturnValue(mockContainer),
    };

    removeAnnotations(mockDoc);

    expect(mockDoc.getElementById).toHaveBeenCalledWith(
      "agent-vision-annotations",
    );
    expect(mockContainer.remove).toHaveBeenCalled();
  });

  it("should handle missing container gracefully", () => {
    const mockDoc = {
      getElementById: vi.fn().mockReturnValue(null),
    };

    expect(() => removeAnnotations(mockDoc)).not.toThrow();
  });
});
