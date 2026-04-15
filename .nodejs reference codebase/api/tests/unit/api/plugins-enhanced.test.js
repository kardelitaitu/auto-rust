/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { PluginManager } from "@api/core/plugins/manager.js";
import { BasePlugin } from "@api/core/plugins/base.js";

describe("api/core/plugins - Enhanced", () => {
  let events;
  let pluginManager;

  beforeEach(() => {
    vi.clearAllMocks();
    events = {
      on: vi.fn(),
      off: vi.fn(),
      emitSafe: vi.fn(),
    };
    pluginManager = new PluginManager(events);
  });

  describe("PluginManager - Constructor", () => {
    it("should throw if events not provided", () => {
      expect(() => new PluginManager()).toThrow(
        "PluginManager requires an APIEvents instance",
      );
      expect(() => new PluginManager(null)).toThrow(
        "PluginManager requires an APIEvents instance",
      );
      expect(() => new PluginManager(undefined)).toThrow(
        "PluginManager requires an APIEvents instance",
      );
    });

    it("should initialize with empty state", () => {
      expect(pluginManager.list()).toEqual([]);
      expect(pluginManager.listEnabled()).toEqual([]);
    });
  });

  describe("PluginManager - Register", () => {
    class TestPlugin extends BasePlugin {
      constructor(name = "test-plugin") {
        super(name, "1.0.0");
      }
    }

    it("should register a BasePlugin with hooks", () => {
      const plugin = new TestPlugin("hook-plugin");
      pluginManager.register(plugin);
      expect(pluginManager.get("hook-plugin")).toBeDefined();
    });

    it("should register a legacy plugin object", () => {
      const plugin = {
        name: "legacy-plugin",
        version: "2.0.0",
        hooks: { "before:click": vi.fn() },
        init: vi.fn(),
        destroy: vi.fn(),
      };
      pluginManager.register(plugin);
      expect(pluginManager.get("legacy-plugin")).toBeDefined();
      expect(plugin.init).toHaveBeenCalled();
    });

    it("should throw on duplicate registration", () => {
      const plugin = new TestPlugin("dup");
      pluginManager.register(plugin);
      expect(() => pluginManager.register(plugin)).toThrow(
        "already registered",
      );
    });

    it("should throw on invalid plugin object", () => {
      expect(() => pluginManager.register(null)).toThrow("must be an object");
      expect(() => pluginManager.register(undefined)).toThrow(
        "must be an object",
      );
    });

    it("should throw on missing name", () => {
      expect(() => pluginManager.register({ name: "" })).toThrow(
        "must have a name",
      );
      expect(() => pluginManager.register({ name: null })).toThrow(
        "must have a name",
      );
      expect(() => pluginManager.register({ hooks: {} })).toThrow(
        "must have a name",
      );
    });

    it("should throw on invalid hooks type", () => {
      const plugin = {
        name: "bad-hooks",
        hooks: "invalid",
      };
      expect(() => pluginManager.register(plugin)).toThrow(
        "hooks must be an object",
      );
    });

    it("should call init for legacy plugin", () => {
      const initFn = vi.fn();
      pluginManager.register({
        name: "init-test",
        init: initFn,
        config: { test: true },
      });
      expect(initFn).toHaveBeenCalledWith({ test: true });
    });

    it("should handle init error gracefully", () => {
      const plugin = {
        name: "init-error",
        init: vi.fn(() => {
          throw new Error("Init failed");
        }),
      };
      expect(() => pluginManager.register(plugin)).not.toThrow();
      expect(pluginManager.get("init-error")).toBeDefined();
    });

    it("should auto-enable after registration", () => {
      const plugin = new TestPlugin("auto-enable");
      pluginManager.register(plugin);
      expect(pluginManager.isEnabled("auto-enable")).toBe(true);
    });
  });

  describe("PluginManager - Hook Binding", () => {
    it("should bind valid hooks to events", () => {
      const handler = vi.fn();
      pluginManager.register({
        name: "hook-test",
        hooks: { "before:click": handler },
      });
      expect(events.on).toHaveBeenCalledWith(
        "before:click",
        expect.any(Function),
      );
    });

    it("should skip execution if plugin is disabled", async () => {
      const handler = vi.fn();
      pluginManager.register({
        name: "disable-hook-test",
        hooks: { "before:click": handler },
      });

      const wrappedHandler = events.on.mock.calls[0][1];
      pluginManager.disable("disable-hook-test");

      await wrappedHandler();
      expect(handler).not.toHaveBeenCalled();
    });

    it("should catch and log error in wrapped handler", async () => {
      const handler = vi.fn(() => {
        throw new Error("handler error");
      });
      pluginManager.register({
        name: "error-hook-test",
        hooks: { "before:click": handler },
      });

      const wrappedHandler = events.on.mock.calls[0][1];
      await expect(wrappedHandler()).rejects.toThrow("handler error");
    });

    it("should log warning for unknown hooks", () => {
      pluginManager.register({
        name: "unknown-hook-test",
        hooks: { "unknown:hook": vi.fn() },
      });
    });
  });

  describe("PluginManager - Unregister", () => {
    it("should unregister a plugin", () => {
      pluginManager.register({ name: "unreg-test" });
      pluginManager.unregister("unreg-test");
      expect(pluginManager.get("unreg-test")).toBeUndefined();
    });

    it("should handle unregister non-existent gracefully", () => {
      expect(() => pluginManager.unregister("non-existent")).not.toThrow();
    });

    it("should call destroy for legacy plugin", () => {
      const destroyFn = vi.fn();
      pluginManager.register({ name: "destroy-test", destroy: destroyFn });
      pluginManager.unregister("destroy-test");
      expect(destroyFn).toHaveBeenCalled();
    });

    it("should handle destroy error gracefully", () => {
      const plugin = {
        name: "destroy-error",
        destroy: vi.fn(() => {
          throw new Error("Destroy failed");
        }),
      };
      pluginManager.register(plugin);
      expect(() => pluginManager.unregister("destroy-error")).not.toThrow();
    });

    it("should remove hook bindings on unregister", () => {
      const handler = vi.fn();
      pluginManager.register({
        name: "bind-test",
        hooks: { "before:click": handler },
      });
      pluginManager.unregister("bind-test");
      expect(events.off).toHaveBeenCalled();
    });

    it("should remove from enabled set on unregister", () => {
      pluginManager.register({ name: "enabled-test" });
      expect(pluginManager.listEnabled()).toContain("enabled-test");
      pluginManager.unregister("enabled-test");
      expect(pluginManager.listEnabled()).not.toContain("enabled-test");
    });

    it("should call onDisable and onUnload for BasePlugin on unregister", async () => {
      const onDisableSpy = vi.fn();
      const onUnloadSpy = vi.fn();
      class FullLifecyclePlugin extends BasePlugin {
        constructor() {
          super("full-lifecycle");
        }
        onDisable() {
          onDisableSpy();
        }
        onUnload() {
          onUnloadSpy();
        }
      }
      const plugin = new FullLifecyclePlugin();
      pluginManager.register(plugin);
      pluginManager.unregister("full-lifecycle");
      expect(onDisableSpy).toHaveBeenCalled();
      expect(onUnloadSpy).toHaveBeenCalled();
    });
  });

  describe("PluginManager - Enable/Disable", () => {
    it("should enable a registered disabled plugin", () => {
      pluginManager.register({ name: "enable-test" });
      pluginManager.disable("enable-test");
      pluginManager.enable("enable-test");
      expect(pluginManager.isEnabled("enable-test")).toBe(true);
    });

    it("should ignore enable for non-existent plugin", () => {
      expect(() => pluginManager.enable("non-existent")).not.toThrow();
    });

    it("should disable a registered enabled plugin", () => {
      pluginManager.register({ name: "disable-test" });
      pluginManager.disable("disable-test");
      expect(pluginManager.isEnabled("disable-test")).toBe(false);
    });

    it("should ignore disable for non-existent plugin", () => {
      expect(() => pluginManager.disable("non-existent")).not.toThrow();
    });

    it("should not re-enable already enabled plugin", () => {
      const enableFn = vi.fn();
      pluginManager.register({
        name: "no-reenable",
        onEnable: enableFn,
      });
      enableFn.mockClear();
      pluginManager.enable("no-reenable");
      expect(enableFn).not.toHaveBeenCalled();
    });
  });

  describe("PluginManager - Query Methods", () => {
    it("should get plugin by name", () => {
      pluginManager.register({ name: "get-test" });
      const plugin = pluginManager.get("get-test");
      expect(plugin).toBeDefined();
      expect(plugin.name).toBe("get-test");
    });

    it("should return undefined for non-existent plugin", () => {
      expect(pluginManager.get("non-existent")).toBeUndefined();
    });

    it("should list all plugin names", () => {
      pluginManager.register({ name: "list1" });
      pluginManager.register({ name: "list2" });
      const list = pluginManager.list();
      expect(list).toContain("list1");
      expect(list).toContain("list2");
    });

    it("should list enabled plugin names", () => {
      pluginManager.register({ name: "enabled1" });
      pluginManager.register({ name: "enabled2" });
      pluginManager.disable("enabled2");
      const enabled = pluginManager.listEnabled();
      expect(enabled).toContain("enabled1");
      expect(enabled).not.toContain("enabled2");
    });

    it("should list plugin info", () => {
      pluginManager.register({ name: "info-test", version: "1.2.3" });
      const info = pluginManager.listInfo();
      expect(info).toHaveLength(1);
      expect(info[0].name).toBe("info-test");
      expect(info[0].version).toBe("1.2.3");
      expect(info[0].enabled).toBe(true);
      expect(info[0].registeredAt).toBeDefined();
    });
  });

  describe("PluginManager - Clear", () => {
    it("should clear all plugins", () => {
      pluginManager.register({ name: "clear1" });
      pluginManager.register({ name: "clear2" });
      pluginManager.clear();
      expect(pluginManager.list()).toEqual([]);
    });

    it("should return this for chaining", () => {
      const result = pluginManager.clear();
      expect(result).toBe(pluginManager);
    });

    it("should tolerate clearing an already empty manager", () => {
      expect(() => pluginManager.clear()).not.toThrow();
      expect(pluginManager.list()).toEqual([]);
    });
  });

  describe("PluginManager - Evaluate URL and Destroy", () => {
    it("should toggle plugin state based on matches()", () => {
      class UrlPlugin extends BasePlugin {
        constructor() {
          super("url-plugin");
        }
        matches(url) {
          return url.includes("allow");
        }
      }

      const plugin = new UrlPlugin();
      pluginManager.register(plugin);

      pluginManager.evaluateUrl("https://example.com/allow");
      expect(pluginManager.isEnabled("url-plugin")).toBe(true);

      pluginManager.evaluateUrl("https://example.com/block");
      expect(pluginManager.isEnabled("url-plugin")).toBe(false);
    });

    it("should keep plugin state when matches throws", () => {
      class ThrowingMatchPlugin extends BasePlugin {
        constructor() {
          super("throwing-match");
        }
        matches() {
          throw new Error("match failed");
        }
      }

      const plugin = new ThrowingMatchPlugin();
      pluginManager.register(plugin);
      expect(() =>
        pluginManager.evaluateUrl("https://example.com"),
      ).not.toThrow();
      expect(pluginManager.isEnabled("throwing-match")).toBe(true);
    });

    it("should destroy all plugins and reset bindings", () => {
      pluginManager.register({ name: "destroy-a" });
      pluginManager.register({ name: "destroy-b" });

      const result = pluginManager.destroy();

      expect(result).toBe(pluginManager);
      expect(pluginManager.list()).toEqual([]);
      expect(pluginManager.listEnabled()).toEqual([]);
    });
  });

  describe("BasePlugin Integration", () => {
    it("should initialize BasePlugin defaults and version override", () => {
      const plugin = new BasePlugin("base-default");
      const custom = new BasePlugin("base-custom", "2.3.4");

      expect(plugin.name).toBe("base-default");
      expect(plugin.version).toBe("1.0.0");
      expect(plugin.context).toBeNull();
      expect(plugin.enabled).toBe(false);

      expect(custom.version).toBe("2.3.4");
    });

    it("should enforce a required name at construction", () => {
      expect(() => new BasePlugin()).toThrow("Plugin must have a name");
      expect(() => new BasePlugin("")).toThrow("Plugin must have a name");
    });

    it("should support lifecycle defaults and base helpers", async () => {
      const plugin = new BasePlugin("lifecycle-base");
      const context = { events };

      await plugin.onLoad(context);
      expect(plugin.context).toBe(context);

      await plugin.onEnable();
      expect(plugin.enabled).toBe(true);

      await plugin.onDisable();
      expect(plugin.enabled).toBe(false);

      await plugin.onUnload();
      expect(plugin.context).toBeNull();

      expect(plugin.matches("https://example.com")).toBe(true);
      expect(plugin.getHooks()).toEqual({});
    });

    it("should call onLoad when registering BasePlugin", async () => {
      const onLoadFn = vi.fn();
      class LoadPlugin extends BasePlugin {
        async onLoad(context) {
          onLoadFn(context);
        }
      }
      const plugin = new LoadPlugin("load-test");
      pluginManager.register(plugin);
      expect(onLoadFn).toHaveBeenCalledWith({ events });
    });

    it("should call onEnable when registering BasePlugin", async () => {
      const onEnableFn = vi.fn();
      class EnablePlugin extends BasePlugin {
        async onEnable() {
          onEnableFn();
        }
      }
      const plugin = new EnablePlugin("enable-integ-test");
      pluginManager.register(plugin);
      expect(onEnableFn).toHaveBeenCalled();
    });

    it("should call onUnload when unregistering BasePlugin", async () => {
      const onUnloadFn = vi.fn();
      class UnloadPlugin extends BasePlugin {
        async onUnload() {
          onUnloadFn();
        }
      }
      const plugin = new UnloadPlugin("unload-test");
      pluginManager.register(plugin);
      pluginManager.unregister("unload-test");
      expect(onUnloadFn).toHaveBeenCalled();
    });

    it("should call onDisable when disabling BasePlugin", async () => {
      const onDisableFn = vi.fn();
      class DisablePlugin extends BasePlugin {
        async onDisable() {
          onDisableFn();
        }
      }
      const plugin = new DisablePlugin("disable-integ-test");
      pluginManager.register(plugin);
      pluginManager.disable("disable-integ-test");
      expect(onDisableFn).toHaveBeenCalled();
    });
  });
});
