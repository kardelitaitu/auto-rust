/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

const mockManager = {
  register: vi.fn(),
  unregister: vi.fn(),
  enable: vi.fn(),
  disable: vi.fn(),
  list: vi.fn(),
  listEnabled: vi.fn(),
};

vi.mock("@api/core/context.js", () => ({
  getPlugins: () => mockManager,
}));

describe("api/core/plugins/index.js", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads builtin plugins and continues when one plugin init fails", async () => {
    const { loadBuiltinPlugins } = await import("@api/core/plugins/index.js");

    mockManager.register.mockImplementation((plugin) => {
      if (plugin.name === "__builtin_fail") {
        throw new Error("boom");
      }
    });

    loadBuiltinPlugins();

    expect(mockManager.register).toHaveBeenCalledTimes(2);
    expect(mockManager.register).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ name: "__builtin_coverage_dummy" }),
    );
    expect(mockManager.register).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({ name: "__builtin_fail" }),
    );
  });

  it("forwards plugin manager operations through the shared context", async () => {
    const {
      getPluginManager,
      registerPlugin,
      unregisterPlugin,
      enablePlugin,
      disablePlugin,
      listPlugins,
      listEnabledPlugins,
      BasePlugin,
      default: pluginIndex,
    } = await import("@api/core/plugins/index.js");

    const plugin = { name: "custom-plugin" };

    mockManager.list.mockReturnValue(["custom-plugin"]);
    mockManager.listEnabled.mockReturnValue(["custom-plugin"]);
    mockManager.register.mockReturnValue("registered");
    mockManager.unregister.mockReturnValue("unregistered");
    mockManager.enable.mockReturnValue("enabled");
    mockManager.disable.mockReturnValue("disabled");

    expect(getPluginManager()).toBe(mockManager);
    expect(registerPlugin(plugin)).toBe("registered");
    expect(unregisterPlugin("custom-plugin")).toBe("unregistered");
    expect(enablePlugin("custom-plugin")).toBe("enabled");
    expect(disablePlugin("custom-plugin")).toBe("disabled");
    expect(listPlugins()).toEqual(["custom-plugin"]);
    expect(listEnabledPlugins()).toEqual(["custom-plugin"]);
    expect(pluginIndex.loadBuiltinPlugins).toBeTypeOf("function");
    expect(BasePlugin).toBeDefined();
  });
});
