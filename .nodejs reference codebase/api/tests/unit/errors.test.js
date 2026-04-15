/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
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

describe('errors', () => {
    it('creates AppError with metadata and cause', () => {
        const cause = new Error('root');
        const err = new AppError('CODE', 'message', { a: 1 }, cause);
        expect(err.name).toBe('AppError');
        expect(err.code).toBe('CODE');
        expect(err.metadata).toEqual({ a: 1 });
        expect(err.cause).toBe(cause);
        const json = err.toJSON();
        expect(json.code).toBe('CODE');
        expect(json.cause.message).toBe('root');
    });

    it('formats string without metadata', () => {
        const err = new AppError('CODE', 'message');
        expect(err.toString()).toBe('[CODE] message');
    });

    it('formats string with metadata', () => {
        const err = new AppError('CODE', 'message', { a: 1 });
        expect(err.toString()).toContain('[CODE] message');
        expect(err.toString()).toContain('"a":1');
    });

    it('serializes without cause when none provided', () => {
        const err = new AppError('CODE', 'message');
        const json = err.toJSON();
        expect(json.cause).toBeNull();
    });

    it('constructs without captureStackTrace when unavailable', () => {
        const original = Error.captureStackTrace;
        Error.captureStackTrace = undefined;
        try {
            const err = new AppError('CODE', 'message');
            expect(err.code).toBe('CODE');
        } finally {
            Error.captureStackTrace = original;
        }
    });

    it('creates typed errors with codes', () => {
        expect(new RouterError('x').code).toBe('ROUTER_ERROR');
        expect(new ProxyError('x').code).toBe('PROXY_ERROR');
        expect(new RateLimitError('x').retryable).toBe(true);
        expect(new ModelError('x').code).toBe('MODEL_ERROR');
        expect(new ConfigError('x').code).toBe('CONFIG_ERROR');
        expect(new ValidationError('x').code).toBe('VALIDATION_ERROR');
        expect(new BrowserError('x').code).toBe('BROWSER_ERROR');
        expect(new TimeoutError('x').retryable).toBe(true);
        expect(new CircuitBreakerError('x').retryable).toBe(false);
    });

    it('classifies http errors by status code', () => {
        const rate = classifyHttpError(429, 'rate');
        expect(rate).toBeInstanceOf(RateLimitError);
        expect(rate.metadata.statusCode).toBe(429);

        const retry = classifyHttpError(503, 'retry');
        expect(retry).toBeInstanceOf(RouterError);
        expect(retry.metadata.retryable).toBe(true);

        const server = classifyHttpError(500, 'server');
        expect(server).toBeInstanceOf(RouterError);

        const config = classifyHttpError(401, 'auth');
        expect(config).toBeInstanceOf(ConfigError);

        const timeout = classifyHttpError(408, 'timeout');
        expect(timeout).toBeInstanceOf(TimeoutError);

        const other = classifyHttpError(418, 'other');
        expect(other).toBeInstanceOf(RouterError);
    });

    it('wraps non-AppError instances', () => {
        const original = new Error('boom');
        const wrapped = wrapError(original, 'CUSTOM', { a: 1 });
        expect(wrapped).toBeInstanceOf(AppError);
        expect(wrapped.code).toBe('CUSTOM');
        expect(wrapped.cause).toBe(original);
    });

    it('returns AppError instances unchanged', () => {
        const err = new AppError('CODE', 'message');
        const wrapped = wrapError(err);
        expect(wrapped).toBe(err);
    });
});
