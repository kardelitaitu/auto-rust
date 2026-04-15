/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { EnvironmentConfig, environmentConfig } from '@api/utils/environment-config.js';

describe('EnvironmentConfig', () => {
    let originalEnv;

    beforeEach(() => {
        originalEnv = { ...process.env };
    });

    afterEach(() => {
        process.env = { ...originalEnv };
    });

    describe('constructor', () => {
        it('should initialize with overrides map', () => {
            const config = new EnvironmentConfig();
            expect(config.overrides).toBeDefined();
            expect(typeof config.overrides).toBe('object');
        });

        it('should create appliedOverrides Map', () => {
            const config = new EnvironmentConfig();
            expect(config.appliedOverrides).toBeInstanceOf(Map);
        });
    });

    describe('getEnvOverrides', () => {
        it('should return environment variable mappings', () => {
            const overrides = environmentConfig.getEnvOverrides();
            expect(overrides).toHaveProperty('TWITTER_CYCLES');
            expect(overrides.TWITTER_CYCLES).toBe('session.cycles');
        });
    });

    describe('applyEnvOverrides', () => {
        it('should apply numeric override from env var', () => {
            process.env.TWITTER_CYCLES = '50';
            const config = { session: {} };
            const result = EnvironmentConfig.applyEnvOverrides(config);
            expect(result.session.cycles).toBe(50);
        });

        it('should apply boolean true override', () => {
            process.env.AI_ENABLED = 'true';
            const config = { ai: {} };
            const result = EnvironmentConfig.applyEnvOverrides(config);
            expect(result.ai.enabled).toBe(true);
        });

        it('should apply boolean false override', () => {
            process.env.AI_ENABLED = 'false';
            const config = { ai: {} };
            const result = EnvironmentConfig.applyEnvOverrides(config);
            expect(result.ai.enabled).toBe(false);
        });

        it('should not override when env var is empty string', () => {
            process.env.TWITTER_CYCLES = '';
            const config = { session: { cycles: 10 } };
            const result = EnvironmentConfig.applyEnvOverrides(config);
            expect(result.session.cycles).toBe(10);
        });

        it('should not override when env var is undefined', () => {
            delete process.env.TWITTER_CYCLES;
            const config = { session: { cycles: 10 } };
            const result = EnvironmentConfig.applyEnvOverrides(config);
            expect(result.session.cycles).toBe(10);
        });
    });

    describe('parseEnvValue', () => {
        it('should parse true string to boolean', () => {
            expect(EnvironmentConfig.parseEnvValue('VAR', 'true')).toBe(true);
        });

        it('should parse false string to boolean', () => {
            expect(EnvironmentConfig.parseEnvValue('VAR', 'false')).toBe(false);
        });

        it('should parse numeric string to number', () => {
            expect(EnvironmentConfig.parseEnvValue('VAR', '123')).toBe(123);
        });

        it('should parse float', () => {
            expect(EnvironmentConfig.parseEnvValue('VAR', '1.5')).toBe(1.5);
        });

        it('should return string by default', () => {
            expect(EnvironmentConfig.parseEnvValue('VAR', 'hello')).toBe('hello');
        });
    });

    describe('getNestedValue', () => {
        it('should get nested value', () => {
            const obj = { a: { b: { c: 'value' } } };
            expect(EnvironmentConfig.getNestedValue(obj, 'a.b.c')).toBe('value');
        });

        it('should return undefined for missing path', () => {
            const obj = { a: {} };
            expect(EnvironmentConfig.getNestedValue(obj, 'a.b.c')).toBeUndefined();
        });
    });

    describe('setNestedValue', () => {
        it('should set nested value', () => {
            const obj = {};
            EnvironmentConfig.setNestedValue(obj, 'a.b.c', 'value');
            expect(obj.a.b.c).toBe('value');
        });

        it('should create intermediate objects', () => {
            const obj = {};
            EnvironmentConfig.setNestedValue(obj, 'a.b.c', 'value');
            expect(typeof obj.a).toBe('object');
            expect(typeof obj.a.b).toBe('object');
        });
    });

    describe('getSupportedEnvVars', () => {
        it('should return array of supported env var names', () => {
            const vars = environmentConfig.getSupportedEnvVars();
            expect(Array.isArray(vars)).toBe(true);
            expect(vars).toContain('TWITTER_CYCLES');
        });
    });

    describe('isSupportedEnvVar', () => {
        it('should return true for supported env var', () => {
            expect(environmentConfig.isSupportedEnvVar('TWITTER_CYCLES')).toBe(true);
        });

        it('should return false for unsupported env var', () => {
            expect(environmentConfig.isSupportedEnvVar('NONEXISTENT_VAR')).toBe(false);
        });
    });

    describe('getConfigPath', () => {
        it('should return config path for supported env var', () => {
            expect(environmentConfig.getConfigPath('TWITTER_CYCLES')).toBe('session.cycles');
        });

        it('should return null for unsupported env var', () => {
            expect(environmentConfig.getConfigPath('NONEXISTENT')).toBeNull();
        });
    });

    describe('getCurrentEnvValues', () => {
        it('should return current env var values', () => {
            process.env.TWITTER_CYCLES = '25';
            const values = environmentConfig.getCurrentEnvValues();
            expect(values).toHaveProperty('TWITTER_CYCLES', '25');
        });
    });

    describe('validateEnvValues', () => {
        it('should return valid for empty env vars', () => {
            Object.keys(process.env).forEach((key) => {
                if (
                    key.startsWith('TWITTER_') ||
                    key.startsWith('GLOBAL_') ||
                    key.startsWith('AI_')
                ) {
                    delete process.env[key];
                }
            });
            const result = environmentConfig.validateEnvValues();
            expect(result).toHaveProperty('valid');
            expect(result).toHaveProperty('errors');
        });
    });

    describe('generateDocumentation', () => {
        it('should generate markdown documentation', () => {
            const docs = environmentConfig.generateDocumentation();
            expect(docs).toContain('# Environment Variable Overrides');
        });
    });
});
