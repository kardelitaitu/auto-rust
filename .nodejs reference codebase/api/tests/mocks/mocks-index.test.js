/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Unit tests for api/tests/mocks/index.js
 */

import { describe, it, expect, vi } from "vitest";

// Mock test-helpers to avoid dependency issues
vi.mock("@api/tests/utils/test-helpers.js", () => ({
  createSilentLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
  createMockPage: vi.fn(() => ({
    goto: vi.fn(),
    url: vi.fn(() => "https://test.com"),
  })),
  createMockLocator: vi.fn(() => ({
    click: vi.fn(),
    textContent: vi.fn(),
  })),
}));

describe("api/tests/mocks/index.js", () => {
  describe("core mocks", () => {
    it("should export loggerMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.loggerMock).toBeDefined();
      expect(mocks.loggerMock.createLogger).toBeDefined();
      expect(mocks.loggerMock.loggerContext).toBeDefined();
    });

    it("should export contextMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.contextMock).toBeDefined();
      expect(mocks.contextMock.withPage).toBeDefined();
      expect(mocks.contextMock.getContext).toBeDefined();
      expect(mocks.contextMock.setContext).toBeDefined();
    });

    it("should export contextStateMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.contextStateMock).toBeDefined();
      expect(mocks.contextStateMock.getStateAgentElementMap).toBeDefined();
      expect(mocks.contextStateMock.getState).toBeDefined();
    });

    it("should export orchestratorMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.orchestratorMock).toBeDefined();
      expect(mocks.orchestratorMock.default).toBeDefined();
    });

    it("should export sessionManagerMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.sessionManagerMock).toBeDefined();
      expect(mocks.sessionManagerMock.default).toBeDefined();
    });

    it("should export discoveryMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.discoveryMock).toBeDefined();
      expect(mocks.discoveryMock.default).toBeDefined();
    });

    it("should export automatorMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.automatorMock).toBeDefined();
      expect(mocks.automatorMock.default).toBeDefined();
    });
  });

  describe("utility mocks", () => {
    it("should export configLoaderMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.configLoaderMock).toBeDefined();
      expect(mocks.configLoaderMock.getSettings).toBeDefined();
      expect(mocks.configLoaderMock.ConfigLoader).toBeDefined();
    });

    it("should export metricsMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.metricsMock).toBeDefined();
      expect(mocks.metricsMock.default).toBeDefined();
    });

    it("should export mathMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.mathMock).toBeDefined();
      expect(mocks.mathMock.mathUtils).toBeDefined();
    });

    it("should export ghostCursorMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.ghostCursorMock).toBeDefined();
      expect(mocks.ghostCursorMock.GhostCursor).toBeDefined();
    });

    it("should export validatorMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.validatorMock).toBeDefined();
      expect(mocks.validatorMock.validateTaskExecution).toBeDefined();
    });
  });

  describe("agent mocks", () => {
    it("should export llmClientMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.llmClientMock).toBeDefined();
      expect(mocks.llmClientMock.llmClient).toBeDefined();
    });

    it("should export actionEngineMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.actionEngineMock).toBeDefined();
      expect(mocks.actionEngineMock.actionEngine).toBeDefined();
    });

    it("should export visionMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.visionMock).toBeDefined();
      expect(mocks.visionMock.screenshot).toBeDefined();
    });
  });

  describe("interaction mocks", () => {
    it("should export waitMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.waitMock).toBeDefined();
      expect(mocks.waitMock.wait).toBeDefined();
      expect(mocks.waitMock.waitFor).toBeDefined();
    });

    it("should export timingMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.timingMock).toBeDefined();
      expect(mocks.timingMock.think).toBeDefined();
      expect(mocks.timingMock.delay).toBeDefined();
    });
  });

  describe("factory functions", () => {
    it("should export createModuleMock", async () => {
      const mocks = await import("./index.js");

      expect(mocks.createModuleMock).toBeDefined();
      expect(typeof mocks.createModuleMock).toBe("function");
    });

    it("should export applyMocks", async () => {
      const mocks = await import("./index.js");

      expect(mocks.applyMocks).toBeDefined();
      expect(typeof mocks.applyMocks).toBe("function");
    });

    it("createModuleMock should return factory function", async () => {
      const mocks = await import("./index.js");

      const mockExports = { foo: "bar" };
      const factory = mocks.createModuleMock("test/path", mockExports);

      expect(factory()).toBe(mockExports);
    });
  });

  describe("preset functions", () => {
    it("should export coreMocks", async () => {
      const mocks = await import("./index.js");

      expect(mocks.coreMocks).toBeDefined();
      expect(typeof mocks.coreMocks).toBe("function");
    });

    it("should export utilMocks", async () => {
      const mocks = await import("./index.js");

      expect(mocks.utilMocks).toBeDefined();
      expect(typeof mocks.utilMocks).toBe("function");
    });

    it("should export allMocks", async () => {
      const mocks = await import("./index.js");

      expect(mocks.allMocks).toBeDefined();
      expect(typeof mocks.allMocks).toBe("function");
    });
  });

  describe("default export", () => {
    it("should export default object with all mocks", async () => {
      const mocks = await import("./index.js");

      expect(mocks.default).toBeDefined();
      expect(mocks.default.loggerMock).toBeDefined();
      expect(mocks.default.contextMock).toBeDefined();
      expect(mocks.default.orchestratorMock).toBeDefined();
      expect(mocks.default.createModuleMock).toBeDefined();
      expect(mocks.default.coreMocks).toBeDefined();
    });
  });
});
