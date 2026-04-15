/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { quoteWithAI } from '@api/actions/quote.js';

vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    evalPage: vi.fn(),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => (min + max) / 2),
        gaussian: vi.fn(() => 0.5),
    },
}));

vi.mock('@api/core/agent-connector.js', () => ({
    default: class {
        constructor() {
            this.chat = vi.fn().mockResolvedValue({ content: 'Test response' });
        }
    },
}));

vi.mock('@api/agent/ai-quote-engine.js', () => ({
    default: class {
        constructor() {
            this.generateQuote = vi.fn().mockResolvedValue({
                success: true,
                quote: 'Test quote text',
            });
            this.quoteMethodB_Retweet = vi.fn().mockResolvedValue({
                success: true,
                method: 'quoteMethodB',
            });
        }
    },
}));

vi.mock('@api/behaviors/human-interaction.js', () => ({
    HumanInteraction: class {
        constructor() {}
        moveTo() {
            return Promise.resolve();
        }
        click() {
            return Promise.resolve();
        }
    },
}));

vi.mock('@api/interactions/scroll.js', () => ({
    scroll: vi.fn().mockResolvedValue(undefined),
    focus: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/interactions/wait.js', () => ({
    wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/interactions/queries.js', () => ({
    text: vi.fn().mockResolvedValue('Test tweet text content'),
    exists: vi.fn().mockResolvedValue(true),
}));

import { getPage, evalPage } from '@api/core/context.js';
import { scroll, focus } from '@api/interactions/scroll.js';
import { text, exists } from '@api/interactions/queries.js';

describe('api/actions/quote.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            url: vi.fn().mockReturnValue('https://x.com/user/status/123456789'),
            locator: vi.fn().mockReturnValue({
                first: vi.fn().mockReturnValue({
                    click: vi.fn().mockResolvedValue(undefined),
                    focus: vi.fn().mockResolvedValue(undefined),
                }),
            }),
        };

        getPage.mockReturnValue(mockPage);
        evalPage.mockResolvedValue(['reply text 1', 'reply text 2']);
    });

    describe('quoteWithAI', () => {
        it('should successfully quote tweet with AI', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('This is a test tweet');

            const result = await quoteWithAI();

            expect(result.success).toBe(true);
            expect(result.quote).toBe('Test quote text');
            expect(scroll).toHaveBeenCalled();
            expect(focus).toHaveBeenCalled();
        });

        it('should extract username from URL', async () => {
            mockPage.url.mockReturnValue('https://x.com/testuser/status/123');
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');

            await quoteWithAI();

            expect(mockPage.url).toHaveBeenCalled();
        });

        it('should handle multiple context steps', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            evalPage.mockResolvedValueOnce(['reply 1', 'reply 2', 'reply 3']);

            await quoteWithAI({ contextSteps: 3 });

            expect(evalPage).toHaveBeenCalled();
        });

        it('should handle when tweet text not found initially', async () => {
            exists
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Found on third selector');

            const result = await quoteWithAI();

            expect(result.success).toBe(true);
        });

        it('should scroll down then up for context extraction', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');

            await quoteWithAI({ contextSteps: 2 });

            expect(scroll).toHaveBeenCalled();
        });

        it('should accept custom options', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');

            await quoteWithAI({
                fallback: 'Custom fallback',
                contextSteps: 10,
            });

            expect(scroll).toHaveBeenCalled();
        });

        it('should pass replies to context', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            evalPage.mockResolvedValue(['context reply 1', 'context reply 2']);

            await quoteWithAI();

            expect(evalPage).toHaveBeenCalled();
        });
    });
});
