/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as configLoader from '@api/utils/configLoader.js';

vi.mock('@api/utils/configLoader.js');
vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('ConfigService', () => {
    const originalEnv = process.env;
    let config;

    beforeEach(async () => {
        vi.resetAllMocks();
        vi.resetModules();
        process.env = { ...originalEnv };
        ({ config } = await import('../../utils/config-service.js'));
        await config.reload();
    });

    afterEach(() => {
        process.env = originalEnv;
    });

    it('should load settings from configLoader', async () => {
        const mockSettings = { twitter: { activity: { test: true } } };
        vi.mocked(configLoader.getSettings).mockResolvedValue(mockSettings);

        await config.reload(); // Reload to pick up new mock
        const settings = await config.getSettings();
        expect(settings.twitter.activity.test).toBe(true);
    });

    it('should use defaults when settings are missing', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        await config.reload();

        const activity = await config.getTwitterActivity();
        expect(activity.defaultCycles).toBe(10); // From DEFAULTS
    });

    it('should override with environment variables', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        process.env.TWITTER_ACTIVITY_CYCLES = '50';
        await config.reload();

        const activity = await config.getTwitterActivity();
        expect(activity.defaultCycles).toBe(50);
    });

    it('should handle nested env overrides', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        process.env.TWITTER_REPLY_PROBABILITY = '0.9';
        await config.reload();

        const replyConfig = await config.getReplyConfig();
        expect(replyConfig.probability).toBe(0.9);
    });

    it('should handle boolean env vars', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        process.env.LLM_CLOUD_ENABLED = 'false';
        await config.reload();

        const enabled = await config.isCloudLLMEnabled();
        expect(enabled).toBe(false);
    });

    it('should return specific config sections', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        await config.reload();

        const humanization = await config.getHumanization();
        expect(humanization.mouse).toBeDefined();

        const limits = await config.getEngagementLimits();
        expect(limits.likes).toBe(5);
    });

    it('should get global scroll multiplier', async () => {
        vi.mocked(configLoader.getSettings).mockResolvedValue({});
        process.env.GLOBAL_SCROLL_MULTIPLIER = '2.5';
        await config.reload();
        expect(await config.getGlobalScrollMultiplier()).toBe(2.5);
    });
});
