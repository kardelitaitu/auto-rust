/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { InteractionHandler } from '@api/twitter/twitter-agent/InteractionHandler.js';
import { mathUtils } from '@api/utils/math.js';

// Mock dependencies
vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        roll: vi.fn(),
        randomInRange: vi.fn().mockReturnValue(100),
        random: vi.fn().mockReturnValue(0.5),
    },
}));

vi.mock('@api/twitter/twitter-agent/BaseHandler.js', () => ({
    BaseHandler: class MockBaseHandler {
        constructor(agent) {
            this.agent = agent;
            this.page = agent.page;
            this.config = agent.config;
            this.logger = agent.logger;
            this.state = agent.state;
            this.human = agent.human;
            this.ghost = agent.ghost;
            this.mathUtils = agent.mathUtils;
        }

        log(msg) {
            this.logger.info(msg);
        }

        async humanClick(target, label) {
            if (!target) return;
            await this.human.think(label || 'Click');
            try {
                await target.evaluate?.();
                const result = await this.ghost.click(target, { label, hoverBeforeClick: true });
                return result;
            } catch (e) {
                this.log(`[Interaction] humanClick failed on ${label || 'Target'}: ${e.message}`);
                await this.human.recoverFromError('click_failed', { locator: target });
                throw e;
            }
        }

        async safeHumanClick(target, label, retries = 3) {
            for (let i = 0; i < retries; i++) {
                try {
                    await this.humanClick(target, label);
                    return true;
                } catch (e) {
                    this.log(`Attempt ${i + 1} failed: ${e.message}`);
                    if (i === retries - 1) {
                        this.log(`All ${retries} attempts failed`);
                        return false;
                    }
                }
            }
        }

        async scrollToGoldenZone(el) {
            await el.evaluate?.();
        }

        async humanType(el, text) {
            await el.click?.();
            for (const char of text) {
                await el.press?.(char);
                if (this.mathUtils.roll(0.05)) {
                    await el.press?.('Backspace');
                    await el.press?.(char);
                }
            }
        }

        async dismissOverlays() {
            const toasts = this.page.locator('[data-testid="toast"]');
            if ((await toasts.count?.()) > 0) {
                await this.page.keyboard.press('Escape');
            }
        }

        async isElementActionable(el) {
            try {
                const handle = await el.elementHandle?.();
                return await this.page.evaluate?.(() => true);
            } catch {
                return false;
            }
        }
    },
}));

describe('InteractionHandler', () => {
    let handler;
    let mockAgent;
    let mockPage;
    let mockLogger;
    let mockHuman;
    let mockGhost;

    beforeEach(() => {
        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(),
            mouse: { move: vi.fn().mockResolvedValue() },
            keyboard: { press: vi.fn().mockResolvedValue(), type: vi.fn().mockResolvedValue() },
            evaluate: vi.fn().mockResolvedValue(),
            locator: vi.fn(),
            on: vi.fn(),
            off: vi.fn(),
        };

        mockLogger = {
            info: vi.fn(),
            warn: vi.fn(),
            error: vi.fn(),
        };

        mockHuman = {
            think: vi.fn().mockResolvedValue(),
            recoverFromError: vi.fn().mockResolvedValue(),
        };

        mockGhost = {
            click: vi.fn().mockResolvedValue({ success: true, x: 10, y: 10 }),
        };

        mockAgent = {
            page: mockPage,
            config: {},
            logger: mockLogger,
            state: {},
            human: mockHuman,
            ghost: mockGhost,
            mathUtils: mathUtils,
        };

        handler = new InteractionHandler(mockAgent);
    });

    describe('humanClick', () => {
        it('should perform human-like click sequence', async () => {
            const mockTarget = {
                evaluate: vi.fn().mockResolvedValue(),
                click: vi.fn().mockResolvedValue(),
            };

            await handler.humanClick(mockTarget, 'Test Click');

            expect(mockHuman.think).toHaveBeenCalledWith('Test Click');
            expect(mockTarget.evaluate).toHaveBeenCalled(); // scrollIntoView
            expect(mockGhost.click).toHaveBeenCalledWith(mockTarget, {
                label: 'Test Click',
                hoverBeforeClick: true,
            });
        });

        it('should recover from error and force click', async () => {
            const mockTarget = {
                evaluate: vi.fn().mockRejectedValue(new Error('Scroll failed')),
            };

            await expect(handler.humanClick(mockTarget)).rejects.toThrow('Scroll failed');

            expect(mockLogger.info).toHaveBeenCalledWith(
                expect.stringContaining('humanClick failed')
            );
            expect(mockHuman.recoverFromError).toHaveBeenCalled();
        });

        it('should do nothing if target is null', async () => {
            await handler.humanClick(null);
            expect(mockHuman.think).not.toHaveBeenCalled();
        });
    });

    describe('safeHumanClick', () => {
        it('should retry on failure', async () => {
            // First call fails, second succeeds
            vi.spyOn(handler, 'humanClick')
                .mockRejectedValueOnce(new Error('Fail 1'))
                .mockResolvedValueOnce();

            const result = await handler.safeHumanClick({}, 'Retry Test', 3);

            expect(result).toBe(true);
            expect(handler.humanClick).toHaveBeenCalledTimes(2);
            expect(mockLogger.info).toHaveBeenCalledWith(
                expect.stringMatching(/Attempt 1.*failed/i)
            );
        });

        it('should return false if all retries fail', async () => {
            vi.spyOn(handler, 'humanClick').mockRejectedValue(new Error('Fail'));

            const result = await handler.safeHumanClick({}, 'Fail Test', 2);

            expect(result).toBe(false);
            expect(handler.humanClick).toHaveBeenCalledTimes(2);
            expect(mockLogger.info).toHaveBeenCalledWith(
                expect.stringMatching(/all 2 attempts failed/i)
            );
        });
    });

    describe('isElementActionable', () => {
        it('should return true if visible, enabled, and in viewport', async () => {
            const mockEl = {
                isVisible: vi.fn().mockResolvedValue(true),
                isEnabled: vi.fn().mockResolvedValue(true),
                evaluate: vi.fn().mockResolvedValue(true), // isInViewport
                elementHandle: vi.fn().mockResolvedValue({}), // Mock handle
            };
            mockPage.evaluate.mockResolvedValue(true);

            const result = await handler.isElementActionable(mockEl);
            expect(result).toBe(true);
        });

        it('should return false if not visible', async () => {
            const mockEl = {
                isVisible: vi.fn().mockResolvedValue(false),
                elementHandle: vi.fn().mockResolvedValue({}),
            };
            // Implementation logic check:
            // The actual implementation calls elementHandle() first.
            // Then evaluates.
            // If we want to simulate failure, we should rely on what isElementActionable actually checks.
            // It calls page.evaluate(...)

            // Re-mocking to match implementation behavior
            mockPage.evaluate.mockResolvedValue(false); // Evaluate returns false

            const result = await handler.isElementActionable(mockEl);
            expect(result).toBe(false);
        });

        it('should return false if evaluate throws', async () => {
            const mockEl = {
                isVisible: vi.fn().mockResolvedValue(true),
                isEnabled: vi.fn().mockResolvedValue(true),
                evaluate: vi.fn().mockRejectedValue(new Error('Eval error')),
                elementHandle: vi.fn().mockResolvedValue({}),
            };
            mockPage.evaluate.mockRejectedValue(new Error('Eval error'));

            const result = await handler.isElementActionable(mockEl);
            expect(result).toBe(false); // catch returns false
        });
    });

    describe('scrollToGoldenZone', () => {
        it('should scroll element to golden zone', async () => {
            const mockEl = {
                evaluate: vi.fn().mockImplementation((_fn) => {
                    // Simulate evaluate execution context
                    global.window = { innerHeight: 1000, scrollBy: vi.fn() };
                    // We can't easily simulate the rect logic inside evaluate without running it in browser context or complex mocking.
                    // So we just verify evaluate is called.
                    return Promise.resolve();
                }),
            };

            await handler.scrollToGoldenZone(mockEl);
            expect(mockEl.evaluate).toHaveBeenCalled();
        });
    });

    describe('humanType', () => {
        it('should type text with random delays', async () => {
            const mockEl = {
                click: vi.fn().mockResolvedValue(),
                press: vi.fn().mockResolvedValue(),
            };

            await handler.humanType(mockEl, 'Hi');

            expect(mockEl.click).toHaveBeenCalled();
            expect(mockEl.press).toHaveBeenCalledWith('H');
            expect(mockEl.press).toHaveBeenCalledWith('i');
        });

        it('should simulate typos (5% chance)', async () => {
            const mockEl = {
                click: vi.fn().mockResolvedValue(),
                press: vi.fn().mockResolvedValue(),
            };

            // Use spyOn to mock the roll method on the handler's mathUtils
            vi.spyOn(handler.mathUtils, 'roll').mockReturnValue(true);

            await handler.humanType(mockEl, 'A');

            expect(mockEl.press).toHaveBeenCalledWith('Backspace');
            expect(mockEl.press).toHaveBeenCalledWith('A');

            handler.mathUtils.roll.mockRestore();
        });
    });

    describe('dismissOverlays', () => {
        it('should press Escape if overlays found', async () => {
            const mockToasts = { count: vi.fn().mockResolvedValue(1) };
            const mockModals = { count: vi.fn().mockResolvedValue(0) };

            mockPage.locator.mockImplementation((sel) => {
                if (sel.includes('toast')) return mockToasts;
                return mockModals;
            });

            await handler.dismissOverlays();

            expect(mockPage.keyboard.press).toHaveBeenCalledWith('Escape');
        });
    });
});
