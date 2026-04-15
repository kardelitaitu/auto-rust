/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    loggerContext: {
        run: vi.fn((ctx, fn) => fn()),
        getStore: vi.fn(),
    },
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
        success: vi.fn(),
    })),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        gaussian: (mean, _dev, min, max) => {
            let value = mean;
            if (min !== undefined) value = Math.max(min, value);
            if (max !== undefined) value = Math.min(max, value);
            return Math.floor(value);
        },
        randomInRange: (_min, _max) => 0,
        roll: () => false,
        sample: (array) => (array?.length ? array[0] : null),
        pidStep: (state, target) => {
            state.pos = target;
            return state.pos;
        },
    },
}));

vi.mock('@api/behaviors/timing.js', () => ({
    think: vi.fn().mockResolvedValue(undefined),
    delay: vi.fn().mockResolvedValue(undefined),
    gaussian: (mean, _dev, min, max) => {
        let value = mean;
        if (min !== undefined) value = Math.max(min, value);
        if (max !== undefined) value = Math.min(max, value);
        return Math.floor(value);
    },
    randomInRange: (_min, _max) => 0,
}));

vi.mock('@api/interactions/wait.js', () => ({
    wait: vi.fn().mockResolvedValue(undefined),
    waitWithAbort: vi.fn().mockResolvedValue(undefined),
    waitFor: vi.fn().mockResolvedValue(undefined),
    waitVisible: vi.fn().mockResolvedValue(undefined),
    waitHidden: vi.fn().mockResolvedValue(undefined),
    waitForLoadState: vi.fn().mockResolvedValue(undefined),
    waitForURL: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/utils/ghostCursor.js', () => ({
    GhostCursor: class {
        constructor(page) {
            this.page = page;
            this.previousPos = { x: 0, y: 0 };
        }
        async move(x, y) {
            this.previousPos = { x, y };
        }
        async click() {
            return { success: true, usedFallback: false };
        }
        async rightClick() {
            return { success: true, usedFallback: false };
        }
        async hoverWithDrift() {
            return undefined;
        }
    },
}));

import { api } from '@api/index.js';

describe('Unified API Integration', () => {
    let mockPage;
    let locator;
    let scrollY;

    beforeEach(() => {
        vi.clearAllMocks();
        scrollY = 0;

        locator = {
            first: () => locator,
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            innerText: vi.fn().mockResolvedValue('hello'),
            getAttribute: vi.fn().mockResolvedValue('value'),
            click: vi.fn().mockResolvedValue(undefined),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
            boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 120, width: 80, height: 20 }),
            evaluate: vi.fn().mockImplementation(async (fn) => {
                const source = fn?.toString?.() || '';
                if (source.includes('elementFromPoint')) {
                    // for isObscured
                    return false; // not obscured
                }
                if (source.includes('getBoundingClientRect')) {
                    return { x: 100, y: 120, width: 80, height: 20 };
                }
                return false;
            }),
            waitFor: vi.fn().mockResolvedValue(undefined),
        };

        mockPage = {
            url: vi.fn().mockReturnValue('https://example.com'),
            goto: vi.fn().mockResolvedValue(undefined),
            reload: vi.fn().mockResolvedValue(undefined),
            goBack: vi.fn().mockResolvedValue({}),
            goForward: vi.fn().mockResolvedValue(undefined),
            setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
            waitForSelector: vi.fn().mockResolvedValue(undefined),
            waitForLoadState: vi.fn().mockResolvedValue(undefined),
            waitForURL: vi.fn().mockResolvedValue(undefined),
            isClosed: vi.fn().mockReturnValue(false),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
            mouse: {
                wheel: vi.fn().mockImplementation(async (_x, y) => {
                    scrollY += y;
                }),
            },
            keyboard: {
                press: vi.fn().mockResolvedValue(undefined),
                type: vi.fn().mockResolvedValue(undefined),
            },
            locator: vi.fn().mockImplementation(() => locator),
            viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
            evaluate: vi.fn().mockImplementation(async (fn, arg) => {
                const source = fn?.toString?.() || '';
                if (source.includes('scrollY')) {
                    return scrollY;
                }
                if (source.includes('innerWidth') || source.includes('innerHeight')) {
                    return { width: 1280, height: 720 };
                }
                if (typeof arg === 'number') {
                    scrollY += arg;
                    return undefined;
                }
                return undefined;
            }),
        };
    });

    afterEach(() => {
        api.clearContext();
        vi.useRealTimers();
    });

    it('runs a unified api flow across modules', async () => {
        await api.withPage(mockPage, async () => {
            const clickPromise = api.click('.btn', { recovery: false, hoverBeforeClick: false });
            await clickPromise;

            const typePromise = api.type('.input', 'hello', { recovery: false, clearFirst: true });
            await typePromise;

            const hoverPromise = api.hover('.btn', { recovery: false });
            await hoverPromise;

            const rightClickPromise = api.rightClick('.btn', { recovery: false });
            await rightClickPromise;

            const focusPromise = api.scroll.focus('.btn');
            await focusPromise;

            const scrollPromise = api.scroll(200);
            await scrollPromise;

            const cursorPromise = api.cursor.move('.btn');
            await cursorPromise;

            const upPromise = api.cursor.up(10);
            await upPromise;

            const downPromise = api.cursor.down(15);
            await downPromise;

            await api.waitFor('.btn');
            await api.waitVisible('.btn');
            await api.waitHidden('.btn');
            await api.waitForLoadState('domcontentloaded');
            await api.waitForURL(/example/);

            await api.goto('https://example.com', { warmup: false });
            await api.reload();
            await api.back();
            await api.forward();
            await api.setExtraHTTPHeaders({ 'x-test': '1' });

            const textValue = await api.text('.btn');
            const attrValue = await api.attr('.btn', 'data-test');
            const isVisible = await api.visible('.btn');
            const countValue = await api.count('.btn');
            const existsValue = await api.exists('.btn');
            const currentUrl = await api.getCurrentUrl();

            expect(textValue).toBe('hello');
            expect(attrValue).toBe('value');
            expect(isVisible).toBe(true);
            expect(countValue).toBe(1);
            expect(existsValue).toBe(true);
            expect(currentUrl).toBe('https://example.com');
        });
    });
});
