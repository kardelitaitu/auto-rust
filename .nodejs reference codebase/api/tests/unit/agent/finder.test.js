import { describe, it, expect, vi, beforeEach } from "vitest";

const mockElementMap = [
  { id: 1, role: "button", label: "Submit", selector: '[data-agent-id="1"]' },
  { id: 2, role: "link", label: "Home Page", selector: '[data-agent-id="2"]' },
  { id: 3, role: "input", label: "Search", selector: '[data-agent-id="3"]' },
  { id: 4, role: "button", label: "Cancel", selector: '[data-agent-id="4"]' },
  {
    id: 5,
    role: "checkbox",
    label: "Remember me",
    selector: '[data-agent-id="5"]',
  },
];

vi.mock("@api/core/context-state.js", () => ({
  getStateAgentElementMap: vi.fn(() => mockElementMap),
}));

describe("api/agent/finder.js", () => {
  let find;

  beforeEach(async () => {
    vi.clearAllMocks();
    const module = await import("@api/agent/finder.js");
    find = module.find;
  });

  describe("find()", () => {
    it("should return null for empty element map", async () => {
      const { getStateAgentElementMap } =
        await import("@api/core/context-state.js");
      getStateAgentElementMap.mockReturnValueOnce([]);
      const result = await find("button");
      expect(result).toBeNull();
    });

    it("should return null for null element map", async () => {
      const { getStateAgentElementMap } =
        await import("@api/core/context-state.js");
      getStateAgentElementMap.mockReturnValueOnce(null);
      const result = await find("button");
      expect(result).toBeNull();
    });

    it("should find element by exact label match", async () => {
      const result = await find("Submit");
      expect(result).toBeDefined();
      expect(result.label).toBe("Submit");
      expect(result.role).toBe("button");
    });

    it("should be case insensitive for exact match", async () => {
      const result = await find("submit");
      expect(result).toBeDefined();
      expect(result.label).toBe("Submit");
    });

    it("should find element by inclusion match", async () => {
      const result = await find("home");
      expect(result).toBeDefined();
      expect(result.label).toBe("Home Page");
    });

    it("should find element by role + label match", async () => {
      const result = await find("button submit");
      expect(result).toBeDefined();
      expect(result.role).toBe("button");
    });

    it("should find input by role + label", async () => {
      const result = await find("input search");
      expect(result).toBeDefined();
      expect(result.role).toBe("input");
    });

    it("should return null when no match found", async () => {
      const result = await find("nonexistent-element-xyz");
      expect(result).toBeNull();
    });

    it("should return element with correct properties", async () => {
      const result = await find("Submit");
      expect(result).toHaveProperty("id");
      expect(result).toHaveProperty("role");
      expect(result).toHaveProperty("label");
      expect(result).toHaveProperty("selector");
    });

    it("should prioritize exact match over inclusion match", async () => {
      const { getStateAgentElementMap } =
        await import("@api/core/context-state.js");
      getStateAgentElementMap.mockReturnValueOnce([
        {
          id: 1,
          role: "button",
          label: "Submit Form",
          selector: '[data-agent-id="1"]',
        },
        {
          id: 2,
          role: "button",
          label: "Submit",
          selector: '[data-agent-id="2"]',
        },
      ]);
      const result = await find("Submit");
      expect(result.label).toBe("Submit");
      expect(result.id).toBe(2);
    });

    it("should handle special characters in search", async () => {
      const result = await find("remember me");
      expect(result).toBeDefined();
      expect(result.label).toBe("Remember me");
    });
  });

  describe("default export", () => {
    it("should export find as default", async () => {
      const module = await import("@api/agent/finder.js");
      expect(module.default).toBe(module.find);
    });
  });
});
