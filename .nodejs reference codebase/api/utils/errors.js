/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Standardized Error Handling Module
 * Provides consistent error classes with codes and metadata for better tracking
 * @module utils/errors
 */

/**
 * Base Application Error Class
 * Extends Error with additional metadata for better error tracking
 */
export class AppError extends Error {
    /**
     * @param {string} code - Error code for categorization
     * @param {string} message - Human-readable error message
     * @param {object} metadata - Additional context (model, attempt, etc.)
     * @param {Error} [cause] - Original error that caused this
     */
    constructor(code, message, metadata = {}, cause = null) {
        super(message);
        this.name = 'AppError';
        this.code = code;
        this.metadata = metadata;
        this.timestamp = new Date().toISOString();
        this.cause = cause;

        // Maintain proper stack trace
        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, this.constructor);
        }
    }

    /**
     * Convert error to JSON for logging
     * @returns {object}
     */
    toJSON() {
        return {
            name: this.name,
            code: this.code,
            message: this.message,
            metadata: this.metadata,
            timestamp: this.timestamp,
            stack: this.stack,
            cause: this.cause
                ? {
                      message: this.cause.message,
                      stack: this.cause.stack,
                  }
                : null,
        };
    }

    /**
     * Get a formatted string representation
     * @returns {string}
     */
    toString() {
        const metaStr =
            Object.keys(this.metadata).length > 0 ? ` | ${JSON.stringify(this.metadata)}` : '';
        return `[${this.code}] ${this.message}${metaStr}`;
    }
}

/**
 * Router/Network Related Errors
 * For failures in API routing, HTTP requests, model calls
 */
export class RouterError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('ROUTER_ERROR', message, metadata, cause);
        this.name = 'RouterError';
    }
}

/**
 * Proxy Connection Errors
 * For proxy failures, timeouts, connection issues
 */
export class ProxyError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('PROXY_ERROR', message, metadata, cause);
        this.name = 'ProxyError';
    }
}

/**
 * Model/API Rate Limit Errors
 * For 429, 503, and other rate limiting responses
 */
export class RateLimitError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('RATE_LIMIT_ERROR', message, metadata, cause);
        this.name = 'RateLimitError';
        this.retryable = true;
    }
}

/**
 * Model Execution Errors
 * For model-specific failures, invalid responses, etc.
 */
export class ModelError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('MODEL_ERROR', message, metadata, cause);
        this.name = 'ModelError';
    }
}

/**
 * Configuration Errors
 * For invalid config, missing settings, etc.
 */
export class ConfigError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('CONFIG_ERROR', message, metadata, cause);
        this.name = 'ConfigError';
    }
}

/**
 * Validation Errors
 * For input validation, schema validation failures
 */
export class ValidationError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('VALIDATION_ERROR', message, metadata, cause);
        this.name = 'ValidationError';
    }
}

/**
 * Browser/Automation Errors
 * For Playwright failures, browser crashes, etc.
 */
export class BrowserError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('BROWSER_ERROR', message, metadata, cause);
        this.name = 'BrowserError';
    }
}

/**
 * Timeout Errors
 * For operation timeouts
 */
export class TimeoutError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('TIMEOUT_ERROR', message, metadata, cause);
        this.name = 'TimeoutError';
        this.retryable = true;
    }
}

/**
 * Circuit Breaker Errors
 * When circuit is open and requests are blocked
 */
export class CircuitBreakerError extends AppError {
    constructor(message, metadata = {}, cause = null) {
        super('CIRCUIT_BREAKER_ERROR', message, metadata, cause);
        this.name = 'CircuitBreakerError';
        this.retryable = false;
    }
}

/**
 * Helper function to classify HTTP status codes
 * @param {number} statusCode - HTTP status code
 * @param {string} message - Error message
 * @param {object} metadata - Additional metadata
 * @returns {AppError} Appropriate error type
 */
export function classifyHttpError(statusCode, message, metadata = {}) {
    switch (statusCode) {
        case 429:
            return new RateLimitError(message, { ...metadata, statusCode });
        case 500:
        case 502:
        case 503:
        case 504:
            return new RouterError(message, { ...metadata, statusCode, retryable: true });
        case 401:
        case 403:
            return new ConfigError(message, { ...metadata, statusCode });
        case 408:
            return new TimeoutError(message, { ...metadata, statusCode });
        default:
            return new RouterError(message, { ...metadata, statusCode });
    }
}

/**
 * Helper to wrap any error in AppError
 * @param {Error} error - Original error
 * @param {string} [code] - Error code (optional)
 * @param {object} [metadata] - Additional metadata
 * @returns {AppError}
 */
export function wrapError(error, code = 'UNKNOWN_ERROR', metadata = {}) {
    if (error instanceof AppError) {
        return error;
    }
    return new AppError(code, error.message, metadata, error);
}

export default {
    AppError,
    RouterError,
    ProxyError,
    RateLimitError,
    ModelError,
    ConfigError,
    ValidationError,
    BrowserError,
    TimeoutError,
    CircuitBreakerError,
    classifyHttpError,
    wrapError,
};
