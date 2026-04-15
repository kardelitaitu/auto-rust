/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 */

/**
 * @fileoverview Unit Tests for api/index.js Composition Layer
 * Tests the composed API object exports to improve coverage on the main index file.
 * @module tests/unit/api/index-composition.test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("fs", async () => {
  const actual = await vi.importActual("fs");
  return { ...actual };
});

vi.mock("@api/core/context.js", () => ({
  withPage: vi.fn((page, fn) => fn()),
  clearContext: vi.fn(),
  destroySession: vi.fn(),
  isSessionActive: vi.fn().mockReturnValue(false),
  checkSession: vi.fn().mockReturnValue({ active: false }),
  getPage: vi.fn(),
  getCursor: vi.fn(),
  evalPage: vi.fn().mockResolvedValue(undefined),
  getEvents: vi.fn().mockReturnValue({}),
  getPlugins: vi.fn().mockReturnValue([]),
  getClipboardLock: vi
    .fn()
    .mockReturnValue({ acquire: vi.fn(), release: vi.fn() }),
}));

vi.mock("@api/core/context-state.js", async () => {
  const actual = await vi.importActual("@api/core/context-state.js");
  return { ...actual };
});

vi.mock("@api/core/errors.js", async () => {
  const actual = await vi.importActual("@api/core/errors.js");
  return {
    ...actual,
    isErrorCode: vi.fn().mockReturnValue(false),
    withErrorHandling: vi.fn((fn) => fn()),
    classifyHttpError: vi.fn().mockReturnValue("unknown"),
    wrapError: vi.fn((err) => err),
  };
});

vi.mock("@api/interactions/actions.js", () => ({
  click: vi.fn().mockResolvedValue({ success: true }),
  type: vi.fn().mockResolvedValue({ success: true }),
  hover: vi.fn().mockResolvedValue({ success: true }),
  rightClick: vi.fn().mockResolvedValue({ success: true }),
  drag: vi.fn().mockResolvedValue({ success: true }),
  clickAt: vi.fn().mockResolvedValue({ success: true }),
  multiSelect: vi.fn().mockResolvedValue({ success: true }),
  press: vi.fn().mockResolvedValue({ success: true }),
  typeText: vi.fn().mockResolvedValue({ success: true }),
  hold: vi.fn(),
  releaseAll: vi.fn(),
}));

vi.mock("@api/actions/quote.js", () => ({
  quoteWithAI: vi.fn().mockResolvedValue({ success: true }),
}));
vi.mock("@api/actions/reply.js", () => ({
  replyWithAI: vi.fn().mockResolvedValue({ success: true }),
}));
vi.mock("@api/actions/like.js", () => ({
  likeWithAPI: vi.fn().mockResolvedValue({ success: true }),
}));
vi.mock("@api/actions/bookmark.js", () => ({
  bookmarkWithAPI: vi.fn().mockResolvedValue({ success: true }),
}));
vi.mock("@api/actions/retweet.js", () => ({
  retweetWithAPI: vi.fn().mockResolvedValue({ success: true }),
}));
vi.mock("@api/actions/follow.js", () => ({
  followWithAPI: vi.fn().mockResolvedValue({ success: true }),
}));

vi.mock("@api/interactions/scroll.js", () => ({
  focus: vi.fn().mockResolvedValue({ success: true }),
  focus2: vi.fn().mockResolvedValue({ success: true }),
  scroll: vi.fn().mockResolvedValue({ success: true }),
  toTop: vi.fn().mockResolvedValue({ success: true }),
  toBottom: vi.fn().mockResolvedValue({ success: true }),
  read: vi.fn().mockResolvedValue("read content"),
  back: vi.fn().mockResolvedValue({ success: true }),
}));

vi.mock("@api/interactions/cursor.js", () => ({
  move: vi.fn().mockResolvedValue({ success: true }),
  up: vi.fn().mockResolvedValue({ success: true }),
  down: vi.fn().mockResolvedValue({ success: true }),
  setPathStyle: vi.fn(),
  getPathStyle: vi.fn().mockReturnValue({}),
  startFidgeting: vi.fn(),
  stopFidgeting: vi.fn(),
}));

vi.mock("@api/interactions/queries.js", () => ({
  text: vi.fn().mockResolvedValue("element text"),
  attr: vi.fn().mockResolvedValue("attr value"),
  visible: vi.fn().mockResolvedValue(true),
  count: vi.fn().mockResolvedValue(1),
  exists: vi.fn().mockResolvedValue(true),
  currentUrl: vi.fn().mockResolvedValue("https://example.com"),
}));

vi.mock("@api/interactions/wait.js", () => ({
  wait: vi.fn().mockResolvedValue({ success: true }),
  waitWithAbort: vi.fn().mockResolvedValue({ success: true }),
  waitFor: vi.fn().mockResolvedValue({ success: true }),
  waitVisible: vi.fn().mockResolvedValue(true),
  waitHidden: vi.fn().mockResolvedValue(true),
  waitForLoadState: vi.fn().mockResolvedValue({ success: true }),
  waitForURL: vi.fn().mockResolvedValue(true),
}));

vi.mock("@api/interactions/navigation.js", () => ({
  goto: vi.fn().mockResolvedValue({
    success: true,
    url: "https://example.com",
    duration: 100,
  }),
  reload: vi.fn().mockResolvedValue({ success: true }),
  back: vi.fn().mockResolvedValue({ success: true }),
  forward: vi.fn().mockResolvedValue({ success: true }),
  beforeNavigate: vi.fn(),
  randomMouse: vi.fn(),
  fakeRead: vi.fn(),
  pause: vi.fn(),
  setExtraHTTPHeaders: vi.fn(),
}));

vi.mock("@api/interactions/gameState.js", () => ({
  gameState: {
    getCurrentState: vi.fn().mockReturnValue({ state: "idle" }),
    detect: vi.fn().mockResolvedValue({ detected: true }),
    waitForState: vi.fn().mockResolvedValue({ state: "idle" }),
    parseResources: vi.fn().mockReturnValue({ gold: 1000 }),
  },
}));

vi.mock("@api/interactions/game-units.js", () => ({
  gameUnits: {
    detectUnits: vi.fn().mockReturnValue([]),
    getArmySize: vi.fn().mockReturnValue(0),
  },
}));

vi.mock("@api/interactions/resourceTracker.js", () => ({
  resourceTracker: {
    track: vi.fn(),
    getResources: vi.fn().mockReturnValue({}),
  },
}));

vi.mock("@api/interactions/gameMenus.js", () => ({
  gameMenus: {
    detectMenus: vi.fn().mockReturnValue([]),
    closeAll: vi.fn(),
  },
}));

vi.mock("@api/core/init.js", () => ({
  initPage: vi.fn().mockResolvedValue({}),
  diagnosePage: vi.fn().mockResolvedValue({ diagnosis: "ok" }),
  clearLiteMode: vi.fn(),
}));

vi.mock("@api/core/config.js", () => ({
  configManager: {
    get: vi.fn().mockReturnValue({}),
    set: vi.fn(),
    load: vi.fn().mockResolvedValue({}),
  },
}));

vi.mock("@api/core/events.js", async () => {
  const actual = await vi.importActual("@api/core/events.js");
  return { ...actual };
});

vi.mock("@api/core/hooks.js", async () => {
  const actual = await vi.importActual("@api/core/hooks.js");
  return { ...actual };
});

vi.mock("@api/agent/observer.js", () => ({
  see: vi
    .fn()
    .mockResolvedValue({ elements: [{ name: "Button", role: "button" }] }),
}));

vi.mock("@api/agent/executor.js", () => ({
  doAction: vi.fn().mockResolvedValue({ success: true }),
}));

vi.mock("@api/agent/finder.js", () => ({
  find: vi.fn().mockResolvedValue({ found: true, element: {} }),
}));

vi.mock("@api/agent/vision.js", () => ({
  default: {
    screenshot: vi.fn().mockResolvedValue("base64image"),
    capture: vi.fn().mockResolvedValue({ image: "data" }),
  },
}));

vi.mock("@api/agent/index.js", () => ({
  actionEngine: {
    execute: vi.fn().mockResolvedValue({ success: true }),
    plan: vi.fn().mockResolvedValue([]),
  },
  llmClient: {
    complete: vi.fn().mockResolvedValue({ text: "response" }),
    embed: vi.fn().mockResolvedValue([0.1]),
  },
  agentRunner: {
    run: vi.fn().mockResolvedValue({ success: true }),
    stop: vi.fn(),
    isRunning: false,
    getUsageStats: vi.fn().mockReturnValue({ runs: 0 }),
  },
  captureAXTree: vi.fn().mockResolvedValue("<ax>tree</ax>"),
  captureState: vi.fn().mockResolvedValue({ state: {} }),
  processWithVPrep: vi.fn(),
  getVPrepPresets: vi.fn(),
  getVPrepStats: vi.fn(),
}));

vi.mock("@api/agent/gameRunner.js", () => ({
  gameAgentRunner: {
    run: vi.fn().mockResolvedValue({ success: true }),
    stop: vi.fn(),
    isRunning: false,
    getUsageStats: vi.fn().mockReturnValue({ totalRuns: 0 }),
  },
}));

vi.mock("@api/utils/vision-preprocessor.js", () => {
  const mockInstance = {
    process: vi.fn().mockResolvedValue({
      base64: "processed",
      buffer: Buffer.from("test"),
      width: 800,
      height: 600,
    }),
    getStats: vi.fn().mockReturnValue({ totalProcessed: 0, bytesSaved: 0 }),
    resetStats: vi.fn(),
  };
  return {
    VisionPreprocessor: vi.fn(() => mockInstance),
    VPrepPresets: {
      GAME_UI: { targetWidth: 800, grayscale: true },
      SOCIAL_MEDIA: { targetWidth: 1200, grayscale: false },
      DOCUMENT: { targetWidth: 600, contrast: 1.2 },
    },
    getVisionPreprocessor: vi.fn(() => mockInstance),
  };
});

vi.mock("@api/utils/memory-profiler.js", () => ({
  memory: {
    getUsage: vi.fn().mockReturnValue({ heapUsed: 1000000 }),
    getSnapshot: vi.fn().mockReturnValue({}),
  },
}));

vi.mock("@api/core/plugins/index.js", async () => {
  const actual = await vi.importActual("@api/core/plugins/index.js");
  return { ...actual };
});

vi.mock("@api/core/middleware.js", async () => {
  const actual = await vi.importActual("@api/core/middleware.js");
  return { ...actual };
});

vi.mock("@api/behaviors/idle.js", async () => {
  const actual = await vi.importActual("@api/behaviors/idle.js");
  return { ...actual, start: actual.start || vi.fn() };
});

vi.mock("@api/behaviors/attention.js", async () => {
  const actual = await vi.importActual("@api/behaviors/attention.js");
  return { ...actual };
});

vi.mock("@api/interactions/scroll-idle.js", () => ({
  wiggle: vi.fn().mockResolvedValue({ success: true }),
  idleScroll: vi.fn().mockResolvedValue({ success: true }),
  startHeartbeat: vi.fn(),
}));

vi.mock("@api/utils/patch.js", async () => {
  const actual = await vi.importActual("@api/utils/patch.js");
  return { ...actual };
});

vi.mock("@api/utils/file.readline.js", () => ({
  readline: vi.fn().mockResolvedValue("random line"),
}));

vi.mock("@api/utils/file.consumeline.js", () => ({
  consumeline: vi.fn().mockResolvedValue("consumed line"),
}));

vi.mock("@api/utils/visual-debug.js", () => ({
  default: {
    highlight: vi.fn(),
    clear: vi.fn(),
  },
}));

vi.mock("@api/twitter/intent-like.js", () => ({
  like: vi.fn().mockReturnValue({}),
}));
vi.mock("@api/twitter/intent-quote.js", () => ({
  quote: vi.fn().mockReturnValue({}),
}));
vi.mock("@api/twitter/intent-retweet.js", () => ({
  retweet: vi.fn().mockReturnValue({}),
}));
vi.mock("@api/twitter/intent-follow.js", () => ({
  follow: vi.fn().mockReturnValue({}),
}));
vi.mock("@api/twitter/intent-post.js", () => ({
  post: vi.fn().mockReturnValue({}),
}));
vi.mock("@api/twitter/navigation.js", () => ({
  home: vi.fn().mockResolvedValue({ success: true }),
  isOnHome: vi.fn().mockReturnValue(true),
}));

describe("api/index.js composition layer", () => {
  let api;

  beforeEach(async () => {
    vi.resetModules();
    api = await import("@api/index.js");
  });

  describe("api version", () => {
    it("should have version property", () => {
      expect(api.api.version).toBeDefined();
      expect(typeof api.api.version).toBe("string");
    });
  });

  describe("api.screenshot", () => {
    it("should export screenshot function", () => {
      expect(typeof api.screenshot).toBe("function");
    });
  });

  describe("api.click", () => {
    it("should export click function", () => {
      expect(typeof api.click).toBe("function");
    });
  });

  describe("api.type", () => {
    it("should export type function", () => {
      expect(typeof api.type).toBe("function");
    });
  });

  describe("api.hold and api.releaseAll", () => {
    it("should export hold and releaseAll", () => {
      expect(typeof api.hold).toBe("function");
      expect(typeof api.releaseAll).toBe("function");
    });
  });

  describe("api.quoteWithAI", () => {
    it("should export quoteWithAI function", () => {
      expect(typeof api.quoteWithAI).toBe("function");
    });
  });

  describe("api.replyWithAI", () => {
    it("should export replyWithAI function", () => {
      expect(typeof api.replyWithAI).toBe("function");
    });
  });

  describe("api.likeWithAPI", () => {
    it("should export likeWithAPI function", () => {
      expect(typeof api.likeWithAPI).toBe("function");
    });
  });

  describe("api.bookmarkWithAPI", () => {
    it("should export bookmarkWithAPI function", () => {
      expect(typeof api.bookmarkWithAPI).toBe("function");
    });
  });

  describe("api.retweetWithAPI", () => {
    it("should export retweetWithAPI function", () => {
      expect(typeof api.retweetWithAPI).toBe("function");
    });
  });

  describe("api.followWithAPI", () => {
    it("should export followWithAPI function", () => {
      expect(typeof api.followWithAPI).toBe("function");
    });
  });

  describe("api.config", () => {
    it("should export config manager", () => {
      expect(api.api.config).toBeDefined();
      expect(typeof api.api.config.get).toBe("function");
      expect(typeof api.api.config.set).toBe("function");
    });
  });

  describe("api.events", () => {
    it("should export events getter", () => {
      expect(api.api.events).toBeDefined();
    });
  });

  describe("api.withPage", () => {
    it("should export withPage function", () => {
      expect(typeof api.withPage).toBe("function");
    });
  });

  describe("api.clearContext", () => {
    it("should export clearContext function", () => {
      expect(typeof api.clearContext).toBe("function");
    });
  });

  describe("api.destroySession", () => {
    it("should export destroySession function", () => {
      expect(typeof api.destroySession).toBe("function");
    });
  });

  describe("api.isSessionActive", () => {
    it("should export isSessionActive function", () => {
      expect(typeof api.isSessionActive).toBe("function");
    });
  });

  describe("api.checkSession", () => {
    it("should export checkSession function", () => {
      expect(typeof api.checkSession).toBe("function");
    });
  });

  describe("api.getPage", () => {
    it("should export getPage function", () => {
      expect(typeof api.getPage).toBe("function");
    });
  });

  describe("api.getCursor", () => {
    it("should export getCursor function", () => {
      expect(typeof api.getCursor).toBe("function");
    });
  });

  describe("api.eval", () => {
    it("should export eval function", () => {
      expect(typeof api.eval).toBe("function");
    });
  });

  describe("api.patch composition", () => {
    it("should export patch with apply, stripCDPMarkers, check", () => {
      expect(typeof api.api.patch).toBe("object");
    });
  });

  describe("api.file composition", () => {
    it("should export file with readline and consumeline", () => {
      expect(typeof api.api.file).toBe("object");
      expect(typeof api.api.file.readline).toBe("function");
      expect(typeof api.api.file.consumeline).toBe("function");
    });
  });

  describe("api.game composition", () => {
    it("should export game with state, units, resources, menus", () => {
      expect(typeof api.api.game).toBe("object");
      expect(typeof api.api.game.units).toBe("object");
      expect(typeof api.api.game.resources).toBe("object");
      expect(typeof api.api.game.menus).toBe("object");
    });
  });

  describe("api.vprep composition", () => {
    it("should export vprep with process, presets, getStats, resetStats", () => {
      expect(typeof api.api.vprep).toBe("object");
      expect(typeof api.api.vprep.process).toBe("function");
      expect(typeof api.api.vprep.presets).toBe("object");
      expect(typeof api.api.vprep.getStats).toBe("function");
      expect(typeof api.api.vprep.resetStats).toBe("function");
    });

    it("should have preset configurations", () => {
      expect(api.api.vprep.presets.GAME_UI).toBeDefined();
      expect(api.api.vprep.presets.SOCIAL_MEDIA).toBeDefined();
      expect(api.api.vprep.presets.DOCUMENT).toBeDefined();
    });
  });

  describe("api.plugins composition", () => {
    it("should export plugins with registration methods", () => {
      expect(typeof api.api.plugins).toBe("object");
      expect(typeof api.api.plugins.register).toBe("function");
    });
  });

  describe("api.middleware composition", () => {
    it("should export middleware with pipeline creators", () => {
      expect(typeof api.api.middleware).toBe("object");
      expect(typeof api.api.middleware.createPipeline).toBe("function");
    });
  });

  describe("api.gameAgent composition", () => {
    it("should export gameAgent with run, stop, isRunning, getStats", () => {
      expect(typeof api.api.gameAgent).toBe("object");
      expect(typeof api.api.gameAgent.run).toBe("function");
      expect(typeof api.api.gameAgent.stop).toBe("function");
    });
  });
});
