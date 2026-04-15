/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { api } from '@api/index.js';

vi.mock('@api/utils/ghostCursor.js', () => ({
    GhostCursor: class {
        constructor(page) {
            this.page = page;
            this.previousPos = { x: 0, y: 0 };
        }
        async move(x, y) {
            if (typeof x === 'string') {
                // Mock selector move
                this.previousPos = { x: 100, y: 100 };
            } else {
                this.previousPos = { x, y };
            }
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

vi.mock('@api/behaviors/timing.js', () => ({
    delay: vi.fn().mockResolvedValue(undefined),
    randomInRange: vi.fn().mockReturnValue(100),
    think: vi.fn().mockResolvedValue(undefined),
    gaussian: vi.fn().mockReturnValue(100),
}));

vi.mock('@api/behaviors/warmup.js', () => ({
    beforeNavigate: vi.fn().mockResolvedValue(undefined),
    randomMouse: vi.fn().mockResolvedValue(undefined),
    fakeRead: vi.fn().mockResolvedValue(undefined),
    pause: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/core/plugins/index.js', () => ({
    getPluginManager: vi.fn(() => ({
        evaluateUrl: vi.fn(),
    })),
    loadBuiltinPlugins: vi.fn(),
    registerPlugin: vi.fn(),
    unregisterPlugin: vi.fn(),
    enablePlugin: vi.fn(),
    disablePlugin: vi.fn(),
    listPlugins: vi.fn(),
    listEnabledPlugins: vi.fn(),
}));

vi.mock('@api/interactions/banners.js', () => ({
    handleBanners: vi.fn().mockResolvedValue(undefined),
}));

describe('Integration: Basic Flow (goto -> type -> click)', () => {
    let mockPage;
    let locator;

    beforeEach(() => {
        vi.clearAllMocks();

        locator = {
            first: () => locator,
            count: vi.fn().mockResolvedValue(1),
            isVisible: vi.fn().mockResolvedValue(true),
            waitFor: vi.fn().mockResolvedValue(undefined),
            click: vi.fn().mockResolvedValue(undefined),
            fill: vi.fn().mockResolvedValue(undefined),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(undefined),
            boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 20 }),
            evaluate: vi.fn().mockImplementation(async (fn) => {
                const source = fn?.toString?.() || '';
                if (source.includes('elementFromPoint')) {
                    // for isObscured
                    return false; // not obscured
                }
                if (source.includes('getBoundingClientRect')) {
                    return { left: 100, top: 100, width: 50, height: 20, right: 150, bottom: 120 };
                }
                return false;
            }),
        };

        mockPage = {
            url: vi.fn().mockReturnValue('https://example.com'),
            goto: vi.fn().mockResolvedValue(undefined),
            waitForSelector: vi.fn().mockResolvedValue(undefined),
            waitForLoadState: vi.fn().mockResolvedValue(undefined),
            viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
            isClosed: vi.fn().mockReturnValue(false),
            locator: vi.fn().mockImplementation(() => locator),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
            keyboard: {
                press: vi.fn().mockResolvedValue(undefined),
                type: vi.fn().mockResolvedValue(undefined),
            },
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
                click: vi.fn().mockResolvedValue(undefined),
            },
            evaluate: vi.fn().mockResolvedValue(undefined),
            reload: vi.fn().mockResolvedValue(undefined),
            goBack: vi.fn().mockResolvedValue({}),
            goForward: vi.fn().mockResolvedValue(undefined),
            setExtraHTTPHeaders: vi.fn().mockResolvedValue(undefined),
        };
    });

    afterEach(() => {
        api.clearContext();
    });

    it('successfully executes a basic interaction chain', async () => {
        await api.withPage(mockPage, async () => {
            // 1. Goto
            await api.goto('https://google.com', { warmup: false });
            expect(mockPage.goto).toHaveBeenCalledWith('https://google.com', expect.any(Object));

            // 2. Type
            await api.type('input[name="q"]', 'vitest testing', { delay: 0 });
            expect(mockPage.locator).toHaveBeenCalledWith('input[name="q"]');

            // 3. Click
            await api.click('button[type="submit"]', { recovery: false });
            expect(mockPage.locator).toHaveBeenCalledWith('button[type="submit"]');
        });
    });
});
