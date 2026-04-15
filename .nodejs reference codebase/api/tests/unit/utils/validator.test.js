/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock logger BEFORE importing the module
vi.mock('@api/core/logger.js', () => {
    const mockLogger = {
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    };
    return {
        createLogger: vi.fn(() => mockLogger),
    };
});

import {
    validatePayload,
    validateApiResponse,
    validateBrowserConnection,
    validateTaskExecution,
} from '@api/utils/validator.js';
import { createLogger } from '@api/core/logger.js';

describe('api/utils/validator.js', () => {
    let mockLogger;

    beforeEach(() => {
        vi.clearAllMocks();
        mockLogger = createLogger('validator.js');
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('validatePayload', () => {
        it('should validate empty payload', () => {
            const result = validatePayload({});
            expect(result.isValid).toBe(true);
            expect(result.errors).toEqual([]);
        });

        it('should validate payload with valid browserInfo', () => {
            const result = validatePayload({ browserInfo: 'chrome-123' });
            expect(result.isValid).toBe(true);
        });

        it('should validate payload with valid URL', () => {
            const result = validatePayload({ url: 'https://example.com' });
            expect(result.isValid).toBe(true);
        });

        it('should reject invalid URL format', () => {
            const result = validatePayload({ url: 'not-a-url' });
            expect(result.isValid).toBe(false);
            expect(result.errors.length).toBeGreaterThan(0);
        });

        it('should validate payload with valid duration', () => {
            const result = validatePayload({ duration: 60 });
            expect(result.isValid).toBe(true);
        });

        it('should reject duration less than 1', () => {
            const result = validatePayload({ duration: 0 });
            expect(result.isValid).toBe(false);
        });

        it('should reject duration greater than 3600', () => {
            const result = validatePayload({ duration: 3601 });
            expect(result.isValid).toBe(false);
        });

        it('should validate payload with multiple valid fields', () => {
            const result = validatePayload({
                browserInfo: 'chrome-123',
                url: 'https://example.com',
                duration: 100,
            });
            expect(result.isValid).toBe(true);
        });

        it('should log warning on validation failure', () => {
            validatePayload({ url: 'invalid' });
            expect(mockLogger.warn).toHaveBeenCalled();
            expect(mockLogger.warn.mock.calls[0][0]).toContain('Payload validation failed');
        });

        it('should merge custom schema with default', () => {
            const customSchema = {
                customField: { type: 'string', required: true },
            };
            const result = validatePayload({ customField: 'value' }, customSchema);
            expect(result.isValid).toBe(true);
        });

        it('should reject when required custom field missing', () => {
            const customSchema = {
                customField: { type: 'string', required: true },
            };
            const result = validatePayload({}, customSchema);
            expect(result.isValid).toBe(false);
        });

        it('should return invalid for non-object data', () => {
            const result = validatePayload(null);
            expect(result.isValid).toBe(false);
            expect(result.errors).toContain('Data must be a valid object');
        });

        it('should return invalid for array data', () => {
            const result = validatePayload([]);
            expect(result.isValid).toBe(false);
        });
    });

    describe('validateApiResponse', () => {
        it('should validate roxybrowser response', () => {
            const response = {
                code: 200,
                msg: 'Success',
                data: [
                    {
                        ws: 'ws://localhost:9222',
                        http: 'http://localhost:9222',
                        windowName: 'window1',
                        sortNum: 1,
                    },
                ],
            };

            const result = validateApiResponse(response, 'roxybrowser');
            expect(result.isValid).toBe(true);
        });

        it('should validate ixbrowser response', () => {
            const response = { code: 200 };
            const result = validateApiResponse(response, 'ixbrowser');
            expect(result.isValid).toBe(true);
        });

        it('should validate morelogin response', () => {
            const response = { code: 200 };
            const result = validateApiResponse(response, 'morelogin');
            expect(result.isValid).toBe(true);
        });

        it('should validate localChrome response', () => {
            const response = { code: 200 };
            const result = validateApiResponse(response, 'localChrome');
            expect(result.isValid).toBe(true);
        });

        it('should return valid for unknown API type', () => {
            const response = { any: 'data' };
            const result = validateApiResponse(response, 'unknown');
            expect(result.isValid).toBe(true);
        });

        it('should log warning for unknown API type', () => {
            validateApiResponse({}, 'unknown');
            expect(mockLogger.warn).toHaveBeenCalledWith(
                expect.stringContaining('No specific validation rules')
            );
        });

        it('should reject roxybrowser response without code', () => {
            const response = { msg: 'Success' };
            const result = validateApiResponse(response, 'roxybrowser');
            expect(result.isValid).toBe(false);
        });

        it('should reject roxybrowser response with invalid data array', () => {
            const response = {
                code: 200,
                data: 'not-an-array',
            };
            const result = validateApiResponse(response, 'roxybrowser');
            expect(result.isValid).toBe(false);
        });

        it('should log error on validation failure', () => {
            validateApiResponse({ invalid: 'data' }, 'roxybrowser');
            expect(mockLogger.error).toHaveBeenCalled();
            expect(mockLogger.error.mock.calls[0][0]).toContain('API response validation failed');
        });

        it('should log debug on validation success', () => {
            validateApiResponse({ code: 200 }, 'ixbrowser');
            expect(mockLogger.debug).toHaveBeenCalledWith(
                expect.stringContaining('API response validation passed')
            );
        });
    });

    describe('validateBrowserConnection', () => {
        it('should validate valid WebSocket endpoint', () => {
            const result = validateBrowserConnection('ws://localhost:9222');
            expect(result.isValid).toBe(true);
        });

        it('should validate valid secure WebSocket endpoint', () => {
            const result = validateBrowserConnection('wss://example.com');
            expect(result.isValid).toBe(true);
        });

        it('should reject non-WebSocket URL', () => {
            const result = validateBrowserConnection('http://localhost:9222');
            expect(result.isValid).toBe(false);
        });

        it('should reject empty string', () => {
            const result = validateBrowserConnection('');
            expect(result.isValid).toBe(false);
        });

        it('should log warning on validation failure', () => {
            validateBrowserConnection('invalid');
            expect(mockLogger.warn).toHaveBeenCalled();
            expect(mockLogger.warn.mock.calls[0][0]).toContain(
                'Browser connection validation failed'
            );
        });
    });

    describe('validateTaskExecution', () => {
        it('should validate with valid browser instance', () => {
            const mockBrowser = {
                newContext: vi.fn(),
            };
            const payload = { url: 'https://example.com' };

            const result = validateTaskExecution(mockBrowser, payload);
            expect(result.isValid).toBe(true);
        });

        it('should validate with valid context instance', () => {
            const mockContext = {
                newPage: vi.fn(),
            };
            const payload = {};

            const result = validateTaskExecution(mockContext, payload);
            expect(result.isValid).toBe(true);
        });

        it('should validate with valid page instance', () => {
            const mockPage = {
                goto: vi.fn(),
            };
            const payload = {};

            const result = validateTaskExecution(mockPage, payload);
            expect(result.isValid).toBe(true);
        });

        it('should reject null browser', () => {
            const result = validateTaskExecution(null, {});
            expect(result.isValid).toBe(false);
            expect(result.errors[0]).toContain('required');
        });

        it('should reject invalid instance', () => {
            const mockInstance = {};
            const result = validateTaskExecution(mockInstance, {});
            expect(result.isValid).toBe(false);
            expect(result.errors[0]).toContain('Invalid browser');
        });

        it('should reject invalid payload', () => {
            const mockBrowser = { newContext: vi.fn() };
            const payload = { url: 'invalid-url' };

            const result = validateTaskExecution(mockBrowser, payload);
            expect(result.isValid).toBe(false);
        });

        it('should validate with custom schema', () => {
            const mockBrowser = { newContext: vi.fn() };
            const payload = { customField: 'value' };
            const customSchema = {
                customField: { type: 'string', required: true },
            };

            const result = validateTaskExecution(mockBrowser, payload, customSchema);
            expect(result.isValid).toBe(true);
        });

        it('should reject when custom required field missing', () => {
            const mockBrowser = { newContext: vi.fn() };
            const customSchema = {
                customField: { type: 'string', required: true },
            };

            const result = validateTaskExecution(mockBrowser, {}, customSchema);
            expect(result.isValid).toBe(false);
        });

        it('should log error on validation failure', () => {
            const result = validateTaskExecution(null, {});
            expect(result.isValid).toBe(false);
            expect(mockLogger.error).toHaveBeenCalled();
        });
    });
});
