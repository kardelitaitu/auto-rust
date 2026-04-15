/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/errors.js
 * @module tests/unit/utils/errors.test
 */

import { describe, it, expect } from 'vitest';
import {
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
} from '@api/utils/errors.js';

describe('utils/errors.js', () => {
    describe('AppError', () => {
        it('should create error with code and message', () => {
            const error = new AppError('TEST_CODE', 'Test message');
            expect(error.code).toBe('TEST_CODE');
            expect(error.message).toBe('Test message');
            expect(error.name).toBe('AppError');
            expect(error.metadata).toEqual({});
        });

        it('should create error with metadata', () => {
            const error = new AppError('TEST_CODE', 'Test message', { key: 'value' });
            expect(error.metadata).toEqual({ key: 'value' });
        });

        it('should create error with cause', () => {
            const cause = new Error('Original error');
            const error = new AppError('TEST_CODE', 'Test message', {}, cause);
            expect(error.cause).toBe(cause);
        });

        it('should have timestamp', () => {
            const error = new AppError('TEST_CODE', 'Test message');
            expect(error.timestamp).toBeDefined();
        });

        it('should convert to JSON', () => {
            const error = new AppError('TEST_CODE', 'Test message', { key: 'value' });
            const json = error.toJSON();
            expect(json.code).toBe('TEST_CODE');
            expect(json.message).toBe('Test message');
            expect(json.metadata).toEqual({ key: 'value' });
            expect(json.timestamp).toBeDefined();
        });

        it('should convert to string', () => {
            const error = new AppError('TEST_CODE', 'Test message');
            const str = error.toString();
            expect(str).toContain('TEST_CODE');
            expect(str).toContain('Test message');
        });

        it('should include metadata in toString', () => {
            const error = new AppError('TEST_CODE', 'Test message', { key: 'value' });
            const str = error.toString();
            expect(str).toContain('key');
            expect(str).toContain('value');
        });

        it('should not include empty metadata in toString', () => {
            const error = new AppError('TEST_CODE', 'Test message');
            const str = error.toString();
            expect(str).not.toContain('{}');
        });
    });

    describe('RouterError', () => {
        it('should create RouterError with correct code', () => {
            const error = new RouterError('Router failed');
            expect(error.code).toBe('ROUTER_ERROR');
            expect(error.name).toBe('RouterError');
        });
    });

    describe('ProxyError', () => {
        it('should create ProxyError with correct code', () => {
            const error = new ProxyError('Proxy failed');
            expect(error.code).toBe('PROXY_ERROR');
            expect(error.name).toBe('ProxyError');
        });
    });

    describe('RateLimitError', () => {
        it('should create RateLimitError with correct code', () => {
            const error = new RateLimitError('Rate limited');
            expect(error.code).toBe('RATE_LIMIT_ERROR');
            expect(error.name).toBe('RateLimitError');
            expect(error.retryable).toBe(true);
        });
    });

    describe('ModelError', () => {
        it('should create ModelError with correct code', () => {
            const error = new ModelError('Model failed');
            expect(error.code).toBe('MODEL_ERROR');
            expect(error.name).toBe('ModelError');
        });
    });

    describe('ConfigError', () => {
        it('should create ConfigError with correct code', () => {
            const error = new ConfigError('Config invalid');
            expect(error.code).toBe('CONFIG_ERROR');
            expect(error.name).toBe('ConfigError');
        });
    });

    describe('ValidationError', () => {
        it('should create ValidationError with correct code', () => {
            const error = new ValidationError('Validation failed');
            expect(error.code).toBe('VALIDATION_ERROR');
            expect(error.name).toBe('ValidationError');
        });
    });

    describe('BrowserError', () => {
        it('should create BrowserError with correct code', () => {
            const error = new BrowserError('Browser crashed');
            expect(error.code).toBe('BROWSER_ERROR');
            expect(error.name).toBe('BrowserError');
        });
    });

    describe('TimeoutError', () => {
        it('should create TimeoutError with correct code', () => {
            const error = new TimeoutError('Operation timed out');
            expect(error.code).toBe('TIMEOUT_ERROR');
            expect(error.name).toBe('TimeoutError');
            expect(error.retryable).toBe(true);
        });
    });

    describe('CircuitBreakerError', () => {
        it('should create CircuitBreakerError with correct code', () => {
            const error = new CircuitBreakerError('Circuit open');
            expect(error.code).toBe('CIRCUIT_BREAKER_ERROR');
            expect(error.name).toBe('CircuitBreakerError');
            expect(error.retryable).toBe(false);
        });
    });

    describe('classifyHttpError', () => {
        it('should return RateLimitError for 429', () => {
            const error = classifyHttpError(429, 'Rate limited');
            expect(error.code).toBe('RATE_LIMIT_ERROR');
        });

        it('should return RouterError for 500', () => {
            const error = classifyHttpError(500, 'Server error');
            expect(error.code).toBe('ROUTER_ERROR');
        });

        it('should return RouterError for 502', () => {
            const error = classifyHttpError(502, 'Bad gateway');
            expect(error.code).toBe('ROUTER_ERROR');
        });

        it('should return RouterError for 503', () => {
            const error = classifyHttpError(503, 'Service unavailable');
            expect(error.code).toBe('ROUTER_ERROR');
        });

        it('should return RouterError for 504', () => {
            const error = classifyHttpError(504, 'Gateway timeout');
            expect(error.code).toBe('ROUTER_ERROR');
        });

        it('should return ConfigError for 401', () => {
            const error = classifyHttpError(401, 'Unauthorized');
            expect(error.code).toBe('CONFIG_ERROR');
        });

        it('should return ConfigError for 403', () => {
            const error = classifyHttpError(403, 'Forbidden');
            expect(error.code).toBe('CONFIG_ERROR');
        });

        it('should return TimeoutError for 408', () => {
            const error = classifyHttpError(408, 'Request timeout');
            expect(error.code).toBe('TIMEOUT_ERROR');
        });

        it('should return RouterError for unknown status', () => {
            const error = classifyHttpError(999, 'Unknown error');
            expect(error.code).toBe('ROUTER_ERROR');
        });

        it('should include statusCode in metadata', () => {
            const error = classifyHttpError(429, 'Rate limited');
            expect(error.metadata.statusCode).toBe(429);
        });
    });

    describe('wrapError', () => {
        it('should wrap plain error in AppError', () => {
            const original = new Error('Original error');
            const wrapped = wrapError(original, 'WRAPPED_CODE');
            expect(wrapped.code).toBe('WRAPPED_CODE');
            expect(wrapped.message).toBe('Original error');
            expect(wrapped.cause).toBe(original);
        });

        it('should return AppError as-is if already wrapped', () => {
            const original = new AppError('CODE', 'Message');
            const wrapped = wrapError(original, 'OTHER_CODE');
            expect(wrapped.code).toBe('CODE');
            expect(wrapped).toBe(original);
        });

        it('should include metadata when wrapping', () => {
            const original = new Error('Original error');
            const wrapped = wrapError(original, 'CODE', { key: 'value' });
            expect(wrapped.metadata).toEqual({ key: 'value' });
        });
    });
});
