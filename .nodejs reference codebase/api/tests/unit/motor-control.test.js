/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { motorControl } from '@api/behaviors/motor-control.js';

vi.mock('@api/index.js', () => ({
    api: {
        visible: vi.fn().mockImplementation(async (el) => {
            if (el && typeof el.isVisible === 'function') return await el.isVisible();
            return true;
        }),
        wait: vi.fn().mockResolvedValue(undefined),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/home'),
    },
}));
import { api } from '@api/index.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        error: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
    })),
}));

describe('motorControl', () => {
    let controller;
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();
        controller = motorControl.createMotorController();

        mockPage = {
            $: vi.fn(),
            evaluate: vi.fn(),
            mouse: {
                click: vi.fn(),
            },
            waitForTimeout: vi.fn(),
            waitForSelector: vi.fn(),
        };
    });

    describe('createMotorController', () => {
        it('should create with default config', () => {
            const ctrl = motorControl.createMotorController();
            expect(ctrl.config.layoutShiftThreshold).toBe(5);
            expect(ctrl.config.spiralSearchAttempts).toBe(4);
        });

        it('should merge custom config', () => {
            const ctrl = motorControl.createMotorController({ maxRetries: 10 });
            expect(ctrl.config.maxRetries).toBe(10);
            expect(ctrl.config.layoutShiftThreshold).toBe(5);
        });
    });

    describe('getXSelectors', () => {
        it('should return selectors for tweet_text', () => {
            const selectors = controller.getXSelectors('tweet_text');
            expect(selectors.primary).toBe('[data-testid="tweetText"]');
            expect(selectors.fallbacks.length).toBeGreaterThan(0);
            expect(selectors.fallbacks[0].selector).toBe(
                'article [role="group"] a[href*="/status"]'
            );
        });

        it('should return selectors for reply', () => {
            const selectors = controller.getXSelectors('reply');
            expect(selectors.primary).toBe('[data-testid="reply"]');
        });

        it('should return selectors for retweet', () => {
            const selectors = controller.getXSelectors('retweet');
            expect(selectors.primary).toBe('[data-testid="retweet"]');
        });

        it('should return selectors for like', () => {
            const selectors = controller.getXSelectors('like');
            expect(selectors.primary).toBe('[data-testid="like"]');
        });

        it('should return selectors for bookmark', () => {
            const selectors = controller.getXSelectors('bookmark');
            expect(selectors.primary).toBe('[data-testid="bookmark"]');
        });

        it('should return selectors for follow', () => {
            const selectors = controller.getXSelectors('follow');
            expect(selectors.primary).toBe('[data-testid="follow"]');
        });

        it('should return selectors for home', () => {
            const selectors = controller.getXSelectors('home');
            expect(selectors.primary).toBe('[aria-label="Home"]');
            expect(selectors.fallbacks.length).toBeGreaterThan(0);
            expect(selectors.fallbacks[0].selector).toBe('[data-testid="appleLogo"]');
        });

        it('should return primary as context for unknown context', () => {
            const selectors = controller.getXSelectors('unknown');
            expect(selectors.primary).toBe('unknown');
            expect(selectors.fallbacks).toEqual([]);
        });
    });

    describe('smartSelector', () => {
        it('should return primary selector if found and visible', async () => {
            const mockElement = { isVisible: vi.fn().mockResolvedValue(true) };
            mockPage.$.mockResolvedValue(mockElement);

            const result = await controller.smartSelector(mockPage, '.test-selector');

            expect(result.selector).toBe('.test-selector');
            expect(result.element).toBe(mockElement);
            expect(result.usedFallback).toBe(false);
        });

        it('should return null element if primary not found', async () => {
            mockPage.$.mockResolvedValue(null);

            const result = await controller.smartSelector(mockPage, '.test-selector');

            expect(result.element).toBeNull();
        });

        it('should try fallbacks if primary not visible', async () => {
            const invisibleElement = { isVisible: vi.fn().mockResolvedValue(false) };
            const visibleFallback = { isVisible: vi.fn().mockResolvedValue(true) };

            mockPage.$.mockResolvedValueOnce(invisibleElement).mockResolvedValueOnce(
                visibleFallback
            );

            const fallbacks = [{ selector: '.fallback1', reason: 'test_reason' }];

            const result = await controller.smartSelector(mockPage, '.primary', fallbacks);

            expect(result.usedFallback).toBe(true);
            expect(result.selector).toBe('.fallback1');
            expect(result.reason).toBe('test_reason');
        });

        it('should handle errors gracefully', async () => {
            mockPage.$.mockRejectedValue(new Error('Selector error'));

            const result = await controller.smartSelector(mockPage, '.test');

            expect(result.element).toBeNull();
        });
    });

    describe('getStableTarget', () => {
        it('should return timeout if element never stabilizes', async () => {
            const mockElement = {};
            const box1 = { x: 100, y: 100 };
            const box2 = { x: 200, y: 200 };

            mockPage.$.mockResolvedValue(mockElement);
            let callCount = 0;
            mockPage.evaluate = vi.fn().mockImplementation(() => {
                callCount++;
                return callCount % 2 === 1 ? box1 : box2;
            });
            mockPage.waitForTimeout = vi.fn().mockResolvedValue();

            const result = await controller.getStableTarget(mockPage, '.test', { timeout: 100 });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('timeout');
        });

        it('should return timeout if element not found', async () => {
            mockPage.$.mockResolvedValue(null);
            mockPage.waitForTimeout = vi.fn().mockResolvedValue();

            const result = await controller.getStableTarget(mockPage, '.test', { timeout: 100 });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('timeout');
        });
    });

    describe('checkOverlap', () => {
        it('should return element at position', async () => {
            const mockElement = { tagName: 'DIV' };
            mockPage.evaluate = vi.fn().mockResolvedValue(mockElement);

            const result = await controller.checkOverlap(mockPage, 100, 100);

            expect(result).toEqual(mockElement);
        });

        it('should return null on error', async () => {
            mockPage.evaluate = vi.fn().mockRejectedValue(new Error('Error'));

            const result = await controller.checkOverlap(mockPage, 100, 100);

            expect(result).toBeNull();
        });
    });

    describe('findUncoveredArea', () => {
        it('should find uncovered area', async () => {
            mockPage.evaluate.mockResolvedValueOnce(null).mockResolvedValue({ tagName: 'DIV' });

            const box = { x: 100, y: 100, width: 50, height: 50 };

            const result = await controller.findUncoveredArea(mockPage, box);

            expect(result.success).toBe(true);
        });

        it('should return failure if no uncovered area', async () => {
            mockPage.evaluate = vi.fn().mockResolvedValue({ tagName: 'DIV' });

            const box = { x: 100, y: 100, width: 50, height: 50 };

            const result = await controller.findUncoveredArea(mockPage, box);

            expect(result.success).toBe(false);
        });
    });

    describe('spiralSearch', () => {
        it('should find uncovered position', async () => {
            mockPage.evaluate.mockResolvedValueOnce(null);

            const result = await controller.spiralSearch(mockPage, 100, 100);

            expect(result.success).toBe(true);
            expect(result.attempts).toBe(1);
        });

        it('should return failure after max attempts', async () => {
            mockPage.evaluate = vi.fn().mockResolvedValue({ tagName: 'DIV' });

            const result = await controller.spiralSearch(mockPage, 100, 100, { maxAttempts: 2 });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('spiral_failed');
        });
    });

    describe('scrollToElement', () => {
        it('should scroll to element', async () => {
            const mockElement = {};
            const box = { x: 100, y: 200, width: 50, height: 50 };

            mockPage.$.mockResolvedValue(mockElement);
            mockElement.boundingBox = vi.fn().mockResolvedValue(box);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            const result = await controller.scrollToElement(mockPage, '.test');

            expect(result.success).toBe(true);
            expect(result.y).toBe(100);
        });

        it('should return failure if element not found', async () => {
            mockPage.$.mockResolvedValue(null);

            const result = await controller.scrollToElement(mockPage, '.test');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_element');
        });

        it('should return failure if no bounding box', async () => {
            const mockElement = {};
            mockPage.$.mockResolvedValue(mockElement);
            mockElement.boundingBox = vi.fn().mockResolvedValue(null);

            const result = await controller.scrollToElement(mockPage, '.test');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_box');
        });
    });

    describe('smartClick', () => {
        it('should return failure if no context or selector', async () => {
            const result = await controller.smartClick(mockPage, null);

            expect(result.success).toBe(false);
            expect(result.reason).toBe('no_context_or_selector');
        });

        it('should return failure if element not found', async () => {
            mockPage.$.mockResolvedValue(null);

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('selector_not_found');
        });
    });

    describe('defaults', () => {
        it('should export defaults', () => {
            expect(motorControl.defaults).toBeDefined();
            expect(motorControl.defaults.layoutShiftThreshold).toBe(5);
        });
    });

    describe('clickWithRecovery', () => {
        it('should click directly if stable', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);
            // Mock getStableTarget to return success
            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
                stable: true,
            });
            // Mock checkOverlap to return null (no overlap)
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(true);
            expect(mockPage.mouse.click).toHaveBeenCalledWith(125, 125);
        });

        it('should attempt scroll recovery if not stable', async () => {
            // First attempt fails, second succeeds (mocked by recursion or just checking logic)
            // Since it's recursive, we need to be careful.
            // Let's mock getStableTarget to fail first time, then succeed?
            // It calls itself recursively with recovery='spiral' if 'scroll' fails.

            const stableSpy = vi
                .spyOn(controller, 'getStableTarget')
                .mockResolvedValueOnce({ success: false }) // First call fails
                .mockResolvedValueOnce({
                    success: true,
                    box: { x: 100, y: 100, width: 50, height: 50 },
                }); // Second call (recursive) succeeds

            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);
            mockPage.$.mockResolvedValue({});

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(true);
            expect(mockPage.evaluate).toHaveBeenCalled(); // scrollBy
            expect(stableSpy).toHaveBeenCalledTimes(2);
        });
    });

    describe('clickWithVerification', () => {
        it('should verify click with selector', async () => {
            vi.spyOn(controller, 'clickWithRecovery').mockResolvedValue({ success: true });
            mockPage.waitForSelector.mockResolvedValue(true);

            const result = await controller.clickWithVerification(mockPage, '.target', {
                verifySelector: '.verified',
            });

            expect(result.verified).toBe(true);
            expect(mockPage.waitForSelector).toHaveBeenCalledWith('.verified', expect.any(Object));
        });

        it('should return unverified if selector does not appear', async () => {
            vi.spyOn(controller, 'clickWithRecovery').mockResolvedValue({ success: true });
            mockPage.waitForSelector.mockRejectedValue(new Error('Timeout'));

            const result = await controller.clickWithVerification(mockPage, '.target', {
                verifySelector: '.verified',
            });

            expect(result.verified).toBe(false);
        });

        it('should return result directly when no verifySelector provided', async () => {
            vi.spyOn(controller, 'clickWithRecovery').mockResolvedValue({
                success: true,
                x: 100,
                y: 100,
            });

            const result = await controller.clickWithVerification(mockPage, '.target');

            expect(result.success).toBe(true);
            expect(result.x).toBe(100);
        });

        it('should handle non-Error rejection from waitForSelector', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);
            mockPage.waitForSelector = vi.fn().mockRejectedValue('String error not an object');

            const result = await controller.smartClick(
                mockPage,
                { primary: '.target' },
                { verifySelector: '.verified' }
            );

            expect(result.success).toBe(true);
            expect(result.verified).toBe(false);
        });
    });

    describe('retryWithBackoff', () => {
        it('should retry on failure', async () => {
            const fn = vi
                .fn()
                .mockRejectedValueOnce(new Error('Fail 1'))
                .mockRejectedValueOnce(new Error('Fail 2'))
                .mockResolvedValue('Success');

            const result = await controller.retryWithBackoff(mockPage, fn, {
                maxRetries: 3,
                baseDelay: 10,
            });

            expect(result).toBe('Success');
            expect(fn).toHaveBeenCalledTimes(3);
        });

        it('should throw after max retries', async () => {
            const fn = vi.fn().mockRejectedValue(new Error('Fail'));

            await expect(
                controller.retryWithBackoff(mockPage, fn, { maxRetries: 3, baseDelay: 10 })
            ).rejects.toThrow('Fail');

            expect(fn).toHaveBeenCalledTimes(3);
        });
    });

    describe('smartClick - additional coverage', () => {
        it('should use stableResult.box over element.boundingBox', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 0, y: 0, width: 10, height: 10 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(true);
            expect(mockElement.boundingBox).not.toHaveBeenCalled();
        });

        it('should fallback to element.boundingBox when stableResult.box undefined', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: undefined,
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(true);
            expect(mockElement.boundingBox).toHaveBeenCalled();
        });

        it('should use default verifyTimeout when not provided', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);
            mockPage.waitForSelector = vi.fn().mockResolvedValue(true);

            const result = await controller.smartClick(mockPage, null, {
                context: 'reply',
                verifySelector: '.verified',
            });

            expect(result.success).toBe(true);
        });

        it('should handle overlap and use findUncoveredArea', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({
                success: true,
                x: 150,
                y: 150,
            });

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(true);
            expect(result.recovered).toBe(true);
            expect(mockPage.mouse.click).toHaveBeenCalledWith(150, 150);
        });

        it('should handle overlap and use spiralSearch when findUncoveredArea fails', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({ success: false });
            vi.spyOn(controller, 'spiralSearch').mockResolvedValue({
                success: true,
                x: 110,
                y: 110,
            });

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(true);
            expect(result.recovered).toBe(true);
            expect(mockPage.mouse.click).toHaveBeenCalledWith(110, 110);
        });

        it('should return failure when overlap cannot be recovered', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({ success: false });
            vi.spyOn(controller, 'spiralSearch').mockResolvedValue({ success: false });

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('overlapped_element');
        });

        it('should attempt recovery when target not stable', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            vi.spyOn(controller, 'getStableTarget')
                .mockResolvedValueOnce({ success: false })
                .mockResolvedValueOnce({
                    success: true,
                    box: { x: 100, y: 100, width: 50, height: 50 },
                });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue(null);

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(true);
        });

        it('should return failure when recovery also fails', async () => {
            const mockElement = {
                isVisible: vi.fn().mockResolvedValue(true),
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            vi.spyOn(controller, 'getStableTarget')
                .mockResolvedValueOnce({ success: false })
                .mockResolvedValueOnce({ success: false });

            const result = await controller.smartClick(mockPage, { primary: '.test' });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('target_not_stable');
        });
    });

    describe('clickWithRecovery - additional coverage', () => {
        it('should handle overlap and use findUncoveredArea', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
                stable: true,
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({
                success: true,
                x: 150,
                y: 150,
            });

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(true);
            expect(result.recovered).toBe(true);
            expect(mockPage.mouse.click).toHaveBeenCalledWith(150, 150);
        });

        it('should handle overlap and use spiralSearch when findUncoveredArea fails', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
                stable: true,
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({ success: false });
            vi.spyOn(controller, 'spiralSearch').mockResolvedValue({
                success: true,
                x: 120,
                y: 120,
            });

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(true);
            expect(result.recovered).toBe(true);
            expect(mockPage.mouse.click).toHaveBeenCalledWith(120, 120);
        });

        it('should return failure when recovery is none but scroll recovery also fails', async () => {
            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({ success: false });
            mockPage.$.mockResolvedValue({});
            mockPage.evaluate = vi.fn();

            const result = await controller.clickWithRecovery(mockPage, '.test', {
                recovery: 'none',
            });

            expect(result.success).toBe(false);
        });

        it('should return failure when overlap cannot be recovered in clickWithRecovery', async () => {
            const mockElement = {
                boundingBox: vi.fn().mockResolvedValue({ x: 100, y: 100, width: 50, height: 50 }),
            };
            mockPage.$.mockResolvedValue(mockElement);

            vi.spyOn(controller, 'getStableTarget').mockResolvedValue({
                success: true,
                box: { x: 100, y: 100, width: 50, height: 50 },
                stable: true,
            });
            vi.spyOn(controller, 'checkOverlap').mockResolvedValue({ tagName: 'SPAN' });
            vi.spyOn(controller, 'findUncoveredArea').mockResolvedValue({ success: false });
            vi.spyOn(controller, 'spiralSearch').mockResolvedValue({ success: false });

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('overlapped');
        });

        it('should catch errors and return failure', async () => {
            vi.spyOn(controller, 'getStableTarget').mockRejectedValue(new Error('Test error'));

            const result = await controller.clickWithRecovery(mockPage, '.test');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('Test error');
        });
    });

    describe('scrollToElement - additional coverage', () => {
        it('should scroll with smooth=true', async () => {
            const mockElement = {};
            const box = { x: 100, y: 200, width: 50, height: 50 };

            mockPage.$.mockResolvedValue(mockElement);
            mockElement.boundingBox = vi.fn().mockResolvedValue(box);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            const result = await controller.scrollToElement(mockPage, '.test', { smooth: true });

            expect(result.success).toBe(true);
            expect(mockPage.evaluate).toHaveBeenCalledWith(expect.any(Function), 100, true);
        });

        it('should scroll with smooth=false', async () => {
            const mockElement = {};
            const box = { x: 100, y: 200, width: 50, height: 50 };

            mockPage.$.mockResolvedValue(mockElement);
            mockElement.boundingBox = vi.fn().mockResolvedValue(box);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            const result = await controller.scrollToElement(mockPage, '.test', { smooth: false });

            expect(result.success).toBe(true);
            expect(mockPage.evaluate).toHaveBeenCalledWith(expect.any(Function), 100, false);
        });

        it('should handle error and return failure', async () => {
            mockPage.$.mockRejectedValue(new Error('Error'));

            const result = await controller.scrollToElement(mockPage, '.test');

            expect(result.success).toBe(false);
            expect(result.reason).toBe('Error');
        });

        it('should use custom offset', async () => {
            const mockElement = {};
            const box = { x: 100, y: 200, width: 50, height: 50 };

            mockPage.$.mockResolvedValue(mockElement);
            mockElement.boundingBox = vi.fn().mockResolvedValue(box);
            mockPage.evaluate = vi.fn();
            mockPage.waitForTimeout = vi.fn();

            const result = await controller.scrollToElement(mockPage, '.test', { offset: 50 });

            expect(result.success).toBe(true);
            expect(result.y).toBe(150);
        });
    });

    describe('getStableTarget - additional coverage', () => {
        it('should handle element becoming null during check', async () => {
            let callCount = 0;
            mockPage.$.mockImplementation(() => {
                callCount++;
                return callCount % 2 === 1 ? null : {};
            });
            mockPage.waitForTimeout = vi.fn().mockResolvedValue();

            const result = await controller.getStableTarget(mockPage, '.test', { timeout: 150 });

            expect(result.success).toBe(false);
        });

        it('should handle bounding box becoming null during check', async () => {
            const mockElement = {};
            let callCount = 0;
            mockPage.$.mockResolvedValue(mockElement);
            mockPage.evaluate = vi.fn().mockImplementation(() => {
                callCount++;
                if (callCount % 2 === 1) return null;
                return { x: 100, y: 100, width: 50, height: 50 };
            });
            mockPage.waitForTimeout = vi.fn().mockResolvedValue();

            const result = await controller.getStableTarget(mockPage, '.test', { timeout: 200 });

            expect(result.success).toBe(false);
        });
    });

    describe('spiralSearch - additional coverage', () => {
        it('should find uncovered position on second attempt', async () => {
            mockPage.evaluate.mockResolvedValueOnce({ tagName: 'DIV' }).mockResolvedValueOnce(null);

            const result = await controller.spiralSearch(mockPage, 100, 100, { maxAttempts: 3 });

            expect(result.success).toBe(true);
            expect(result.attempts).toBe(2);
        });

        it('should use custom maxAttempts', async () => {
            mockPage.evaluate = vi.fn().mockResolvedValue({ tagName: 'DIV' });

            const result = await controller.spiralSearch(mockPage, 100, 100, { maxAttempts: 5 });

            expect(result.success).toBe(false);
            expect(result.reason).toBe('spiral_failed');
        });
    });

    describe('retryWithBackoff - additional coverage', () => {
        it('should use custom jitter options', async () => {
            const fn = vi
                .fn()
                .mockRejectedValueOnce(new Error('Fail'))
                .mockResolvedValue('Success');

            const result = await controller.retryWithBackoff(mockPage, fn, {
                maxRetries: 2,
                baseDelay: 10,
                jitterMin: 1.0,
                jitterMax: 1.0,
            });

            expect(result).toBe('Success');
        });

        it('should use custom factor and maxDelay', async () => {
            const fn = vi
                .fn()
                .mockRejectedValueOnce(new Error('Fail'))
                .mockResolvedValue('Success');

            const result = await controller.retryWithBackoff(mockPage, fn, {
                maxRetries: 2,
                baseDelay: 10,
                factor: 3,
                maxDelay: 100,
            });

            expect(result).toBe('Success');
        });
    });

    describe('findUncoveredArea - additional coverage', () => {
        it('should try all offsets', async () => {
            mockPage.evaluate = vi.fn().mockResolvedValue({ tagName: 'DIV' });

            const box = { x: 100, y: 100, width: 50, height: 50 };

            await controller.findUncoveredArea(mockPage, box);

            expect(mockPage.evaluate).toHaveBeenCalledTimes(6);
        });
    });

    describe('smartSelector - additional coverage', () => {
        it('should skip fallback if element not visible', async () => {
            const invisibleElement1 = { isVisible: vi.fn().mockResolvedValue(false) };
            const invisibleElement2 = { isVisible: vi.fn().mockResolvedValue(false) };

            mockPage.$.mockResolvedValueOnce(invisibleElement1).mockResolvedValueOnce(
                invisibleElement2
            );

            const fallbacks = [
                { selector: '.fallback1', reason: 'test_reason1' },
                { selector: '.fallback2', reason: 'test_reason2' },
            ];

            const result = await controller.smartSelector(mockPage, '.primary', fallbacks);

            expect(result.element).toBeNull();
            expect(result.selector).toBe('.primary');
        });
    });
});
