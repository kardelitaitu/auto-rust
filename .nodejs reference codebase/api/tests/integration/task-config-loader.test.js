/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Integration tests for task-config-loader
 * Tests actual module imports and real functionality
 * @module tests/integration/task-config-loader.test
 */

import { describe, it, expect } from 'vitest';
import {
    taskConfigLoader,
    TaskConfigLoader,
    loadAiTwitterActivityConfig,
} from '@api/utils/task-config-loader.js';

describe('taskConfigLoader Integration', () => {
    describe('Module Export', () => {
        it('should export taskConfigLoader singleton', () => {
            expect(taskConfigLoader).toBeDefined();
            expect(typeof taskConfigLoader).toBe('object');
        });

        it('should export TaskConfigLoader class', () => {
            expect(TaskConfigLoader).toBeDefined();
            expect(typeof TaskConfigLoader).toBe('function');
        });

        it('should export convenience function', () => {
            expect(loadAiTwitterActivityConfig).toBeDefined();
            expect(typeof loadAiTwitterActivityConfig).toBe('function');
        });
    });

    describe('TaskConfigLoader Class', () => {
        it('should instantiate with default properties', () => {
            const loader = new TaskConfigLoader();

            expect(loader.validator).toBeDefined();
            expect(loader.cache).toBeDefined();
            expect(loader.envConfig).toBeDefined();
            expect(loader.loadCount).toBe(0);
            expect(loader.hitCount).toBe(0);
        });

        it('should have all required methods', () => {
            const loader = new TaskConfigLoader();

            expect(loader.loadAiTwitterActivityConfig).toBeDefined();
            expect(typeof loader.loadAiTwitterActivityConfig).toBe('function');

            expect(loader.buildConfig).toBeDefined();
            expect(typeof loader.buildConfig).toBe('function');

            expect(loader.generateCacheKey).toBeDefined();
            expect(typeof loader.generateCacheKey).toBe('function');

            expect(loader.getStats).toBeDefined();
            expect(typeof loader.getStats).toBe('function');

            expect(loader.clearCache).toBeDefined();
            expect(typeof loader.clearCache).toBe('function');
        });
    });

    describe('loadAiTwitterActivityConfig', () => {
        it('should load configuration with default payload', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config).toBeDefined();
            expect(config.session).toBeDefined();
            expect(config.engagement).toBeDefined();
            expect(config.timing).toBeDefined();
            expect(config.humanization).toBeDefined();
            expect(config.ai).toBeDefined();
            expect(config.browser).toBeDefined();
            expect(config.system).toBeDefined();
        });

        it('should accept payload overrides', async () => {
            const payload = {
                cycles: 30,
                minDuration: 600,
                maxDuration: 900,
                theme: 'light',
            };

            const config = await taskConfigLoader.loadAiTwitterActivityConfig(payload);

            expect(config.session.cycles).toBe(30);
            expect(config.session.minDuration).toBe(600);
            expect(config.session.maxDuration).toBe(900);
            expect(config.browser.theme).toBe('light');
        });

        it('should use defaults when payload values not provided', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            // Check default values exist
            expect(config.session.cycles).toBeDefined();
            expect(config.session.minDuration).toBeDefined();
            expect(config.session.maxDuration).toBeDefined();
            expect(config.engagement.limits).toBeDefined();
            expect(config.timing.warmup).toBeDefined();
        });

        it('should include engagement probabilities', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.engagement.probabilities).toBeDefined();
            expect(config.engagement.probabilities.reply).toBeDefined();
            expect(config.engagement.probabilities.quote).toBeDefined();
            expect(config.engagement.probabilities.like).toBeDefined();
            expect(config.engagement.probabilities.bookmark).toBeDefined();
        });

        it('should include timing configuration', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.timing.warmup).toBeDefined();
            expect(config.timing.warmup.min).toBeDefined();
            expect(config.timing.warmup.max).toBeDefined();
            expect(config.timing.scroll).toBeDefined();
            expect(config.timing.read).toBeDefined();
            expect(config.timing.diveRead).toBeDefined();
            expect(config.timing.globalScrollMultiplier).toBeDefined();
        });

        it('should include humanization settings', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.humanization).toBeDefined();
            expect(config.humanization.mouse).toBeDefined();
            expect(config.humanization.typing).toBeDefined();
            expect(config.humanization.session).toBeDefined();
        });

        it('should include AI configuration', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.ai).toBeDefined();
            expect(config.ai.enabled).toBeDefined();
            expect(config.ai.localEnabled).toBeDefined();
            expect(config.ai.visionEnabled).toBeDefined();
            expect(config.ai.timeout).toBeDefined();
        });

        it('should include browser configuration', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.browser).toBeDefined();
            expect(config.browser.theme).toBeDefined();
            expect(config.browser.referrer).toBeDefined();
            expect(config.browser.headers).toBeDefined();
        });

        it('should include system configuration', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.system).toBeDefined();
            expect(config.system.debugMode).toBeDefined();
            expect(config.system.performanceTracking).toBeDefined();
            expect(config.system.errorRecovery).toBeDefined();
        });

        it('should include monitoring configuration', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig({});

            expect(config.monitoring).toBeDefined();
            expect(config.monitoring.queueMonitor).toBeDefined();
            expect(config.monitoring.engagementProgress).toBeDefined();
        });
    });

    describe('Cache Functionality', () => {
        it('should cache configuration after first load', async () => {
            // Clear cache first
            taskConfigLoader.clearCache();

            // First load
            await taskConfigLoader.loadAiTwitterActivityConfig({});

            // Get stats
            const stats = taskConfigLoader.getStats();
            expect(stats.cache.loadCount).toBeGreaterThan(0);
        });

        it('should have cache statistics', async () => {
            const stats = taskConfigLoader.getStats();

            expect(stats.cache).toBeDefined();
            expect(stats.cache.hitCount).toBeDefined();
            expect(stats.cache.loadCount).toBeDefined();
            expect(stats.cache.hitRate).toBeDefined();
            // cacheSize may not exist depending on implementation
        });

        it('should clear cache properly', async () => {
            // Load something
            await taskConfigLoader.loadAiTwitterActivityConfig({});

            // Clear cache
            taskConfigLoader.clearCache();

            // Check stats are reset
            const stats = taskConfigLoader.getStats();
            expect(stats.cache.hitCount).toBe(0);
            expect(stats.cache.loadCount).toBe(0);
        });
    });

    describe('generateCacheKey', () => {
        it('should generate consistent keys for same payload', () => {
            const loader = new TaskConfigLoader();

            const key1 = loader.generateCacheKey({ cycles: 10 });
            const key2 = loader.generateCacheKey({ cycles: 10 });

            expect(key1).toBe(key2);
        });

        it('should generate different keys for different payloads', () => {
            const loader = new TaskConfigLoader();

            const key1 = loader.generateCacheKey({ cycles: 10 });
            const key2 = loader.generateCacheKey({ cycles: 20 });

            expect(key1).not.toBe(key2);
        });

        it('should include environment variables in key', () => {
            const loader = new TaskConfigLoader();

            // Use one of the tracked env vars (DEBUG_MODE)
            process.env.DEBUG_MODE = 'true';

            const key = loader.generateCacheKey({});
            expect(key).toContain('DEBUG_MODE');
            expect(key).toContain('true');

            // Cleanup
            delete process.env.DEBUG_MODE;
        });
    });

    describe('buildConfig', () => {
        it('should build configuration with all sections', () => {
            const loader = new TaskConfigLoader();

            const mockConfig = {
                settings: { twitter: { reply: { probability: 0.5 } } },
                activityConfig: {
                    defaultCycles: 10,
                    defaultMinDuration: 300,
                    defaultMaxDuration: 540,
                },
                timingConfig: {
                    warmupMin: 2000,
                    warmupMax: 15000,
                    scrollMin: 300,
                    scrollMax: 700,
                    readMin: 5000,
                    readMax: 15000,
                },
                engagementLimits: { replies: 3, likes: 5 },
                humanizationConfig: { mouse: { speed: 1.0 } },
                payload: {},
            };

            const built = loader.buildConfig(mockConfig);

            expect(built.session).toBeDefined();
            expect(built.engagement).toBeDefined();
            expect(built.timing).toBeDefined();
            expect(built.humanization).toBeDefined();
            expect(built.ai).toBeDefined();
            expect(built.browser).toBeDefined();
            expect(built.monitoring).toBeDefined();
            expect(built.system).toBeDefined();
        });

        it('should apply payload overrides to session config', () => {
            const loader = new TaskConfigLoader();

            const mockConfig = {
                settings: {},
                activityConfig: {
                    defaultCycles: 10,
                    defaultMinDuration: 300,
                    defaultMaxDuration: 540,
                },
                timingConfig: { warmupMin: 2000, warmupMax: 15000 },
                engagementLimits: { replies: 3 },
                humanizationConfig: {},
                payload: { cycles: 50, minDuration: 600 },
            };

            const built = loader.buildConfig(mockConfig);

            expect(built.session.cycles).toBe(50);
            expect(built.session.minDuration).toBe(600);
            expect(built.session.maxDuration).toBe(540); // Not overridden
        });

        it('should use defaults when settings are missing', () => {
            const loader = new TaskConfigLoader();

            const mockConfig = {
                settings: {},
                activityConfig: {
                    defaultCycles: 10,
                    defaultMinDuration: 300,
                    defaultMaxDuration: 540,
                },
                timingConfig: {},
                engagementLimits: {},
                humanizationConfig: {},
                payload: {},
            };

            const built = loader.buildConfig(mockConfig);

            expect(built.engagement.probabilities.reply).toBe(0.5); // Default
            expect(built.engagement.probabilities.quote).toBe(0.2); // Default
            expect(built.timing.warmup.min).toBe(2000); // Default fallback
        });
    });

    describe('Convenience Function', () => {
        it('should work as backward compatibility function', async () => {
            const config = await loadAiTwitterActivityConfig({});

            expect(config).toBeDefined();
            expect(config.session).toBeDefined();
        });

        it('should pass payload to loader', async () => {
            const payload = { cycles: 25 };
            const config = await loadAiTwitterActivityConfig(payload);

            expect(config.session.cycles).toBe(25);
        });
    });

    describe('Error Handling', () => {
        it('should handle empty payload gracefully', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig();

            expect(config).toBeDefined();
            expect(config.session).toBeDefined();
        });

        it('should handle undefined payload gracefully', async () => {
            const config = await taskConfigLoader.loadAiTwitterActivityConfig(undefined);

            expect(config).toBeDefined();
            expect(config.session).toBeDefined();
        });
    });
});
