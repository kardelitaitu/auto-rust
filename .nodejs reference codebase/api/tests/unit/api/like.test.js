/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { likeWithAPI } from '@api/actions/like.js';

// Mock dependencies
vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    })),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => (min + max) / 2),
    },
}));

vi.mock('@api/interactions/scroll.js', () => ({
    scroll: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/interactions/wait.js', () => ({
    wait: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@api/interactions/queries.js', () => ({
    visible: vi.fn(),
}));

vi.mock('@api/interactions/actions.js', () => ({
    click: vi.fn().mockResolvedValue(undefined),
}));

import { getPage } from '@api/core/context.js';
import { visible } from '@api/interactions/queries.js';
import { click } from '@api/interactions/actions.js';

describe('api/actions/like.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockPage = {
            url: vi.fn().mockReturnValue('https://x.com/home'),
        };

        getPage.mockReturnValue(mockPage);
    });

    describe('likeWithAPI', () => {
        it('should like tweet successfully', async () => {
            visible.mockResolvedValueOnce(false); // not already liked
            visible.mockResolvedValueOnce(true); // toast visible

            const result = await likeWithAPI();

            expect(click).toHaveBeenCalled();
            expect(result.success).toBe(true);
            expect(result.method).toBe('likeAPI');
        });

        it('should skip if already liked', async () => {
            visible.mockResolvedValueOnce(true); // already liked

            const result = await likeWithAPI();

            expect(click).not.toHaveBeenCalled();
            expect(result.success).toBe(true);
            expect(result.reason).toBe('already_liked');
        });

        it('should return failure if verification fails', async () => {
            visible.mockResolvedValueOnce(false);
            visible.mockResolvedValueOnce(false);
            visible.mockResolvedValueOnce(false);

            const result = await likeWithAPI();

            expect(result.success).toBe(false);
            expect(result.reason).toBe('verification_failed');
        });

        it('should handle click error', async () => {
            visible.mockResolvedValueOnce(false);
            click.mockRejectedValueOnce(new Error('Click failed'));

            const result = await likeWithAPI();

            expect(result.success).toBe(false);
            expect(result.reason).toContain('Click failed');
        });
    });
});
