/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/scroll-helper.js
 * @module tests/unit/scroll-helper.test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Create mock scroll with all methods before vi.mock hoisting
const mockScroll = vi.fn().mockResolvedValue(undefined);
mockScroll.toTop = vi.fn().mockResolvedValue(undefined);
mockScroll.toBottom = vi.fn().mockResolvedValue(undefined);
mockScroll.focus = vi.fn().mockResolvedValue(undefined);

vi.mock('@api/index.js', () => ({
    api: {
        wait: vi.fn().mockResolvedValue(undefined),
        scroll: mockScroll,
    },
}));

describe('utils/scroll-helper', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('scrollWheel should wait if delay is provided', async () => {
        const { scrollWheel } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollWheel(100, { delay: 500 });

        expect(api.wait).toHaveBeenCalledWith(500);
        expect(api.scroll).toHaveBeenCalledWith(100);
    });

    it('scrollWheel should not wait if delay is not provided', async () => {
        const { scrollWheel } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollWheel(100);

        expect(api.wait).not.toHaveBeenCalled();
        expect(api.scroll).toHaveBeenCalledWith(100);
    });

    it('scrollDown should scroll down with delay', async () => {
        const { scrollDown } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollDown(200, { delay: 500 });

        expect(api.wait).toHaveBeenCalledWith(500);
        expect(api.scroll).toHaveBeenCalledWith(200);
    });

    it('scrollUp should scroll up with delay', async () => {
        const { scrollUp } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollUp(200, { delay: 500 });

        expect(api.wait).toHaveBeenCalledWith(500);
        expect(api.scroll).toHaveBeenCalledWith(-200);
    });

    it('scrollRandom should use mathUtils and scroll with delay', async () => {
        const { scrollRandom } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        const mathUtils = (await import('../../utils/math.js')).mathUtils;
        vi.spyOn(mathUtils, 'randomInRange').mockReturnValue(150);

        await scrollRandom(100, 200, { delay: 500 });

        expect(api.wait).toHaveBeenCalledWith(500);
        expect(mathUtils.randomInRange).toHaveBeenCalledWith(100, 200);
        expect(api.scroll).toHaveBeenCalledWith(150);
    });

    it('scrollToTop should call api.scroll.toTop', async () => {
        const { scrollToTop } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollToTop();

        expect(api.scroll.toTop).toHaveBeenCalled();
    });

    it('scrollToBottom should call api.scroll.toBottom', async () => {
        const { scrollToBottom } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollToBottom();

        expect(api.scroll.toBottom).toHaveBeenCalled();
    });

    it('scroll should call api.scroll', async () => {
        const { scroll } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scroll(300);

        expect(api.scroll).toHaveBeenCalledWith(300);
    });

    it('getScrollMultiplier should return 1.0', async () => {
        const { getScrollMultiplier } = await import('../../behaviors/scroll-helper.js');
        expect(getScrollMultiplier()).toBe(1.0);
    });

    it('scrollToElement should call api.scroll.focus', async () => {
        const { scrollToElement } = await import('../../behaviors/scroll-helper.js');
        const api = (await import('@api/index.js')).api;

        await scrollToElement('#test-el', { behavior: 'smooth' });

        expect(api.scroll.focus).toHaveBeenCalledWith('#test-el', { behavior: 'smooth' });
    });
});
