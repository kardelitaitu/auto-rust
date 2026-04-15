/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Middleware Pipeline
 * Transform and validate actions before execution.
 *
 * @module api/middleware
 */

import { createLogger } from "./logger.js";
import { getEvents } from "./context.js";
import { isErrorCode, ActionError } from "./errors.js";
import { getStateSection, updateStateSection } from "./context-state.js";

const logger = createLogger("api/middleware.js");

/**
 * Create a middleware pipeline.
 * Middlewares execute in order, each can transform context or short-circuit.
 *
 * @param {...Function} middlewares - Middleware functions
 * @returns {Function} Pipeline function
 */
export function createPipeline(...middlewares) {
  return async (action, context) => {
    let index = 0;

    const next = async () => {
      if (index >= middlewares.length) {
        return await action(context);
      }
      const middleware = middlewares[index++];
      return await middleware(context, next);
    };

    return await next();
  };
}

/**
 * Create a sync middleware pipeline.
 * For middlewares that don't need async operations.
 *
 * @param {...Function} middlewares - Middleware functions
 * @returns {Function} Pipeline function
 */
export function createSyncPipeline(...middlewares) {
  return (action, context) => {
    let index = 0;

    const next = () => {
      if (index >= middlewares.length) {
        return action(context);
      }
      const middleware = middlewares[index++];
      return middleware(context, next);
    };

    return next();
  };
}

// ─── Common Middlewares ───────────────────────────────────────────

/**
 * Logging middleware - logs action execution.
 * @param {object} [options]
 * @param {boolean} [options.logArgs=true] - Log arguments
 * @param {boolean} [options.logResult=true] - Log result
 * @param {boolean} [options.logTime=false] - Log execution time
 * @returns {Function}
 */
export function loggingMiddleware(options = {}) {
  const { logArgs = true, logResult = true, logTime = false } = options;

  return async (context, next) => {
    const { action, selector, options: actionOptions } = context;

    if (logArgs) {
      logger.debug(`[Middleware] ${action}:`, {
        selector,
        options: actionOptions,
      });
    }

    const startTime = Date.now();

    try {
      const result = await next();

      if (logTime) {
        logger.debug(`[Middleware] ${action} took ${Date.now() - startTime}ms`);
      }

      if (logResult) {
        logger.debug(`[Middleware] ${action} result:`, result);
      }

      return result;
    } catch (e) {
      logger.debug(`[Middleware] ${action} error:`, e.message);
      throw e;
    }
  };
}

/**
 * Validation middleware - validates selectors and options.
 * @param {object} [rules] - Validation rules
 * @returns {Function}
 */
export function validationMiddleware() {
  return async (context, next) => {
    const { action, selector, options = {} } = context;

    // Validate selector for DOM actions
    if (["click", "type", "hover"].includes(action)) {
      if (!selector) {
        throw new Error(`Selector is required for ${action}`);
      }
      const isString = typeof selector === "string";
      const isLocator =
        selector &&
        typeof selector === "object" &&
        selector.constructor.name === "Locator";

      if (!isString && !isLocator) {
        throw new Error(
          `Invalid selector type for ${action}: ${typeof selector}. Expected string or Locator.`,
        );
      }

      if (isString && selector.trim() === "") {
        throw new Error(`Empty selector for ${action}`);
      }
    }

    // Validate options
    if (options.timeoutMs !== undefined && options.timeoutMs < 0) {
      throw new Error("timeoutMs must be non-negative");
    }

    if (options.maxRetries !== undefined && options.maxRetries < 0) {
      throw new Error("maxRetries must be non-negative");
    }

    return await next();
  };
}

const NON_RETRYABLE_ERRORS = [
  "Target closed",
  "Context closed",
  "Browser has been closed",
  "SessionDisconnectedError",
  "SESSION_DISCONNECTED",
  "PAGE_CLOSED",
];

export function isNonRetryableError(error) {
  if (!error) return false;
  const msg = (error.message || "").toLowerCase();
  const code = error.code || "";
  return NON_RETRYABLE_ERRORS.some(
    (err) => msg.includes(err.toLowerCase()) || code === err,
  );
}

/**
 * Retry middleware - auto-retry on failure.
 */
export function retryMiddleware(options = {}) {
  const {
    maxRetries = 3,
    backoffMultiplier = 2,
    shouldRetry = (e) => !isNonRetryableError(e),
  } = options;

  return async (context, next) => {
    let lastError;

    const budget = getStateSection("retryBudget");
    if (budget.used >= budget.max) {
      throw new ActionError(
        "RETRY_BUDGET_EXCEEDED",
        "Session retry budget exhausted",
      );
    }

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await next();
      } catch (e) {
        lastError = e;

        if (!shouldRetry(lastError) || attempt >= maxRetries) {
          throw lastError;
        }

        const delay = Math.pow(backoffMultiplier, attempt) * 100;
        logger.debug(
          `[Retry] Attempt ${attempt + 1} failed, retrying in ${delay}ms...`,
        );
        await new Promise((r) => setTimeout(r, delay));
      }
    }

    updateStateSection("retryBudget", { used: budget.used + 1 });
    throw lastError;
  };
}

/**
 * Recovery middleware - handles common errors with recovery strategies.
 * @param {object} [options]
 * @param {boolean} [options.scrollOnDetached=true] - Scroll when element detached
 * @param {boolean} [options.retryOnObscured=true] - Retry when element obscured
 * @returns {Function}
 */
export function recoveryMiddleware(options = {}) {
  const { scrollOnDetached = true, retryOnObscured = true } = options;

  return async (context, next) => {
    const { action, selector: _selector } = context;

    try {
      return await next();
    } catch (e) {
      // Element detached - focal recovery (wait + retry)
      if (
        scrollOnDetached &&
        (isErrorCode(e, "ELEMENT_DETACHED") ||
          isErrorCode(e, "ELEMENT_NOT_FOUND") ||
          (e.message || "").toLowerCase().includes("detached") ||
          (e.message || "").toLowerCase().includes("stale"))
      ) {
        logger.warn(`[Recovery] Element detached for ${action}. Retrying...`);
        await new Promise((r) => setTimeout(r, 500));
        return await next();
      }

      // Element obscured - retry once with force if it's a click
      if (
        retryOnObscured &&
        (isErrorCode(e, "ELEMENT_OBSCURED") ||
          (e.message || "").toLowerCase().includes("obscured"))
      ) {
        logger.warn(
          `[Recovery] Element obscured during ${action}. Retrying with force...`,
        );
        if (context.options) {
          context.options.force = true;
        }
        return await next();
      }

      throw e;
    }
  };
}

/**
 * Metrics middleware - tracks action timing and success.
 * @param {object} [options]
 * @param {boolean} [options.emitEvents=true] - Emit metrics events
 * @returns {Function}
 */
export function metricsMiddleware(options = {}) {
  const { emitEvents = true } = options;

  return async (context, next) => {
    const { action } = context;
    const startTime = Date.now();
    let success = false;

    try {
      const result = await next();
      success = true;
      return result;
    } finally {
      const duration = Date.now() - startTime;

      const metric = {
        action,
        duration,
        success,
        timestamp: Date.now(),
      };

      if (emitEvents) {
        getEvents().emitSafe("on:metrics", metric);
      }

      logger.debug(
        `[Metrics] ${action}: ${duration}ms (${success ? "success" : "failure"})`,
      );
    }
  };
}

/**
 * Rate limiting middleware.
 * @param {object} [options]
 * @param {number} [options.maxPerSecond=10] - Max actions per second
 * @param {object} [options.state] - Optional shared state object { actionCount, windowStart }
 * @returns {Function}
 */
export function rateLimitMiddleware(options = {}) {
  const { maxPerSecond = 10, state = null } = options;

  const _state = state || { actionCount: 0, windowStart: Date.now() };

  return async (context, next) => {
    const now = Date.now();

    if (now - _state.windowStart >= 1000) {
      _state.actionCount = 0;
      _state.windowStart = now;
    }

    if (_state.actionCount >= maxPerSecond) {
      const waitTime = 1000 - (now - _state.windowStart);
      logger.debug(`[RateLimit] Throttling, waiting ${waitTime}ms`);
      await new Promise((r) => setTimeout(r, waitTime));
    }

    _state.actionCount++;
    return await next();
  };
}

export default {
  createPipeline,
  createSyncPipeline,
  loggingMiddleware,
  validationMiddleware,
  retryMiddleware,
  recoveryMiddleware,
  metricsMiddleware,
  rateLimitMiddleware,
};
