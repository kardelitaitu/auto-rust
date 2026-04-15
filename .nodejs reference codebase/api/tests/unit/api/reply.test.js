/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { replyWithAI } from '@api/actions/reply.js';

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

vi.mock('@api/agent/ai-reply-engine/index.js', () => ({
    default: class {
        constructor() {
            this.generateReply = vi.fn().mockResolvedValue({
                success: true,
                reply: 'Test reply text',
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
    text: vi.fn().mockResolvedValue('Test tweet text'),
    exists: vi.fn().mockResolvedValue(true),
    visible: vi.fn().mockResolvedValue(true),
}));

vi.mock('@api/interactions/actions.js', () => ({
    click: vi.fn().mockResolvedValue(undefined),
    type: vi.fn().mockResolvedValue(undefined),
}));

import { getPage, evalPage } from '@api/core/context.js';
import { scroll, focus } from '@api/interactions/scroll.js';
import { text, exists, visible } from '@api/interactions/queries.js';
import { click, type } from '@api/interactions/actions.js';

describe('api/actions/reply.js', () => {
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

    describe('replyWithAI', () => {
        it('should successfully reply to tweet with AI', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('This is a test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);

            const result = await replyWithAI();

            expect(result.success).toBe(true);
            expect(result.reply).toBe('Test reply text');
            expect(click).toHaveBeenCalled();
            expect(type).toHaveBeenCalled();
            expect(focus).toHaveBeenCalled();
        });

        it('should extract username from URL', async () => {
            mockPage.url.mockReturnValue('https://x.com/testuser/status/123');
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);

            await replyWithAI();

            expect(mockPage.url).toHaveBeenCalled();
        });

        it('should handle multiple context steps', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);
            evalPage.mockResolvedValueOnce(['reply 1', 'reply 2', 'reply 3']);

            await replyWithAI({ contextSteps: 3 });

            expect(evalPage).toHaveBeenCalled();
        });

        it('should return success after posting (happy path)', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockReset();
            visible
                .mockResolvedValueOnce(true)
                .mockResolvedValueOnce(true)
                .mockResolvedValueOnce(true);

            const result = await replyWithAI();

            expect(result.success).toBe(true);
            expect(result.method).toBe('replyA');
        });

        it('should handle when tweet text not found initially', async () => {
            exists
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(false)
                .mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Found on third selector');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);

            const result = await replyWithAI();

            expect(result.success).toBe(true);
        });

        it('should scroll for context extraction', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);

            await replyWithAI({ contextSteps: 2 });

            expect(scroll).toHaveBeenCalled();
        });

        it('should accept custom options', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);

            await replyWithAI({
                fallback: 'Custom fallback',
                contextSteps: 10,
            });

            expect(scroll).toHaveBeenCalled();
        });

        it('should pass replies and url to context', async () => {
            exists.mockResolvedValueOnce(true);
            text.mockResolvedValueOnce('Test tweet');
            visible.mockResolvedValueOnce(true);
            visible.mockResolvedValueOnce(true);
            evalPage.mockResolvedValue(['context reply']);

            await replyWithAI();

            expect(evalPage).toHaveBeenCalled();
        });
    });
});
