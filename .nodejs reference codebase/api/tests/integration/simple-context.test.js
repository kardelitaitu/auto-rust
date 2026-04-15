/**
 * Simple Context Integration Test
 * Basic context and session functionality
 */

import { describe, it, expect, vi } from "vitest";

vi.mock("@api/core/logger.js", () => ({
  loggerContext: {
    run: vi.fn((ctx, fn) => fn()),
    getStore: vi.fn(),
  },
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  })),
}));

vi.mock("@api/core/events.js", () => ({
  APIEvents: class {
    constructor() {
      this.listeners = new Map();
    }
    on(event, fn) {
      if (!this.listeners.has(event)) this.listeners.set(event, []);
      this.listeners.get(event).push(fn);
    }
  },
  getAvailableHooks: vi.fn(() => ({})),
  getHookDescription: vi.fn(() => ({})),
}));

vi.mock("@api/core/plugins/manager.js", () => ({
  PluginManager: class {
    constructor() {
      this.plugins = new Map();
    }
  },
}));

describe("Simple Context Integration", () => {
  it("should import context module", async () => {
    const context = await import("../../core/context.js");
    expect(context.withPage).toBeDefined();
    expect(context.clearContext).toBeDefined();
    expect(context.isSessionActive).toBeDefined();
  });

  it("should import context-state module", async () => {
    const contextState = await import("../../core/context-state.js");
    expect(contextState.getContextState).toBeDefined();
    expect(contextState.setContextState).toBeDefined();
  });

  it("should have context functions exported", async () => {
    const context = await import("../../core/context.js");
    expect(typeof context.withPage).toBe("function");
    expect(typeof context.clearContext).toBe("function");
    expect(typeof context.isSessionActive).toBe("function");
  });
});
