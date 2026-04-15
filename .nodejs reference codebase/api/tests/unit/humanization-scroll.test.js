/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { HumanScroll } from '@api/behaviors/humanization/scroll.js';
import { mathUtils } from '@api/utils/math.js';
import { entropy } from '@api/utils/entropyController.js';
import * as scrollHelper from '@api/behaviors/scroll-helper.js';

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn(),
        gaussian: vi.fn(),
        roll: vi.fn(),
        sample: vi.fn(),
    },
}));

vi.mock('@api/utils/entropyController.js', () => ({
    entropy: {
        reactionTime: vi.fn(),
    },
    EntropyController: function () {
        return {
            reactionTime: vi.fn(),
        };
    },
}));

vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollRandom: vi.fn().mockResolvedValue(undefined),
    scrollDown: vi.fn().mockResolvedValue(undefined),
    scrollUp: vi.fn().mockResolvedValue(undefined),
    scrollToTop: vi.fn().mockResolvedValue(undefined),
    scrollToBottom: vi.fn().mockResolvedValue(undefined),
    getScrollMultiplier: vi.fn().mockReturnValue(1.0),
    globalScroll: {
        scrollBy: vi.fn().mockResolvedValue(undefined),
        scrollRandom: vi.fn().mockResolvedValue(undefined),
    },
}));

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
        scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
            toTop: vi.fn().mockResolvedValue(undefined),
            back: vi.fn().mockResolvedValue(undefined),
            read: vi.fn().mockResolvedValue(undefined),
        }),
        visible: vi.fn().mockResolvedValue(true),
        exists: vi.fn().mockResolvedValue(true),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/home'),
    },
}));
import { api } from '@api/index.js';

describe('HumanScroll', () => {
    let humanScroll;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        const mockPageForApi = {
            isClosed: () => false,
            context: () => ({ browser: () => ({ isConnected: () => true }) }),
        };
        if (typeof api !== 'undefined' && api.getPage) api.getPage.mockReturnValue(mockPageForApi);
        vi.clearAllMocks();

        mockPage = {
            evaluate: vi.fn(),
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            mouse: {
                wheel: vi.fn().mockResolvedValue(undefined),
            },
        };

        mockLogger = {
            log: vi.fn(),
            debug: vi.fn(),
            error: vi.fn(),
        };

        mathUtils.randomInRange.mockImplementation((min, _max) => min);
        mathUtils.gaussian.mockReturnValue(100);
        mathUtils.roll.mockReturnValue(false);
        entropy.reactionTime.mockReturnValue(100);

        humanScroll = new HumanScroll(mockPage, mockLogger);
    });

    describe('execute', () => {
        it('should scroll down with normal intensity by default', async () => {
            mathUtils.randomInRange.mockReturnValue(100);

            await humanScroll.execute();

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should handle "up" direction', async () => {
            mathUtils.randomInRange.mockReturnValue(100);

            await humanScroll.execute('up');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should respect intensity configurations', async () => {
            mathUtils.randomInRange.mockImplementation((min, max) => {
                if (min === 2 && max === 4) return 2;
                return 100;
            });

            await humanScroll.execute('down', 'light');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should log if agent is set', async () => {
            const mockAgent = { log: vi.fn() };
            humanScroll.setAgent(mockAgent);

            await humanScroll.execute();

            expect(mockAgent.log).toHaveBeenCalled();
        });

        it('should handle random direction', async () => {
            vi.spyOn(Math, 'random').mockReturnValue(0.6);

            await humanScroll.execute('random');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should fallback to normal intensity for unknown', async () => {
            mathUtils.randomInRange.mockReturnValue(100);

            await humanScroll.execute('down', 'unknown');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should scroll back slightly on roll when not first burst', async () => {
            mathUtils.randomInRange.mockImplementation((min, max) => {
                if (min === 2 && max === 4) return 2;
                return 100;
            });
            mathUtils.roll.mockReturnValue(true);

            await humanScroll.execute('down', 'normal');

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });
    });

    describe('toElement', () => {
        let mockLocator;
        let mockElement;
        let mockBox;

        beforeEach(() => {
            const mockPageForApi = {
                isClosed: () => false,
                context: () => ({ browser: () => ({ isConnected: () => true }) }),
            };
            if (typeof api !== 'undefined' && api.getPage)
                api.getPage.mockReturnValue(mockPageForApi);
            mockBox = { x: 0, y: 1000, width: 100, height: 100 };
            mockElement = {
                boundingBox: vi.fn().mockResolvedValue(mockBox),
            };
            mockLocator = {
                first: vi.fn().mockResolvedValue(mockElement),
            };

            mockPage.evaluate.mockResolvedValue(800);
        });

        it('should scroll to element when it is far away', async () => {
            await humanScroll.toElement(mockLocator);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should micro-adjust when already close', async () => {
            mockBox.y = 400;

            await humanScroll.toElement(mockLocator);

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should handle missing element gracefully', async () => {
            mockLocator.first.mockResolvedValue(null);

            await humanScroll.toElement(mockLocator);

            expect(scrollHelper.scrollRandom).not.toHaveBeenCalled();
        });

        it('should return when element has no bounding box', async () => {
            mockElement.boundingBox.mockResolvedValue(null);

            await humanScroll.toElement(mockLocator);

            expect(scrollHelper.scrollRandom).not.toHaveBeenCalled();
        });

        it('should fallback to direct scroll on error', async () => {
            mockLocator.first.mockRejectedValue(new Error('Locator error'));

            await humanScroll.toElement(mockLocator);

            expect(scrollHelper.scrollRandom).toHaveBeenCalledWith(200, 200);
        });
    });

    describe('microAdjustments', () => {
        it('should perform random small scrolls', async () => {
            mathUtils.randomInRange.mockReturnValue(2);

            await humanScroll.microAdjustments();

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('quickCheck', () => {
        it('should execute light down scroll', async () => {
            const executeSpy = vi.spyOn(humanScroll, 'execute');

            await humanScroll.quickCheck();

            expect(executeSpy).toHaveBeenCalledWith('down', 'light');
        });
    });

    describe('scrollToTop', () => {
        it('should scroll up multiple times', async () => {
            await humanScroll.scrollToTop();

            expect(scrollHelper.scrollRandom).toHaveBeenCalledTimes(4);
        });
    });

    describe('deepScroll', () => {
        it('should perform multiple scroll sessions', async () => {
            mathUtils.randomInRange.mockReturnValue(3);
            mathUtils.roll.mockReturnValue(true);

            await humanScroll.deepScroll();

            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });
    });
});
