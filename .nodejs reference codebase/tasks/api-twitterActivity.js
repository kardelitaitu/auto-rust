/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * AI-Enhanced Twitter Activity Task using Unified API
 * Full feature parity with ai-twitterActivity.js using api/ modules
 * @module tasks/api-twitterActivity
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";
import { AITwitterAgent } from "../api/twitter/ai-twitterAgent.js";
import { profileManager } from "../api/utils/profileManager.js";
import { mathUtils } from "../api/utils/math.js";
import { ReferrerEngine } from "../api/utils/urlReferrer.js";
import {
  getLoggingConfig,
  formatEngagementSummary,
} from "../api/utils/logging-config.js";

import { loadAiTwitterActivityConfig } from "../api/utils/task-config-loader.js";
import PopupCloser from "../api/utils/popup-closer.js";
import { humanTiming } from "../api/behaviors/human-timing.js";
import { TWITTER_TIMEOUTS } from "../api/constants/twitter-timeouts.js";
import { likeWithAPI } from "../api/actions/like.js";
import { bookmarkWithAPI } from "../api/actions/bookmark.js";
import { retweetWithAPI } from "../api/actions/retweet.js";
import { followWithAPI } from "../api/actions/follow.js";

const DEFAULT_MIN_DURATION = 540;
const DEFAULT_MAX_DURATION = 840;
const WAIT_UNTIL = "domcontentloaded";

const MAX_RETRIES = 2;
const LOGIN_CHECK_LOOPS = 3;
const LOGIN_CHECK_DELAY = 3000;
const PAGE_TIMEOUT_MS = 60000;

const ENTRY_POINTS = [
  // Primary Entry (Remainder: 100% - 32% - 4% - 5% = 59%)
  { url: "https://x.com/", weight: 59 },

  // 4% Weight Group (Total 32%)
  { url: "https://x.com/i/jf/global-trending/home", weight: 4 },
  { url: "https://x.com/explore", weight: 4 },
  { url: "https://x.com/explore/tabs/for-you", weight: 4 },
  { url: "https://x.com/explore/tabs/trending", weight: 4 },
  { url: "https://x.com/i/bookmarks", weight: 4 },
  { url: "https://x.com/notifications", weight: 4 },
  { url: "https://x.com/notifications/mentions", weight: 4 },
  { url: "https://x.com/i/chat/", weight: 4 },

  // 2% Weight Group (Total 4%)
  { url: "https://x.com/i/connect_people?show_topics=false", weight: 2 },
  { url: "https://x.com/i/connect_people?is_creator_only=true", weight: 2 },

  // Legacy/Supplementary Exploratory Points (1% each to fill entropy, Total 5%)
  { url: "https://x.com/explore/tabs/news", weight: 1 },
  { url: "https://x.com/explore/tabs/sports", weight: 1 },
  { url: "https://x.com/explore/tabs/entertainment", weight: 1 },
  { url: "https://x.com/explore/tabs/for_you", weight: 1 }, // Note: legacy underscore version
  { url: "https://x.com/notifications", weight: 1 }, // Secondary notification hit
];

function selectEntryPoint() {
  const totalWeight = ENTRY_POINTS.reduce((sum, ep) => sum + ep.weight, 0);
  let random = Math.random() * totalWeight;

  for (const entry of ENTRY_POINTS) {
    random -= entry.weight;
    if (random <= 0) {
      return entry.url;
    }
  }

  return ENTRY_POINTS[0].url;
}

function extractProfileType(profile) {
  if (!profile) return null;
  if (typeof profile.type === "string" && profile.type.trim().length > 0) {
    return profile.type.trim();
  }
  if (typeof profile.id === "string") {
    const parts = profile.id.split("-");
    if (parts.length > 1) {
      return parts.slice(1).join("-").trim();
    }
  }
  if (typeof profile.description === "string") {
    const match = profile.description.match(/Type:\s*([A-Za-z]+)/);
    if (match?.[1]) return match[1];
  }
  return null;
}

function resolvePersona(profile) {
  const available = new Set(api.listPersonas());
  const rawPersona =
    typeof profile?.persona === "string" ? profile.persona.trim() : "";
  if (rawPersona && available.has(rawPersona)) return rawPersona;

  const type = extractProfileType(profile);
  const byType = {
    skimmer: "efficient",
    balanced: "casual",
    deepdiver: "researcher",
    lurker: "hesitant",
    doomscroller: "distracted",
    newsjunkie: "focused",
    stalker: "focused",
  };
  const mapped = type ? byType[type.toLowerCase()] : null;
  if (mapped && available.has(mapped)) return mapped;
  return "casual";
}

/**
 * Safely extract engagement probabilities with defaults.
 * Prevents TypeError if config structure changes.
 */
function getEngagementProbabilities(config) {
  const probs = config?.engagement?.probabilities ?? {};
  return {
    reply: probs.reply ?? 0.5,
    quote: probs.quote ?? 0.2,
    like: probs.like ?? 0.15,
    bookmark: probs.bookmark ?? 0.05,
    retweet: probs.retweet ?? 0.2,
    follow: probs.follow ?? 0.1,
  };
}

/**
 * Safely extract warmup timing with defaults.
 */
function getWarmupTiming(config) {
  const warmup = config?.timing?.warmup ?? {};
  return { min: warmup.min ?? 2000, max: warmup.max ?? 15000 };
}

/**
 * Configure persona, theme, and idle simulation for the session.
 * @returns {Promise<{theme: string}>}
 */
async function setupEnvironment(profile, api, logger, withPageLock) {
  if (profile) {
    await api.setPersona(resolvePersona(profile));
    logger.info(`Persona: ${api.getPersonaName()}`);

    const persona = api.getPersona();
    const distractionChance =
      typeof persona.microMoveChance === "number"
        ? persona.microMoveChance
        : typeof persona.idleChance === "number"
          ? persona.idleChance
          : 0.2;

    api.setDistractionChance(distractionChance);
    logger.info(`Distraction chance: ${(distractionChance * 100).toFixed(0)}%`);
  }

  const theme = profile?.theme || "dark";
  logger.info(`Enforcing theme: ${theme}`);
  await withPageLock(async () => api.emulateMedia({ colorScheme: theme }));

  const persona = api.getPersona();
  const idleChance =
    typeof persona.idleChance === "number" ? persona.idleChance : 0.02;
  const speed =
    typeof persona.speed === "number" && persona.speed > 0 ? persona.speed : 1;
  const idleRoll = Math.random();
  const shouldIdle = idleRoll < Math.max(0.05, Math.min(0.4, idleChance * 2));

  if (shouldIdle) {
    api.idle.start({
      wiggle: true,
      scroll: idleChance > 0.02,
      frequency: Math.min(8000, Math.max(2000, Math.round(4000 / speed))),
      magnitude: api.getPersonaName() === "glitchy" ? 8 : 3,
    });
    logger.info(`Idle simulation started`);
  }

  return { theme };
}

/**
 * Navigate to weighted entry point and simulate reading if not on home.
 * @returns {Promise<{entryName: string}>}
 */
async function navigateAndRead(
  agent,
  entryUrl,
  api,
  logger,
  withPageLock,
  abortSignal,
) {
  const entryName =
    entryUrl.replace("https://x.com/", "").replace("https://x.com", "") ||
    "home";
  logger.info(`🎲 Rolled entry point: ${entryName} → ${entryUrl}`);

  const referrerEngine = new ReferrerEngine({ addUTM: true });
  const ctx = referrerEngine.generateContext(entryUrl);

  await withPageLock(async () => {
    await api.goto(entryUrl, {
      waitUntil: WAIT_UNTIL,
      timeout: PAGE_TIMEOUT_MS,
      referer: ctx.referrer || undefined,
    });
  });

  const xLoaded = await withPageLock(async () =>
    Promise.race([
      api
        .waitVisible('[data-testid="AppTabBar_Home_Link"]', {
          timeout: TWITTER_TIMEOUTS.ELEMENT_VISIBLE,
        })
        .then(() => "home")
        .catch((e) => {
          logger.debug(`Page detection: home link not visible (${e.message})`);
          return null;
        }),
      api
        .waitVisible('[data-testid="loginButton"]', {
          timeout: TWITTER_TIMEOUTS.ELEMENT_VISIBLE,
        })
        .then(() => "login")
        .catch((e) => {
          logger.debug(
            `Page detection: login button not visible (${e.message})`,
          );
          return null;
        }),
      api
        .waitVisible('[role="main"]', {
          timeout: TWITTER_TIMEOUTS.ELEMENT_VISIBLE,
        })
        .then(() => "main")
        .catch((e) => {
          logger.debug(`Page detection: main role not visible (${e.message})`);
          return null;
        }),
      api
        .wait(TWITTER_TIMEOUTS.NAVIGATION)
        .then(() => {
          throw new Error("X.com load timeout");
        })
        .catch((e) => {
          logger.debug(`Page detection: navigation timeout (${e.message})`);
          return null;
        }),
    ]),
  ).catch(() => null);

  logger.info(`X.com loaded (${xLoaded || "partial"})`);

  const idleTimeout = xLoaded ? 4000 : 12000;
  // logger.info(`Waiting for network settlement (${idleTimeout}ms)...`);

  try {
    await withPageLock(async () =>
      api.waitForLoadState("networkidle", { timeout: idleTimeout }),
    );
    // logger.info(`Network idle reached.`);
  } catch (_e) {
    // logger.info(`Network active, proceeding after ${idleTimeout}ms...`);
  }

  const currentUrl = await api.getCurrentUrl();
  const onHome =
    currentUrl.includes("/home") ||
    currentUrl === "https://x.com/" ||
    currentUrl === "https://x.com";

  if (!onHome) {
    const scrollDuration = mathUtils.randomInRange(10000, 20000);
    const _scrollDurationSec = (scrollDuration / 1000).toFixed(2);
    // logger.info(`📖 Simulating reading on ${entryName} for ${scrollDurationSec}s...`);

    const scrollStart = Date.now();
    while (Date.now() - scrollStart < scrollDuration) {
      await withPageLock(async () =>
        api.scroll.read(null, {
          pauses: 1,
          scrollAmount: mathUtils.randomInRange(200, 600),
        }),
      );
      await api.waitWithAbort(mathUtils.randomInRange(200, 500), abortSignal);
    }
    // logger.info(`✅ Finished reading, navigating to home...`);
    await withPageLock(async () => agent.navigateHome());
  }

  return { entryName };
}

/**
 * Check login state with retry loop.
 * @returns {Promise<boolean>}
 */
async function checkLoginWithRetry(
  agent,
  api,
  logger,
  abortSignal,
  withPageLock,
) {
  // logger.info(`Checking login state...`);
  let loginCheckDelay = LOGIN_CHECK_DELAY;

  for (let i = 0; i < LOGIN_CHECK_LOOPS; i++) {
    if (abortSignal.aborted) throw new Error("Aborted");

    const loggedIn = await withPageLock(async () => agent.checkLoginState());

    if (loggedIn) {
      logger.info(`✅ Logged in (check ${i + 1}/${LOGIN_CHECK_LOOPS})`);
      return true;
    }

    if (i < LOGIN_CHECK_LOOPS - 1) {
      // logger.info(`Not logged in yet, waiting ${loginCheckDelay}ms...`);
      await api.waitWithAbort(loginCheckDelay, abortSignal);
      loginCheckDelay = Math.min(loginCheckDelay + 1000, 5000);
    }
  }

  logger.warn(`Login check failed after ${LOGIN_CHECK_LOOPS} attempts`);
  return false;
}

/**
 * Log final session statistics (AI stats, queue, engagement progress).
 */
async function logFinalStats({
  getAIStats,
  getQueueStats,
  getEngagementProgress,
  sessionStart,
  abortSignal,
  logger,
}) {
  const aiStatsSnapshot = getAIStats?.();
  if (aiStatsSnapshot) {
    logger.info(`Final AI Stats: ${JSON.stringify(aiStatsSnapshot)}`);
  }

  const queueStatsSnapshot = getQueueStats?.();
  if (!queueStatsSnapshot) return;

  const progressSnapshot = getEngagementProgress?.();
  const sessionStartTime = sessionStart || Date.now();
  const duration = ((Date.now() - sessionStartTime) / 1000 / 60).toFixed(1);

  if (!abortSignal.aborted) {
    try {
      const logConfig = await getLoggingConfig();

      if (
        queueStatsSnapshot &&
        logConfig?.finalStats?.showQueueStatus !== false
      ) {
        logger.info(
          `DiveQueue: queue=${queueStatsSnapshot.queue.queueLength}, active=${queueStatsSnapshot.queue.activeCount}, utilization=${queueStatsSnapshot.queue.utilizationPercent}%`,
        );
      }

      if (
        logConfig?.finalStats?.showEngagement !== false &&
        logConfig?.engagementProgress?.enabled
      ) {
        if (progressSnapshot) {
          logger.info(
            `Engagement Progress: ${formatEngagementSummary(progressSnapshot, logConfig.engagementProgress)}`,
          );
        }
      }
    } catch (loggingError) {
      logger.warn(`Final stats logging error: ${loggingError.message}`);
    }
  }

  logger.info(`Task Finished. Duration: ${duration}m`);
}

/**
 * API-based Twitter Activity Task
 * Full feature parity with original using unified api
 * @param {object} page - Playwright page instance
 * @param {object} payload - Task payload
 * @returns {Promise<object>} Task result
 */
export default async function apiTwitterActivityTask(page, payload) {
  const startTime = process.hrtime.bigint();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger("api-twitterActivity.js");

  logger.info(`Initializing with Unified API...`);

  return await api.withPage(
    page,
    async () => {
      // Initialize API context
      await api.init(page, {
        logger,
        patch: false,
        humanizationPatch: true,
        autoInitNewPages: true,
        colorScheme: "dark",
        sensors: false,
      });

      // Profile resolution
      const resolveProfile = () => {
        const resolved = payload.profileId
          ? profileManager.getById(payload.profileId) ||
            profileManager.getStarter()
          : profileManager.getStarter();
        if (resolved) {
          resolved.persona = resolvePersona(resolved);
          const p = resolved.probabilities || {};
          const inputMethod = resolved.inputMethods
            ? resolved.inputMethods[0]
            : "n/a";
          const profileDesc = `${resolved.id}-${resolved.type} | Input: ${inputMethod} | Dive: ${((p.tweetDive || p.profileDive || 0) * 100).toFixed(0)}% | Like: ${((p.likeTweetafterDive || p.like || 0) * 100).toFixed(0)}% | Follow: ${((p.followOnProfile || p.follow || 0) * 100).toFixed(0)}%`;
          logger.info(`Profile: ${profileDesc}`);
        }
        return resolved;
      };

      // Startup jitter
      const startupJitter = Math.floor(Math.random() * 5000); // 5 seconds
      // logger.info(
      //     `⏳ Startup: Running parallel initialization (Jitter: ${startupJitter}ms)...`
      // );

      // Parallel initialization
      let taskConfig, profile;
      try {
        [taskConfig, profile] = await Promise.all([
          loadAiTwitterActivityConfig(payload),
          Promise.resolve().then(resolveProfile),
          api.wait(startupJitter),
        ]);

        if (taskConfig.system.debugMode) {
          logger.info(
            `Config: ${taskConfig.session.cycles} cycles, reply=${taskConfig.engagement.probabilities.reply}`,
          );
        }
      } catch (initError) {
        logger.error(`Initialization failed: ${initError.message}`);
        throw new Error(`Task initialization failed: ${initError.message}`, {
          cause: initError,
        });
      }

      let agent;
      let hasAgent = false;
      let getAIStats = () => null;
      let getQueueStats = () => null;
      let getEngagementProgress = () => null;
      let sessionStart;
      let popupCloser;
      let hasPopupCloser = false;
      let stopPopupCloser = () => {};
      const abortController = new AbortController();
      const abortSignal = abortController.signal;

      const throwIfAborted = () => {
        if (abortSignal.aborted) {
          const reason =
            abortSignal.reason instanceof Error
              ? abortSignal.reason
              : new Error("Aborted");
          throw reason;
        }
      };

      try {
        const hardTimeoutMs =
          payload.taskTimeoutMs ||
          Math.max(DEFAULT_MIN_DURATION * 1000, DEFAULT_MAX_DURATION * 1000);
        let timeoutId;

        try {
          await Promise.race([
            (async () => {
              for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
                try {
                  throwIfAborted();

                  if (attempt > 0) {
                    const delay = Math.pow(2, attempt) * 1000;
                    // logger.info(
                    //     `Retry ${attempt}/${MAX_RETRIES} in ${delay}ms...`
                    // );
                    await api.waitWithAbort(delay, abortSignal);
                  }

                  throwIfAborted();

                  // Initialize agent
                  const probs = getEngagementProbabilities(taskConfig);
                  const rawActions = taskConfig?.actions ?? {};
                  agent = new AITwitterAgent(page, profile, logger, {
                    replyProbability: probs.reply,
                    quoteProbability: probs.quote,
                    engagementLimits: taskConfig.engagement.limits,
                    config: {
                      ...taskConfig,
                      // Inject .actions so ActionRunner.loadConfig() reads the correct
                      // probabilities from settings.json instead of its hardcoded defaults
                      actions: Object.fromEntries(
                        [
                          "reply",
                          "quote",
                          "like",
                          "bookmark",
                          "retweet",
                          "follow",
                        ].map((name) => [
                          name,
                          {
                            probability: probs[name],
                            enabled: rawActions[name]?.enabled !== false,
                          },
                        ]),
                      ),
                    },
                  });

                  // ─── Unified Action API Executors ─────────────────────────────────
                  // Factory: wraps any api-layer function as a standardised action.execute()
                  const makeApiExecutor =
                    (name, action, apiFn, engType) =>
                    async (context = {}) => {
                      action.stats.attempts++;
                      if (!agent.diveQueue?.canEngage(engType)) {
                        // logger.info(`${name} limit reached, skipping.`);
                        return {
                          success: false,
                          executed: false,
                          reason: "engagement_limit_reached",
                          engagementType: engType,
                        };
                      }
                      try {
                        // logger.info(
                        //     `Delegating ${name} to api.${name}WithAI/API()...`
                        // );
                        const result = await api.withPage(page, () =>
                          apiFn(context),
                        );
                        if (result.success) {
                          action.stats.successes++;
                          agent.diveQueue?.recordEngagement(engType);
                          return {
                            success: true,
                            executed: true,
                            reason: "success",
                            data: result,
                            engagementType: engType,
                          };
                        }
                        action.stats.failures++;
                        return {
                          success: false,
                          executed: true,
                          reason: result.reason || `api_${name}_failed`,
                          data: result,
                          engagementType: engType,
                        };
                      } catch (error) {
                        action.stats.failures++;
                        return {
                          success: false,
                          executed: true,
                          reason: "exception",
                          data: {
                            error: error.message,
                            errorType: error.constructor?.name || "Error",
                            stack: error.stack,
                            code: error.code,
                          },
                          engagementType: engType,
                        };
                      }
                    };

                  // Map: actionName → { engagementType, apiFn }
                  const ACTION_API_MAP = {
                    reply: { eng: "replies", fn: () => api.replyWithAI() },
                    quote: { eng: "quotes", fn: () => api.quoteWithAI() },
                    retweet: {
                      eng: "retweets",
                      fn: (ctx) => retweetWithAPI(ctx),
                    },
                    like: { eng: "likes", fn: (ctx) => likeWithAPI(ctx) },
                    bookmark: {
                      eng: "bookmarks",
                      fn: (ctx) => bookmarkWithAPI(ctx),
                    },
                    follow: { eng: "follows", fn: (ctx) => followWithAPI(ctx) },
                  };

                  for (const [name, { eng, fn }] of Object.entries(
                    ACTION_API_MAP,
                  )) {
                    const action = agent.actions[name];
                    if (!action) continue;
                    action.execute = makeApiExecutor(name, action, fn, eng);
                    // Disable legacy context pre-fetching; api handlers manage context internally
                    action.needsContext = false;
                  }
                  // ─────────────────────────────────────────────────────────────────

                  hasAgent = true;
                  getAIStats = agent.getAIStats.bind(agent);
                  getQueueStats = () => agent.diveQueue?.getFullStatus?.();
                  getEngagementProgress = () =>
                    agent.diveQueue?.getEngagementProgress?.();
                  sessionStart = agent.sessionStart;

                  const withPageLock = async (fn, opts) => {
                    const res = await agent.diveQueue.add(fn, opts);
                    return res.success ? res.result : null;
                  };

                  if (taskConfig.system.debugMode) {
                    logger.info(`AITwitterAgent initialized`);
                  }

                  // Setup persona, theme, and idle simulation
                  await setupEnvironment(profile, api, logger, withPageLock);

                  // Popup closer
                  if (!popupCloser) {
                    popupCloser = new PopupCloser(page, logger, {
                      lock: withPageLock,
                      signal: abortSignal,
                      api,
                    });
                    stopPopupCloser = async () => {
                      try {
                        await popupCloser.stop();
                      } catch (error) {
                        logger.warn(
                          `Popup closer stop failed: ${error.message}`,
                        );
                      }
                    };
                    hasPopupCloser = true;
                    await popupCloser.start();
                  }

                  // Warmup
                  const warmup = getWarmupTiming(taskConfig);
                  const wakeUp = humanTiming.getWarmupDelay(warmup);
                  // logger.info(`Warm-up ${humanTiming.formatDuration(wakeUp)}...`);
                  await api.waitWithAbort(wakeUp, abortSignal);

                  throwIfAborted();

                  // Navigation and reading simulation
                  await navigateAndRead(
                    agent,
                    selectEntryPoint(),
                    api,
                    logger,
                    withPageLock,
                    abortSignal,
                  );

                  throwIfAborted();
                  await checkLoginWithRetry(
                    agent,
                    api,
                    logger,
                    abortSignal,
                    withPageLock,
                  );

                  throwIfAborted();
                  const { cycles, minDuration, maxDuration } =
                    taskConfig.session;

                  logger.info(
                    `Starting session (${cycles} cycles, ${minDuration}-${maxDuration}s)...`,
                  );

                  let sessionSuccess = false;
                  try {
                    await agent.runSession(cycles, minDuration, maxDuration, {
                      abortSignal,
                    });
                    sessionSuccess = true;
                  } catch (sessionError) {
                    sessionSuccess = false;
                    if (abortSignal.aborted) {
                      throw sessionError;
                    }
                    logger.warn(`Session error: ${sessionError.message}`);
                    try {
                      if (agent && agent.page && !agent.page.isClosed()) {
                        await withPageLock(async () => agent.navigateHome());
                        logger.info("Recovered to home page");
                      }
                    } catch (recoveryError) {
                      logger.warn(
                        `Recovery attempt failed: ${recoveryError.message}`,
                      );
                    }
                  }

                  if (sessionSuccess) {
                    logger.info(`Session completed successfully`);
                  } else {
                    logger.warn(`Session completed with errors`);
                  }
                  return;
                } catch (innerError) {
                  logger.warn(
                    `Attempt ${attempt + 1} failed: ${innerError.message}`,
                  );
                  if (attempt === MAX_RETRIES) throw innerError;
                }
              }
            })(),
            new Promise((_, reject) => {
              timeoutId = setTimeout(() => {
                const timeoutError = new Error("Timeout");
                abortController.abort(timeoutError);
                reject(timeoutError);
              }, hardTimeoutMs);
            }),
          ]);
        } finally {
          if (timeoutId) clearTimeout(timeoutId);
        }
      } catch (error) {
        logger.error(`Error: ${error.message}`);
      } finally {
        if (api.idle.isRunning()) {
          api.idle.stop();
        }

        if (hasAgent) {
          await logFinalStats({
            getAIStats,
            getQueueStats,
            getEngagementProgress,
            sessionStart,
            abortSignal,
            logger,
          });

          if (typeof agent.shutdown === "function") {
            await agent.shutdown();
          }
        }

        try {
          if (hasPopupCloser) {
            await stopPopupCloser();
            hasPopupCloser = false;
          }
        } catch (closeError) {
          logger.warn(`Cleanup warning: ${closeError.message}`);
        }

        api.clearContext();
        const duration = (
          Number(process.hrtime.bigint() - startTime) / 1e9
        ).toFixed(2);
        logger.info(`Done in ${duration}s`);
      }
    },
    { sessionId: browserInfo },
  );
}
