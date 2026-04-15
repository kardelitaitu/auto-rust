/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, beforeEach } from "vitest";
import { BasePlugin } from "@api/core/plugins/base.js";

describe("api/core/plugins/base.js", () => {
  describe("BasePlugin", () => {
    it("should create a plugin with name and version", () => {
      const plugin = new BasePlugin("test-plugin", "2.0.0");
      expect(plugin.name).toBe("test-plugin");
      expect(plugin.version).toBe("2.0.0");
      expect(plugin.enabled).toBe(false);
      expect(plugin.context).toBe(null);
    });

    it("should use default version if not provided", () => {
      const plugin = new BasePlugin("test-plugin");
      expect(plugin.version).toBe("1.0.0");
    });

    it("should throw error if name is missing", () => {
      expect(() => new BasePlugin()).toThrow("Plugin must have a name");
      expect(() => new BasePlugin("")).toThrow("Plugin must have a name");
      expect(() => new BasePlugin(null)).toThrow("Plugin must have a name");
    });

    describe("lifecycle methods", () => {
      let plugin;

      beforeEach(() => {
        plugin = new BasePlugin("lifecycle-test", "1.0.0");
      });

      it("should set context on onLoad", async () => {
        const mockContext = { api: "context" };
        await plugin.onLoad(mockContext);
        expect(plugin.context).toBe(mockContext);
      });

      it("should set enabled to true on onEnable", async () => {
        expect(plugin.enabled).toBe(false);
        await plugin.onEnable();
        expect(plugin.enabled).toBe(true);
      });

      it("should set enabled to false on onDisable", async () => {
        plugin.enabled = true;
        await plugin.onDisable();
        expect(plugin.enabled).toBe(false);
      });

      it("should clear context on onUnload", async () => {
        plugin.context = { some: "context" };
        await plugin.onUnload();
        expect(plugin.context).toBe(null);
      });

      it("should return empty hooks by default", () => {
        const hooks = plugin.getHooks();
        expect(hooks).toEqual({});
      });
    });

    describe("custom hooks", () => {
      class CustomPlugin extends BasePlugin {
        getHooks() {
          return {
            "before:click": () => {},
            "after:click": () => {},
          };
        }
      }

      it("should allow overriding getHooks", () => {
        const customPlugin = new CustomPlugin("custom", "1.0.0");
        const hooks = customPlugin.getHooks();
        expect(hooks).toHaveProperty("before:click");
        expect(hooks).toHaveProperty("after:click");
      });
    });
  });
});
