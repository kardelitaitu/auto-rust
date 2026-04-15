/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/envLoader.js
 * @module tests/unit/envLoader.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('utils/envLoader', () => {
    let envLoader;
    const originalEnv = process.env;

    beforeEach(async () => {
        vi.clearAllMocks();
        process.env = { ...originalEnv };
        const module = await import('../../utils/envLoader.js');
        envLoader = module;
    });

    afterEach(() => {
        process.env = originalEnv;
    });

    describe('getEnv', () => {
        it('should return value from process.env', () => {
            process.env.TEST_VAR = 'test_value';
            expect(envLoader.getEnv('TEST_VAR')).toBe('test_value');
        });

        it('should return default value if not set', () => {
            expect(envLoader.getEnv('NON_EXISTENT', 'default')).toBe('default');
        });

        it('should return undefined and warn if not set and no default', () => {
            const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
            expect(envLoader.getEnv('NON_EXISTENT')).toBeUndefined();
            expect(warnSpy).toHaveBeenCalled();
        });
    });

    describe('getRequiredEnv', () => {
        it('should return value if set', () => {
            process.env.REQUIRED_VAR = 'required';
            expect(envLoader.getRequiredEnv('REQUIRED_VAR')).toBe('required');
        });

        it('should throw error if not set', () => {
            expect(() => envLoader.getRequiredEnv('MISSING_VAR')).toThrow(
                /Required environment variable/
            );
        });
    });

    describe('resolveEnvVars', () => {
        it('should resolve placeholders', () => {
            process.env.VAR1 = 'val1';
            expect(envLoader.resolveEnvVars('Hello ${VAR1}')).toBe('Hello val1');
        });

        it('should return original string if no match', () => {
            expect(envLoader.resolveEnvVars('Hello World')).toBe('Hello World');
        });

        it('should warn and return original placeholder if var not set', () => {
            const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
            expect(envLoader.resolveEnvVars('Hello ${MISSING}')).toBe('Hello ${MISSING}');
            expect(warnSpy).toHaveBeenCalled();
        });
    });

    describe('resolveEnvVarsInObject', () => {
        it('should recursively resolve vars in object', () => {
            process.env.V1 = 'one';
            process.env.V2 = 'two';
            const input = {
                a: '${V1}',
                b: {
                    c: '${V2}',
                    d: 123,
                },
                e: ['${V1}', 456],
            };
            const expected = {
                a: 'one',
                b: {
                    c: 'two',
                    d: 123,
                },
                e: ['one', 456],
            };
            expect(envLoader.resolveEnvVarsInObject(input)).toEqual(expected);
        });
    });

    describe('Environment checks', () => {
        it('getNodeEnv should return development by default', () => {
            delete process.env.NODE_ENV;
            expect(envLoader.getNodeEnv()).toBe('development');
        });

        it('isProduction should return true when NODE_ENV is production', () => {
            process.env.NODE_ENV = 'production';
            expect(envLoader.isProduction()).toBe(true);
        });

        it('isDevelopment should return true when NODE_ENV is development', () => {
            process.env.NODE_ENV = 'development';
            expect(envLoader.isDevelopment()).toBe(true);
        });
    });

    describe('validateRequiredEnvVars', () => {
        it('should not throw if all vars are set', () => {
            process.env.V1 = 'val1';
            process.env.V2 = 'val2';
            expect(() => envLoader.validateRequiredEnvVars(['V1', 'V2'])).not.toThrow();
        });

        it('should throw if any var is missing', () => {
            process.env.V1 = 'val1';
            expect(() => envLoader.validateRequiredEnvVars(['V1', 'V2'])).toThrow(
                /Missing required environment variables/
            );
        });
    });
});
