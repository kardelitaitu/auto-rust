/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Game Agent Runner
 * Enhanced agent loop for strategy games with:
 * - Action verification (did the click work?)
 * - Visual state change detection
 * - Retry on failure
 * - Coordinate-based actions
 * - Faster step delay for games
 * @module api/agent/gameRunner
 */

import { createLogger } from "../core/logger.js";
import { getPage } from "../core/context.js";
import { llmClient } from "./llmClient.js";
import { actionEngine } from "./actionEngine.js";
import { estimateConversationTokens as _estimateConversationTokens } from "./tokenCounter.js";
import { configManager as _configManager } from "../core/config.js";
import { visualDiffEngine } from "./visualDiff.js";
import { adaptiveTiming } from "./adaptiveTiming.js";
import { goalDecomposer } from "./goalDecomposer.js";
import { sessionStore } from "./sessionStore.js";
import { progressTracker } from "./progressTracker.js";
import { actionRollback } from "./actionRollback.js";
import { semanticMapper } from "./semanticMapper.js";
import { parallelExecutor } from "./parallelExecutor.js";
import {
  VisionPreprocessor,
  VPrepPresets,
} from "../utils/vision-preprocessor.js";

const logger = createLogger("api/agent/gameRunner.js");

const GAME_SYSTEM_PROMPT = `You are a strategy game automation agent.

## Your Task
Analyze the screenshot and accessibility tree to understand the current game state, then take actions to complete the goal.

## Available Actions
Respond with a JSON object or array of objects:

\`\`\`json
// Single action
{ "action": "click", "selector": "#build-btn", "rationale": "Click build button to start construction" }

// Coordinate click (for canvas games)
{ "action": "clickAt", "x": 450, "y": 320, "clickType": "single", "rationale": "Click on unit at coordinates" }

// Type text
{ "action": "type", "selector": "#chat-input", "value": "Hello team!", "rationale": "Send message to team chat" }

// Wait for animation
{ "action": "wait", "value": "1000", "rationale": "Wait for build animation to complete" }

// Verify something happened
{ "action": "verify", "description": "Check if barracks appeared", "rationale": "Confirm building was placed" }

// Task complete
{ "action": "done", "rationale": "All 5 footmen have been trained" }
\`\`\`

## Reasoning Process
Before taking action, think through:
1. **Current State**: What is the current state of the page?
2. **Obstacles**: What obstacles or challenges exist?
3. **Best Action**: What is the best next action to achieve the goal?
4. **Risks**: What could go wrong with this action?
5. **Verification**: How will I verify this action succeeded?

## Rules
1. Output ONLY valid JSON - no markdown, no explanation outside JSON
2. Always include "rationale" explaining your tactical decision
3. After clicking, add a "verify" action to confirm it worked
4. Use coordinates (clickAt) for canvas-based games
5. Use "wait" with 500-1500ms for animations
6. If stuck, try alternative approaches before giving up
7. Think step-by-step before acting

## Example Response
\`\`\`json
[
  { "action": "click", "selector": ".unit-card[data-type='footman']", "rationale": "Select footman unit to train" },
  { "action": "wait", "value": "800", "rationale": "Wait for selection animation" },
  { "action": "click", "selector": "#train-btn", "rationale": "Click train button to start training" },
  { "action": "verify", "description": "Check if training started", "rationale": "Confirm action succeeded" }
]
\`\`\``;

class GameAgentRunner {
  constructor() {
    this.isRunning = false;
    this.currentGoal = null;
    this.history = [];
    this.maxHistorySize = 20; // Keep last 10 exchanges (20 messages)
    this.maxSteps = 30;
    this.stepDelay = 500;
    this.verifyAction = true;
    this.retryOnFail = true;
    this.maxRetries = 3;
    this.lastAction = null;
    this.lastState = null;
    this.consecutiveFailures = 0;
    this.stuckDetection = true;
    this.useAXTree = true;
    this.maxAttemptsWithoutChange = 5;
    this.lastProgressStep = 0;
    this._abortController = null;

    // Screenshot caching
    this.screenshotCache = null;
    this.screenshotCacheTime = 0;
    this.screenshotCacheTTL = 1000; // 1 second TTL

    // V-PREP coordinate scaling
    this.vprepScaleFactor = 1.0; // Scale factor for coordinate mapping
    this.originalViewport = { width: 1280, height: 720 };

    // Action memoization
    this.actionCache = new Map();
    this.maxActionCacheSize = 1000;
  }

  /**
   * Generate cache key for action memoization
   * @private
   */
  _getActionCacheKey(url, goal, element) {
    const elementHash = this._hashElement(element);
    return `${url}|${goal}|${elementHash}`;
  }

  /**
   * Simple hash function for elements
   * @private
   */
  _hashElement(element) {
    if (!element) return "null";
    const str = JSON.stringify(element);
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = (hash << 5) - hash + char;
      hash = hash & hash;
    }
    return hash.toString(36);
  }

  /**
   * Get or compute action with memoization
   * @private
   */
  async _getOrComputeAction(url, goal, element, computeFn) {
    const key = this._getActionCacheKey(url, goal, element);

    if (this.actionCache.has(key)) {
      logger.info(`[Memoizer] Cache hit for action`);
      return this.actionCache.get(key);
    }

    const result = await computeFn();
    this.actionCache.set(key, result);

    // Evict oldest if over limit
    if (this.actionCache.size > this.maxActionCacheSize) {
      const firstKey = this.actionCache.keys().next().value;
      this.actionCache.delete(firstKey);
    }

    return result;
  }

  /**
   * Save stuck screenshot to logs
   * @private
   */
  async _saveStuckScreenshot(page, stepCount) {
    try {
      const fs = await import("fs/promises");
      const path = await import("path");

      const logsDir = path.resolve(process.cwd(), "logs");
      await fs.mkdir(logsDir, { recursive: true }).catch(() => {});

      const filename = path.join(
        logsDir,
        `stuck-${Date.now()}-step${stepCount}.png`,
      );
      await page.screenshot({ path: filename });
      logger.info(`Stuck screenshot saved: ${filename}`);

      return filename;
    } catch (e) {
      logger.warn("Failed to save stuck screenshot:", e.message);
      return null;
    }
  }

  /**
   * Check if agent is stuck (no progress)
   * @private
   */
  _checkStuck(currentStep) {
    const attemptsWithoutChange = currentStep - this.lastProgressStep;

    if (attemptsWithoutChange >= this.maxAttemptsWithoutChange) {
      logger.error(
        `Agent stuck: No progress for ${attemptsWithoutChange} attempts`,
      );
      return true;
    }

    return false;
  }

  /**
   * Run the game agent with a goal
   * @param {string} goal - The goal to accomplish
   * @param {object} config - Configuration options
   * @returns {Promise<object>} Result with success and stats
   */
  async run(goal, config = {}) {
    if (this.isRunning) {
      throw new Error("Game Agent is already running. Call stop() first.");
    }

    await llmClient.init();

    this.isRunning = true;
    this.currentGoal = goal;
    this.history = [];
    this.maxSteps = config.maxSteps || 30;
    this.stepDelay = config.stepDelay || 500;
    this.verifyAction = config.verifyAction ?? true; // Default true for action verification
    this.retryOnFail = config.retryOnFail !== false;
    this.maxRetries = config.maxRetries || 3;
    this.stuckDetection = config.stuckDetection ?? true;
    this.useAXTree = config.useAXTree ?? true;
    this.maxAttemptsWithoutChange = config.maxAttemptsWithoutChange || 5;
    this.lastAction = null;
    this.lastState = null;
    this.consecutiveFailures = 0;
    this.lastProgressStep = 0;
    this._abortController = new AbortController();

    const page = getPage();
    if (!page) {
      throw new Error("No page available. Call api.init(page) first.");
    }

    logger.info(
      `Starting Game Agent: "${goal}" (Max: ${this.maxSteps} steps, ${this.stepDelay}ms delay)`,
    );

    // Start progress tracking
    progressTracker.startSession(goal);

    // Measure site performance for adaptive timing
    const timingProfile = await adaptiveTiming.measureSitePerformance(page);
    logger.info(
      `[AdaptiveTiming] Using timing profile: click=${timingProfile.click}ms, wait=${timingProfile.waitMultiplier.toFixed(2)}x`,
    );

    // Decompose complex goal into sub-goals
    let decomposition = await goalDecomposer.decompose(goal, page.url());
    logger.info(
      `[GoalDecomposer] Decomposed into ${decomposition.totalSteps} steps (${decomposition.pattern} pattern)`,
    );

    let stepCount = 0;
    let lastResult = null;
    let preActionState;
    let actionSuccess = false;

    while (stepCount < this.maxSteps && !this._abortController.signal.aborted) {
      stepCount++;
      logger.info(`--- Step ${stepCount}/${this.maxSteps} ---`);

      try {
        await page.bringToFront();
      } catch (e) {
        logger.debug("bringToFront failed:", e.message);
      }

      preActionState = await this._captureState(page);

      // Update progress tracker with current URL
      progressTracker.updateUrl(preActionState.url);

      logger.info(
        `[DEBUG] Screenshot size: ${preActionState.screenshot?.length || 0} chars, URL: ${preActionState.url}`,
      );

      // Get current sub-goal for LLM prompt
      const currentSubGoal = goalDecomposer.getNextStep(decomposition);
      const effectiveGoal = currentSubGoal ? currentSubGoal.subGoal : goal;

      const messages = this._buildPrompt(
        effectiveGoal,
        preActionState.screenshot,
        preActionState.axTree,
        preActionState.url,
      );

      let llmResponse;
      try {
        const msgLen = messages[messages.length - 1].content?.length || 0;
        logger.info(
          `[DEBUG] Sending ${messages.length} messages, last msg length: ${msgLen}`,
        );

        // Use memoization for LLM calls
        const startTime = Date.now();
        llmResponse = await this._getOrComputeAction(
          preActionState.url,
          goal,
          { axTree: preActionState.axTree?.substring(0, 500) }, // Use first 500 chars of AXTree as key
          async () => llmClient.generateCompletionWithRetry(messages, 3),
        );

        logger.info(
          `[DEBUG] LLM Response keys: ${Object.keys(llmResponse).join(", ")}`,
        );
        logger.info(`LLM Decision: ${JSON.stringify(llmResponse)}`);

        // Record LLM call success
        progressTracker.recordLlmCall(true, Date.now() - startTime);
      } catch (_e) {
        logger.error("LLM Failure:", _e.message);

        // Record LLM call failure
        progressTracker.recordLlmCall(false, 0);
        this.consecutiveFailures++;
        if (this.consecutiveFailures >= 3) {
          logger.error("Aborting after 3 consecutive LLM failures.");
          break;
        }
        await page.waitForTimeout(3000);
        continue;
      }

      if (
        !llmResponse ||
        (typeof llmResponse !== "object" && !Array.isArray(llmResponse))
      ) {
        logger.warn("Invalid LLM response. Retrying...");
        continue;
      }

      const actionsToExecute = Array.isArray(llmResponse)
        ? llmResponse
        : [llmResponse];
      let _sequenceSuccess = true;

      // Check if we can use parallel execution
      if (parallelExecutor.canParallelize(actionsToExecute)) {
        const speedup = parallelExecutor.estimateSpeedup(actionsToExecute);
        logger.info(
          `[ParallelExecutor] Using parallel execution (estimated speedup: ${speedup.speedup}x)`,
        );

        // Execute actions in parallel
        const executeFn = async (actionObj) => {
          if (!actionObj.action) {
            return { success: false, error: "No action specified" };
          }

          if (actionObj.action === "done") {
            return { success: true, done: true };
          }

          // Scale coordinates if V-PREP was applied
          const scaledAction = this._scaleActionCoordinates(actionObj);

          // Capture pre-state for rollback if action is critical
          let preState = null;
          if (actionRollback.isCriticalAction(scaledAction)) {
            preState = await actionRollback.capturePreState(page, scaledAction);
          }

          const result = await actionEngine.execute(
            page,
            scaledAction,
            config.sessionId || "game",
          );

          // Record action for potential rollback
          if (preState) {
            actionRollback.recordAction(preState, actionObj, result);
          }

          // Invalidate screenshot cache after any action that changes the page
          if (
            result.success &&
            actionObj.action !== "wait" &&
            actionObj.action !== "verify"
          ) {
            this._invalidateScreenshotCache();
          }

          return result;
        };

        const parallelResults = await parallelExecutor.executeSequence(
          actionsToExecute,
          executeFn,
        );

        // Process parallel results
        for (const { action: actionObj, result } of parallelResults) {
          if (!actionObj.action) continue;

          if (actionObj.action === "done") {
            logger.info("Game Agent completed the task successfully.");
            this.isRunning = false;
            return { success: true, done: true, steps: stepCount };
          }

          lastResult = result;

          // Record step in progress tracker
          progressTracker.recordStep(stepCount, actionObj, result);

          logger.info(
            `[DEBUG] Action result: success=${result.success}, error=${result.error || "none"}`,
          );

          if (result.success) {
            if (
              this.verifyAction &&
              actionObj.action !== "wait" &&
              actionObj.action !== "verify"
            ) {
              logger.info(`[DEBUG] Verifying action...`);
              const verified = await this._verifyAction(
                page,
                preActionState,
                actionObj,
              );
              if (verified) {
                logger.info(`Action verified successfully`);
                actionSuccess = true;
                this.consecutiveFailures = 0;
              } else {
                logger.warn(`Action verification failed`);
                actionSuccess = false;
                this.consecutiveFailures++;
                _sequenceSuccess = false;
              }
            } else {
              actionSuccess = true;
              this.consecutiveFailures = 0;
            }
          } else {
            logger.error(`Action failed: ${result.error}`);
            this.consecutiveFailures++;
            actionSuccess = false;
            _sequenceSuccess = false;
          }
        }
      } else {
        // Execute actions sequentially (original logic)
        for (let i = 0; i < actionsToExecute.length; i++) {
          const actionObj = actionsToExecute[i];

          if (!actionObj.action) {
            logger.warn("Invalid action object in sequence. Skipping.");
            continue;
          }

          if (actionObj.action === "done") {
            logger.info("Game Agent completed the task successfully.");
            this.isRunning = false;
            return { success: true, done: true, steps: stepCount };
          }

          // Scale coordinates if V-PREP was applied
          const scaledAction = this._scaleActionCoordinates(actionObj);

          // Capture pre-state for rollback if action is critical
          let preState = null;
          if (actionRollback.isCriticalAction(scaledAction)) {
            preState = await actionRollback.capturePreState(page, scaledAction);
          }

          const result = await actionEngine.execute(
            page,
            scaledAction,
            config.sessionId || "game",
          );
          lastResult = result;

          // Record action for potential rollback
          if (preState) {
            actionRollback.recordAction(preState, actionObj, result);
          }

          // Invalidate screenshot cache after any action that changes the page
          if (
            result.success &&
            actionObj.action !== "wait" &&
            actionObj.action !== "verify"
          ) {
            this._invalidateScreenshotCache();
          }

          // Record step in progress tracker
          progressTracker.recordStep(stepCount, actionObj, result);

          logger.info(
            `[DEBUG] Action result: success=${result.success}, error=${result.error || "none"}`,
          );

          if (result.success) {
            if (
              this.verifyAction &&
              actionObj.action !== "wait" &&
              actionObj.action !== "verify"
            ) {
              logger.info(`[DEBUG] Verifying action...`);
              const verified = await this._verifyAction(
                page,
                preActionState,
                actionObj,
              );
              if (verified) {
                logger.info(`Action verified successfully`);
                actionSuccess = true;
                this.consecutiveFailures = 0;
              } else {
                logger.warn(`Action verification failed`);
                actionSuccess = false;
                this.consecutiveFailures++;
                _sequenceSuccess = false;
                break; // Stop sequence on verification failure
              }
            } else {
              actionSuccess = true;
              this.consecutiveFailures = 0;
            }
          } else {
            logger.error(`Action failed: ${result.error}`);
            this.consecutiveFailures++;
            actionSuccess = false;
            _sequenceSuccess = false;

            // Attempt rollback on critical action failure
            if (
              actionRollback.isCriticalAction(actionObj) &&
              this.consecutiveFailures >= 2
            ) {
              logger.warn(
                "[Rollback] Attempting rollback due to consecutive failures...",
              );
              const rolledBack = await actionRollback.rollbackLast(page);
              if (rolledBack) {
                logger.info("[Rollback] Rollback successful, continuing...");
                this.consecutiveFailures = 0; // Reset after successful rollback
              }
            }

            break; // Stop sequence on execution failure
          }

          // Add a small delay between actions in an array
          if (i < actionsToExecute.length - 1 && actionSuccess) {
            await page.waitForTimeout(1000);
            // Update preActionState for the next verify in the sequence
            preActionState = await this._captureState(page);
          }
        }
      }

      if (!actionSuccess) {
        logger.error(
          `Action sequence failed after ${this.maxRetries} attempts`,
        );
        if (this.consecutiveFailures >= 5) {
          logger.error("Too many consecutive failures. Stopping.");
          break;
        }
      }

      if (actionSuccess) {
        this.lastProgressStep = stepCount;

        // Advance to next sub-goal if current one completed
        if (currentSubGoal && llmResponse?.action === "done") {
          decomposition = goalDecomposer.advanceStep(decomposition);
          const progress = goalDecomposer.getProgress(decomposition);
          logger.info(
            `[GoalDecomposer] Progress: ${progress}% (${decomposition.currentStep}/${decomposition.totalSteps})`,
          );

          // Check if all sub-goals completed
          if (goalDecomposer.isComplete(decomposition)) {
            logger.info("[GoalDecomposer] All sub-goals completed!");
          }
        }
      }

      if (this.stuckDetection && this._checkStuck(stepCount)) {
        const screenshotPath = await this._saveStuckScreenshot(page, stepCount);
        this.isRunning = false;

        // Record stuck in progress tracker
        progressTracker.recordStuck(
          stepCount,
          `No state change after ${this.maxAttemptsWithoutChange} attempts`,
        );

        throw new Error(
          `Agent stuck - no state change after ${this.maxAttemptsWithoutChange} attempts. ` +
            `Last successful step: ${this.lastProgressStep}, Current step: ${stepCount}. ` +
            `Screenshot saved: ${screenshotPath || "none"}`,
        );
      }

      this.history.push({
        role: "assistant",
        content: JSON.stringify(llmResponse),
      });
      this.history.push({
        role: "user",
        content: actionSuccess
          ? "Action succeeded."
          : `Action failed: ${lastResult?.error}`,
      });

      // Trim history if it exceeds maxHistorySize
      if (this.history.length > this.maxHistorySize) {
        this.history = this.history.slice(-this.maxHistorySize);
      }

      // Use adaptive timing for step delay
      const adjustedDelay = adaptiveTiming.getAdjustedDelay(
        preActionState.url,
        "navigation",
        this.stepDelay,
      );
      await page.waitForTimeout(adjustedDelay);
    }

    this.isRunning = false;

    const finalSuccess = actionSuccess === undefined ? false : actionSuccess;
    const result = {
      success: finalSuccess,
      done: false,
      steps: stepCount,
      reason: stepCount >= this.maxSteps ? "max_steps" : "stopped",
    };

    // Record session for persistence
    const endTime = Date.now();
    const durationMs = endTime - this.startTime;
    sessionStore.recordSession({
      id: `session-${Date.now()}`,
      goal: this.currentGoal,
      url: page.url(),
      success: finalSuccess,
      steps: stepCount,
      durationMs,
    });

    // Complete progress tracking
    progressTracker.completeSession(finalSuccess, result.reason);

    logger.info(
      `[SessionStore] Session recorded: ${finalSuccess ? "SUCCESS" : "FAILED"} in ${stepCount} steps (${durationMs}ms)`,
    );

    return result;
  }

  /**
   * Verify action had effect using multiple strategies
   * @private
   */
  async _verifyAction(page, preState, action) {
    if (!this.useAXTree) {
      logger.debug(
        `Verification bypassed (AXTree disabled) for action: ${action.action}`,
      );
      return true;
    }

    try {
      await page.waitForTimeout(300);
      const postState = await this._captureState(page);

      if (action.action === "click" || action.action === "clickAt") {
        // Multiple verification strategies
        const verifications = [
          this._compareAXTree(preState.axTree, postState.axTree),
          this._compareUrl(preState.url, postState.url),
          await this._compareVisual(preState.screenshot, postState.screenshot),
        ];

        // At least one verification should pass
        const passed = verifications.filter((v) => v === true).length;
        logger.debug(`Verification results: ${passed}/3 passed`);

        return passed >= 1;
      }

      if (action.action === "type") {
        // Check if typed text appears in AXTree
        if (this.useAXTree && postState.axTree.includes(action.value)) {
          return true;
        }
        // Fallback: check URL or visual change
        return (
          this._compareUrl(preState.url, postState.url) ||
          (await this._compareVisual(preState.screenshot, postState.screenshot))
        );
      }

      return true;
    } catch (e) {
      logger.warn("Verification error:", e.message);
      return false; // Fail closed
    }
  }

  /**
   * Compare URLs for navigation detection
   * @private
   */
  _compareUrl(preUrl, postUrl) {
    return preUrl !== postUrl;
  }

  /**
   * Visual comparison using Visual Diff Engine
   * @private
   */
  async _compareVisual(preScreenshot, postScreenshot) {
    if (!preScreenshot || !postScreenshot) return false;

    try {
      // Convert base64 to buffers
      const preBuffer = Buffer.from(preScreenshot, "base64");
      const postBuffer = Buffer.from(postScreenshot, "base64");

      const result = await visualDiffEngine.compareScreenshots(
        preBuffer,
        postBuffer,
        {
          threshold: 0.05, // 5% threshold
          minPixels: 50,
        },
      );

      logger.debug(
        `[VisualDiff] Changed: ${result.changed}, Ratio: ${result.diffRatio.toFixed(4)}, Method: ${result.method}`,
      );
      return result.changed;
    } catch (error) {
      logger.warn(
        "Visual diff comparison failed, using fallback:",
        error.message,
      );
      // Fallback to simple length comparison
      const diff = Math.abs(preScreenshot.length - postScreenshot.length);
      return diff > 100;
    }
  }

  /**
   * Simple AXTree comparison
   * @private
   */
  _compareAXTree(tree1, tree2) {
    const stripped1 = this._stripDynamicContent(tree1);
    const stripped2 = this._stripDynamicContent(tree2);
    return stripped1 !== stripped2;
  }

  /**
   * Strip dynamic content for comparison
   * @private
   */
  _stripDynamicContent(treeJson) {
    let text =
      typeof treeJson === "string" ? treeJson : JSON.stringify(treeJson);
    text = text.replace(/"\d{10,14}"/g, '"[TIMESTAMP]"');
    text = text.replace(/"id":\s*"[^"]{15,}"/g, '"id":"[DYNAMIC_ID]"');
    return text;
  }

  /**
   * Capture current page state
   * @private
   */
  async _captureState(page) {
    const screenshot = await this._captureScreenshot(page);
    const axTree = this.useAXTree
      ? await this._captureAXTree(page)
      : "[AXTree Disabled]";
    const url = page.url();

    return { screenshot, axTree, url };
  }

  /**
   * Capture screenshot with caching and V-PREP optimization
   * @private
   * @param {boolean} forceRefresh - Force new screenshot capture
   * @returns {Promise<string>} Base64 encoded screenshot
   */
  async _captureScreenshot(page, forceRefresh = false) {
    const now = Date.now();

    // Return cached if fresh enough
    if (
      !forceRefresh &&
      this.screenshotCache &&
      now - this.screenshotCacheTime < this.screenshotCacheTTL
    ) {
      logger.debug(
        `[LLM Image] Using cached screenshot (${Math.round(now - this.screenshotCacheTime)}ms old)`,
      );
      return this.screenshotCache;
    }

    try {
      let viewport = page.viewportSize();
      if (!viewport) {
        viewport = await page.evaluate(() => ({
          width: window.innerWidth,
          height: window.innerHeight,
        }));
      }

      // Store original viewport for coordinate scaling
      this.originalViewport = {
        width: viewport.width,
        height: viewport.height,
      };

      // Capture raw screenshot
      const rawBuffer = await page.screenshot({
        type: "jpeg",
        quality: 90,
        scale: "css",
        timeout: 5000,
      });

      const rawSizeKB = Math.round(rawBuffer.length / 1024);
      logger.info(
        `[LLM Image] Viewport: ${viewport.width}x${viewport.height}, Raw: ${rawSizeKB} KB`,
      );

      // Apply V-PREP optimization for better LLM vision
      const vprep = new VisionPreprocessor();
      const result = await vprep.process(rawBuffer, VPrepPresets.OWB_GAME);

      // Calculate scale factor for coordinate mapping
      // V-PREP resizes to targetWidth (1024), so scale = original / target
      const targetWidth = VPrepPresets.OWB_GAME.targetWidth || 1024;
      this.vprepScaleFactor = viewport.width / targetWidth;

      const sizeKB = Math.round(result.base64.length / 1024);
      logger.info(
        `[LLM Image] V-PREP: ${rawSizeKB}KB → ${sizeKB}KB (${result.stats.compressionRatio}x), Scale: ${this.vprepScaleFactor.toFixed(3)} (LLM coords × ${this.vprepScaleFactor.toFixed(2)} = browser coords)`,
      );

      // Update cache with V-PREP processed image
      this.screenshotCache = result.base64;
      this.screenshotCacheTime = now;

      return result.base64;
    } catch (e) {
      logger.error("Screenshot failed:", e.message);
      this.vprepScaleFactor = 1.0; // No scaling if V-PREP fails
      // Fallback to basic screenshot without V-PREP
      try {
        const fallbackBuffer = await page.screenshot({
          type: "jpeg",
          quality: 80,
          scale: "css",
          timeout: 5000,
        });
        return fallbackBuffer.toString("base64");
      } catch (e2) {
        logger.error("Fallback screenshot also failed:", e2.message);
        return "";
      }
    }
  }

  /**
   * Invalidate screenshot cache (call after actions that change the page)
   * @private
   */
  _invalidateScreenshotCache() {
    this.screenshotCache = null;
    this.screenshotCacheTime = 0;
  }

  /**
   * Scale action coordinates from LLM space to browser space
   * When V-PREP resizes the image, LLM coordinates need to be scaled back
   * @private
   * @param {object} action - Action object from LLM
   * @returns {object} Action with scaled coordinates
   */
  _scaleActionCoordinates(action) {
    if (!action || this.vprepScaleFactor === 1.0) {
      return action; // No scaling needed
    }

    // Deep clone to avoid modifying original
    const scaledAction = JSON.parse(JSON.stringify(action));

    // Scale clickAt coordinates
    if (scaledAction.action === "clickAt") {
      if (typeof scaledAction.x === "number") {
        const originalX = scaledAction.x;
        scaledAction.x = Math.round(scaledAction.x * this.vprepScaleFactor);
        logger.debug(
          `[V-PREP Scale] X: ${originalX} → ${scaledAction.x} (×${this.vprepScaleFactor.toFixed(3)})`,
        );
      }
      if (typeof scaledAction.y === "number") {
        const originalY = scaledAction.y;
        scaledAction.y = Math.round(scaledAction.y * this.vprepScaleFactor);
        logger.debug(
          `[V-PREP Scale] Y: ${originalY} → ${scaledAction.y} (×${this.vprepScaleFactor.toFixed(3)})`,
        );
      }
    }

    // Scale drag coordinates (has source and target)
    if (scaledAction.action === "drag") {
      if (typeof scaledAction.x === "number") {
        scaledAction.x = Math.round(scaledAction.x * this.vprepScaleFactor);
      }
      if (typeof scaledAction.y === "number") {
        scaledAction.y = Math.round(scaledAction.y * this.vprepScaleFactor);
      }
      if (scaledAction.targetX !== undefined) {
        scaledAction.targetX = Math.round(
          scaledAction.targetX * this.vprepScaleFactor,
        );
      }
      if (scaledAction.targetY !== undefined) {
        scaledAction.targetY = Math.round(
          scaledAction.targetY * this.vprepScaleFactor,
        );
      }
    }

    return scaledAction;
  }

  /**
   * Capture accessibility tree with incremental extraction and semantic enrichment
   * @private
   * @param {boolean} compact - If true, return only interactive elements for LLM
   */
  async _captureAXTree(page, compact = true) {
    try {
      const tree = await page.accessibility.snapshot();
      const fullTree = JSON.stringify(tree, null, 2);

      // Store full tree for verification
      this.lastFullAXTree = fullTree;

      if (compact) {
        // Extract only interactive elements for LLM
        const compactTree = this._extractInteractiveElements(tree);

        // Enrich with semantic mapping
        const enrichedTree = semanticMapper.enrichAXTree(compactTree);

        // Add page summary
        const summary = semanticMapper.getPageSummary(enrichedTree);
        enrichedTree._pageSummary = summary;

        return JSON.stringify(enrichedTree, null, 2);
      }

      return fullTree;
    } catch (e) {
      logger.error("AXTree capture failed:", e.message);
      return "";
    }
  }

  /**
   * Extract only interactive elements from AXTree for LLM consumption
   * @private
   */
  _extractInteractiveElements(tree, depth = 0) {
    if (!tree) return null;

    const result = {
      role: tree.role,
      name: tree.name,
    };

    // Only include interactive roles
    const interactiveRoles = [
      "button",
      "link",
      "textbox",
      "checkbox",
      "radio",
      "menuitem",
      "tab",
      "combobox",
      "slider",
    ];
    if (interactiveRoles.includes(tree.role)) {
      if (tree.selector) result.selector = tree.selector;
      if (tree.value) result.value = tree.value;
    }

    if (tree.children && depth < 3) {
      // Limit depth to reduce size
      result.children = tree.children
        .map((child) => this._extractInteractiveElements(child, depth + 1))
        .filter(Boolean);
    }

    return result;
  }

  /**
   * Build prompt for LLM
   * @private
   */
  _buildPrompt(goal, screenshot, axTree, currentUrl) {
    const systemMessage = {
      role: "system",
      content: GAME_SYSTEM_PROMPT,
    };

    const recentHistory = this.history.slice(-4);

    const contentParts = [
      { type: "text", text: `Goal: ${goal}` },
      { type: "text", text: `Current URL: ${currentUrl}` },
      {
        type: "text",
        text: `CRITICAL: You MUST respond ONLY with raw JSON. Do NOT wrap it in markdown \`\`\` blocks. Start directly with { or [ and end with } or ]`,
      },
    ];

    if (this.useAXTree && axTree && axTree !== "[AXTree Disabled]") {
      contentParts.push({
        type: "text",
        text: `Current Page State (AXTree):\n${axTree.substring(0, 2000)}`,
      });
    } else {
      contentParts.push({
        type: "text",
        text: `AXTree: Disabled for this session. Use visual information only.`,
      });
    }

    const config = llmClient.config;
    if (config?.useVision !== false) {
      contentParts.push({
        type: "text",
        text: "Analyze the screenshot to understand the game state.",
      });
      contentParts.push({
        type: "image_url",
        image_url: { url: `data:image/jpeg;base64,${screenshot}` },
      });
    }

    const userMessage = { role: "user", content: contentParts };

    return [systemMessage, ...recentHistory, userMessage];
  }

  /**
   * Stop the running agent
   */
  stop() {
    if (this._abortController) {
      this._abortController.abort();
    }
    this.isRunning = false;
    logger.info("Game Agent stopped.");
  }

  /**
   * Get usage statistics
   */
  getUsageStats() {
    return {
      isRunning: this.isRunning,
      goal: this.currentGoal,
      steps: this.history.length / 2,
      maxSteps: this.maxSteps,
      historySize: this.history.length,
    };
  }
}

const gameAgentRunner = new GameAgentRunner();

export { gameAgentRunner };
export default gameAgentRunner;
