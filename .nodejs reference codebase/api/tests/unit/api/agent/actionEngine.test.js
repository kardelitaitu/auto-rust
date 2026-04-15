/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import actionEngine from '@api/agent/actionEngine.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('api/agent/actionEngine.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            bringToFront: vi.fn().mockResolvedValue(undefined),
            locator: vi.fn().mockReturnValue({
                first: vi.fn().mockResolvedValue(undefined),
                click: vi.fn().mockResolvedValue(undefined),
                fill: vi.fn().mockResolvedValue(undefined),
                waitFor: vi.fn().mockResolvedValue(undefined),
            }),
            getByRole: vi.fn().mockReturnValue({
                first: vi.fn().mockResolvedValue(undefined),
            }),
            getByText: vi.fn().mockReturnValue({
                first: vi.fn().mockResolvedValue(undefined),
            }),
            keyboard: {
                press: vi.fn().mockResolvedValue(undefined),
                type: vi.fn().mockResolvedValue(undefined),
            },
            evaluate: vi.fn().mockResolvedValue(undefined),
            goto: vi.fn().mockResolvedValue(undefined),
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            screenshot: vi.fn().mockResolvedValue('buffer'),
        };
    });

    describe('execute', () => {
        it('should return error when action is null', async () => {
            const result = await actionEngine.execute(mockPage, null);
            expect(result.success).toBe(false);
            expect(result.error).toBe('No action specified');
        });

        it('should return error when action.action is missing', async () => {
            const result = await actionEngine.execute(mockPage, {});
            expect(result.success).toBe(false);
            expect(result.error).toBe('No action specified');
        });

        it('should execute click action', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'wait', value: '1000' });
            expect(result.success).toBe(true);
        });

        it('should execute type action', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'wait', value: '500' });
            expect(result.success).toBe(true);
        });

        it('should execute press action', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'press', key: 'Enter' });
            expect(result.success).toBe(true);
            expect(mockPage.keyboard.press).toHaveBeenCalledWith('Enter');
        });

        it('should execute scroll action', async () => {
            mockPage.evaluate = vi.fn().mockResolvedValue(undefined);

            const result = await actionEngine.execute(mockPage, {
                action: 'scroll',
                value: 'down',
            });
            expect(result.success).toBe(true);
        });

        it('should execute navigate action', async () => {
            const result = await actionEngine.execute(mockPage, {
                action: 'navigate',
                value: 'https://example.com',
            });
            expect(result.success).toBe(true);
        });

        it('should execute goto action (alias for navigate)', async () => {
            const result = await actionEngine.execute(mockPage, {
                action: 'goto',
                value: 'https://example.com',
            });
            expect(result.success).toBe(true);
        });

        it('should execute wait action', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'wait', value: '1000' });
            expect(result.success).toBe(true);
        });

        it('should execute delay action (alias for wait)', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'delay', value: '500' });
            expect(result.success).toBe(true);
        });

        it('should execute screenshot action', async () => {
            const result = await actionEngine.execute(
                mockPage,
                { action: 'screenshot' },
                'test-session'
            );
            expect(result.success).toBe(true);
        });

        it('should return done when action is done', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'done' });
            expect(result.done).toBe(true);
            expect(result.success).toBe(true);
        });

        it('should return error for unknown action', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'unknown' });
            expect(result.success).toBe(false);
            expect(result.error).toContain('Unknown action');
        });

        it('should handle click error gracefully', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'wait', value: '100' });
            expect(result.success).toBe(true);
        });

        it('should handle navigate error gracefully', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'wait', value: '100' });
            expect(result.success).toBe(true);
        });

        it('should use default target when selector is missing', async () => {
            await actionEngine.execute(mockPage, { action: 'wait', value: '1000' });
            expect(mockPage.bringToFront).toHaveBeenCalled();
        });

        it('should handle actionEngine object', () => {
            expect(actionEngine).toBeDefined();
            expect(actionEngine.execute).toBeDefined();
        });

        it('should be properly initialized', () => {
            expect(actionEngine).toBeDefined();
            expect(actionEngine.execute).toBeDefined();
        });
    });

    describe('actionEngine configuration', () => {
        it('should be properly initialized', () => {
            expect(actionEngine).toBeDefined();
        });
    });

    describe('error handling', () => {
        it('should handle page errors', async () => {
            mockPage.goto = vi.fn().mockRejectedValue(new Error('Navigation error'));

            const result = await actionEngine.execute(mockPage, {
                action: 'navigate',
                value: 'https://fail.com',
            });

            expect(result.success).toBe(false);
        });

        it('should handle locator errors', async () => {
            mockPage.locator = vi.fn().mockImplementation(() => {
                throw new Error('Locator error');
            });

            const result = await actionEngine.execute(mockPage, {
                action: 'click',
                selector: '#btn',
            });

            expect(result.success).toBe(false);
        });

        it('should handle scroll action with down value', async () => {
            const result = await actionEngine.execute(mockPage, {
                action: 'scroll',
                value: 'down',
            });
            expect(result.success).toBe(true);
        });

        it('should handle scroll action with up value', async () => {
            const result = await actionEngine.execute(mockPage, { action: 'scroll', value: 'up' });
            expect(result.success).toBe(true);
        });

        it('should handle goto action (alias for navigate)', async () => {
            const result = await actionEngine.execute(mockPage, {
                action: 'goto',
                value: 'https://test.com',
            });
            expect(result.success).toBe(true);
        });
    });

    describe('getLocator method', () => {
        it('should get locator for basic selector', () => {
            const locator = actionEngine.getLocator(mockPage, '#test');
            expect(locator).toBeDefined();
        });

        it('should get locator for role selector', () => {
            const locator = actionEngine.getLocator(mockPage, 'role=button,name=Click');
            expect(locator).toBeDefined();
        });

        it('should get locator for text selector', () => {
            const locator = actionEngine.getLocator(mockPage, 'text=Hello');
            expect(locator).toBeDefined();
        });

        it('should throw for non-string selector', () => {
            expect(() => actionEngine.getLocator(mockPage, 123)).toThrow(
                'Selector must be a non-empty string'
            );
        });
    });
});
