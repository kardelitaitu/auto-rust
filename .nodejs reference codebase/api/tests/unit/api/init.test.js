/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@api/behaviors/persona.js", () => ({
  setPersona: vi.fn(),
  getPersona: vi.fn().mockReturnValue({}),
  getPersonaName: vi.fn().mockReturnValue("casual"),
  listPersonas: vi.fn().mockReturnValue([]),
  PERSONAS: { casual: {}, professional: {} },
}));

vi.mock("@api/utils/patch.js", () => ({
  apply: vi.fn().mockResolvedValue(true),
  check: vi.fn().mockResolvedValue({ status: "ok" }),
  stripCDPMarkers: vi.fn(),
}));

vi.mock("@api/utils/sensors.js", () => ({
  injectSensors: vi.fn().mockResolvedValue(true),
}));

vi.mock("@api/interactions/cursor.js", () => ({
  move: vi.fn(),
  up: vi.fn(),
  down: vi.fn(),
  setPathStyle: vi.fn(),
  getPathStyle: vi.fn(),
  startFidgeting: vi.fn(),
  stopFidgeting: vi.fn(),
}));

vi.mock("@api/core/plugins/index.js", () => ({
  loadBuiltinPlugins: vi.fn(),
  registerPlugin: vi.fn(),
  unregisterPlugin: vi.fn(),
  enablePlugin: vi.fn(),
  disablePlugin: vi.fn(),
  listPlugins: vi.fn().mockReturnValue([]),
  listEnabledPlugins: vi.fn().mockReturnValue([]),
  getPluginManager: vi.fn(),
}));

vi.mock("@api/utils/fingerprint.js", () => ({
  fingerprintManager: {
    matchUserAgent: vi.fn((ua) => {
      if (!ua) throw new Error("UA missing");
      return {};
    }),
    getRandom: vi.fn().mockReturnValue({}),
  },
}));

vi.mock("@api/utils/browserPatch.js", () => ({
  applyHumanizationPatch: vi.fn().mockResolvedValue(true),
}));

import { initPage, diagnosePage, clearLiteMode } from "@api/core/init.js";
import * as persona from "@api/behaviors/persona.js";
import * as patch from "@api/utils/patch.js";
import * as sensors from "@api/utils/sensors.js";
import * as cursor from "@api/interactions/cursor.js";
import * as plugins from "@api/core/plugins/index.js";
import { fingerprintManager } from "@api/utils/fingerprint.js";
import { applyHumanizationPatch } from "@api/utils/browserPatch.js";
import * as context from "@api/core/context.js";

describe("api/core/init.js", () => {
  let mockPage;

  beforeEach(() => {
    vi.clearAllMocks();
    mockPage = {
      isClosed: vi.fn().mockReturnValue(false),
      url: vi.fn().mockReturnValue("https://example.com"),
      emulateMedia: vi.fn().mockResolvedValue(true),
      route: vi.fn().mockResolvedValue(undefined),
      setMuted: vi.fn().mockResolvedValue(undefined),
      unroute: vi.fn().mockResolvedValue(undefined),
      evaluate: vi.fn().mockResolvedValue("user-agent"),
      addInitScript: vi.fn().mockResolvedValue(true),
      context: vi.fn().mockReturnValue({
        browser: vi.fn().mockReturnValue({
          isConnected: vi.fn().mockReturnValue(true),
        }),
        on: vi.fn(),
      }),
      on: vi.fn(),
    };

    // Mock getEvents for safeEmitError
    vi.spyOn(context, "getEvents").mockReturnValue({
      emitSafe: vi.fn(),
    });
  });

  describe("initPage", () => {
    it("should handle null page", async () => {
      const result = await initPage(null);
      expect(result).toBeUndefined();
    });

    it("should initialize page with default options", async () => {
      await initPage(mockPage);

      expect(plugins.loadBuiltinPlugins).toHaveBeenCalled();
      expect(patch.apply).toHaveBeenCalled();
      expect(applyHumanizationPatch).toHaveBeenCalledWith(mockPage, undefined);
      expect(sensors.injectSensors).toHaveBeenCalled();
      expect(cursor.startFidgeting).toHaveBeenCalled();
    });

    it("should emulate color scheme if provided", async () => {
      await initPage(mockPage, { colorScheme: "dark" });
      expect(mockPage.emulateMedia).toHaveBeenCalledWith({
        colorScheme: "dark",
      });
    });

    it("should set persona if provided", async () => {
      await initPage(mockPage, { persona: "professional" });
      expect(persona.setPersona).toHaveBeenCalledWith("professional", {});
    });

    it("should apply persona overrides", async () => {
      await initPage(mockPage, { personaOverrides: { extra: true } });
      expect(persona.setPersona).toHaveBeenCalledWith("casual", {
        extra: true,
      });
    });

    it("should skip patches if requested", async () => {
      await initPage(mockPage, { patch: false, humanizationPatch: false });
      expect(patch.apply).not.toHaveBeenCalled();
      expect(applyHumanizationPatch).not.toHaveBeenCalled();
    });

    it("should not re-patch already patched page", async () => {
      await initPage(mockPage);
      vi.clearAllMocks();
      await initPage(mockPage);
      expect(patch.apply).not.toHaveBeenCalled();
    });

    it("should install context hooks if autoInitNewPages is true", async () => {
      await initPage(mockPage, { autoInitNewPages: true });
      const browserContext = mockPage.context();
      expect(browserContext.on).toHaveBeenCalledWith(
        "page",
        expect.any(Function),
      );
      expect(mockPage.on).toHaveBeenCalledWith("popup", expect.any(Function));
    });

    it("should use provided fingerprint", async () => {
      const fp = { ua: "test-ua" };
      await initPage(mockPage, { fingerprint: fp });
      expect(patch.apply).toHaveBeenCalledWith(fp);
    });

    it("should match fingerprint if not provided", async () => {
      await initPage(mockPage, { patch: true });
      expect(fingerprintManager.matchUserAgent).toHaveBeenCalledWith(
        "user-agent",
      );
    });

    it("should fallback to random fingerprint on UA error", async () => {
      mockPage.evaluate.mockRejectedValue(new Error("fail"));
      await initPage(mockPage, { patch: true });
      expect(fingerprintManager.getRandom).toHaveBeenCalled();
    });

    it("should handle already hooked context", async () => {
      const browserContext = mockPage.context();
      await initPage(mockPage, { autoInitNewPages: true });
      vi.clearAllMocks();
      await initPage(mockPage, { autoInitNewPages: true });
      expect(browserContext.on).not.toHaveBeenCalled();
    });

    it("should handle missing browser context", async () => {
      mockPage.context.mockReturnValue(null);
      await initPage(mockPage, { autoInitNewPages: true });
      // Should not throw, just return
      expect(plugins.loadBuiltinPlugins).toHaveBeenCalled();
    });

    it("should handle auto-initialization of new pages", async () => {
      let pageHandler;
      mockPage.context().on.mockImplementation((event, handler) => {
        if (event === "page") pageHandler = handler;
      });

      await initPage(mockPage, { autoInitNewPages: true });

      const newPage = { ...mockPage, context: vi.fn(), on: vi.fn() };
      await pageHandler(newPage);

      // Should have called initPage for the new page
      expect(plugins.loadBuiltinPlugins).toHaveBeenCalledTimes(2);
    });

    it("should handle auto-initialization of popups", async () => {
      let popupHandler;
      mockPage.on.mockImplementation((event, handler) => {
        if (event === "popup") popupHandler = handler;
      });

      await initPage(mockPage, { autoInitNewPages: true });

      const popupPage = { ...mockPage, context: vi.fn(), on: vi.fn() };
      await popupHandler(popupPage);

      expect(plugins.loadBuiltinPlugins).toHaveBeenCalledTimes(2);
    });

    it("should handle error when installing context hooks", async () => {
      mockPage.context.mockImplementation(() => {
        throw new Error("Context error");
      });
      await initPage(mockPage, { autoInitNewPages: true });
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "installContextHooks" }),
      );
    });

    it("should handle error in emulateMedia", async () => {
      mockPage.emulateMedia.mockRejectedValue(new Error("emulate fail"));
      await initPage(mockPage, { colorScheme: "dark" });
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "emulateMedia" }),
      );
    });

    it("should handle missing page.on in installContextHooks", async () => {
      mockPage.on = vi.fn();
      await initPage(mockPage, { autoInitNewPages: true });
      expect(mockPage.on).toHaveBeenCalledWith("close", expect.any(Function));
    });

    it("should handle error in auto-init-page", async () => {
      let pageHandler;
      mockPage.context().on.mockImplementation((event, handler) => {
        if (event === "page") pageHandler = handler;
      });
      await initPage(mockPage, { autoInitNewPages: true });

      // Force initPage to fail for the new page by giving it no context
      const newPage = null;
      // Wait, if newPage is null, initPage returns immediately.
      // Let's force an error by mocking withPage to throw.
      vi.spyOn(context, "withPage").mockImplementationOnce(() => {
        throw new Error("withPage fail");
      });

      await pageHandler({});
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "auto-init-page" }),
      );
    });

    it("should handle error in auto-init-popup", async () => {
      let popupHandler;
      mockPage.on.mockImplementation((event, handler) => {
        if (event === "popup") popupHandler = handler;
      });
      await initPage(mockPage, { autoInitNewPages: true });

      vi.spyOn(context, "withPage").mockImplementationOnce(() => {
        throw new Error("withPage fail");
      });

      await popupHandler({});
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "auto-init-popup" }),
      );
    });

    it("should skip humanization patch if requested", async () => {
      await initPage(mockPage, { humanizationPatch: false });
      expect(applyHumanizationPatch).not.toHaveBeenCalled();
      expect(sensors.injectSensors).not.toHaveBeenCalled();
      expect(cursor.startFidgeting).not.toHaveBeenCalled();
    });

    it("should skip patch but apply humanization if requested", async () => {
      await initPage(mockPage, { patch: false, humanizationPatch: true });
      expect(patch.apply).not.toHaveBeenCalled();
      expect(applyHumanizationPatch).toHaveBeenCalled();
    });

    it("should skip context hooks if autoInitNewPages is false", async () => {
      await initPage(mockPage, { autoInitNewPages: false });
      expect(mockPage.context().on).not.toHaveBeenCalled();
    });

    it("should enable lite mode route blocking", async () => {
      const logger = { info: vi.fn() };
      await initPage(mockPage, { lite: true, logger });

      expect(mockPage.route).toHaveBeenCalledWith("**/*", expect.any(Function));
      expect(logger.info).toHaveBeenCalledWith(
        "Ultra-Lite mode enabled: media, styles, and tracking scripts blocked",
      );
    });

    it("should execute lite route callback branches", async () => {
      const logger = { info: vi.fn() };
      await initPage(mockPage, { lite: true, logger });

      const routeHandler = mockPage.route.mock.calls[0][1];
      const abortRoute = {
        request: () => ({
          resourceType: () => "image",
          url: () => "https://example.com/image.png",
        }),
        abort: vi.fn().mockResolvedValue(undefined),
        continue: vi.fn().mockResolvedValue(undefined),
      };
      await routeHandler(abortRoute);
      expect(abortRoute.abort).toHaveBeenCalled();

      const continueRoute = {
        request: () => ({
          resourceType: () => "document",
          url: () => "https://example.com/page",
        }),
        abort: vi.fn().mockResolvedValue(undefined),
        continue: vi.fn().mockResolvedValue(undefined),
      };
      await routeHandler(continueRoute);
      expect(continueRoute.continue).toHaveBeenCalled();
    });

    it("should block notifications via grantPermissions", async () => {
      const grantPermissions = vi.fn().mockResolvedValue(undefined);
      mockPage.context.mockReturnValue({
        browser: vi
          .fn()
          .mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
        on: vi.fn(),
        grantPermissions,
      });

      await initPage(mockPage, { blockNotifications: true });

      expect(grantPermissions).toHaveBeenCalledWith([], {
        origin: "https://example.com",
      });
    });

    it("should dismiss dialogs when blockDialogs is true", async () => {
      let dialogHandler;
      mockPage.on.mockImplementation((event, handler) => {
        if (event === "dialog") dialogHandler = handler;
      });

      await initPage(mockPage, {
        blockDialogs: true,
        logger: { debug: vi.fn() },
      });

      const dialog = {
        type: vi.fn().mockReturnValue("alert"),
        message: vi.fn().mockReturnValue("hi"),
        dismiss: vi.fn().mockResolvedValue(undefined),
      };
      await dialogHandler(dialog);

      expect(dialog.dismiss).toHaveBeenCalled();
    });

    it("should mute audio when requested", async () => {
      const logger = { info: vi.fn() };
      await initPage(mockPage, { muteAudio: true, logger });

      expect(mockPage.setMuted).toHaveBeenCalledWith(true);
      expect(logger.info).toHaveBeenCalledWith("Audio muted");
    });
  });

  describe("safeEmitError", () => {
    it("should fallback to console.error when context is missing", async () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      context.getEvents.mockImplementation(() => {
        throw new Error("No context");
      });

      // Trigger safeEmitError via diagnosePage with error
      mockPage.url.mockImplementation(() => {
        throw new Error("URL fail");
      });
      await diagnosePage(mockPage);

      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("[Init Error]"),
        expect.any(Error),
      );
      consoleSpy.mockRestore();
    });
  });

  describe("diagnosePage", () => {
    it("should return page status", async () => {
      await initPage(mockPage);
      const status = await diagnosePage(mockPage);

      expect(status).toEqual({
        url: "https://example.com",
        personaName: "casual",
        persona: {},
        patch: { status: "ok" },
        patched: true,
      });
    });

    it("should work without explicit page if context exists", async () => {
      // Use withPage to set context
      await context.withPage(mockPage, async () => {
        const status = await diagnosePage();
        expect(status.url).toBe("https://example.com");
      });
    });

    it("should handle missing page.url function", async () => {
      const weirdPage = { ...mockPage, url: undefined };
      const status = await diagnosePage(weirdPage);
      expect(status.url).toBeNull();
    });

    it("should fallback to getPage if no page provided", async () => {
      vi.spyOn(context, "getPage").mockReturnValue(mockPage);
      const status = await diagnosePage();
      expect(status.url).toBe("https://example.com");
    });

    it("should handle case where getPage also returns null", async () => {
      vi.spyOn(context, "getPage").mockReturnValue(null);
      // This will throw because withPage requires a page
      await expect(diagnosePage()).rejects.toThrow();
    });

    it("should handle errors in patchCheck", async () => {
      patch.check.mockRejectedValue(new Error("patch check fail"));
      const status = await diagnosePage(mockPage);
      expect(status.patch).toBeNull();
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "patchCheck" }),
      );
    });

    it("should handle errors in page.url()", async () => {
      mockPage.url.mockImplementation(() => {
        throw new Error("url check fail");
      });
      const status = await diagnosePage(mockPage);
      expect(status.url).toBeNull();
      expect(context.getEvents().emitSafe).toHaveBeenCalledWith(
        "on:error",
        expect.objectContaining({ context: "diagnosePage-url" }),
      );
    });
  });

  describe("safeEmitError fallback", () => {
    it("should log to console when events are unavailable", async () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      vi.spyOn(context, "getEvents").mockImplementation(() => {
        throw new Error("no events");
      });

      mockPage.url.mockImplementation(() => {
        throw new Error("url fail");
      });

      await diagnosePage(mockPage);

      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe("clearLiteMode", () => {
    it("should unrout page requests", async () => {
      await initPage(mockPage, { lite: true });
      await clearLiteMode(mockPage);
      expect(mockPage.unroute).toHaveBeenCalledWith("**/*");
    });
  });
});
