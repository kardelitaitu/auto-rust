/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Open World Browser Task - Returns structured result
 * Uses the autonomous vision-based game agent to complete goals on any website.
 * @module tasks/owb
 */

import { api } from "../api/index.js";
import { createLogger } from "../api/core/logger.js";
import { createSuccessResult, createFailedResult } from "../api/core/task-result.js";

/**
 * Open World Browser - Autonomous Agent Task
 * @param {Page} page - Playwright page object
 * @param {object} payload - Task payload
 * @returns {Promise<object>} Structured task result
 */
export default async function owb(page, payload) {
  const startTime = Date.now();
  const logger = createLogger("owb.js");
  const browserInfo = payload?.browserInfo || "unknown";

  logger.info("Starting Open World Browser task...");

  const goal = payload?.goal || payload?.value || payload;

  // Input validation
  if (!goal || typeof goal !== "string") {
    return createFailedResult('owb', 'OWB task requires a string goal', { sessionId: browserInfo });
  }

  if (goal.length < 3) {
    return createFailedResult('owb', 'Goal must be at least 3 characters', { sessionId: browserInfo });
  }

  if (goal.length > 500) {
    return createFailedResult('owb', 'Goal must be under 500 characters', { sessionId: browserInfo });
  }

  const config = {
    maxSteps: payload?.maxSteps || 30,
    stepDelay: payload?.stepDelay || 500,
    stuckDetection: payload?.stuckDetection !== false,
    maxAttemptsWithoutChange: payload?.maxAttempts || 5,
    verifyAction: payload?.verifyAction !== false,
    sessionId: browserInfo,
  };

  logger.info(`Goal: "${goal}"`);
  logger.info(`Config: maxSteps=${config.maxSteps}, stepDelay=${config.stepDelay}`);

  try {
    await api.init(page, { logger });
    logger.info("Running autonomous agent...");

    const result = await api.gameAgent.run(goal, config);

    logger.info(`OWB Result: Success=${result.success}, Steps=${result.steps}`);

    if (result.success) {
      return createSuccessResult('owb', {
        goal,
        steps: result.steps,
        reason: result.reason
      }, { startTime, sessionId: browserInfo });
    } else {
      return createFailedResult('owb', result.reason || 'Agent did not complete', {
        partialData: { goal, steps: result.steps },
        sessionId: browserInfo
      });
    }
  } catch (error) {
    logger.error(`OWB task failed: ${error.message}`);
    return createFailedResult('owb', error, {
      partialData: { goal },
      sessionId: browserInfo
    });
  }
}
