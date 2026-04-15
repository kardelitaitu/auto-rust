/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        goto: vi.fn().mockResolvedValue(undefined),
        getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
    },
}));
import { api } from '@api/index.js';
import { ActionPredictor } from '@api/behaviors/humanization/action.js';
import { mathUtils } from '@api/utils/math.js';
import * as scrollHelper from '@api/behaviors/scroll-helper.js';

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn(),
        gaussian: vi.fn(),
        roll: vi.fn(),
    },
}));

vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollRandom: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/utils/ghostCursor.js', () => ({
    GhostCursor: class {
        constructor(page) {
            this.page = page;
        }
        async click(element) {
            if (element && element.click) await element.click();
        }
        async move() {}
        async moveTo() {}
    },
}));

describe('ActionPredictor', () => {
    let actionPredictor;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            goto: vi.fn().mockResolvedValue(undefined),
            goBack: vi.fn().mockResolvedValue(undefined),
            $$: vi.fn().mockResolvedValue([]),
            $: vi.fn().mockResolvedValue(null),
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
            },
            evaluate: vi.fn().mockResolvedValue(undefined),
            isClosed: vi.fn().mockReturnValue(false),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
        };
        api.getPage.mockReturnValue(mockPage);

        mathUtils.randomInRange.mockImplementation((min, _max) => min);
        mathUtils.gaussian.mockImplementation((m) => m);
        mathUtils.roll.mockReturnValue(false);

        mockLogger = {
            log: vi.fn(),
            debug: vi.fn(),
        };

        actionPredictor = new ActionPredictor(mockLogger);
    });

    describe('predict', () => {
        it('should return a valid prediction object', () => {
            const prediction = actionPredictor.predict(0);

            expect(prediction).toHaveProperty('type');
            expect(prediction).toHaveProperty('confidence');
            expect(prediction).toHaveProperty('probabilities');
            expect(prediction).toHaveProperty('phase');
        });

        it('should identify warmup phase', () => {
            const prediction = actionPredictor.predict(4);
            expect(prediction.phase).toBe('warmup');
        });

        it('should identify active phase', () => {
            const prediction = actionPredictor.predict(10);
            expect(prediction.phase).toBe('active');
        });

        it('should identify established phase', () => {
            const prediction = actionPredictor.predict(30);
            expect(prediction.phase).toBe('established');
        });

        it('should identify fatigued phase', () => {
            const prediction = actionPredictor.predict(50);
            expect(prediction.phase).toBe('fatigued');
        });

        it('should adjust probabilities for fatigue', () => {
            const prediction = actionPredictor.predict(100);
            expect(prediction.probabilities).toBeDefined();
        });
    });

    describe('getProbabilities', () => {
        it('should return adjusted probabilities based on cycle count', () => {
            actionPredictor.cycleCount = 60;
            const probs = actionPredictor.getProbabilities();
            expect(probs.scroll).toBeDefined();
            expect(probs.click).toBeDefined();
            expect(probs.idle).toBeDefined();
        });
    });

    describe('selectAction', () => {
        it('should return an action type', () => {
            const action = actionPredictor.selectAction();
            expect(typeof action).toBe('string');
        });
    });

    describe('executeAction', () => {
        it('should execute scroll action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionScroll');
            await actionPredictor.executeAction(mockPage, 'scroll');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should execute click action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionClick');
            await actionPredictor.executeAction(mockPage, 'click');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should execute back action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionBack');
            await actionPredictor.executeAction(mockPage, 'back');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should execute explore action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionExplore');
            await actionPredictor.executeAction(mockPage, 'explore');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should execute profile action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionProfile');
            await actionPredictor.executeAction(mockPage, 'profile');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should execute idle action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionIdle');
            await actionPredictor.executeAction(mockPage, 'idle');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });

        it('should default to scroll for unknown action', async () => {
            const spy = vi.spyOn(actionPredictor, '_actionScroll');
            await actionPredictor.executeAction(mockPage, 'unknown');
            expect(spy).toHaveBeenCalledWith(mockPage);
        });
    });

    describe('_actionScroll', () => {
        it('should perform human scroll', async () => {
            await actionPredictor._actionScroll(mockPage);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should use random direction when Math.random is low', async () => {
            vi.spyOn(Math, 'random')
                .mockReturnValueOnce(0.2)
                .mockReturnValueOnce(0.8)
                .mockReturnValue(0.6);

            await actionPredictor._actionScroll(mockPage);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });
    });

    describe('_actionClick', () => {
        it('should click tweet if found', async () => {
            const scrollIntoView = vi.fn();
            const mockTweet = {
                evaluate: vi.fn((fn) => fn({ scrollIntoView })),
                $: vi.fn().mockResolvedValue({ click: vi.fn() }),
            };
            mockPage.$$.mockResolvedValue([mockTweet]);

            await actionPredictor._actionClick(mockPage);

            expect(mockTweet.evaluate).toHaveBeenCalled();
            expect(scrollIntoView).toHaveBeenCalled();
            expect(mockTweet.$).toHaveBeenCalled();
        });

        it('should do nothing if no tweets found', async () => {
            mockPage.$$.mockResolvedValue([]);

            await actionPredictor._actionClick(mockPage);
        });
    });

    describe('_actionBack', () => {
        it('should go back and wait', async () => {
            await actionPredictor._actionBack(mockPage);

            expect(mockPage.goBack).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should tolerate goBack rejection', async () => {
            mockPage.goBack.mockRejectedValue(new Error('fail'));

            await actionPredictor._actionBack(mockPage);

            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('_actionExplore', () => {
        it('should navigate to explore and wait', async () => {
            await actionPredictor._actionExplore(mockPage);

            expect(api.goto).toHaveBeenCalledWith('https://x.com/explore', expect.any(Object));
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('_actionProfile', () => {
        it('should click profile link if found', async () => {
            const mockLink = { click: vi.fn().mockResolvedValue(undefined) };
            mockPage.$$.mockResolvedValue([mockLink]);

            await actionPredictor._actionProfile(mockPage);

            expect(mockLink.click).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should tolerate profile click rejection', async () => {
            const mockLink = { click: vi.fn().mockRejectedValue(new Error('fail')) };
            mockPage.$$.mockResolvedValue([mockLink]);

            await actionPredictor._actionProfile(mockPage);

            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('_actionIdle', () => {
        it('should wait and potentially move mouse', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.6);

            await actionPredictor._actionIdle(mockPage);

            expect(api.wait).toHaveBeenCalled();
            expect(mockPage.mouse.move).toHaveBeenCalled();
        });
    });

    describe('_humanScroll', () => {
        it('should handle unknown intensity and up direction', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.8);

            await actionPredictor._humanScroll(mockPage, 'up', 'unknown');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });
    });

    describe('_weightedRandom', () => {
        it('should fall back to first item on invalid weights', () => {
            const result = actionPredictor._weightedRandom({
                first: { weight: Number.NaN },
                second: { weight: Number.NaN },
            });
            expect(result).toBe('first');
        });
    });

    describe('_calculateConfidence', () => {
        it('should cap confidence at 0.95 and handle missing max', () => {
            vi.spyOn(Math, 'random').mockReturnValue(0);
            const confidence = actionPredictor._calculateConfidence({ weight: 1 });
            expect(confidence).toBe(0.95);
        });
    });
});
