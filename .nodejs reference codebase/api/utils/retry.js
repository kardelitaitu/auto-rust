/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview A utility for retrying an async operation with exponential backoff.
 * Enhanced with circuit breaker integration and provider fallback.
 * @module utils/retry
 */

import { createLogger } from "../core/logger.js";

const logger = createLogger("retry.js");

/**
 * Default retry configuration
 */
const DEFAULT_OPTIONS = {
  retries: 2,
  delay: 1000,
  factor: 2,
  maxDelay: 30000,
  jitterMin: 0.5,
  jitterMax: 1.5,
  description: "operation",
  circuitBreaker: null,
  providerId: null,
  shouldRetry: null,
  onRetry: null,
};

/**
 * Calculate delay with exponential backoff and jitter
 * @param {number} attempt - Current attempt (0-indexed)
 * @param {object} options - Delay options
 * @returns {number} Delay in milliseconds
 */
export function calculateBackoffDelay(attempt, options = {}) {
  const {
    baseDelay = DEFAULT_OPTIONS.delay,
    maxDelay = DEFAULT_OPTIONS.maxDelay,
    factor = DEFAULT_OPTIONS.factor,
    jitterMin = DEFAULT_OPTIONS.jitterMin,
    jitterMax = DEFAULT_OPTIONS.jitterMax,
  } = options;

  const delay = Math.min(baseDelay * Math.pow(factor, attempt), maxDelay);
  const jitterMultiplier = jitterMin + Math.random() * (jitterMax - jitterMin);

  return Math.floor(delay * jitterMultiplier);
}

/**
 * Check if error is non-retryable
 * @private
 */
function isNonRetryableError(error) {
  const nonRetryableCodes = [
    "INVALID_REQUEST",
    "AUTHENTICATION_ERROR",
    "PERMISSION_DENIED",
    "NOT_FOUND",
    "INVALID_ARGUMENT",
  ];
  return error.code && nonRetryableCodes.includes(error.code);
}

/**
 * Retries an async operation with exponential backoff.
 *
 * Enhanced features:
 * - Circuit breaker integration
 * - Custom retry conditions
 * - Retry callbacks
 * - Jitter for thundering herd prevention
 *
 * @param {Function} operation - Async function to execute
 * @param {object} [options] - Retry options
 * @param {number} [options.retries=2] - Maximum retry attempts
 * @param {number} [options.delay=1000] - Initial delay in ms
 * @param {number} [options.factor=2] - Exponential backoff factor
 * @param {number} [options.maxDelay=30000] - Maximum delay in ms
 * @param {number} [options.jitterMin=0.5] - Minimum jitter multiplier
 * @param {number} [options.jitterMax=1.5] - Maximum jitter multiplier
 * @param {string} [options.description='operation'] - Description for logging
 * @param {object} [options.circuitBreaker] - Circuit breaker instance
 * @param {string} [options.providerId] - Provider ID for circuit breaker
 * @param {Function} [options.shouldRetry] - Custom retry condition (error) => boolean
 * @param {Function} [options.onRetry] - Callback on retry (error, attempt, delay)
 * @returns {Promise<any>} Operation result
 * @throws {Error} If all retries fail
 *
 * @example
 * // Basic retry
 * const result = await withRetry(() => callAPI());
 *
 * @example
 * // With circuit breaker
 * const result = await withRetry(
 *     () => callProvider(provider),
 *     { circuitBreaker, providerId: provider }
 * );
 */
export async function withRetry(operation, options = {}) {
  const config = { ...DEFAULT_OPTIONS, ...options };
  const {
    retries,
    circuitBreaker,
    providerId,
    shouldRetry,
    onRetry,
    description,
  } = config;

  let lastError = null;
  let currentDelay = config.delay;

  for (let attempt = 0; attempt < retries; attempt++) {
    // Check circuit breaker before attempting
    if (circuitBreaker && providerId) {
      const check = circuitBreaker.check(providerId);
      if (!check.allowed) {
        logger.warn(
          `[${providerId}] Circuit breaker OPEN (${check.state}), failing fast`,
        );
        const error = new Error(
          `Circuit breaker open for ${providerId}. Retry after ${Math.ceil(check.retryAfter / 1000)}s`,
        );
        error.code = "CIRCUIT_OPEN";
        error.retryAfter = check.retryAfter;
        throw error;
      }
    }

    try {
      const result = await operation();

      // Record success with circuit breaker
      if (circuitBreaker && providerId) {
        circuitBreaker.recordSuccess(providerId);
      }

      return result;
    } catch (error) {
      lastError = error;

      // Record failure with circuit breaker
      if (circuitBreaker && providerId) {
        circuitBreaker.recordFailure(providerId);
      }

      // Check if we should retry
      const canRetry = attempt < retries - 1;
      const customRetry = shouldRetry ? shouldRetry(error) : true;
      const retryable = !isNonRetryableError(error);

      if (!canRetry || !customRetry || !retryable) {
        logger.debug(
          `[${description}] Not retrying: canRetry=${canRetry}, customRetry=${customRetry}, retryable=${retryable}`,
        );
        throw error;
      }

      // Calculate delay for this retry
      currentDelay = calculateBackoffDelay(attempt, config);

      // Log retry attempt
      logger.warn(
        `[${description}] Attempt ${attempt + 1}/${retries} failed: ${error.message}. Retrying in ${currentDelay}ms...`,
      );

      // Call onRetry callback if provided
      if (onRetry) {
        await onRetry(error, attempt, currentDelay);
      }

      // Wait before retry
      await new Promise((resolve) => setTimeout(resolve, currentDelay));
    }
  }

  logger.error(`[${description}] All ${retries} attempts failed`);
  throw lastError;
}

/**
 * Retry with provider fallback
 *
 * Tries providers in order, falling back on failure.
 *
 * @param {Array<Function>} providers - Array of provider functions
 * @param {object} [options] - Retry options
 * @returns {Promise<any>} Result from first successful provider
 *
 * @example
 * const result = await withProviderFallback(
 *     [
 *         () => callProvider('primary'),
 *         () => callProvider('backup1'),
 *         () => callProvider('backup2')
 *     ],
 *     { retries: 2 }
 * );
 */
export async function withProviderFallback(providers, options = {}) {
  let lastError = null;

  for (let i = 0; i < providers.length; i++) {
    try {
      logger.debug(`Trying provider ${i + 1}/${providers.length}`);
      return await withRetry(providers[i], options);
    } catch (error) {
      lastError = error;
      logger.warn(`Provider ${i + 1} failed: ${error.message}`);

      // If circuit breaker opened, try next provider immediately
      if (error.code === "CIRCUIT_OPEN") {
        logger.info("Circuit open, moving to next provider");
        continue;
      }
    }
  }

  throw lastError;
}

/**
 * Retry statistics tracker
 */
export class RetryStats {
  constructor() {
    this.attempts = 0;
    this.successes = 0;
    this.failures = 0;
    this.totalRetries = 0;
    this.totalDelay = 0;
  }

  record(success, retries, delay) {
    this.attempts++;
    this.totalRetries += retries;
    this.totalDelay += delay;
    if (success) {
      this.successes++;
    } else {
      this.failures++;
    }
  }

  getStats() {
    return {
      attempts: this.attempts,
      successes: this.successes,
      failures: this.failures,
      successRate:
        this.attempts > 0
          ? ((this.successes / this.attempts) * 100).toFixed(2) + "%"
          : "0%",
      averageRetries:
        this.successes > 0
          ? (this.totalRetries / this.successes).toFixed(2)
          : "0",
    };
  }

  reset() {
    this.attempts = 0;
    this.successes = 0;
    this.failures = 0;
    this.totalRetries = 0;
    this.totalDelay = 0;
  }
}

/**
 * Default retry stats instance
 */
export const retryStats = new RetryStats();

export default {
  withRetry,
  withProviderFallback,
  calculateBackoffDelay,
  RetryStats,
  retryStats,
  DEFAULT_OPTIONS,
};
