import { describe, it, expect, vi, beforeEach } from "vitest";

const mockPage = {
  evaluate: vi.fn().mockResolvedValue([]),
};

vi.mock("@api/core/context.js", () => ({
  getPage: vi.fn(() => mockPage),
}));

vi.mock("@api/core/context-state.js", () => ({
  setStateAgentElementMap: vi.fn(),
}));

describe("api/agent/observer.js", () => {
  let processDomElements;
  let see;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/observer.js");
    processDomElements = module.processDomElements;
    see = module.see;
  });

  describe("processDomElements()", () => {
    let mockDoc;
    let mockWindow;

    beforeEach(() => {
      mockWindow = {
        getComputedStyle: vi.fn().mockReturnValue({
          display: "block",
          visibility: "visible",
          opacity: "1",
        }),
      };

      mockDoc = {
        defaultView: mockWindow,
        querySelectorAll: vi.fn().mockReturnValue([]),
      };
    });

    it("should return empty array when no interactive elements", () => {
      mockDoc.querySelectorAll.mockReturnValue([]);
      const result = processDomElements(mockDoc);
      expect(result).toEqual([]);
    });

    it("should find button elements", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Click me",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("button");
      expect(result[0].label).toBe("Click me");
    });

    it("should find link elements", () => {
      const mockLink = createMockElement({
        tagName: "A",
        innerText: "Home",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockLink]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("link");
    });

    it("should find input elements", () => {
      const mockInput = createMockElement({
        tagName: "INPUT",
        type: "text",
        placeholder: "Enter text",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockInput]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("text");
      expect(result[0].label).toBe("Enter text");
    });

    it('should find elements with role="button"', () => {
      const mockDiv = createMockElement({
        tagName: "DIV",
        innerText: "Click",
        role: "button",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockDiv]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("div");
    });

    it("should skip hidden elements (display: none)", () => {
      mockWindow.getComputedStyle.mockReturnValue({
        display: "none",
        visibility: "visible",
        opacity: "1",
      });
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Hidden",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(0);
    });

    it("should skip hidden elements (visibility: hidden)", () => {
      mockWindow.getComputedStyle.mockReturnValue({
        display: "block",
        visibility: "hidden",
        opacity: "1",
      });
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Hidden",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(0);
    });

    it("should skip hidden elements (opacity: 0)", () => {
      mockWindow.getComputedStyle.mockReturnValue({
        display: "block",
        visibility: "visible",
        opacity: "0",
      });
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Hidden",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(0);
    });

    it("should skip elements with zero dimensions", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Zero size",
        rect: { x: 0, y: 0, width: 0, height: 0 },
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(0);
    });

    it("should truncate long labels", () => {
      const longText =
        "This is a very long label that exceeds fifty characters limit";
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: longText,
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result[0].label.length).toBeLessThanOrEqual(50);
      expect(result[0].label).toMatch(/\.\.\.$|...$/);
    });

    it("should use aria-label when no innerText", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "",
        ariaLabel: "Close dialog",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result[0].label).toBe("Close dialog");
    });

    it("should use placeholder when no innerText or aria-label", () => {
      const mockInput = createMockElement({
        tagName: "INPUT",
        innerText: "",
        ariaLabel: "",
        placeholder: "Search...",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockInput]);

      const result = processDomElements(mockDoc);
      expect(result[0].label).toBe("Search...");
    });

    it("should use data-testid as label when no other label", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "",
        ariaLabel: "",
        dataTestId: "submit-btn",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result[0].label).toBe("[submit-btn]");
    });

    it("should skip unlabeled elements without role", () => {
      const mockDiv = createMockElement({
        tagName: "DIV",
        innerText: "",
        ariaLabel: "",
        dataTestId: "",
        role: "",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockDiv]);

      const result = processDomElements(mockDoc);
      expect(result.length).toBe(0);
    });

    it("should assign sequential IDs to elements", () => {
      const mockButton1 = createMockElement({
        tagName: "BUTTON",
        innerText: "A",
      });
      const mockButton2 = createMockElement({
        tagName: "BUTTON",
        innerText: "B",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton1, mockButton2]);

      const result = processDomElements(mockDoc);
      expect(result[0].id).toBe(1);
      expect(result[1].id).toBe(2);
    });

    it("should set data-agent-id attribute on elements", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Test",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      processDomElements(mockDoc);
      expect(mockButton.setAttribute).toHaveBeenCalledWith(
        "data-agent-id",
        "1",
      );
    });

    it("should generate correct selector", () => {
      const mockButton = createMockElement({
        tagName: "BUTTON",
        innerText: "Test",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockButton]);

      const result = processDomElements(mockDoc);
      expect(result[0].selector).toBe('[data-agent-id="1"]');
    });

    it("should use input type as role", () => {
      const mockInput = createMockElement({
        tagName: "INPUT",
        type: "checkbox",
        innerText: "",
        ariaLabel: "Accept terms",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockInput]);

      const result = processDomElements(mockDoc);
      expect(result[0].role).toBe("checkbox");
    });

    it("should default to input role if type missing", () => {
      const mockInput = createMockElement({
        tagName: "INPUT",
        type: "",
        innerText: "",
        ariaLabel: "Unknown input",
      });
      mockDoc.querySelectorAll.mockReturnValue([mockInput]);

      const result = processDomElements(mockDoc);
      expect(result[0].role).toBe("input");
    });
  });

  describe("see()", () => {
    it("should return compact format by default", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Click" }];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();
      expect(typeof result).toBe("string");
      expect(result).toContain("[1]");
      expect(result).toContain("button");
      expect(result).toContain("Click");
    });

    it("should return array when compact is false", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Click" }];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see({ compact: false });
      expect(Array.isArray(result)).toBe(true);
      expect(result[0]).toHaveProperty("id");
      expect(result[0]).toHaveProperty("role");
      expect(result[0]).toHaveProperty("label");
    });

    it("should store elements in context state", async () => {
      const mockElements = [{ id: 1, role: "button", label: "Test" }];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const { setStateAgentElementMap } =
        await import("@api/core/context-state.js");
      await see();
      expect(setStateAgentElementMap).toHaveBeenCalledWith(mockElements);
    });

    it("should format multiple elements correctly", async () => {
      const mockElements = [
        { id: 1, role: "button", label: "Submit" },
        { id: 2, role: "input", label: "Email" },
      ];
      mockPage.evaluate.mockResolvedValue(mockElements);

      const result = await see();
      const lines = result.split("\n");
      expect(lines.length).toBe(2);
      expect(lines[0]).toContain("[1]");
      expect(lines[1]).toContain("[2]");
    });

    it("should handle empty element list", async () => {
      mockPage.evaluate.mockResolvedValue([]);

      const result = await see();
      expect(result).toBe("");
    });
  });

  describe("default export", () => {
    it("should export see as default", async () => {
      const module = await import("@api/agent/observer.js");
      expect(module.default).toBe(module.see);
    });
  });
});

function createMockElement(options) {
  const {
    tagName = "BUTTON",
    innerText = "",
    ariaLabel = "",
    placeholder = "",
    title = "",
    alt = "",
    value = "",
    type = "",
    role = "",
    dataTestId = "",
    rect = { x: 100, y: 100, width: 100, height: 50 },
  } = options;

  return {
    tagName,
    innerText,
    type, // Direct property for input type
    value,
    getAttribute: vi.fn((attr) => {
      switch (attr) {
        case "aria-label":
          return ariaLabel || null;
        case "placeholder":
          return placeholder || null;
        case "title":
          return title || null;
        case "alt":
          return alt || null;
        case "role":
          return role || null;
        case "data-testid":
          return dataTestId || null;
        case "type":
          return type || null;
        default:
          return null;
      }
    }),
    getBoundingClientRect: vi.fn().mockReturnValue(rect),
    setAttribute: vi.fn(),
  };
}
