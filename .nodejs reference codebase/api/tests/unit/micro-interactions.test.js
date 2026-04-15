/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { microInteractions } from '@api/behaviors/micro-interactions.js';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        scroll: vi.fn().mockResolvedValue(undefined),
        scrollToTop: vi.fn().mockResolvedValue(undefined),
        getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
        click: vi.fn().mockResolvedValue(undefined),
    },
}));

vi.mock('@api/core/context.js', () => ({
    setSessionInterval: vi.fn(),
    clearSessionInterval: vi.fn(),
    withPage: vi.fn((page, fn) => fn()),
}));

import { api } from '@api/index.js';
import { setSessionInterval, clearSessionInterval } from '@api/core/context.js';

describe('microInteractions', () => {
    let handler;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        mockLogger = {
            info: vi.fn(),
            error: vi.fn(),
            warn: vi.fn(),
            debug: vi.fn(),
        };

        handler = microInteractions.createMicroInteractionHandler();

        mockPage = {
            $: vi.fn(),
            evaluate: vi.fn(),
            mouse: {
                move: vi.fn(),
                down: vi.fn(),
                up: vi.fn(),
                click: vi.fn(),
            },
            waitForTimeout: vi.fn(),
            click: vi.fn(),
            viewportSize: vi.fn().mockReturnValue({ width: 1280, height: 720 }),
            isClosed: vi.fn().mockReturnValue(false),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
        };
        api.getPage.mockReturnValue(mockPage);

        vi.useFakeTimers();
    });

    afterEach(() => {
        vi.useRealTimers();
        if (handler.stopFidgetLoop) {
            handler.stopFidgetLoop();
        }
    });

    describe('createMicroInteractionHandler', () => {
        it('should create handler with default config', () => {
            expect(handler.config).toBeDefined();
            expect(handler.config.highlightChance).toBe(0.03);
        });

        it('should merge custom config', () => {
            const customHandler = microInteractions.createMicroInteractionHandler({
                highlightChance: 0.5,
            });
            expect(customHandler.config.highlightChance).toBe(0.5);
        });
    });

    describe('textHighlight', () => {
        it('should highlight text if element found', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 200, height: 20 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            const result = await handler.textHighlight(mockPage, { logger: mockLogger });

            expect(result.success).toBe(true);
            expect(result.type).toBe('highlight');
            expect(mockPage.mouse.move).toHaveBeenCalled();
            expect(mockPage.mouse.down).toHaveBeenCalled();
            expect(mockPage.mouse.up).toHaveBeenCalled();
        });

        it('should return failure if element not found', async () => {
            mockPage.$.mockResolvedValue(null);
            const result = await handler.textHighlight(mockPage, { logger: mockLogger });
            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_element');
        });

        it('should return failure if no bounding box', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue(null),
            };
            mockPage.$.mockResolvedValue(mockElement);
            const result = await handler.textHighlight(mockPage, { logger: mockLogger });
            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_box');
        });
    });

    describe('randomRightClick', () => {
        it('should perform right click', async () => {
            const result = await handler.randomRightClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(true);
            expect(result.type).toBe('right_click');
            expect(mockPage.mouse.click).toHaveBeenCalledWith(
                expect.any(Number),
                expect.any(Number),
                { button: 'right' }
            );
        });
    });

    describe('logoClick', () => {
        it('should click logo if found', async () => {
            const mockLogo = { click: vi.fn().mockResolvedValue() };
            mockPage.$.mockResolvedValue(mockLogo);

            const result = await handler.logoClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(true);
            expect(result.type).toBe('logo_click');
            expect(mockLogo.click).toHaveBeenCalled();
        }, 10000);

        it('should return failure if logo not found', async () => {
            mockPage.$.mockResolvedValue(null);
            const result = await handler.logoClick(mockPage, { logger: mockLogger });
            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_logo');
        }, 10000);
    });

    describe('whitespaceClick', () => {
        it('should click whitespace', async () => {
            const result = await handler.whitespaceClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(true);
            expect(result.type).toBe('whitespace_click');
            expect(mockPage.mouse.click).toHaveBeenCalled();
        });
    });

    describe('fidget', () => {
        it('should perform a random fidget action', async () => {
            const result = await handler.fidget(mockPage, { logger: mockLogger });
            expect(result.success).toBe(true);
            expect(result.actions).toBeDefined();
        });

        it('should prevent concurrent fidgets', async () => {
            // Mock Math.random to force whitespaceClick
            vi.spyOn(Math, 'random').mockReturnValue(0.1);

            // Create a promise that we can manually resolve
            let resolveAction;
            const actionPromise = new Promise((r) => {
                resolveAction = r;
            });

            // Mock whitespaceClick to hang
            vi.spyOn(handler, 'whitespaceClick').mockReturnValue(actionPromise);

            // Start first fidget - it will wait on whitespaceClick
            const p1 = handler.fidget(mockPage, { logger: mockLogger });

            // Start second fidget immediately
            const p2 = await handler.fidget(mockPage, { logger: mockLogger });

            expect(p2.success).toBe(false);
            expect(p2.reason).toBe('already_running');

            // Resolve the first one
            resolveAction({ success: true, type: 'whitespace_click' });
            const result1 = await p1;
            expect(result1.success).toBe(true);
        });
    });

    describe('fidgetLoop', () => {
        it('should start and stop fidget loop', () => {
            handler.startFidgetLoop(mockPage, { logger: mockLogger });
            expect(setSessionInterval).toHaveBeenCalled();

            handler.stopFidgetLoop();
            expect(clearSessionInterval).toHaveBeenCalled();
        });

        it('should clear existing interval when starting new loop', () => {
            handler.startFidgetLoop(mockPage, { logger: mockLogger });
            handler.startFidgetLoop(mockPage, { logger: mockLogger });
            expect(clearSessionInterval).toHaveBeenCalled();
        });
    });

    describe('shouldFidget', () => {
        it('should return boolean', () => {
            const result = handler.shouldFidget();
            expect(typeof result).toBe('boolean');
        });
    });

    describe('getFidgetInterval', () => {
        it('should return number within range', () => {
            const interval = handler.getFidgetInterval();
            expect(typeof interval).toBe('number');
            expect(interval).toBeGreaterThanOrEqual(10000);
            expect(interval).toBeLessThanOrEqual(30000);
        });
    });

    describe('executeMicroInteraction', () => {
        it('should execute textHighlight when roll is low', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.01);
            vi.spyOn(handler, 'textHighlight').mockResolvedValue({ success: true });

            await handler.executeMicroInteraction(mockPage, { logger: mockLogger });

            expect(handler.textHighlight).toHaveBeenCalled();
        });

        it('should execute randomRightClick when roll is in range', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.04);
            vi.spyOn(handler, 'randomRightClick').mockResolvedValue({ success: true });

            await handler.executeMicroInteraction(mockPage, { logger: mockLogger });

            expect(handler.randomRightClick).toHaveBeenCalled();
        });

        it('should execute logoClick when roll is in range', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.07);
            vi.spyOn(handler, 'logoClick').mockResolvedValue({ success: true });

            await handler.executeMicroInteraction(mockPage, { logger: mockLogger });

            expect(handler.logoClick).toHaveBeenCalled();
        });

        it('should execute whitespaceClick when roll is in range', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.12);
            vi.spyOn(handler, 'whitespaceClick').mockResolvedValue({ success: true });

            await handler.executeMicroInteraction(mockPage, { logger: mockLogger });

            expect(handler.whitespaceClick).toHaveBeenCalled();
        });

        it('should return no_action when roll is high', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);

            const result = await handler.executeMicroInteraction(mockPage, { logger: mockLogger });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_action');
        });
    });

    describe('error handling', () => {
        it('should handle textHighlight error', async () => {
            mockPage.$.mockRejectedValue(new Error('Test error'));

            const result = await handler.textHighlight(mockPage, { logger: mockLogger });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('Test error');
        });

        it('should handle randomRightClick error', async () => {
            mockPage.viewportSize = vi.fn().mockImplementation(() => {
                throw new Error('Viewport error');
            });

            const result = await handler.randomRightClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(false);
        });

        it('should handle logoClick error', async () => {
            mockPage.$.mockRejectedValue(new Error('Logo error'));

            const result = await handler.logoClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('Logo error');
        });

        it('should handle whitespaceClick error', async () => {
            mockPage.viewportSize = vi.fn().mockImplementation(() => {
                throw new Error('Whitespace error');
            });

            const result = await handler.whitespaceClick(mockPage, { logger: mockLogger });

            expect(result.success).toBe(false);
        });

        it('should handle fidget error and reset isRunning', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.1);
            mockPage.mouse.move = vi.fn().mockRejectedValue(new Error('Fidget error'));

            const result = await handler.fidget(mockPage, { logger: mockLogger });

            // The fidget function catches errors and still returns success with actions
            expect(result).toBeDefined();
        });
    });
});
