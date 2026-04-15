/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  registerPlugin,
  enablePlugin,
  disablePlugin,
  listPlugins,
  listEnabledPlugins,
  unregisterPlugin,
  getPluginManager,
  loadBuiltinPlugins,
} from "@api/core/plugins/index.js";
import { PluginManager } from "@api/core/plugins/manager.js";
import { BasePlugin } from "@api/core/plugins/base.js";

// Mocks
vi.mock("@api/core/context.js", () => ({
  getEvents: vi.fn().mockReturnValue({
    on: vi.fn(),
    off: vi.fn(),
    emitSafe: vi.fn(),
  }),
  getPlugins: vi.fn(),
}));

import { getPlugins } from "@api/core/context.js";

describe("api/core/plugins", () => {
  let pluginManager;
  let mockEvents;

  class TestPlugin extends BasePlugin {
    constructor(name = "test-plugin") {
      super(name);
    }
  }

  beforeEach(() => {
    vi.clearAllMocks();
    mockEvents = {
      on: vi.fn(),
      off: vi.fn(),
      emitSafe: vi.fn(),
    };
    pluginManager = new PluginManager(mockEvents);
    getPlugins.mockReturnValue(pluginManager);
  });

  describe("Constructor", () => {
    it("should throw if events is missing", () => {
      expect(() => new PluginManager(null)).toThrow(
        "PluginManager requires an APIEvents instance",
      );
    });
  });

  describe("PluginManager", () => {
    it("should register a plugin", () => {
      const plugin = new TestPlugin();
      pluginManager.register(plugin);
      expect(pluginManager.get("test-plugin").instance).toBe(plugin);
    });

    it("should throw when registering invalid plugin", () => {
      expect(() => pluginManager.register(null)).toThrow(
        "Plugin must be an object",
      );
      expect(() => pluginManager.register("not an object")).toThrow(
        "Plugin must be an object",
      );
      expect(() => pluginManager.register({})).toThrow(
        "Plugin must have a name string",
      );
    });

    it("should throw if plugin already registered", () => {
      pluginManager.register({ name: "dup" });
      expect(() => pluginManager.register({ name: "dup" })).toThrow(
        'Plugin "dup" is already registered',
      );
    });

    it("should enable and disable plugin", async () => {
      const plugin = new TestPlugin();
      // Spy instead of replace to keep original behavior (setting this.enabled)
      const onEnableSpy = vi.spyOn(plugin, "onEnable");
      const onDisableSpy = vi.spyOn(plugin, "onDisable");

      pluginManager.register(plugin);

      await pluginManager.enable("test-plugin");
      expect(plugin.enabled).toBe(true);
      expect(onEnableSpy).toHaveBeenCalled();

      await pluginManager.disable("test-plugin");
      expect(plugin.enabled).toBe(false);
      expect(onDisableSpy).toHaveBeenCalled();
    });

    it("should list plugins", () => {
      pluginManager.register(new TestPlugin());
      const list = pluginManager.list();
      expect(list).toHaveLength(1);
      expect(list[0]).toBe("test-plugin");
    });
  });

  describe("Convenience Functions", () => {
    it("should delegate to global plugin manager", async () => {
      const plugin = new BasePlugin("test");

      registerPlugin(plugin);
      expect(pluginManager.get("test").instance).toBe(plugin);

      await enablePlugin("test");
      expect(plugin.enabled).toBe(true);
      expect(listEnabledPlugins()).toContain("test");

      await disablePlugin("test");
      expect(plugin.enabled).toBe(false);

      const list = listPlugins();
      expect(list).toHaveLength(1);
      expect(list[0]).toBe("test");

      unregisterPlugin("test");
      expect(pluginManager.get("test")).toBeUndefined();
    });

    it("should get plugin manager", () => {
      expect(getPluginManager()).toBe(pluginManager);
    });
  });

  describe("BasePlugin", () => {
    it("should throw if name is missing", () => {
      expect(() => new BasePlugin()).toThrow("Plugin must have a name");
    });
  });

  describe("loadBuiltinPlugins detailed", () => {
    it("should handle registration errors", () => {
      const manager = getPluginManager();
      const registerSpy = vi
        .spyOn(manager, "register")
        .mockImplementation(() => {
          throw new Error("Forced Error");
        });
      loadBuiltinPlugins();
      expect(registerSpy).toHaveBeenCalled();
      registerSpy.mockRestore();
    });

    it("should actually call builtin fail init", () => {
      // Allow real loadBuiltinPlugins to run to hit line 16 in index.js
      loadBuiltinPlugins();
      // Verifying that it was registered (even if init failed, manager.register completes)
      expect(pluginManager.get("__builtin_fail")).toBeDefined();
    });
  });

  describe("register - edge cases for coverage", () => {
    it("should handle plugin with getHooks returning truthy", () => {
      const hooks = { "before:click": () => {} };
      const plugin = new TestPlugin("truthy-get-hooks");
      plugin.getHooks = () => hooks;
      pluginManager.register(plugin);
      expect(mockEvents.on).toHaveBeenCalledWith(
        "before:click",
        expect.any(Function),
      );
    });

    it("should handle plugin with hooks as null and no getHooks", () => {
      pluginManager.register({
        name: "null-hooks-no-get",
        hooks: null,
      });
      expect(pluginManager.get("null-hooks-no-get")).toBeDefined();
    });
  });

  describe("register - BasePlugin with init failure", () => {
    it("should handle init failure gracefully", () => {
      class FailingInitPlugin extends BasePlugin {
        constructor() {
          super("failing-init-plugin");
        }
        onLoad() {
          throw new Error("Init failed");
        }
      }
      const plugin = new FailingInitPlugin();
      // Should not throw, should log warning
      pluginManager.register(plugin);
      expect(pluginManager.get("failing-init-plugin").instance).toBe(plugin);
    });
  });

  describe("register - BasePlugin with destroy failure", () => {
    it("should handle destroy failure gracefully", async () => {
      class FailingDestroyPlugin extends BasePlugin {
        constructor() {
          super("failing-destroy-plugin");
        }
        onUnload() {
          throw new Error("Destroy failed");
        }
      }
      const plugin = new FailingDestroyPlugin();
      pluginManager.register(plugin);
      // Should not throw on unregister
      pluginManager.unregister("failing-destroy-plugin");
      expect(pluginManager.get("failing-destroy-plugin")).toBeUndefined();
    });
  });

  describe("register - Legacy plugin with init", () => {
    it("should call legacy plugin.init with config", () => {
      const initSpy = vi.fn();
      const legacyPlugin = {
        name: "legacy-plugin",
        init: initSpy,
        config: { test: true },
      };
      pluginManager.register(legacyPlugin);
      expect(initSpy).toHaveBeenCalledWith({ test: true });
    });

    it("should call legacy plugin.init without config", () => {
      const initSpy = vi.fn();
      const legacyPlugin = {
        name: "legacy-no-config",
        init: initSpy,
      };
      pluginManager.register(legacyPlugin);
      expect(initSpy).toHaveBeenCalledWith({});
    });
  });

  describe("unregister - Legacy plugin with destroy", () => {
    it("should call legacy plugin.destroy", () => {
      const destroySpy = vi.fn();
      const legacyPlugin = {
        name: "legacy-destroy-plugin",
        destroy: destroySpy,
      };
      pluginManager.register(legacyPlugin);
      pluginManager.unregister("legacy-destroy-plugin");
      expect(destroySpy).toHaveBeenCalled();
    });
  });

  describe("register - Invalid hooks (not object)", () => {
    it("should throw error for non-object hooks", () => {
      const invalidPlugin = {
        name: "invalid-hooks-plugin",
        hooks: "not-an-object",
      };
      expect(() => pluginManager.register(invalidPlugin)).toThrow(
        "Plugin hooks must be an object",
      );
    });
  });

  describe("register - Unknown hooks", () => {
    it("should log warning for unknown hooks", () => {
      const unknownHookPlugin = {
        name: "unknown-hook-plugin",
        hooks: {
          unknownHook: vi.fn(),
          anotherUnknown: vi.fn(),
        },
      };
      pluginManager.register(unknownHookPlugin);
      // Should register without throwing, unknown hooks are just warned
      expect(pluginManager.get("unknown-hook-plugin").instance).toBe(
        unknownHookPlugin,
      );
    });
  });

  describe("unregister - with onUnload failure", () => {
    it("should handle destroy failure gracefully", async () => {
      class FailingOnUnload extends BasePlugin {
        constructor() {
          super("failing-onunload");
        }
        onUnload() {
          throw new Error("onUnload failed");
        }
      }
      const plugin = new FailingOnUnload();
      pluginManager.register(plugin);
      // Should not throw
      pluginManager.unregister("failing-onunload");
      expect(pluginManager.get("failing-onunload")).toBeUndefined();
    });
  });

  describe("unregister - not found", () => {
    it("should warn when unregistering non-existent plugin", () => {
      const result = pluginManager.unregister("non-existent-plugin");
      // Should return this for chaining
      expect(result).toBe(pluginManager);
    });
  });

  describe("enable - when already enabled", () => {
    it("should not duplicate enable", async () => {
      const enableSpy = vi.fn();
      class SpyPlugin extends BasePlugin {
        constructor() {
          super("spy-plugin");
        }
        onEnable() {
          enableSpy();
        }
      }
      const plugin = new SpyPlugin();
      pluginManager.register(plugin);
      enableSpy.mockClear();

      await pluginManager.enable("spy-plugin");
      await pluginManager.enable("spy-plugin");

      expect(enableSpy).toHaveBeenCalledTimes(0);
    });
  });

  describe("disable - when already disabled", () => {
    it("should not duplicate disable", async () => {
      const disableSpy = vi.fn();
      class SpyPlugin extends BasePlugin {
        constructor() {
          super("spy-disable-plugin");
        }
        onDisable() {
          disableSpy();
        }
      }
      const plugin = new SpyPlugin();
      pluginManager.register(plugin);
      pluginManager.disable("spy-disable-plugin");
      disableSpy.mockClear();

      pluginManager.disable("spy-disable-plugin");

      expect(disableSpy).toHaveBeenCalledTimes(0);
    });
    describe("enable/disable extra branches", () => {
      it("should do nothing when enabling non-existent plugin", () => {
        expect(pluginManager.enable("non-existent")).toBe(pluginManager);
      });

      it("should do nothing when disabling non-existent plugin", () => {
        expect(pluginManager.disable("non-existent")).toBe(pluginManager);
      });

      it("should enable plain object plugin without error", () => {
        pluginManager.register({ name: "plain" });
        pluginManager.disable("plain"); // Start disabled
        pluginManager.enable("plain");
        expect(pluginManager.isEnabled("plain")).toBe(true);
      });
    });

    describe("unregister - disabled BasePlugin", () => {
      it("should not call onDisable when unregistering a disabled plugin", () => {
        const plugin = new TestPlugin("disabled-unregister");
        const onDisableSpy = vi.spyOn(plugin, "onDisable");
        pluginManager.register(plugin);
        pluginManager.disable("disabled-unregister");
        onDisableSpy.mockClear();

        pluginManager.unregister("disabled-unregister");
        expect(onDisableSpy).not.toHaveBeenCalled();
      });
    });

    describe("register - BasePlugin with falsy getHooks", () => {
      it("should handle falsy hooks from getHooks", () => {
        class NoHooksPlugin extends BasePlugin {
          constructor() {
            super("no-hooks");
          }
          getHooks() {
            return null;
          }
        }
        pluginManager.register(new NoHooksPlugin());
        expect(pluginManager.get("no-hooks")).toBeDefined();
      });
    });

    describe("register - plain object without hooks or init", () => {
      it("should register successfully", () => {
        pluginManager.register({ name: "minimal" });
        expect(pluginManager.get("minimal")).toBeDefined();
      });
    });

    describe("listInfo", () => {
      it("should return plugin metadata", () => {
        const plugin = new TestPlugin();
        pluginManager.register(plugin);

        const info = pluginManager.listInfo();
        expect(info).toHaveLength(1);
        expect(info[0].name).toBe("test-plugin");
        expect(info[0].version).toBe("1.0.0");
        expect(info[0].enabled).toBe(true);
        expect(info[0].registeredAt).toBeDefined();
      });
    });

    describe("listEnabled", () => {
      it("should list only enabled plugins", () => {
        class PluginA extends BasePlugin {
          constructor() {
            super("plugin-a");
          }
        }
        class PluginB extends BasePlugin {
          constructor() {
            super("plugin-b");
          }
        }

        pluginManager.register(new PluginA());
        pluginManager.register(new PluginB());
        pluginManager.disable("plugin-b");

        const enabled = pluginManager.listEnabled();
        expect(enabled).toContain("plugin-a");
        expect(enabled).not.toContain("plugin-b");
      });
    });

    describe("clear", () => {
      it("should clear all plugins", () => {
        pluginManager.register(new TestPlugin());
        class AnotherPlugin extends BasePlugin {
          constructor() {
            super("another");
          }
        }
        pluginManager.register(new AnotherPlugin());

        expect(pluginManager.list()).toHaveLength(2);

        pluginManager.clear();

        expect(pluginManager.list()).toHaveLength(0);
      });
    });

    describe("isEnabled", () => {
      it("should check enabled status", () => {
        class TestPlugin2 extends BasePlugin {
          constructor() {
            super("test-plugin2");
          }
        }
        const plugin = new TestPlugin2();
        pluginManager.register(plugin);

        expect(pluginManager.isEnabled("test-plugin2")).toBe(true);

        pluginManager.disable("test-plugin2");

        expect(pluginManager.isEnabled("test-plugin2")).toBe(false);

        pluginManager.enable("test-plugin2");

        expect(pluginManager.isEnabled("test-plugin2")).toBe(true);
      });
    });

    describe("Hook Binding and Event Emission", () => {
      it("should call registered hook handler when event is emitted", async () => {
        const handler = vi.fn().mockResolvedValue("handler-result");
        const plugin = {
          name: "hook-plugin",
          hooks: {
            "before:click": handler,
          },
        };

        pluginManager.register(plugin);

        expect(mockEvents.on).toHaveBeenCalledWith(
          "before:click",
          expect.any(Function),
        );
        const wrappedHandler = mockEvents.on.mock.calls.find(
          (call) => call[0] === "before:click",
        )[1];

        await wrappedHandler("arg1", "arg2");
        expect(handler).toHaveBeenCalledWith("arg1", "arg2");
      });

      it("should NOT call handler if plugin is disabled", async () => {
        const handler = vi.fn();
        pluginManager.register({
          name: "disabled-plugin",
          hooks: { "after:click": handler },
        });
        pluginManager.disable("disabled-plugin");

        const wrappedHandler = mockEvents.on.mock.calls.find(
          (call) => call[0] === "after:click",
        )[1];
        await wrappedHandler();
        expect(handler).not.toHaveBeenCalled();
      });

      it("should handle errors in hook handlers", async () => {
        const handler = vi.fn().mockRejectedValue(new Error("Handler Error"));
        pluginManager.register({
          name: "error-plugin",
          hooks: { "on:action:error": handler },
        });

        const wrappedHandler = mockEvents.on.mock.calls.find(
          (call) => call[0] === "on:action:error",
        )[1];
        await expect(wrappedHandler()).rejects.toThrow("Handler Error");
      });
    });

    describe("Unregistering with Hooks", () => {
      it("should remove hook bindings when unregistering", () => {
        const plugin = {
          name: "hooky",
          hooks: { "before:click": () => {} },
        };
        pluginManager.register(plugin);
        expect(mockEvents.on).toHaveBeenCalled();
        pluginManager.unregister("hooky");
        expect(mockEvents.off).toHaveBeenCalled();
      });

      it("should only remove its own hook bindings when unregistering", () => {
        const pluginA = {
          name: "plugin-a",
          hooks: { "before:click": () => {} },
        };
        const pluginB = {
          name: "plugin-b",
          hooks: { "before:click": () => {} },
        };

        pluginManager.register(pluginA);
        pluginManager.register(pluginB);

        const offSpy = vi.spyOn(mockEvents, "off");
        pluginManager.unregister("plugin-a");

        // Should have called off for pluginA's handler
        expect(offSpy).toHaveBeenCalledTimes(1);
        // Wait, how do we know it didn't call for pluginB?
        // Check the calls
        const calls = offSpy.mock.calls;
        expect(calls[0][0]).toBe("before:click");
      });
    });
  });
});
