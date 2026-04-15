/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/core/errors.js
 * @module tests/unit/core-errors.test
 */

import { describe, it, expect } from 'vitest';
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
    TaskTimeoutError,
    ConfigError,
    ConfigNotFoundError,
    LLMError,
    LLMTimeoutError,
    LLMRateLimitError,
    LLMCircuitOpenError,
    ValidationError,
    isErrorCode,
    withErrorHandling,
} from '@api/core/errors.js';

describe('api/core/errors', () => {
    describe('Error Classes', () => {
        it('AutomationError should have correct properties', () => {
            const err = new AutomationError('msg', 'CODE');
            expect(err.message).toBe('msg');
            expect(err.code).toBe('CODE');
            expect(err.name).toBe('AutomationError');
            expect(err instanceof Error).toBe(true);
        });

        it('SessionNotFoundError should format message with sessionId', () => {
            const err = new SessionNotFoundError('s-123');
            expect(err.message).toBe('Session not found: s-123');
            expect(err.code).toBe('SESSION_NOT_FOUND');
            expect(err instanceof SessionError).toBe(true);
        });

        it('ElementNotFoundError should format message with selector', () => {
            const err = new ElementNotFoundError('.btn');
            expect(err.message).toBe('Element not found: .btn');
            expect(err.code).toBe('ELEMENT_NOT_FOUND');
            expect(err instanceof ElementError).toBe(true);
        });

        it('ElementTimeoutError should format message with selector and timeout', () => {
            const err = new ElementTimeoutError('.btn', 5000);
            expect(err.message).toBe('Element not found within timeout (5000ms): .btn');
            expect(err.code).toBe('ELEMENT_TIMEOUT');
        });

        it('ActionFailedError should store action and reason', () => {
            const err = new ActionFailedError('click', 'timeout');
            expect(err.message).toBe("Action 'click' failed: timeout");
            expect(err.action).toBe('click');
        });

        it('LLMCircuitOpenError should store modelId and retryAfter', () => {
            const err = new LLMCircuitOpenError('gpt-4', 5000);
            expect(err.message).toBe('Circuit breaker OPEN for gpt-4. Retry after 5s');
            expect(err.modelId).toBe('gpt-4');
            expect(err.retryAfter).toBe(5000);
        });

        it('Other error classes should instantiate correctly', () => {
            expect(new SessionDisconnectedError().code).toBe('SESSION_DISCONNECTED');
            expect(new SessionTimeoutError().code).toBe('SESSION_TIMEOUT');
            expect(new ContextNotInitializedError().code).toBe('CONTEXT_NOT_INITIALIZED');
            expect(new PageClosedError().code).toBe('PAGE_CLOSED');
            expect(new ElementDetachedError().code).toBe('ELEMENT_DETACHED');
            expect(new ElementObscuredError().code).toBe('ELEMENT_OBSCURED');
            expect(new NavigationError('url', 'reason').code).toBe('NAVIGATION_ERROR');
            expect(new TaskTimeoutError('task', 100).code).toBe('TASK_TIMEOUT');
            expect(new ConfigNotFoundError('key').code).toBe('CONFIG_NOT_FOUND');
            expect(new LLMTimeoutError().code).toBe('LLM_TIMEOUT');
            expect(new LLMRateLimitError().code).toBe('LLM_RATE_LIMIT');
            expect(new ValidationError('msg').code).toBe('VALIDATION_ERROR');
        });
    });

    describe('isErrorCode', () => {
        it('should return true if code matches', () => {
            const err = new AutomationError('msg', 'MY_CODE');
            expect(isErrorCode(err, 'MY_CODE')).toBe(true);
        });

        it('should return true if name matches', () => {
            const err = new AutomationError('msg', 'CODE');
            expect(isErrorCode(err, 'AutomationError')).toBe(true);
        });

        it('should return false if neither matches', () => {
            const err = new AutomationError('msg', 'CODE');
            expect(isErrorCode(err, 'OTHER')).toBe(false);
        });

        it('should handle null/undefined error', () => {
            expect(isErrorCode(null, 'CODE')).toBe(false);
        });
    });

    describe('withErrorHandling', () => {
        it('should return result if fn succeeds', async () => {
            const res = await withErrorHandling(async () => 'success');
            expect(res).toBe('success');
        });

        it('should rethrow AutomationError if fn fails with it', async () => {
            const automationErr = new AutomationError('msg', 'CODE');
            const fn = async () => {
                throw automationErr;
            };
            await expect(withErrorHandling(fn)).rejects.toThrow(automationErr);
        });

        it('should wrap other errors in ActionError', async () => {
            const rawErr = new Error('raw');
            const fn = async () => {
                throw rawErr;
            };
            await expect(withErrorHandling(fn, 'test')).rejects.toThrow(/Error during test: raw/);
            await expect(withErrorHandling(fn, 'test')).rejects.toThrow(ActionError);
        });
    });
});
