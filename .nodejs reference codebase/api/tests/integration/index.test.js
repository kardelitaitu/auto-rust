/**
 * Unit tests for api/index.js - Export Structure
 * Tests for the unified API export
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { api as apiModule } from "@api/index.js";

// Mock all heavy dependencies
vi.mock("./core/context.js", () => ({
  withPage: vi.fn(),
  clearContext: vi.fn(),
  isSessionActive: vi.fn(),
  checkSession: vi.fn(),
  getPage: vi.fn(() => ({ screenshot: vi.fn() })),
  getCursor: vi.fn(),
  evalPage: vi.fn(),
  getEvents: vi.fn(() => ({ emitSafe: vi.fn() })),
  getPlugins: vi.fn(),
}));

vi.mock("./core/context-state.js", () => ({
  getContextState: vi.fn(),
  setContextState: vi.fn(),
  getStateSection: vi.fn(),
  updateStateSection: vi.fn(),
}));

vi.mock("./core/errors.js", () => ({
  AutomationError: class extends Error {},
  SessionError: class extends Error {},
  SessionDisconnectedError: class extends Error {},
  SessionNotFoundError: class extends Error {},
  SessionTimeoutError: class extends Error {},
  ContextError: class extends Error {},
  ContextNotInitializedError: class extends Error {},
  PageClosedError: class extends Error {},
  ElementError: class extends Error {},
  ElementNotFoundError: class extends Error {},
  ElementDetachedError: class extends Error {},
  ElementObscuredError: class extends Error {},
  ElementTimeoutError: class extends Error {},
  ActionError: class extends Error {},
  ActionFailedError: class extends Error {},
  NavigationError: class extends Error {},
  ConfigError: class extends Error {},
  ConfigNotFoundError: class extends Error {},
  LLMError: class extends Error {},
  LLMTimeoutError: class extends Error {},
  LLMRateLimitError: class extends Error {},
  LLMCircuitOpenError: class extends Error {},
  ValidationError: class extends Error {},
  isErrorCode: vi.fn(),
  withErrorHandling: vi.fn(),
}));

vi.mock("./interactions/actions.js", () => ({
  click: vi.fn(),
  type: vi.fn(),
  hover: vi.fn(),
  rightClick: vi.fn(),
  drag: vi.fn(),
  clickAt: vi.fn(),
  multiSelect: vi.fn(),
  press: vi.fn(),
  typeText: vi.fn(),
  hold: vi.fn(),
  releaseAll: vi.fn(),
}));

vi.mock("./actions/quote.js", () => ({ quoteWithAI: vi.fn() }));
vi.mock("./actions/reply.js", () => ({ replyWithAI: vi.fn() }));
vi.mock("./actions/like.js", () => ({ likeWithAPI: vi.fn() }));
vi.mock("./actions/bookmark.js", () => ({ bookmarkWithAPI: vi.fn() }));
vi.mock("./actions/retweet.js", () => ({ retweetWithAPI: vi.fn() }));
vi.mock("./actions/follow.js", () => ({ followWithAPI: vi.fn() }));

vi.mock("./interactions/scroll.js", () => ({
  focus: vi.fn(),
  focus2: vi.fn(),
  scroll: vi.fn(),
  toTop: vi.fn(),
  toBottom: vi.fn(),
  read: vi.fn(),
  back: vi.fn(),
}));

vi.mock("./interactions/cursor.js", () => ({
  move: vi.fn(),
  up: vi.fn(),
  down: vi.fn(),
  setPathStyle: vi.fn(),
  getPathStyle: vi.fn(),
  startFidgeting: vi.fn(),
  stopFidgeting: vi.fn(),
}));

vi.mock("./interactions/queries.js", () => ({
  text: vi.fn(),
  attr: vi.fn(),
  visible: vi.fn(),
  count: vi.fn(),
  exists: vi.fn(),
  currentUrl: vi.fn(),
}));

vi.mock("./interactions/wait.js", () => ({
  wait: vi.fn(),
  waitWithAbort: vi.fn(),
  waitFor: vi.fn(),
  waitVisible: vi.fn(),
  waitHidden: vi.fn(),
  waitForLoadState: vi.fn(),
  waitForURL: vi.fn(),
}));

vi.mock("./interactions/navigation.js", () => ({
  goto: vi.fn(),
  reload: vi.fn(),
  back: vi.fn(),
  forward: vi.fn(),
  beforeNavigate: vi.fn(),
  randomMouse: vi.fn(),
  fakeRead: vi.fn(),
  pause: vi.fn(),
  setExtraHTTPHeaders: vi.fn(),
}));

vi.mock("./interactions/banners.js", () => ({ handleBanners: vi.fn() }));
vi.mock("./behaviors/timing.js", () => ({
  think: vi.fn(),
  delay: vi.fn(),
  gaussian: vi.fn(),
  randomInRange: vi.fn(),
}));
vi.mock("./behaviors/persona.js", () => ({
  setPersona: vi.fn(),
  getPersona: vi.fn(),
  getPersonaName: vi.fn(),
  listPersonas: vi.fn(),
  getSessionDuration: vi.fn(),
}));
vi.mock("./behaviors/recover.js", () => ({
  recover: vi.fn(),
  goBack: vi.fn(),
  findElement: vi.fn(),
  smartClick: vi.fn(),
  undo: vi.fn(),
  urlChanged: vi.fn(),
}));
vi.mock("./behaviors/attention.js", () => ({
  gaze: vi.fn(),
  attention: vi.fn(),
  distraction: vi.fn(),
  beforeLeave: vi.fn(),
  focusShift: vi.fn(),
  maybeDistract: vi.fn(),
  setDistractionChance: vi.fn(),
  getDistractionChance: vi.fn(),
}));
vi.mock("./behaviors/idle.js", () => ({
  start: vi.fn(),
  stop: vi.fn(),
  isRunning: vi.fn(),
  wiggle: vi.fn(),
  idleScroll: vi.fn(),
  startHeartbeat: vi.fn(),
}));
vi.mock("./utils/patch.js", () => ({
  apply: vi.fn(),
  stripCDPMarkers: vi.fn(),
  check: vi.fn(),
}));
vi.mock("./utils/file.readline.js", () => ({ readline: vi.fn() }));
vi.mock("./utils/file.consumeline.js", () => ({ consumeline: vi.fn() }));
vi.mock("./utils/visual-debug.js", () => ({ default: {} }));

vi.mock("./agent/observer.js", () => ({ see: vi.fn() }));
vi.mock("./agent/executor.js", () => ({ doAction: vi.fn() }));
vi.mock("./agent/finder.js", () => ({ find: vi.fn() }));
vi.mock("./agent/vision.js", () => ({ default: { screenshot: vi.fn() } }));

vi.mock("./agent/index.js", () => ({
  actionEngine: {},
  llmClient: {},
  agentRunner: {
    run: vi.fn(),
    stop: vi.fn(),
    isRunning: false,
    getUsageStats: vi.fn(),
  },
  captureAXTree: vi.fn(),
  captureState: vi.fn(),
  processWithVPrep: vi.fn(),
  getVPrepPresets: vi.fn(),
  getVPrepStats: vi.fn(),
}));

vi.mock("./agent/gameRunner.js", () => ({
  gameAgentRunner: {
    run: vi.fn(),
    stop: vi.fn(),
    isRunning: false,
    getUsageStats: vi.fn(),
  },
}));

vi.mock("./utils/vision-preprocessor.js", () => ({
  VisionPreprocessor: vi.fn().mockImplementation(() => ({
    process: vi.fn(),
    getStats: vi.fn(),
    resetStats: vi.fn(),
  })),
  VPrepPresets: {},
  processForVision: vi.fn(),
}));

vi.mock("./interactions/gameState.js", () => ({}));
vi.mock("./interactions/game-units.js", () => ({}));
vi.mock("./interactions/resourceTracker.js", () => ({}));
vi.mock("./interactions/gameMenus.js", () => ({}));

vi.mock("./core/init.js", () => ({
  initPage: vi.fn(),
  diagnosePage: vi.fn(),
  clearLiteMode: vi.fn(),
}));

vi.mock("./core/config.js", () => ({ configManager: {} }));
vi.mock("./core/events.js", () => ({
  getAvailableHooks: vi.fn(),
  getHookDescription: vi.fn(),
}));
vi.mock("./core/hooks.js", () => ({
  createHookWrapper: vi.fn(),
  withErrorHook: vi.fn(),
}));

vi.mock("./core/plugins/index.js", () => ({
  loadBuiltinPlugins: vi.fn(),
  registerPlugin: vi.fn(),
  unregisterPlugin: vi.fn(),
  enablePlugin: vi.fn(),
  disablePlugin: vi.fn(),
  listPlugins: vi.fn(),
  listEnabledPlugins: vi.fn(),
  getPluginManager: vi.fn(),
}));

vi.mock("./core/middleware.js", () => ({
  createPipeline: vi.fn(),
  createSyncPipeline: vi.fn(),
  loggingMiddleware: vi.fn(),
  validationMiddleware: vi.fn(),
  retryMiddleware: vi.fn(),
  recoveryMiddleware: vi.fn(),
  metricsMiddleware: vi.fn(),
  rateLimitMiddleware: vi.fn(),
}));

vi.mock("./utils/memory-profiler.js", () => ({ memory: {} }));

vi.mock("./twitter/intent-like.js", () => ({ like: vi.fn() }));
vi.mock("./twitter/intent-quote.js", () => ({ quote: vi.fn() }));
vi.mock("./twitter/intent-retweet.js", () => ({ retweet: vi.fn() }));
vi.mock("./twitter/intent-follow.js", () => ({ follow: vi.fn() }));
vi.mock("./twitter/intent-post.js", () => ({ post: vi.fn() }));
vi.mock("./twitter/navigation.js", () => ({
  home: vi.fn(),
  isOnHome: vi.fn(),
}));

vi.mock("./core/logger.js", () => ({
  createLogger: vi.fn(() => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  })),
}));

describe("api/index.js - Export Structure", () => {
  let api;

  beforeEach(() => {
    vi.clearAllMocks();
    api = apiModule;
  });

  describe("api object exists", () => {
    it("should export api object", () => {
      expect(api).toBeDefined();
      expect(typeof api).toBe("object");
    });

    it("should have version property", () => {
      expect(api.version).toBeDefined();
    });
  });

  describe("context methods", () => {
    it("should export withPage", () => expect(api.withPage).toBeDefined());
    it("should export clearContext", () =>
      expect(api.clearContext).toBeDefined());
    it("should export isSessionActive", () =>
      expect(api.isSessionActive).toBeDefined());
    it("should export getPage", () => expect(api.getPage).toBeDefined());
    it("should export getCursor", () => expect(api.getCursor).toBeDefined());
    it("should export eval", () => expect(api.eval).toBeDefined());
  });

  describe("action methods", () => {
    it("should export click", () => expect(api.click).toBeDefined());
    it("should export type", () => expect(api.type).toBeDefined());
    it("should export hover", () => expect(api.hover).toBeDefined());
    it("should export drag", () => expect(api.drag).toBeDefined());
    it("should export rightClick", () => expect(api.rightClick).toBeDefined());
    it("should export press", () => expect(api.press).toBeDefined());
    it("should export hold", () => expect(api.hold).toBeDefined());
    it("should export releaseAll", () => expect(api.releaseAll).toBeDefined());
  });

  describe("scroll namespace", () => {
    it("should export scroll as function", () => {
      expect(api.scroll).toBeDefined();
      expect(typeof api.scroll).toBe("function");
    });
    it("should have scroll.focus", () =>
      expect(api.scroll.focus).toBeDefined());
    it("should have scroll.toTop", () =>
      expect(api.scroll.toTop).toBeDefined());
    it("should have scroll.toBottom", () =>
      expect(api.scroll.toBottom).toBeDefined());
  });

  describe("cursor namespace", () => {
    it("should export cursor as function", () => {
      expect(api.cursor).toBeDefined();
      expect(typeof api.cursor).toBe("function");
    });
    it("should have cursor.move", () => expect(api.cursor.move).toBeDefined());
    it("should have cursor.up", () => expect(api.cursor.up).toBeDefined());
    it("should have cursor.down", () => expect(api.cursor.down).toBeDefined());
  });

  describe("query methods", () => {
    it("should export text", () => expect(api.text).toBeDefined());
    it("should export attr", () => expect(api.attr).toBeDefined());
    it("should export visible", () => expect(api.visible).toBeDefined());
    it("should export count", () => expect(api.count).toBeDefined());
    it("should export exists", () => expect(api.exists).toBeDefined());
    it("should export getCurrentUrl", () =>
      expect(api.getCurrentUrl).toBeDefined());
  });

  describe("wait methods", () => {
    it("should export wait", () => expect(api.wait).toBeDefined());
    it("should export waitFor", () => expect(api.waitFor).toBeDefined());
    it("should export waitVisible", () =>
      expect(api.waitVisible).toBeDefined());
    it("should export waitHidden", () => expect(api.waitHidden).toBeDefined());
  });

  describe("navigation methods", () => {
    it("should export goto", () => expect(api.goto).toBeDefined());
    it("should export reload", () => expect(api.reload).toBeDefined());
    it("should export back", () => expect(api.back).toBeDefined());
    it("should export forward", () => expect(api.forward).toBeDefined());
  });

  describe("timing methods", () => {
    it("should export think", () => expect(api.think).toBeDefined());
    it("should export delay", () => expect(api.delay).toBeDefined());
    it("should export gaussian", () => expect(api.gaussian).toBeDefined());
    it("should export randomInRange", () =>
      expect(api.randomInRange).toBeDefined());
  });

  describe("persona methods", () => {
    it("should export setPersona", () => expect(api.setPersona).toBeDefined());
    it("should export getPersona", () => expect(api.getPersona).toBeDefined());
    it("should export getPersonaName", () =>
      expect(api.getPersonaName).toBeDefined());
    it("should export listPersonas", () =>
      expect(api.listPersonas).toBeDefined());
  });

  describe("recovery methods", () => {
    it("should export recover", () => expect(api.recover).toBeDefined());
    it("should export goBack", () => expect(api.goBack).toBeDefined());
    it("should export findElement", () =>
      expect(api.findElement).toBeDefined());
    it("should export smartClick", () => expect(api.smartClick).toBeDefined());
    it("should export undo", () => expect(api.undo).toBeDefined());
  });

  describe("namespace objects", () => {
    it("should have idle namespace", () => {
      expect(api.idle).toBeDefined();
      expect(api.idle.start).toBeDefined();
      expect(api.idle.stop).toBeDefined();
    });

    it("should have patch namespace", () => {
      expect(api.patch).toBeDefined();
      expect(api.patch.apply).toBeDefined();
      expect(api.patch.check).toBeDefined();
    });

    it("should have file namespace", () => {
      expect(api.file).toBeDefined();
      expect(api.file.readline).toBeDefined();
      expect(api.file.consumeline).toBeDefined();
    });

    it("should have agent namespace", () => {
      expect(api.agent).toBeDefined();
      expect(typeof api.agent).toBe("function");
    });

    it("should have gameAgent namespace", () => {
      expect(api.gameAgent).toBeDefined();
      expect(api.gameAgent.run).toBeDefined();
    });

    it("should have vprep namespace", () => {
      expect(api.vprep).toBeDefined();
      expect(api.vprep.process).toBeDefined();
    });

    it("should have game namespace", () => {
      expect(api.game).toBeDefined();
    });

    it("should have plugins namespace", () => {
      expect(api.plugins).toBeDefined();
      expect(api.plugins.register).toBeDefined();
    });

    it("should have middleware namespace", () => {
      expect(api.middleware).toBeDefined();
      expect(api.middleware.createPipeline).toBeDefined();
    });

    it("should have twitter namespace", () => {
      expect(api.twitter).toBeDefined();
      expect(api.twitter.intent).toBeDefined();
    });
  });

  describe("named exports", async () => {
    it("should export context functions as named exports", async () => {
      const mod = await import("@api/index.js");
      expect(mod.withPage).toBeDefined();
      expect(mod.clearContext).toBeDefined();
    });

    it("should export action functions as named exports", async () => {
      const mod = await import("@api/index.js");
      expect(mod.click).toBeDefined();
      expect(mod.type).toBeDefined();
    });

    it("should export error classes as named exports", async () => {
      const mod = await import("@api/index.js");
      expect(mod.AutomationError).toBeDefined();
      expect(mod.SessionError).toBeDefined();
      expect(mod.ElementError).toBeDefined();
    });
  });

  describe("helper functions", () => {
    it("should have init function", () => {
      expect(api.init).toBeDefined();
      expect(typeof api.init).toBe("function");
    });

    it("should have diagnose function", () => {
      expect(api.diagnose).toBeDefined();
      expect(typeof api.diagnose).toBe("function");
    });

    it("should have screenshot function", () => {
      expect(api.screenshot).toBeDefined();
      expect(typeof api.screenshot).toBe("function");
    });
  });
});
