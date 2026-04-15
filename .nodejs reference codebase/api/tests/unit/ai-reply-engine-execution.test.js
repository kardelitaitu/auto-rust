/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/ai-reply-engine/execution.js
 * @module tests/unit/ai-reply-engine-execution.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        error: vi.fn(),
    })),
}));

vi.mock('@api/index.js', () => ({
    api: {
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
    },
}));

vi.mock('@api/utils/human-interaction.js', () => ({
    HumanInteraction: class {
        constructor() {
            this.debugMode = false;
        }
        selectMethod(methods) {
            return methods[0];
        }
        logStep() {}
        verifyComposerOpen() {
            return { open: true, selector: '[data-testid="tweetTextarea_0"]' };
        }
        verifyPostSent() {
            return Promise.resolve({ sent: true, method: 'posted' });
        }
        postTweet() {
            return Promise.resolve({ success: true, reason: 'posted' });
        }
        typeText() {
            return Promise.resolve();
        }
        safeHumanClick() {
            return Promise.resolve(true);
        }
        findElement() {
            return Promise.resolve({
                selector: 'test',
                element: { click: vi.fn() },
            });
        }
    },
}));

describe('ai-reply-engine/execution', () => {
    let executeReply;
    let AIReplyEngine;
    let mockEngine;
    let mockPage;

    const createMockPage = () => ({
        waitForSelector: vi.fn().mockResolvedValue(undefined),
        waitForTimeout: vi.fn().mockResolvedValue(undefined),
        evaluate: vi.fn(),
        keyboard: {
            press: vi.fn().mockResolvedValue(undefined),
        },
        mouse: {
            click: vi.fn().mockResolvedValue(undefined),
            move: vi.fn().mockResolvedValue(undefined),
        },
        locator: vi.fn(() => ({
            count: vi.fn().mockResolvedValue(1),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(undefined),
        })),
        viewportSize: vi.fn().mockReturnValue({ width: 1920, height: 1080 }),
    });

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../agent/ai-reply-engine/execution.js');
        executeReply = module.executeReply;

        const engineModule = await import('../../agent/ai-reply-engine/index.js');
        AIReplyEngine = engineModule.default;

        mockEngine = new AIReplyEngine({ processRequest: vi.fn(), sessionId: 'test' });
        mockEngine.logger = {
            info: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
            debug: vi.fn(),
        };

        mockPage = createMockPage();
    });

    describe('executeReply', () => {
        it('should log reply execution start', async () => {
            const result = await executeReply(mockEngine, mockPage, 'Test reply');

            expect(mockEngine.logger.info).toHaveBeenCalled();
        });

        it('should return result with success', async () => {
            const result = await executeReply(mockEngine, mockPage, 'Test reply');

            expect(result).toHaveProperty('success');
            expect(result).toHaveProperty('method');
        });

        it('should handle method from selected execution', async () => {
            const result = await executeReply(mockEngine, mockPage, 'Test reply');

            expect(result.method).toBeDefined();
        });

        it('should handle when page is undefined gracefully', async () => {
            const result = await executeReply(mockEngine, undefined, 'Test reply');

            expect(result.success).toBe(false);
        });
    });
});
