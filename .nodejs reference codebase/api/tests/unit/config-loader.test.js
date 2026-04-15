/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocked = vi.hoisted(() => ({
    readFile: vi.fn(),
    logger: {
        debug: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    },
}));

vi.mock('fs/promises', () => ({
    default: {
        readFile: mocked.readFile,
    },
    readFile: mocked.readFile,
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => mocked.logger),
}));

describe('configLoader', () => {
    let configLoader;
    let getTimeouts;
    let getTimeoutValue;
    let getBrowserAPI;
    let getSettings;

    beforeEach(async () => {
        vi.resetModules();
        vi.clearAllMocks();
        const module = await import('../../utils/configLoader.js');
        configLoader = module.default;
        getTimeouts = module.getTimeouts;
        getTimeoutValue = module.getTimeoutValue;
        getBrowserAPI = module.getBrowserAPI;
        getSettings = module.getSettings;
        configLoader.clearCache();
    });

    it('loads and caches configuration files', async () => {
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ foo: 1 }));
        const first = await configLoader.loadConfig('sample', { bar: 2 });
        const second = await configLoader.loadConfig('sample', { bar: 2 });
        expect(first).toEqual({ bar: 2, foo: 1 });
        expect(second).toEqual({ bar: 2, foo: 1 });
        expect(mocked.readFile).toHaveBeenCalledTimes(1);
    });

    it('returns defaults for missing files', async () => {
        mocked.readFile.mockRejectedValueOnce(
            Object.assign(new Error('missing'), { code: 'ENOENT' })
        );
        const result = await configLoader.loadConfig('missing', { a: 1 });
        expect(result).toEqual({ a: 1 });
        expect(mocked.logger.warn).toHaveBeenCalled();
    });

    it('returns defaults for other read errors', async () => {
        mocked.readFile.mockRejectedValueOnce(Object.assign(new Error('bad'), { code: 'EACCES' }));
        const result = await configLoader.loadConfig('bad', { a: 1 });
        expect(result).toEqual({ a: 1 });
        expect(mocked.logger.error).toHaveBeenCalled();
    });

    it('gets nested values with dot paths', async () => {
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ a: { b: 2 } }));
        const value = await configLoader.getValue('sample.a.b', 9);
        expect(value).toBe(2);
        const missing = await configLoader.getValue('sample.a.c', 9);
        expect(missing).toBe(9);
    });

    it('reloads configuration by bypassing cache', async () => {
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ a: 1 }));
        await configLoader.loadConfig('sample', {});
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ a: 2 }));
        const reloaded = await configLoader.reloadConfig('sample');
        expect(reloaded).toEqual({ a: 2 });
        expect(mocked.readFile).toHaveBeenCalledTimes(2);
    });

    it('clears cache and logs', () => {
        configLoader.cache.set('x', { a: 1 });
        configLoader.clearCache();
        expect(configLoader.cache.size).toBe(0);
        expect(mocked.logger.debug).toHaveBeenCalledWith('Configuration cache cleared');
    });

    it('validates configuration with schema rules', () => {
        const result = configLoader.validateConfig(
            { a: -1, b: 2 },
            {
                a: { required: true, type: 'number', min: 0, max: 2 },
                b: { type: 'string' },
                c: { required: true },
            }
        );
        expect(result.isValid).toBe(false);
        expect(result.errors.length).toBe(3);
    });

    it('returns timeout defaults when missing', async () => {
        mocked.readFile.mockRejectedValueOnce(
            Object.assign(new Error('missing'), { code: 'ENOENT' })
        );
        const timeouts = await getTimeouts();
        expect(timeouts.session.timeoutMs).toBe(1800000);
    });

    it('returns timeout values by path', async () => {
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ session: { timeoutMs: 123 } }));
        const value = await getTimeoutValue('session.timeoutMs', 0);
        expect(value).toBe(123);
    });

    it('returns browser api defaults when missing', async () => {
        mocked.readFile.mockRejectedValueOnce(
            Object.assign(new Error('missing'), { code: 'ENOENT' })
        );
        const config = await getBrowserAPI();
        expect(config.default).toBe('roxybrowser');
    });

    it('returns settings from file', async () => {
        mocked.readFile.mockResolvedValueOnce(JSON.stringify({ theme: 'dark' }));
        const settings = await getSettings();
        expect(settings.theme).toBe('dark');
    });
});
