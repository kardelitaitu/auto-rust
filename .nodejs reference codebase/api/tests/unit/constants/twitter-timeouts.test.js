/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from 'vitest';
import { TWITTER_TIMEOUTS, importTimeouts } from '@api/constants/twitter-timeouts.js';

describe('api/constants/twitter-timeouts.js', () => {
    describe('TWITTER_TIMEOUTS', () => {
        it('should export an object with timeout constants', () => {
            expect(TWITTER_TIMEOUTS).toBeDefined();
            expect(typeof TWITTER_TIMEOUTS).toBe('object');
        });

        it('should have PAGE_LOAD timeout', () => {
            expect(TWITTER_TIMEOUTS.PAGE_LOAD).toBe(30000);
        });

        it('should have ELEMENT_VISIBLE timeout', () => {
            expect(TWITTER_TIMEOUTS.ELEMENT_VISIBLE).toBe(5000);
        });

        it('should have ELEMENT_CLICKABLE timeout', () => {
            expect(TWITTER_TIMEOUTS.ELEMENT_CLICKABLE).toBe(10000);
        });

        it('should have NAVIGATION timeout', () => {
            expect(TWITTER_TIMEOUTS.NAVIGATION).toBe(15000);
        });

        it('should have COMPOSER_OPEN timeout', () => {
            expect(TWITTER_TIMEOUTS.COMPOSER_OPEN).toBe(8000);
        });

        it('should have POST_SENT timeout', () => {
            expect(TWITTER_TIMEOUTS.POST_SENT).toBe(5000);
        });

        it('should have AI_GENERATION timeout', () => {
            expect(TWITTER_TIMEOUTS.AI_GENERATION).toBe(60000);
        });

        it('should have DIVE_TIMEOUT timeout', () => {
            expect(TWITTER_TIMEOUTS.DIVE_TIMEOUT).toBe(120000);
        });

        it('should have QUICK_MODE_TIMEOUT timeout', () => {
            expect(TWITTER_TIMEOUTS.QUICK_MODE_TIMEOUT).toBe(30000);
        });

        it('should have QUEUE_ITEM_TIMEOUT timeout', () => {
            expect(TWITTER_TIMEOUTS.QUEUE_ITEM_TIMEOUT).toBe(5000);
        });

        it('should have FALLBACK_TIMEOUT timeout', () => {
            expect(TWITTER_TIMEOUTS.FALLBACK_TIMEOUT).toBe(3000);
        });

        it('should have all timeouts as positive numbers', () => {
            Object.values(TWITTER_TIMEOUTS).forEach((value) => {
                expect(typeof value).toBe('number');
                expect(value).toBeGreaterThan(0);
            });
        });
    });

    describe('importTimeouts', () => {
        it('should return default timeouts when no settings provided', () => {
            const result = importTimeouts();
            expect(result).toEqual(TWITTER_TIMEOUTS);
        });

        it('should return default timeouts when empty settings provided', () => {
            const result = importTimeouts({});
            expect(result).toEqual(TWITTER_TIMEOUTS);
        });

        it('should merge custom twitter timeouts', () => {
            const settings = {
                timeouts: {
                    twitter: {
                        PAGE_LOAD: 45000,
                        NEW_CUSTOM: 1000,
                    },
                },
            };

            const result = importTimeouts(settings);

            expect(result.PAGE_LOAD).toBe(45000);
            expect(result.NEW_CUSTOM).toBe(1000);
            expect(result.ELEMENT_VISIBLE).toBe(5000);
        });

        it('should not modify original TWITTER_TIMEOUTS', () => {
            const settings = {
                timeouts: {
                    twitter: {
                        PAGE_LOAD: 99999,
                    },
                },
            };

            importTimeouts(settings);

            expect(TWITTER_TIMEOUTS.PAGE_LOAD).toBe(30000);
        });

        it('should handle settings without twitter timeouts', () => {
            const settings = { other: 'value' };
            const result = importTimeouts(settings);

            expect(result).toEqual(TWITTER_TIMEOUTS);
        });

        it('should handle settings with empty twitter timeouts', () => {
            const settings = { timeouts: { twitter: {} } };
            const result = importTimeouts(settings);

            expect(result).toEqual(TWITTER_TIMEOUTS);
        });

        it('should handle settings with null twitter timeouts', () => {
            const settings = { timeouts: { twitter: null } };
            const result = importTimeouts(settings);

            expect(result).toEqual(TWITTER_TIMEOUTS);
        });
    });

    describe('default export', () => {
        it('should export TWITTER_TIMEOUTS as default', async () => {
            const module = await import('@api/constants/twitter-timeouts.js');
            expect(module.default).toEqual(TWITTER_TIMEOUTS);
        });
    });
});
