/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/index.js', () => {
    const api = {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        getPersona: vi.fn().mockReturnValue({ microMoveChance: 0.1, fidgetChance: 0.05 }),
        scroll: Object.assign(vi.fn().mockResolvedValue(undefined), {
            toTop: vi.fn().mockResolvedValue(undefined),
            back: vi.fn().mockResolvedValue(undefined),
            read: vi.fn().mockResolvedValue(undefined),
            focus: vi.fn().mockResolvedValue(undefined),
        }),
        visible: vi.fn().mockResolvedValue(true),
        exists: vi.fn().mockResolvedValue(true),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/home'),
        goto: vi.fn().mockResolvedValue(undefined),
        reload: vi.fn().mockResolvedValue(undefined),
        eval: vi.fn().mockResolvedValue('mock result'),
        text: vi.fn().mockResolvedValue('mock text'),
        click: vi.fn().mockResolvedValue(undefined),
        type: vi.fn().mockResolvedValue(undefined),
    };
    return { api, default: api };
});
import { api } from '@api/index.js';

import ContentSkimmer from '@api/behaviors/humanization/content.js';
import { mathUtils } from '@api/utils/math.js';
import { scrollRandom } from '@api/behaviors/scroll-helper.js';

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => min),
        roll: vi.fn(() => true),
    },
}));

vi.mock('@api/utils/scroll-helper.js', () => ({
    scrollRandom: vi.fn().mockResolvedValue(undefined),
}));

describe('ContentSkimmer', () => {
    let contentSkimmer;
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();
        mockPage = {
            mouse: {
                move: vi.fn().mockResolvedValue(undefined),
            },
            isClosed: vi.fn().mockReturnValue(false),
            context: vi.fn().mockReturnValue({
                browser: vi.fn().mockReturnValue({ isConnected: vi.fn().mockReturnValue(true) }),
            }),
        };
        api.getPage.mockReturnValue(mockPage);
        api.setPage.mockReturnValue(undefined);
        contentSkimmer = new ContentSkimmer(mockPage);
    });

    describe('reading', () => {
        it('should wait for normal duration', async () => {
            await contentSkimmer.reading('normal');
            expect(api.wait).toHaveBeenCalled();
        });
    });

    describe('_skimProfile', () => {
        it('should pause for pinned tweet when roll is true', async () => {
            mathUtils.roll.mockReturnValue(true);
            await contentSkimmer._skimProfile({ read: 100, scroll: 50, pause: 50 });
            expect(api.wait).toHaveBeenCalledWith(50);
        });
    });

    describe('deepRead', () => {
        it('should perform long read session', async () => {
            await contentSkimmer.deepRead();
            expect(api.wait).toHaveBeenCalled();
        });
    });
});
