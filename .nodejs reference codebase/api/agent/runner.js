/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Agent Runner - Main Agent Logic (The Brain)
 * Orchestrates the Perception-Action loop with LLM
 * Merged from local-agent/core/agent.js
 * @module api/agent/runner
 */

import { createLogger } from "../core/logger.js";
import { getPage } from "../core/context.js";
import { llmClient } from "./llmClient.js";
import { actionEngine } from "./actionEngine.js";
import { estimateConversationTokens } from "./tokenCounter.js";
import { configManager } from "../core/config.js";

const logger = createLogger("api/agent/runner.js");

const DEFAULT_SYSTEM_PROMPT = `You are a browser automation agent.

You will receive an Accessibility Tree (JSON/Text structure) of the page.
Use it to identify elements and their selectors.

Output strictly valid JSON.
Available actions:
- { "action": "navigate", "value": "https://..." }
- { "action": "wait", "value": "5000" } (Wait in milliseconds)
- { "action": "click", "selector": "..." }
- { "action": "type", "selector": "...", "value": "..." }
- { "action": "press", "key": "Enter" }
- { "action": "scroll", "value": "down" }
- { "action": "done" }

CRITICAL RULES:
1. If goal is complete, output { "action": "done" }.
2. AFTER clicking a submit button, YOU MUST use { "action": "wait", "value": "5000" }.
3. Do not repeat the same action if the state hasn't changed.
4. If you typed something and the Value is visible in AXTree, you are done => { "action": "done" }.`;

class AgentRunner {
  constructor() {
    this.isRunning = false;
    this.currentGoal = null;
    this.history = [];
    this.maxSteps = 20;
    this.stepDelay = 2000;
    this.lastAction = null;
    this.lastState = null;
    this.consecutiveLlmFailures = 0;
    this.consecutiveActionCount = 0;
    this.stateVisitCounts = {};
    this._abortController = null;
  }

  /**
   * Dumps the current run diagnostics to a file for debugging
   */
  async _dumpDiagnostics(reason, error = null) {
    try {
      const fs = await import("fs/promises");
      const path = await import("path");

      const dump = {
        timestamp: new Date().toISOString(),
        reason,
        error: error ? error.message : null,
        goal: this.currentGoal,
        history: this.history,
        lastAction: this.lastAction,
        lastState: this.lastState,
        llmConfig: configManager.get("agent.llm"),
        stats: this.getUsageStats(),
      };

      const logsDir = path.resolve(process.cwd(), "logs");
      await fs.mkdir(logsDir, { recursive: true }).catch(() => null);

      const file = path.join(logsDir, `telemetry-${Date.now()}.json`);
      await fs.writeFile(file, JSON.stringify(dump, null, 2), "utf-8");
      logger.info(`Diagnostics dumped to ${file}`);
    } catch (e) {
      logger.warn("Failed to dump diagnostics:", e.message);
    }
  }

  /**
   * Run the agent with a goal
   * @param {string} goal - The goal to accomplish
   * @param {object} config - Configuration options
   * @returns {Promise<object>} Result with success and stats
   */
  async run(goal, config = {}) {
    if (this.isRunning) {
      throw new Error("Agent is already running. Call stop() first.");
    }

    await llmClient.init();

    this.isRunning = true;
    this.currentGoal = goal;
    this.history = [];
    this.maxSteps = config.maxSteps || 20;
    this.stepDelay = config.stepDelay || 2000;
    this.lastAction = null;
    this.lastState = null;
    this.consecutiveLlmFailures = 0;
    this.consecutiveActionCount = 0;
    this.stateVisitCounts = {};
    this._abortController = new AbortController();

    const page = getPage();
    if (!page) {
      throw new Error("No page available. Call api.init(page) first.");
    }

    logger.info(
      `Starting Agent with goal: "${goal}" (Max Steps: ${this.maxSteps})`,
    );

    let stepCount = 0;
    let lastResult = null;

    while (stepCount < this.maxSteps && !this._abortController.signal.aborted) {
      stepCount++;
      logger.info(`--- Step ${stepCount}/${this.maxSteps} ---`);

      try {
        await page.bringToFront();
      } catch (_e) {
        /* ignore */
      }

      const state = await this._captureState(page);

      const messages = this._buildPrompt(
        goal,
        state.screenshot,
        state.axTree,
        state.url,
      );

      let llmResponse;
      try {
        llmResponse = await llmClient.generateCompletion(messages);
        logger.info(`LLM Decision: ${JSON.stringify(llmResponse)}`);
        this.consecutiveLlmFailures = 0;
      } catch (_e) {
        logger.error("LLM Failure:", _e.message);
        this.consecutiveLlmFailures++;
        if (this.consecutiveLlmFailures >= 3) {
          logger.error("Aborting after 3 consecutive LLM failures.");
          await this._dumpDiagnostics("llm_failure", _e);
          break;
        }
        await page.waitForTimeout(5000);
        continue;
      }

      if (!llmResponse || !llmResponse.action) {
        logger.warn("Invalid LLM response (no action). Retrying...");
        await page.waitForTimeout(1000);
        continue;
      }

      const actionSignature = JSON.stringify(llmResponse);
      const stateSignature = this._generateSemanticStateHash(state.axTree);

      if (this.lastAction === actionSignature) {
        this.consecutiveActionCount++;
      } else {
        this.consecutiveActionCount = 1;
      }

      this.stateVisitCounts[stateSignature] =
        (this.stateVisitCounts[stateSignature] || 0) + 1;

      if (
        (this.lastState === stateSignature &&
          this.consecutiveActionCount >= 2) ||
        this.consecutiveActionCount >= 3 ||
        this.stateVisitCounts[stateSignature] >= 5
      ) {
        logger.warn("Loop detected. Stopping.");
        await this._dumpDiagnostics("loop_detected");
        break;
      }

      this.lastAction = actionSignature;
      this.lastState = stateSignature;

      const result = await actionEngine.execute(
        page,
        llmResponse,
        config.sessionId || "unknown",
      );
      lastResult = result;

      this.history.push({
        role: "assistant",
        content: JSON.stringify(llmResponse),
      });
      this.history.push({
        role: "user",
        content: result.success
          ? "Action succeeded."
          : `Action failed: ${result.error}`,
      });

      if (result.done) {
        logger.info("Agent completed the task successfully.");
        this.isRunning = false;
        return { success: true, done: true, steps: stepCount, result };
      }

      const useAdaptive = configManager.get("agent.runner.adaptiveDelay", true);
      if (useAdaptive) {
        try {
          await page.waitForLoadState("networkidle", {
            timeout: this.stepDelay,
          });
        } catch {
          // Timeout hit, which is fine as a fallback
        }
      } else {
        await page.waitForTimeout(this.stepDelay);
      }
    }

    this.isRunning = false;
    const reason = stepCount >= this.maxSteps ? "max_steps" : "stopped";
    logger.warn(`Agent reached max steps (${this.maxSteps}) or was stopped.`);
    await this._dumpDiagnostics(reason);

    return {
      success: false,
      done: false,
      steps: stepCount,
      result: lastResult,
      reason: stepCount >= this.maxSteps ? "max_steps" : "stopped",
    };
  }

  /**
   * Stop the running agent
   */
  stop() {
    if (this._abortController) {
      this._abortController.abort();
    }
    this.isRunning = false;
    logger.info("Agent stopped.");
  }

  /**
   * Capture current page state
   * @private
   */
  async _captureState(page) {
    const { screenshot } = await this._captureScreenshot(page);
    const axTree = await this._captureAXTree(page);
    const url = page.url();

    return { screenshot, axTree, url };
  }

  /**
   * Generate a stable semantic hash from the AX tree to prevent false negative loop detection
   * due to timestamps, dynamic iframes, or rapidly changing coordinates.
   * @param {string} axTreeJson - The raw AXTree string
   * @returns {string} - The sanitized semantic representation
   * @private
   */
  _generateSemanticStateHash(axTreeJson) {
    if (!axTreeJson) return "";
    try {
      let text =
        typeof axTreeJson === "string"
          ? axTreeJson
          : JSON.stringify(axTreeJson);
      // 1. Strip exact timestamps (e.g. 13 digit epoch)
      text = text.replace(/"\d{10,14}"/g, '"[TIMESTAMP]"');
      // 2. Strip dynamically generated ids that look like random hashes (e.g. id="a1b2c3d4...")
      text = text.replace(/"id":\s*"[^"]{15,}"/g, '"id":"[DYNAMIC_ID]"');
      // 3. Strip coordinate jitter
      text = text.replace(/"[xy]":\s*\d+\.\d+/g, '"coord":0');
      return text;
    } catch (_e) {
      return String(axTreeJson);
    }
  }

  /**
   * Capture screenshot
   * @private
   */
  async _captureScreenshot(page) {
    try {
      let viewport = page.viewportSize();
      if (!viewport) {
        viewport = await page.evaluate(() => ({
          width: window.innerWidth,
          height: window.innerHeight,
        }));
      }
      const screenshot = await page.screenshot({
        type: "jpeg",
        quality: 40,
        scale: "css",
        timeout: 10000,
        animations: "disabled",
        caret: "hide",
      });
      const base64 = screenshot.toString("base64");
      const sizeKB = Math.round(base64.length / 1024);
      logger.info(
        `[_captureScreenshot] Viewport: ${viewport.width}x${viewport.height}, Captured: ${sizeKB} KB`,
      );
      return base64;
    } catch (e) {
      logger.error("Screenshot failed:", e.message);
      return "";
    }
  }

  /**
   * Capture accessibility tree
   * @private
   */
  async _captureAXTree(page) {
    try {
      const tree = await page.accessibility.snapshot();
      return JSON.stringify(tree, null, 2);
    } catch (e) {
      logger.error("AXTree capture failed:", e.message);
      return "";
    }
  }

  /**
   * Build prompt for LLM
   * @private
   */
  _buildPrompt(goal, screenshot, axTree, currentUrl) {
    const systemMessage = {
      role: "system",
      content: DEFAULT_SYSTEM_PROMPT,
    };

    const recentHistory = this.history.slice(-4).map((msg) => {
      if (Array.isArray(msg.content)) {
        const textOnly = msg.content
          .filter((c) => c.type === "text")
          .map((c) => c.text)
          .join("\n");
        return { role: msg.role, content: textOnly };
      }
      return msg;
    });

    const contentParts = [
      { type: "text", text: `Goal: ${goal}` },
      { type: "text", text: `Current URL: ${currentUrl}` },
      {
        type: "text",
        text: `Current Page State (Accessibility Tree):\n${axTree}`,
      },
    ];

    const config = llmClient.config;
    if (config?.useVision !== false) {
      contentParts.push({
        type: "text",
        text: "Look at the screenshot to understand the visual layout.",
      });
      contentParts.push({
        type: "image_url",
        image_url: { url: `data:image/jpeg;base64,${screenshot}` },
      });
    } else {
      contentParts.push({
        type: "text",
        text: "Use the accessibility tree to identify elements.",
      });
    }

    const userMessage = { role: "user", content: contentParts };

    return [systemMessage, ...recentHistory, userMessage];
  }

  /**
   * Get usage statistics
   * @returns {object}
   */
  getUsageStats() {
    return {
      isRunning: this.isRunning,
      goal: this.currentGoal,
      steps: this.history.length / 2,
      maxSteps: this.maxSteps,
      estimatedTokens: estimateConversationTokens(this.history),
      historySize: this.history.length,
      llmStats: llmClient.getUsageStats(),
    };
  }
}

const agentRunner = new AgentRunner();

export { agentRunner };
export default agentRunner;
