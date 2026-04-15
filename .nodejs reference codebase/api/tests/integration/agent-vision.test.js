/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { injectAnnotations, removeAnnotations } from "@api/agent/vision.js";

describe("api/agent/vision.js - Integration Tests", () => {
  let mockDocument;
  let mockBody;
  let mockElement;
  let mockDefaultView;

  beforeEach(() => {
    vi.clearAllMocks();

    mockElement = {
      getBoundingClientRect: vi.fn().mockReturnValue({
        width: 100,
        height: 50,
        top: 10,
        left: 10,
      }),
      querySelector: vi.fn(),
    };

    mockBody = {
      appendChild: vi.fn(),
    };

    mockDefaultView = {
      scrollY: 0,
      scrollX: 0,
    };

    mockDocument = {
      createElement: vi.fn().mockImplementation((tag) => ({
        id: "",
        style: {},
        innerText: "",
        appendChild: vi.fn(),
        getAttribute: vi.fn(),
        setAttribute: vi.fn(),
      })),
      querySelector: vi.fn().mockReturnValue(mockElement),
      getElementById: vi.fn(),
      body: mockBody,
      defaultView: mockDefaultView,
    };
  });

  describe("injectAnnotations - Browser Integration", () => {
    const createMockElement = () => ({
      id: "",
      style: {},
      innerText: "",
      appendChild: vi.fn(),
      getAttribute: vi.fn(),
      setAttribute: vi.fn(),
    });

    it("should inject annotations into document", () => {
      const mockMap = [
        { id: 1, role: "button", label: "Submit" },
        { id: 2, role: "input", label: "Email" },
      ];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(mockDocument.createElement).toHaveBeenCalledWith("div");
      expect(mockBody.appendChild).toHaveBeenCalledWith(container);
    });

    it("should handle missing elements gracefully", () => {
      const mockMap = [{ id: 1, role: "button", label: "Submit" }];

      mockDocument.querySelector.mockReturnValue(null);

      const container = createMockElement();

      mockDocument.createElement.mockReturnValueOnce(container);

      injectAnnotations(mockDocument, mockMap);

      expect(mockBody.appendChild).toHaveBeenCalledWith(container);
    });

    it("should handle zero-dimension elements", () => {
      const mockMap = [{ id: 1, role: "button", label: "Submit" }];

      mockElement.getBoundingClientRect.mockReturnValue({
        width: 0,
        height: 0,
      });

      const container = createMockElement();

      mockDocument.createElement.mockReturnValueOnce(container);

      injectAnnotations(mockDocument, mockMap);

      expect(mockBody.appendChild).toHaveBeenCalledWith(container);
    });

    it("should apply all styling to container", () => {
      const mockMap = [{ id: 1, role: "button", label: "Test" }];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(container.style.position).toBe("absolute");
      expect(container.style.top).toBe("0");
      expect(container.style.left).toBe("0");
      expect(container.style.width).toBe("100%");
      expect(container.style.height).toBe("100%");
      expect(container.style.zIndex).toBe("999999");
      expect(container.style.pointerEvents).toBe("none");
    });

    it("should apply styling to box elements", () => {
      const mockMap = [{ id: 1, role: "button", label: "Test" }];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(box.style.position).toBe("absolute");
      expect(box.style.border).toBe("2px solid red");
      expect(box.style.boxSizing).toBe("border-box");
    });

    it("should apply styling to label elements", () => {
      const mockMap = [{ id: 1, role: "button", label: "Test" }];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(label.style.backgroundColor).toBe("red");
      expect(label.style.color).toBe("white");
      expect(label.style.fontSize).toBe("12px");
      expect(label.style.fontWeight).toBe("bold");
    });

    it("should set label innerText to element id", () => {
      const mockMap = [{ id: 42, role: "button", label: "Test" }];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(label.innerText).toBe(42);
    });

    it("should calculate position with scroll offsets", () => {
      mockDefaultView.scrollY = 100;
      mockDefaultView.scrollX = 50;

      const mockMap = [{ id: 1, role: "button", label: "Test" }];

      const container = createMockElement();
      const box = createMockElement();
      const label = createMockElement();

      mockDocument.createElement
        .mockReturnValueOnce(container)
        .mockReturnValueOnce(box)
        .mockReturnValueOnce(label);

      injectAnnotations(mockDocument, mockMap);

      expect(box.style.top).toBe("110px"); // rect.top (10) + scrollY (100)
      expect(box.style.left).toBe("60px"); // rect.left (10) + scrollX (50)
    });
  });

  describe("removeAnnotations - Browser Integration", () => {
    it("should remove annotation container when exists", () => {
      const mockContainer = { remove: vi.fn() };
      mockDocument.getElementById.mockReturnValue(mockContainer);

      removeAnnotations(mockDocument);

      expect(mockDocument.getElementById).toHaveBeenCalledWith(
        "agent-vision-annotations",
      );
      expect(mockContainer.remove).toHaveBeenCalled();
    });

    it("should do nothing when container does not exist", () => {
      mockDocument.getElementById.mockReturnValue(null);

      expect(() => removeAnnotations(mockDocument)).not.toThrow();
      expect(mockDocument.getElementById).toHaveBeenCalledWith(
        "agent-vision-annotations",
      );
    });
  });
});
