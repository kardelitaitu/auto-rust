/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ReferrerEngine } from '@api/utils/urlReferrer.js';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        goto: vi.fn(),
        setExtraHTTPHeaders: vi.fn(),
        click: vi.fn(),
        waitForURL: vi.fn(),
    },
}));
import { api } from '@api/index.js';

describe('ReferrerEngine Extra Coverage', () => {
    let engine;
    let mockPage;

    beforeEach(() => {
        engine = new ReferrerEngine({ addUTM: true });
        mockPage = {
            setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
            goto: vi.fn().mockResolvedValue(undefined),
            route: vi.fn().mockResolvedValue(undefined),
            unroute: vi.fn().mockResolvedValue(undefined),
            click: vi.fn().mockResolvedValue(undefined),
            waitForURL: vi.fn().mockResolvedValue(undefined),
            url: vi.fn().mockReturnValue('about:blank'),
            isClosed: vi.fn().mockReturnValue(false),
            viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
                click: vi.fn().mockResolvedValue(undefined),
                dblclick: vi.fn().mockResolvedValue(undefined),
                down: vi.fn().mockResolvedValue(undefined),
                up: vi.fn().mockResolvedValue(undefined),
            },
            context: () => ({
                browser: () => ({ isConnected: () => true }),
            }),
        };
        api.setPage(mockPage);
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('Strategy Coverage', () => {
        it('should generate telegram_web strategy (execution path only)', () => {
            // We need to trigger telegram_web execution.
            // Range: 0.70 <= r < 0.80
            // Inside telegram_web, it calls Math.random() again for version.

            // Case 1: Version 'k' (random > 0.5)
            vi.spyOn(Math, 'random')
                .mockReturnValueOnce(0.75) // Strategy
                .mockReturnValueOnce(0.6); // Version k

            let ctx = engine.generateContext('https://target.com');
            expect(ctx.strategy).toBe('telegram_web');
            // Naturalized to origin
            expect(ctx.referrer).toBe('https://web.telegram.org/');

            // Case 2: Version 'a' (random <= 0.5)
            vi.spyOn(Math, 'random')
                .mockReturnValueOnce(0.75) // Strategy
                .mockReturnValueOnce(0.4); // Version a

            ctx = engine.generateContext('https://target.com');
            expect(ctx.strategy).toBe('telegram_web');
            expect(ctx.referrer).toBe('https://web.telegram.org/');
        });

        it('should use profile context for google_search', () => {
            // google_search: r < 0.25
            vi.spyOn(Math, 'random').mockReturnValue(0.2);

            // Profile URL
            const ctx = engine.generateContext('https://twitter.com/elonmusk');
            expect(ctx.strategy).toBe('google_search');

            // Check query contains username (decoded)
            const decoded = decodeURIComponent(ctx.referrer);
            expect(decoded).toContain('elonmusk');
            // It might pick from various templates, but all should contain the username
            // e.g. "elonmusk twitter", "who is elonmusk twitter"
        });

        it('should use profile context with subpage for google_search', () => {
            // google_search: r < 0.25
            vi.spyOn(Math, 'random').mockReturnValue(0.2);

            // Profile Subpage URL
            const ctx = engine.generateContext('https://twitter.com/elonmusk/media');
            expect(ctx.strategy).toBe('google_search');

            const decoded = decodeURIComponent(ctx.referrer);
            expect(decoded).toContain('elonmusk');
        });

        it('should generate snowflake IDs for discord strategy', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.68); // 0.65 <= r < 0.70 -> discord_channel

            // discord_channel strategy generates a long URL which is then naturalized (truncated)
            // but generateSnowflake is called internally.
            // We can't verify the ID in the output because it's truncated to origin.
            // But we can verify the strategy selection and origin.
            const ctx = engine.generateContext('https://target.com');
            expect(ctx.strategy).toBe('discord_channel');
            expect(ctx.referrer).toBe('https://discord.com/');
        });

        it('should generate medium_article strategy', () => {
            vi.spyOn(Math, 'random')
                .mockReturnValueOnce(0.96) // r >= 0.95 (Long Tail)
                .mockReturnValueOnce(0.4) // r <= 0.5 (pick array)
                .mockReturnValueOnce(0.4); // pick index 1 (medium_article) [0.33 < 0.4 < 0.66]

            const ctx = engine.generateContext('https://target.com');
            expect(ctx.strategy).toBe('medium_article');
            expect(ctx.referrer).toBe('https://medium.com/'); // Naturalized to origin
        });

        it('should generate substack strategy', () => {
            vi.spyOn(Math, 'random')
                .mockReturnValueOnce(0.96) // r >= 0.95 (Long Tail)
                .mockReturnValueOnce(0.4) // r <= 0.5 (pick array)
                .mockReturnValueOnce(0.8); // pick index 2 (substack) [0.8 >= 0.66]

            const ctx = engine.generateContext('https://target.com');
            expect(ctx.strategy).toBe('substack');
            // Naturalized to origin? substack URL is `https://...substack.com/p/...`
            // PrivacyEngine naturalizes it?
            // "For everything else ... a real browser usually sends ONLY the Origin."
            // So it should be `https://technews.substack.com/` (or similar)
            expect(ctx.referrer).toMatch(/^https:\/\/[a-z]+\.substack\.com\/$/);
        });
    });

    describe('_extractContext Edge Cases', () => {
        // We test _extractContext indirectly via generateQuery (which is used by search strategies)
        // or by checking the resulting referrer query params if possible.
        // Or we can just trust that if we request a search strategy, it uses generateQuery.

        it('should treat reserved words as generic context (null context)', () => {
            // Force google_search strategy to use generateQuery
            vi.spyOn(Math, 'random').mockReturnValue(0.2);

            const reservedWords = ['notifications', 'messages', 'settings', 'search'];

            reservedWords.forEach((word) => {
                const ctx = engine.generateContext(`https://twitter.com/${word}`);
                // If context is null, generateQuery uses generic DICT topics
                // If context was found (profile), it would use "word twitter", "word x profile"

                // Generic fallback: "topic action context"
                // Profile fallback: "username twitter"

                // It's hard to distinguish deterministically without mocking DICT,
                // but we can check if it DOESN'T contain "notifications twitter" or "notifications x profile"
                // which would be the case if it treated it as a profile.

                const decoded = decodeURIComponent(ctx.referrer);
                // We expect it NOT to focus on the reserved word as a username
                // But since "notifications" is the "username" in the URL, if it WAS treated as profile,
                // the query would be "notifications twitter".
                // If it's generic, it's random words from DICT.

                // Let's assume generic dictionary doesn't contain "notifications".
                expect(decoded).not.toContain(`${word} twitter`);
                expect(decoded).not.toContain(`${word} x profile`);
            });
        });

        it('should treat root url as generic context', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2);
            const ctx = engine.generateContext('https://twitter.com/');
            const decoded = decodeURIComponent(ctx.referrer);
            expect(decoded).not.toContain('twitter twitter');
        });

        it('should treat invalid profile subpages as generic context', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2);
            const ctx = engine.generateContext('https://twitter.com/username/invalid_subpage');
            const decoded = decodeURIComponent(ctx.referrer);
            expect(decoded).not.toContain('username twitter');
        });

        it('should treat incomplete status url as generic context', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2);
            const ctx = engine.generateContext('https://twitter.com/username/status'); // Missing ID
            const decoded = decodeURIComponent(ctx.referrer);
            expect(decoded).not.toContain('username status');
        });
    });

    describe('HeaderEngine Catch Block', () => {
        it('should return empty headers for invalid target URL with non-direct strategy', () => {
            // Force a non-direct strategy (e.g., google_search) to test the catch block
            vi.spyOn(Math, 'random').mockReturnValue(0.2); // google_search strategy

            const ctx = engine.generateContext('http://:invalid');
            // When URL parsing fails in HeaderEngine, it returns {}
            expect(ctx.headers['Sec-Fetch-Site']).toBeUndefined();
            expect(ctx.headers['Sec-Fetch-Mode']).toBeUndefined();
        });

        it('should return "none" headers for direct traffic even with invalid URL', () => {
            // Force direct strategy (r < 0.10)
            vi.spyOn(Math, 'random').mockReturnValue(0.05);

            const ctx = engine.generateContext('http://:invalid');
            // Direct traffic returns 'none' headers regardless of URL validity
            expect(ctx.headers['Sec-Fetch-Site']).toBe('none');
            expect(ctx.headers['Sec-Fetch-Mode']).toBe('navigate');
        });
    });

    describe('Trampoline Navigation Edge Cases', () => {
        it('should continue navigation if page.click fails', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2); // Trampoline strategy

            // Setup successful route interception
            mockPage.route.mockImplementation((pattern, handler) => {
                if (typeof handler === 'function') {
                    // Simulate handler calling fulfill
                    handler({ fulfill: vi.fn() });
                }
                return Promise.resolve();
            });

            // Fail the click
            mockPage.click.mockRejectedValue(new Error('Click failed'));

            await engine.navigate(mockPage, 'https://target.com');

            // Should still wait for URL (meaning it didn't throw and proceeded)
            expect(api.waitForURL).toHaveBeenCalled();
            expect(api.goto).toHaveBeenCalledWith(
                expect.stringContaining('google.com'),
                expect.any(Object)
            );
        });

        it('should fallback to direct goto if trampoline setup fails (route error)', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.2);

            mockPage.route.mockRejectedValue(new Error('Route failed'));

            await engine.navigate(mockPage, 'https://target.com');

            // Should fallback to goto with headers
            expect(api.goto).toHaveBeenCalledWith(
                expect.stringContaining('https://target.com'),
                expect.objectContaining({
                    referer: expect.any(String),
                })
            );
        });
    });

    describe('UTM Injection Edge Cases', () => {
        it('should handle URL parsing error in UTM injection', () => {
            // If targetUrl is invalid, new URL(targetUrl) throws in addUTM block
            // It should catch and leave targetWithParams as original
            const ctx = engine.generateContext('http://:invalid');
            expect(ctx.targetWithParams).toBe('http://:invalid');
        });
    });
});
