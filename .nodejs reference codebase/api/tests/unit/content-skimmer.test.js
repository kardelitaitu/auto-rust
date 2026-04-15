/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ContentSkimmer } from '@api/behaviors/humanization/content.js';
import { api } from '@api/index.js';
import * as scrollHelper from '@api/behaviors/scroll-helper.js';
import { mathUtils } from '@api/utils/math.js';

vi.mock('@api/index.js', () => ({
    api: {
        wait: vi.fn().mockResolvedValue(true),
        scroll: Object.assign(vi.fn().mockResolvedValue(true), {
            toTop: vi.fn().mockResolvedValue(true),
            toBottom: vi.fn().mockResolvedValue(true),
            focus: vi.fn().mockResolvedValue(true),
        }),
    },
}));

vi.mock('@api/behaviors/scroll-helper.js', () => ({
    scrollRandom: vi.fn().mockResolvedValue(true),
}));

describe('ContentSkimmer', () => {
    let skimmer;
    let mockPage;
    let mockLogger;

    beforeEach(() => {
        vi.clearAllMocks();
        mockPage = {
            mouse: {
                move: vi.fn().mockResolvedValue(true),
            },
        };
        mockLogger = {
            info: vi.fn(),
            debug: vi.fn(),
        };
        skimmer = new ContentSkimmer(mockPage, mockLogger);
    });

    it('should set agent correctly', () => {
        const mockAgent = { log: vi.fn() };
        skimmer.setAgent(mockAgent);
        expect(skimmer.agent).toBe(mockAgent);
    });

    describe('skipping', () => {
        it('should handle tweet type', async () => {
            await skimmer.skipping('tweet', 'skim');
            expect(api.wait).toHaveBeenCalled();
            expect(mockPage.mouse.move).toHaveBeenCalled();
        });

        it('should handle thread type', async () => {
            vi.spyOn(mathUtils, 'randomInRange').mockReturnValue(2);
            await skimmer.skipping('thread', 'read');
            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should handle media type', async () => {
            await skimmer.skipping('media', 'glance');
            expect(api.wait).toHaveBeenCalled();
        });

        it('should handle profile type', async () => {
            await skimmer.skipping('profile', 'deep');
            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
        });

        it('should fallback to tweet for unknown type', async () => {
            await skimmer.skipping('unknown');
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('reading', () => {
        it('should wait for random duration and log if agent set', async () => {
            const mockAgent = { log: vi.fn() };
            skimmer.setAgent(mockAgent);
            await skimmer.reading('normal');
            expect(api.wait).toHaveBeenCalled();
            expect(mockAgent.log).toHaveBeenCalledWith(expect.stringContaining('[Read]'));
        });
    });

    describe('Specialized Patterns', () => {
        it('should execute skimFeed', async () => {
            await skimmer.skimFeed();
            expect(scrollHelper.scrollRandom).toHaveBeenCalled();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should execute deepRead', async () => {
            await skimmer.deepRead();
            expect(api.wait).toHaveBeenCalled();
        });

        it('should execute quickGlance', async () => {
            await skimmer.quickGlance();
            expect(api.wait).toHaveBeenCalled();
            expect(mockPage.mouse.move).toHaveBeenCalled();
        });
    });
});
