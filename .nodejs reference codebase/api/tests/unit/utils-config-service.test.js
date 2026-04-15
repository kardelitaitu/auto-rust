/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/config-service.js (real module)
 * @module tests/unit/utils-config-service.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

vi.mock('@api/utils/configLoader.js', () => ({
    getSettings: vi.fn(() =>
        Promise.resolve({
            twitter: {
                activity: {
                    defaultCycles: 10,
                    defaultMinDuration: 300,
                    defaultMaxDuration: 540,
                    engagementLimits: {
                        replies: 3,
                        retweets: 1,
                        quotes: 1,
                        likes: 5,
                        follows: 2,
                        bookmarks: 2,
                    },
                },
                reply: {
                    probability: 0.6,
                    minChars: 10,
                    maxChars: 200,
                },
                quote: {
                    probability: 0.2,
                },
                timing: {
                    warmupMin: 2000,
                    warmupMax: 15000,
                    scrollMin: 300,
                    scrollMax: 700,
                    globalScrollMultiplier: 1.0,
                },
                phases: {
                    warmupPercent: 0.1,
                    activePercent: 0.7,
                    cooldownPercent: 0.2,
                },
            },
            humanization: {
                mouse: {
                    speed: { mean: 1.0, deviation: 0.2 },
                    jitter: { x: 10, y: 5 },
                },
                typing: {
                    keystrokeDelay: { mean: 80, deviation: 40 },
                    errorRate: 0.05,
                },
                session: {
                    minMinutes: 5,
                    maxMinutes: 9,
                },
            },
            llm: {
                local: {
                    vllm: { enabled: false },
                    ollama: { enabled: true },
                    docker: { enabled: false },
                },
                cloud: {
                    enabled: true,
                },
            },
        })
    ),
}));

describe('utils/config-service', () => {
    let config;

    beforeEach(async () => {
        const { config: configModule } = await import('../../utils/config-service.js');
        config = configModule;
        await config.reload();
    });

    describe('Initialization', () => {
        it('should be initialized after import', async () => {
            expect(config._initialized).toBe(true);
        });

        it('should load settings on init', async () => {
            const settings = await config.getSettings();
            expect(settings).toBeDefined();
            expect(settings.twitter).toBeDefined();
        });

        it('should be idempotent', async () => {
            await config.init();
            await config.init();
            expect(config._initialized).toBe(true);
        });
    });

    describe('getTwitterActivity', () => {
        it('should return twitter activity config', async () => {
            const activity = await config.getTwitterActivity();
            expect(activity).toBeDefined();
            expect(activity.defaultCycles).toBe(10);
            expect(activity.defaultMinDuration).toBe(300);
            expect(activity.defaultMaxDuration).toBe(540);
        });

        it('should include engagement limits', async () => {
            const activity = await config.getTwitterActivity();
            expect(activity.engagementLimits).toBeDefined();
            expect(activity.engagementLimits.replies).toBe(3);
        });
    });

    describe('getEngagementLimits', () => {
        it('should return engagement limits', async () => {
            const limits = await config.getEngagementLimits();
            expect(limits).toBeDefined();
            expect(limits.replies).toBe(3);
            expect(limits.likes).toBe(5);
            expect(limits.retweets).toBe(1);
            expect(limits.quotes).toBe(1);
            expect(limits.follows).toBe(2);
            expect(limits.bookmarks).toBe(2);
        });
    });

    describe('getReplyConfig', () => {
        it('should return reply config', async () => {
            const reply = await config.getReplyConfig();
            expect(reply).toBeDefined();
            expect(reply.probability).toBe(0.6);
            expect(reply.minChars).toBe(10);
            expect(reply.maxChars).toBe(200);
        });
    });

    describe('getQuoteConfig', () => {
        it('should return quote config', async () => {
            const quote = await config.getQuoteConfig();
            expect(quote).toBeDefined();
            expect(quote.probability).toBe(0.2);
        });
    });

    describe('getTiming', () => {
        it('should return timing config', async () => {
            const timing = await config.getTiming();
            expect(timing).toBeDefined();
            expect(timing.warmupMin).toBe(2000);
            expect(timing.warmupMax).toBe(15000);
            expect(timing.scrollMin).toBe(300);
            expect(timing.scrollMax).toBe(700);
        });

        it('should include globalScrollMultiplier', async () => {
            const timing = await config.getTiming();
            expect(timing.globalScrollMultiplier).toBe(1.0);
        });
    });

    describe('getSessionPhases', () => {
        it('should return session phases config', async () => {
            const phases = await config.getSessionPhases();
            expect(phases).toBeDefined();
            expect(phases.warmupPercent).toBe(0.1);
            expect(phases.activePercent).toBe(0.7);
            expect(phases.cooldownPercent).toBe(0.2);
        });
    });

    describe('getGlobalScrollMultiplier', () => {
        it('should return global scroll multiplier', async () => {
            const multiplier = await config.getGlobalScrollMultiplier();
            expect(multiplier).toBe(1.0);
        });
    });

    describe('getHumanization', () => {
        it('should return humanization config', async () => {
            const humanization = await config.getHumanization();
            expect(humanization).toBeDefined();
            expect(humanization.mouse).toBeDefined();
            expect(humanization.typing).toBeDefined();
            expect(humanization.session).toBeDefined();
        });

        it('should include mouse config', async () => {
            const mouse = await config.getMouseConfig();
            expect(mouse.speed).toBeDefined();
            expect(mouse.jitter).toBeDefined();
        });

        it('should include typing config', async () => {
            const typing = await config.getTypingConfig();
            expect(typing.keystrokeDelay).toBeDefined();
            expect(typing.errorRate).toBeDefined();
        });

        it('should include session config', async () => {
            const session = await config.getSessionConfig();
            expect(session.minMinutes).toBeDefined();
            expect(session.maxMinutes).toBeDefined();
        });
    });

    describe('getLLMConfig', () => {
        it('should return LLM config', async () => {
            const llm = await config.getLLMConfig();
            expect(llm).toBeDefined();
            expect(llm.local).toBeDefined();
            expect(llm.cloud).toBeDefined();
        });

        it('should detect local LLM enabled', async () => {
            const enabled = await config.isLocalLLMEnabled();
            expect(enabled).toBeDefined();
            expect(typeof enabled).toBe('boolean');
        });

        it('should detect cloud LLM enabled', async () => {
            const enabled = await config.isCloudLLMEnabled();
            expect(enabled).toBe(true);
        });
    });

    describe('getProfileConfig', () => {
        it('should return profile config for NewsJunkie', async () => {
            const profile = await config.getProfileConfig('NewsJunkie');
            expect(profile).toBeDefined();
            expect(profile.dive).toBe(0.45);
        });

        it('should return profile config for Casual', async () => {
            const profile = await config.getProfileConfig('Casual');
            expect(profile).toBeDefined();
            expect(profile.dive).toBe(0.25);
        });

        it('should return profile config for PowerUser', async () => {
            const profile = await config.getProfileConfig('PowerUser');
            expect(profile).toBeDefined();
            expect(profile.dive).toBe(0.55);
        });

        it('should return default profile for unknown type', async () => {
            const profile = await config.getProfileConfig('UnknownType');
            expect(profile).toBeDefined();
            expect(profile.dive).toBe(0.35); // Balanced default
        });
    });

    describe('get section method', () => {
        it('should get twitter section', async () => {
            const twitter = await config.get('twitter');
            expect(twitter).toBeDefined();
            expect(twitter.activity).toBeDefined();
        });

        it('should get humanization section', async () => {
            const humanization = await config.get('humanization');
            expect(humanization).toBeDefined();
        });

        it('should get llm section', async () => {
            const llm = await config.get('llm');
            expect(llm).toBeDefined();
        });

        it('should get subsection', async () => {
            const mouse = await config.get('humanization', 'mouse');
            expect(mouse).toBeDefined();
            expect(mouse.speed).toBeDefined();
        });
    });

    describe('reload', () => {
        it('should reload configuration', async () => {
            await config.reload();
            const settings = await config.getSettings();
            expect(settings).toBeDefined();
        });
    });

    describe('Environment variable overrides', () => {
        it('should use env var for global scroll multiplier', async () => {
            const original = process.env.GLOBAL_SCROLL_MULTIPLIER;
            process.env.GLOBAL_SCROLL_MULTIPLIER = '2.5';

            await config.reload();
            const multiplier = await config.getGlobalScrollMultiplier();

            expect(multiplier).toBe(2.5);

            if (original !== undefined) {
                process.env.GLOBAL_SCROLL_MULTIPLIER = original;
            } else {
                delete process.env.GLOBAL_SCROLL_MULTIPLIER;
            }
        });

        it('should use env var for twitter activity cycles', async () => {
            const original = process.env.TWITTER_ACTIVITY_CYCLES;
            process.env.TWITTER_ACTIVITY_CYCLES = '25';

            await config.reload();
            const activity = await config.getTwitterActivity();

            expect(activity.defaultCycles).toBe(25);

            if (original !== undefined) {
                process.env.TWITTER_ACTIVITY_CYCLES = original;
            } else {
                delete process.env.TWITTER_ACTIVITY_CYCLES;
            }
        });

        it('should use env var for reply probability', async () => {
            const original = process.env.TWITTER_REPLY_PROBABILITY;
            process.env.TWITTER_REPLY_PROBABILITY = '0.8';

            await config.reload();
            const reply = await config.getReplyConfig();

            expect(reply.probability).toBe(0.8);

            if (original !== undefined) {
                process.env.TWITTER_REPLY_PROBABILITY = original;
            } else {
                delete process.env.TWITTER_REPLY_PROBABILITY;
            }
        });

        it('should use env var for ollama enabled', async () => {
            const original = process.env.OLLAMA_ENABLED;
            process.env.OLLAMA_ENABLED = 'false';

            await config.reload();
            const enabled = await config.isLocalLLMEnabled();

            expect(enabled).toBe(false);

            if (original !== undefined) {
                process.env.OLLAMA_ENABLED = original;
            } else {
                delete process.env.OLLAMA_ENABLED;
            }
        });
    });

    describe('Convenience exports', () => {
        it('getTwitterActivity should work as standalone export', async () => {
            const { getTwitterActivity } = await import('../../utils/config-service.js');
            const activity = await getTwitterActivity();
            expect(activity).toBeDefined();
            expect(activity.defaultCycles).toBe(10);
        });

        it('getEngagementLimits should work as standalone export', async () => {
            const { getEngagementLimits } = await import('../../utils/config-service.js');
            const limits = await getEngagementLimits();
            expect(limits).toBeDefined();
            expect(limits.replies).toBe(3);
        });

        it('getTiming should work as standalone export', async () => {
            const { getTiming } = await import('../../utils/config-service.js');
            const timing = await getTiming();
            expect(timing).toBeDefined();
            expect(timing.warmupMin).toBe(2000);
        });
    });
});

describe('utils/config-service defaults', () => {
    let config;

    beforeEach(async () => {
        vi.mock('@api/utils/configLoader.js', () => ({
            getSettings: vi.fn(() => Promise.resolve({})),
        }));

        const { config: configModule } = await import('../../utils/config-service.js');
        config = configModule;
        await config.reload();
    });

    describe('getTwitterActivity with no settings', () => {
        it('should use defaults when no settings', async () => {
            const activity = await config.getTwitterActivity();
            expect(activity.defaultCycles).toBe(10);
            expect(activity.defaultMinDuration).toBe(300);
            expect(activity.defaultMaxDuration).toBe(540);
        });
    });

    describe('getEngagementLimits with no settings', () => {
        it('should use defaults when no settings', async () => {
            const limits = await config.getEngagementLimits();
            expect(limits.replies).toBe(3);
            expect(limits.likes).toBe(5);
        });
    });

    describe('getTiming with no settings', () => {
        it('should use defaults when no settings', async () => {
            const timing = await config.getTiming();
            expect(timing.warmupMin).toBe(2000);
            expect(timing.warmupMax).toBe(15000);
            expect(timing.globalScrollMultiplier).toBe(1.0);
        });
    });

    describe('getHumanization with no settings', () => {
        it('should use defaults when no settings', async () => {
            const humanization = await config.getHumanization();
            expect(humanization.mouse).toBeDefined();
            expect(humanization.typing).toBeDefined();
            expect(humanization.session).toBeDefined();
        });
    });
});

describe('utils/config-service edge cases', () => {
    describe('_getWithDefaults edge cases', () => {
        it('should return data when key not in defaults', async () => {
            vi.mock('@api/utils/configLoader.js', () => ({
                getSettings: vi.fn(() =>
                    Promise.resolve({
                        customSection: {
                            customKey: 'customValue',
                        },
                    })
                ),
            }));

            const { config } = await import('../../utils/config-service.js');
            await config.reload();

            const result = await config.get('customSection');
            expect(result.customKey).toBe('customValue');
        });
    });
});
