/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for api/core/config.js
 * @module tests/unit/core-config.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock dependencies
vi.mock('@api/utils/config.js', () => ({
    getSettings: vi.fn().mockResolvedValue({
        agent: {
            llm: { model: 'test-model' },
        },
        timeouts: { navigation: 5000 },
    }),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        warn: vi.fn(),
        info: vi.fn(),
        error: vi.fn(),
    })),
}));

import { ConfigurationManager } from '@api/core/config.js';

describe('api/core/config', () => {
    let configManager;

    beforeEach(() => {
        vi.clearAllMocks();
        configManager = new ConfigurationManager();
    });

    it('init should load and merge settings', async () => {
        const config = await configManager.init();

        expect(config.agent.llm.model).toBe('test-model');
        expect(config.agent.llm.temperature).toBe(0.7); // Default
        expect(config.timeouts.navigation).toBe(5000); // Overridden
        expect(config.timeouts.element).toBe(10000); // Default
    });

    it('init should use DEFAULTS if getSettings fails', async () => {
        const { getSettings } = await import('@api/utils/config.js');
        getSettings.mockRejectedValueOnce(new Error('File not found'));

        const config = await configManager.init();

        expect(config.agent.llm.model).toBe('qwen3.5:4b'); // Default
    });

    it('init should return cached config if already initialized', async () => {
        const { getSettings } = await import('@api/utils/config.js');
        await configManager.init();
        await configManager.init();

        expect(getSettings).toHaveBeenCalledTimes(1);
    });

    it('get should return value from config', async () => {
        await configManager.init();
        expect(configManager.get('agent.llm.model')).toBe('test-model');
        expect(configManager.get('timeouts.navigation')).toBe(5000);
    });

    it('get should return defaultValue if not initialized', () => {
        expect(configManager.get('any.path', 'fallback')).toBe('fallback');
    });

    it('get should return defaultValue if path not found', async () => {
        await configManager.init();
        expect(configManager.get('non.existent.path', 'fallback')).toBe('fallback');
    });

    it('get should honor overrides', async () => {
        await configManager.init();
        configManager.setOverride('agent.llm.model', 'overridden-model');
        expect(configManager.get('agent.llm.model')).toBe('overridden-model');
    });

    it('clearOverrides should remove overrides', async () => {
        await configManager.init();
        configManager.setOverride('agent.llm.model', 'overridden-model');
        configManager.clearOverrides();
        expect(configManager.get('agent.llm.model')).toBe('test-model');
    });

    it('getFullConfig should return the config object', async () => {
        const config = await configManager.init();
        expect(configManager.getFullConfig()).toBe(config);
    });

    it('getFullConfig should return DEFAULTS if not initialized', () => {
        const config = configManager.getFullConfig();
        expect(config.agent.llm.model).toBe('qwen3.5:4b');
    });

    it('_getDefaults should return DEFAULTS', () => {
        const defaults = configManager._getDefaults();
        expect(defaults.agent.llm.model).toBe('qwen3.5:4b');
    });
});
