/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unified Browser Tool API — Central Export
 * Assembles all modules into a composable `api` object.
 *
 * Usage:
 *   import { api } from './api/index.js';
 *
 *   // Async context isolation (required for all API methods)
 *   await api.withPage(page, async () => {
 *       await api.init(page, { persona: 'casual' });
 *       await api.click('.btn');
 *       await api.type('.input', 'hello');
 *       await api.scroll.focus('.element');
 *       etc ...
 *       details in api/index.js or api/docs
 *   });
 *
 *   // File Utilities
 *   const line = await api.file.readline('data.txt'); // Read random line
 *   const consumed = await api.file.consumeline('data.txt'); // Read and remove random line
 *
 * @module api
 */

// ─── Version ─────────────────────────────────────────────────
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(
  readFileSync(join(__dirname, "..", "package.json"), "utf8"),
);

// ─── Type Definitions ─────────────────────────────────────────────

/**
 * @typedef {Object} ApiOptions
 * @property {string} [persona] - Persona name (casual, focused, etc.)
 * @property {Object} [personaOverrides] - Persona overrides
 * @property {boolean} [patch] - Enable detection patching
 * @property {boolean} [humanizationPatch] - Enable humanization
 * @property {boolean} [autoInitNewPages] - Auto-init new pages
 * @property {string} [colorScheme] - 'light' or 'dark'
 * @property {Object} [logger] - Custom logger instance
 * @property {boolean} [sensors] - Enable sensor simulation
 */

/**
 * @typedef {Object} ClickOptions
 * @property {boolean} [recovery] - Enable auto-recovery on failure
 * @property {number} [maxRetries] - Max retry attempts
 * @property {boolean} [hoverBeforeClick] - Hover before clicking
 * @property {string} [precision] - Precision mode: 'exact', 'safe', 'rough'
 * @property {string} [button] - Mouse button: 'left', 'right', 'middle'
 * @property {boolean} [force] - Force click even if obscured
 */

/**
 * @typedef {Object} TypeOptions
 * @property {number} [delay] - Delay between keystrokes in ms
 * @property {boolean} [noClear] - Don't clear field before typing
 * @property {boolean} [humanize] - Apply human-like keystroke timing
 */

/**
 * @typedef {Object} ScrollOptions
 * @property {number} [pauses] - Number of scroll+pause cycles
 * @property {number} [scrollAmount] - Pixels per scroll
 * @property {boolean} [variableSpeed] - Vary scroll speed
 * @property {boolean} [backScroll] - Occasional back-scroll
 */

/**
 * @typedef {Object} WaitOptions
 * @property {number} [timeout] - Timeout in ms
 * @property {string} [state] - Wait state: 'visible', 'hidden', 'attached'
 * @property {boolean} [throwOnTimeout] - Throw on timeout vs return false
 */

/**
 * @typedef {Object} NavigationOptions
 * @property {number} [timeout] - Navigation timeout in ms
 * @property {string} [waitUntil] - Wait until: 'load', 'domcontentloaded', 'networkidle'
 * @property {Object} [headers] - Extra HTTP headers
 */

/**
 * @typedef {Object} GotoResult
 * @property {boolean} success - Whether navigation succeeded
 * @property {string} url - Final URL after navigation
 * @property {number} duration - Time taken in ms
 */

/**
 * @typedef {Object} ElementResult
 * @property {boolean} success - Whether operation succeeded
 * @property {string} [selector] - The selector used
 * @property {*} [result] - Operation result if any
 */

/**
 * @typedef {Object} QueryResult
 * @property {string|number|boolean} value - The queried value
 * @property {boolean} success - Whether query succeeded
 */

// ─── Core Context ─────────────────────────────────────────────────
import {
  withPage,
  clearContext,
  destroySession,
  isSessionActive,
  checkSession,
  getPage,
  getCursor,
  evalPage,
  getEvents,
  getPlugins,
  getClipboardLock,
} from "./core/context.js";
import {
  getContextState,
  setContextState,
  getStateSection,
  updateStateSection,
} from "./core/context-state.js";
import {
  AutomationError,
  SessionError,
  SessionDisconnectedError,
  SessionNotFoundError,
  SessionTimeoutError,
  ContextError,
  ContextNotInitializedError,
  PageClosedError,
  ElementError,
  ElementNotFoundError,
  ElementDetachedError,
  ElementObscuredError,
  ElementTimeoutError,
  ActionError,
  ActionFailedError,
  NavigationError,
  ConfigError,
  ConfigNotFoundError,
  LLMError,
  LLMTimeoutError,
  LLMRateLimitError,
  LLMCircuitOpenError,
  ValidationError,
  isErrorCode,
  withErrorHandling,
  // HTTP/API errors (re-exported from utils/errors.js)
  AppError,
  RouterError,
  ProxyError,
  RateLimitError,
  ModelError,
  BrowserError,
  TimeoutError,
  CircuitBreakerError,
  classifyHttpError,
  wrapError,
} from "./core/errors.js";

// ─── Actions ──────────────────────────────────────────────────────
import {
  click,
  type,
  hover,
  rightClick,
  drag,
  clickAt,
  multiSelect,
  press,
  typeText,
  hold,
  releaseAll,
} from "./interactions/actions.js";
import { quoteWithAI } from "./actions/quote.js";
import { replyWithAI } from "./actions/reply.js";
import { likeWithAPI } from "./actions/like.js";
import { bookmarkWithAPI } from "./actions/bookmark.js";
import { retweetWithAPI } from "./actions/retweet.js";
import { followWithAPI } from "./actions/follow.js";

// ─── Scroll ───────────────────────────────────────────────────────
import {
  focus,
  focus2,
  scroll,
  toTop,
  toBottom,
  read,
  back as scrollBack,
} from "./interactions/scroll.js";

// ─── Cursor ───────────────────────────────────────────────────────
import {
  move,
  up,
  down,
  setPathStyle,
  getPathStyle,
  startFidgeting,
  stopFidgeting,
} from "./interactions/cursor.js";

// ─── Queries ──────────────────────────────────────────────────────
import {
  text,
  attr,
  visible,
  count,
  exists,
  currentUrl,
} from "./interactions/queries.js";

// ─── Wait ────────────────────────────────────────────────────────
import {
  wait,
  waitWithAbort,
  waitFor,
  waitVisible,
  waitHidden,
  waitForLoadState,
  waitForURL,
} from "./interactions/wait.js";

// ─── Navigation ─────────────────────────────────────────────────
import {
  goto,
  reload,
  back,
  forward,
  beforeNavigate,
  randomMouse,
  fakeRead,
  pause as warmupPause,
  setExtraHTTPHeaders,
} from "./interactions/navigation.js";

// ─── Banners ──────────────────────────────────────────────────
import { handleBanners } from "./interactions/banners.js";

// ─── Timing ─────────────────────────────────────────────────────
import { think, delay, gaussian, randomInRange } from "./behaviors/timing.js";

// ─── Persona ────────────────────────────────────────────────────
import {
  setPersona,
  getPersona,
  getPersonaName,
  listPersonas,
  getSessionDuration,
} from "./behaviors/persona.js";

// ─── Recovery ───────────────────────────────────────────────────
import {
  recover,
  goBack,
  findElement,
  smartClick,
  undo,
  urlChanged,
} from "./behaviors/recover.js";

// ─── Attention ─────────────────────────────────────────────────
import {
  gaze,
  attention,
  distraction,
  beforeLeave,
  focusShift,
  maybeDistract,
  setDistractionChance,
  getDistractionChance,
} from "./behaviors/attention.js";

// ─── Idle ───────────────────────────────────────────────────────
import {
  start as idleStart,
  stop as idleStop,
  isRunning as idleIsRunning,
  wiggle,
  idleScroll,
  startHeartbeat,
} from "./behaviors/idle.js";

// ─── Patch ─────────────────────────────────────────────────────
import {
  apply as patchApply,
  stripCDPMarkers,
  check as patchCheck,
} from "./utils/patch.js";

// ─── File I/O ──────────────────────────────────────────────────
import { readline } from "./utils/file.readline.js";
import { consumeline } from "./utils/file.consumeline.js";

// ─── Visual Debug ─────────────────────────────────────────────
import visualDebugModule from "./utils/visual-debug.js";
const visualDebug = visualDebugModule;

// ─── Agent ─────────────────────────────────────────────────────
import { see } from "./agent/observer.js";
import { doAction } from "./agent/executor.js";
import { find as agentFind } from "./agent/finder.js";
import * as visionModule from "./agent/vision.js";
const agentVision = visionModule.default;
import {
  actionEngine,
  llmClient,
  agentRunner,
  captureAXTree,
  captureState,
  processWithVPrep as _processWithVPrep,
  getVPrepPresets as _getVPrepPresets,
  getVPrepStats as _getVPrepStats,
} from "./agent/index.js";
import { gameAgentRunner } from "./agent/gameRunner.js";

// ─── Vision Preprocessor (V-PREP) ─────────────────────────────
import {
  VisionPreprocessor,
  VPrepPresets,
  processForVision as _processForVision,
} from "./utils/vision-preprocessor.js";
let visionPreprocessor = null;
function getVisionPreprocessor() {
  if (!visionPreprocessor) visionPreprocessor = new VisionPreprocessor();
  return visionPreprocessor;
}

// ─── Game State ───────────────────────────────────────────────
import * as gameState from "./interactions/gameState.js";
import * as gameUnits from "./interactions/game-units.js";
import * as resourceTracker from "./interactions/resourceTracker.js";
import * as gameMenus from "./interactions/gameMenus.js";

// ─── Init ───────────────────────────────────────────────────────
import { initPage, diagnosePage, clearLiteMode } from "./core/init.js";

// ─── Config ─────────────────────────────────────────────────────
import { configManager } from "./core/config.js";

// ─── Events & Plugins ─────────────────────────────────────────
import { getAvailableHooks, getHookDescription } from "./core/events.js";
import { createHookWrapper, withErrorHook } from "./core/hooks.js";
import {
  loadBuiltinPlugins,
  registerPlugin,
  unregisterPlugin,
  enablePlugin,
  disablePlugin,
  listPlugins,
  listEnabledPlugins,
  getPluginManager,
} from "./core/plugins/index.js";

// ─── Middleware ────────────────────────────────────────────────
import {
  createPipeline,
  createSyncPipeline,
  loggingMiddleware,
  validationMiddleware,
  retryMiddleware,
  recoveryMiddleware,
  metricsMiddleware,
  rateLimitMiddleware,
} from "./core/middleware.js";

// ─── Memory ─────────────────────────────────────────────────────
import { memory } from "./utils/memory-profiler.js";

// ─── Twitter Intents ───────────────────────────────────────────
import { like as intentLike } from "./twitter/intent-like.js";
import { quote as intentQuote } from "./twitter/intent-quote.js";
import { retweet as intentRetweet } from "./twitter/intent-retweet.js";
import { follow as intentFollow } from "./twitter/intent-follow.js";
import { post as intentPost } from "./twitter/intent-post.js";

// ─── Twitter Navigation ────────────────────────────────────────
import { home as twitterHome, isOnHome } from "./twitter/navigation.js";

// ─── Build Dual-Callable APIs ──────────────────────────────────

// Scroll: api.scroll(300) + api.scroll.focus('.el') + api.scroll.focus2('.el')
const scrollFn = Object.assign(scroll, {
  focus,
  focus2,
  toTop,
  toBottom,
  read,
  back: scrollBack,
});

// Cursor: api.cursor(selector) + api.cursor.move() + api.cursor.up()
const cursorFn = Object.assign((selector) => move(selector), {
  move,
  up,
  down,
  setPathStyle,
  getPathStyle,
  startFidgeting,
  stopFidgeting,
});

// Agent: api.agent('goal') + api.agent.see()
const agentFn = Object.assign(
  async (goal, config) => {
    return await agentRunner.run(goal, config);
  },
  {
    see,
    do: doAction,
    find: agentFind,
    vision: agentVision,
    screenshot: agentVision.screenshot,
    captureAXTree,
    captureState,
    run: (goal, config) => agentRunner.run(goal, config),
    stop: () => agentRunner.stop(),
    isRunning: () => agentRunner.isRunning,
    engine: actionEngine,
    llm: llmClient,
    getStats: () => agentRunner.getUsageStats(),
  },
);

async function init(page, options = {}) {
  return initPage(page, options);
}

async function diagnose(page) {
  return diagnosePage(page);
}

async function screenshot(options = {}) {
  const page = getPage();
  const {
    path: outputPath,
    fullPage = false,
    type = "jpeg",
    quality = 80,
  } = options;

  return page.screenshot({ path: outputPath, fullPage, type, quality });
}

async function emulateMedia(options = {}) {
  const page = getPage();
  return page.emulateMedia(options);
}

/**
 * Unified API Object
 * Provides ergonomic access to all modules.
 */
export const api = {
  // ── Version ─────────────────────────────────────────────────
  version: pkg.version,

  // ── Context ──────────────────────────────────────────────────
  // Note: Use api.withPage(page, fn) for context isolation
  withPage,
  clearContext,
  destroySession,
  isSessionActive,
  checkSession,
  getPage,
  getCursor,
  eval: evalPage,
  init,
  diagnose,
  screenshot,
  emulateMedia,
  clearLiteMode,
  getClipboardLock,
  config: configManager,

  // ── Actions (top-level for ergonomics) ───────────────────────
  /**
   * Human-like click with automatic scrolling and cursor movement
   * @example
   * await api.click('.btn-submit'); // Simple click
   * await api.click('#login', { button: 'right' }); // Right-click
   * await api.click('.dropdown', { hoverBeforeClick: true }); // Hover first
   */
  click,
  /**
   * Type text into an input field with human-like keystroke timing
   * @example
   * await api.type('.username', 'john_doe'); // Type into input
   * await api.type('.search', 'query', { delay: 50 }); // Custom delay
   * await api.type('.textarea', ' multiline\ntext', { noClear: true }); // Append text
   */
  type,
  hover,
  rightClick,
  drag,
  clickAt,
  multiSelect,
  press,
  typeText,
  hold,
  releaseAll,
  quoteWithAI,
  replyWithAI,
  likeWithAPI,
  bookmarkWithAPI,
  retweetWithAPI,
  followWithAPI,

  // ── Scroll (dual: api.scroll(300) + api.scroll.focus('.el')) ─
  /**
   * Scroll actions - dual API: api.scroll(pixels) or api.scroll.focus(selector)
   * @example
   * await api.scroll(300); // Scroll down 300px
   * await api.scroll(-200); // Scroll up 200px
   * await api.scroll.focus('.element'); // Scroll to element
   * await api.scroll.toTop(); // Scroll to top of page
   * await api.scroll.toBottom(); // Scroll to bottom
   */
  scroll: scrollFn,

  // ── Cursor (low-level) ───────────────────────────────────────
  cursor: cursorFn,

  // ── Queries (read-only) ──────────────────────────────────────
  /**
   * Get text content of an element
   * @example
   * const text = await api.text('.heading'); // Get element text
   */
  text,
  /**
   * Get element attribute value
   * @example
   * const href = await api.attr('a', 'href'); // Get href attribute
   * const value = await api.attr('input', 'placeholder'); // Get placeholder
   */
  attr,
  /**
   * Check if element is visible
   * @example
   * const isVisible = await api.visible('.modal'); // Returns boolean
   */
  visible,
  /**
   * Count elements matching selector
   * @example
   * const count = await api.count('.item'); // Number of matching elements
   */
  count,
  /**
   * Check if element exists in DOM
   * @example
   * const exists = await api.exists('.missing'); // Returns boolean
   */
  exists,
  getUrl: currentUrl,
  getCurrentUrl: currentUrl,

  // ── Wait (synchronization) ───────────────────────────────────
  /**
   * Wait for a duration or condition
   * @example
   * await api.wait(1000); // Wait 1 second
   * await api.waitFor('.element'); // Wait for element to exist
   * await api.waitVisible('.modal', { timeout: 5000 }); // Wait for visible
   * await api.waitHidden('.loader'); // Wait for hidden
   */
  wait,
  waitWithAbort,
  waitFor,
  waitVisible,
  waitHidden,
  waitForLoadState,
  waitForURL,

  // ── Navigation ───────────────────────────────────────────────
  /**
   * Navigate to a URL with human-like behavior and warmup
   * @example
   * await api.goto('https://example.com'); // Simple navigation
   * await api.goto('https://example.com', { timeout: 30000 }); // Custom timeout
   * await api.goto('https://example.com', { waitUntil: 'networkidle' }); // Wait for network idle
   */
  goto,
  /**
   * Reload the current page
   * @example
   * await api.reload();
   */
  reload,
  /**
   * Go back to previous page in browser history
   * @example
   * await api.back();
   */
  back,
  /**
   * Go forward to next page in browser history
   * @example
   * await api.forward();
   */
  forward,
  setExtraHTTPHeaders,

  // ── Banners ──────────────────────────────────────────────────
  handleBanners,

  // ── Warmup ───────────────────────────────────────────────────
  beforeNavigate,
  randomMouse,
  fakeRead,
  warmupPause,

  // ── Timing ───────────────────────────────────────────────────
  /**
   * Simulate thinking/reading time with human-like delays
   * @example
   * await api.think(); // Random think time (2-5s)
   * await api.think(3000); // Think for 3 seconds
   */
  think,
  /**
   * Fixed delay in milliseconds
   * @example
   * await api.delay(500); // Wait 500ms
   */
  delay,
  /**
   * Gaussian random delay (bell-curve distribution)
   * @example
   * await api.gaussian(1000, 200); // ~1000ms with std dev 200ms
   */
  gaussian,
  /**
   * Random delay within a range
   * @example
   * await api.randomInRange(100, 500); // Random between 100-500ms
   */
  randomInRange,

  // ── Persona ──────────────────────────────────────────────────
  /**
   * Set the active persona for humanization behaviors
   * @example
   * await api.setPersona('casual'); // Set casual persona
   * await api.setPersona('focused', { delay: 20 }); // Custom overrides
   */
  setPersona,
  /**
   * Get current persona settings
   * @example
   * const persona = await api.getPersona();
   */
  getPersona,
  /**
   * Get current persona name
   * @example
   * const name = await api.getPersonaName(); // 'casual', 'focused', etc.
   */
  getPersonaName,
  /**
   * List all available personas
   * @example
   * const personas = await api.listPersonas(); // ['casual', 'focused', 'fast', ...]
   */
  listPersonas,

  // ── Recovery ─────────────────────────────────────────────────
  recover,
  goBack,
  findElement,
  smartClick,
  undo,

  // ── Attention ────────────────────────────────────────────────
  gaze,
  attention,
  distraction,
  beforeLeave,
  focusShift,
  maybeDistract,
  setDistractionChance,
  getDistractionChance,

  // ── Idle ────────────────────────────────────────────────────
  idle: {
    start: idleStart,
    stop: idleStop,
    isRunning: idleIsRunning,
    wiggle,
    scroll: idleScroll,
    heartbeat: startHeartbeat,
  },

  // ── Patch ────────────────────────────────────────────────────
  patch: {
    apply: patchApply,
    stripCDPMarkers,
    check: patchCheck,
  },

  // ── File I/O ─────────────────────────────────────────────────
  /**
   * File I/O Utilities
   * @example
   * const line = await api.file.readline('data.txt');
   * const consumed = await api.file.consumeline('data.txt');
   */
  file: {
    readline,
    consumeline,
  },

  // ── Agent ────────────────────────────────────────────────────
  /**
   * Agent Interaction Layer (LLM-friendly)
   * @example
   * await api.agent('Find the login button and click it'); // Run full agent loop
   * const view = await api.agent.see(); // Get semantic map
   * await api.agent.do('click', 'Login'); // Click by label
   * await api.agent.do('type', 1, 'username'); // Type by ID
   */
  agent: agentFn,

  /**
   * Game Agent - Enhanced agent for strategy games with verification
   * @example
   * await api.gameAgent('Build a barracks and train 5 footmen');
   */
  gameAgent: {
    run: (goal, config) => gameAgentRunner.run(goal, config),
    stop: () => gameAgentRunner.stop(),
    isRunning: () => gameAgentRunner.isRunning,
    getStats: () => gameAgentRunner.getUsageStats(),
  },

  // ── V-PREP (Vision Pre-Processing) ──────────────────────────
  /**
   * Vision Pre-Processing and Resolution Enhancement Protocol
   * Optimizes screenshots for LLM vision consumption.
   *
   * @example
   * // Quick process with preset
   * const result = await api.vprep.process(buffer, api.vprep.presets.GAME_UI);
   *
   * // Custom configuration
   * const result = await api.vprep.process(buffer, {
   *     targetWidth: 800,
   *     grayscale: true,
   *     contrast: 1.3,
   *     edgeEnhance: true,
   * });
   *
   * // Use with captureState
   * const state = await api.agent.captureState({ vprep: true, vprepConfig: { grayscale: true } });
   */
  vprep: {
    /**
     * Process an image buffer for optimal LLM consumption
     * @param {Buffer|string} input - Image buffer or base64 string
     * @param {object} [config] - Processing options
     * @returns {Promise<object>} Result with base64, buffer, and stats
     */
    process: (input, config) => getVisionPreprocessor().process(input, config),

    /**
     * Preset configurations for common use cases
     */
    presets: VPrepPresets,

    /**
     * Get processing statistics
     * @returns {object} Stats including total processed, bytes saved
     */
    getStats: () => getVisionPreprocessor().getStats(),

    /**
     * Reset statistics
     */
    resetStats: () => getVisionPreprocessor().resetStats(),

    /**
     * The VisionPreprocessor instance for advanced usage
     */
    get instance() {
      return getVisionPreprocessor();
    },
  },

  // ── Game State ───────────────────────────────────────────────
  game: {
    ...gameState,
    units: gameUnits,
    resources: resourceTracker,
    menus: gameMenus,
  },

  // ── Events & Plugins ────────────────────────────────────────
  get events() {
    return getEvents();
  },
  plugins: {
    register: registerPlugin,
    unregister: unregisterPlugin,
    enable: enablePlugin,
    disable: disablePlugin,
    list: listPlugins,
    listEnabled: listEnabledPlugins,
    getManager: getPluginManager,
  },

  // ── Middleware ───────────────────────────────────────────────
  middleware: {
    createPipeline,
    createSyncPipeline,
    logging: loggingMiddleware,
    validation: validationMiddleware,
    retry: retryMiddleware,
    recovery: recoveryMiddleware,
    metrics: metricsMiddleware,
    rateLimit: rateLimitMiddleware,
  },

  // ── Memory ─────────────────────────────────────────────────────
  memory,

  // ── Twitter ────────────────────────────────────────────────────
  /**
   * Twitter-specific actions and intents
   * @example
   * await api.twitter.intent.like(tweetUrl); // Like a tweet
   * await api.twitter.intent.follow(username); // Follow a user
   * await api.twitter.intent.quote(tweetUrl, 'Great post!'); // Quote tweet
   * await api.twitter.intent.retweet(tweetUrl); // Retweet
   * await api.twitter.intent.post('Hello world!'); // Post new tweet
   */
  twitter: {
    intent: {
      like: intentLike,
      quote: intentQuote,
      retweet: intentRetweet,
      follow: intentFollow,
      post: intentPost,
    },
    home: twitterHome,
    isOnHome,
  },

  // ── Visual Debug ───────────────────────────────────────────────
  visualDebug,
};

export default api;

// Re-export all named exports
export {
  // Note: setPage is deprecated - use withPage instead
  withPage,
  clearContext,
  destroySession,
  isSessionActive,
  checkSession,
  getPage,
  getCursor,
  evalPage as eval,
  getClipboardLock,
  getContextState,
  setContextState,
  getStateSection,
  updateStateSection,
  click,
  type,
  hover,
  rightClick,
  drag,
  clickAt,
  multiSelect,
  press,
  typeText,
  hold,
  releaseAll,
  quoteWithAI,
  replyWithAI,
  likeWithAPI,
  bookmarkWithAPI,
  retweetWithAPI,
  followWithAPI,
  focus,
  scroll,
  toTop,
  toBottom,
  read,
  scrollBack,
  move,
  up,
  down,
  setPathStyle,
  getPathStyle,
  startFidgeting,
  stopFidgeting,
  text,
  attr,
  visible,
  count,
  exists,
  currentUrl,
  wait,
  waitWithAbort,
  waitFor,
  waitVisible,
  waitHidden,
  waitForLoadState,
  waitForURL,
  goto,
  reload,
  back,
  forward,
  beforeNavigate,
  randomMouse,
  fakeRead,
  warmupPause,
  setExtraHTTPHeaders,
  handleBanners,
  think,
  delay,
  gaussian,
  randomInRange,
  setPersona,
  getPersona,
  getPersonaName,
  listPersonas,
  getSessionDuration,
  recover,
  goBack,
  findElement,
  smartClick,
  undo,
  urlChanged,
  gaze,
  attention,
  distraction,
  beforeLeave,
  focusShift,
  maybeDistract,
  setDistractionChance,
  getDistractionChance,
  idleStart,
  idleStop,
  idleIsRunning,
  wiggle,
  idleScroll,
  startHeartbeat,
  patchApply,
  stripCDPMarkers,
  patchCheck,
  initPage,
  diagnosePage,
  clearLiteMode,
  screenshot,
  readline,
  consumeline,
  // Visual Debug
  visualDebug,
  see,
  doAction as do,
  agentFind as find,
  agentVision as vision,
  getAvailableHooks,
  getHookDescription,
  createHookWrapper,
  withErrorHook,
  getEvents,
  getPlugins,
  loadBuiltinPlugins,
  registerPlugin,
  unregisterPlugin,
  enablePlugin,
  disablePlugin,
  listPlugins,
  listEnabledPlugins,
  getPluginManager,
  createPipeline,
  createSyncPipeline,
  loggingMiddleware,
  validationMiddleware,
  retryMiddleware,
  recoveryMiddleware,
  metricsMiddleware,
  rateLimitMiddleware,
  // Errors
  AutomationError,
  SessionError,
  SessionDisconnectedError,
  SessionNotFoundError,
  SessionTimeoutError,
  ContextError,
  ContextNotInitializedError,
  PageClosedError,
  ElementError,
  ElementNotFoundError,
  ElementDetachedError,
  ElementObscuredError,
  ElementTimeoutError,
  ActionError,
  ActionFailedError,
  NavigationError,
  ConfigError,
  ConfigNotFoundError,
  LLMError,
  LLMTimeoutError,
  LLMRateLimitError,
  LLMCircuitOpenError,
  ValidationError,
  isErrorCode,
  withErrorHandling,
  // HTTP/API Errors
  AppError,
  RouterError,
  ProxyError,
  RateLimitError,
  ModelError,
  BrowserError,
  TimeoutError,
  CircuitBreakerError,
  classifyHttpError,
  wrapError,
};
