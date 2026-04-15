/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { recover, goBack, urlChanged } from '@api/behaviors/recover.js';
import * as context from '@api/core/context.js';
import * as contextState from '@api/core/context-state.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: () => ({
        warn: vi.fn(),
        debug: vi.fn(),
    }),
}));

// Mock timing functions to return immediately for faster tests
vi.mock('@api/behaviors/timing.js', () => ({
    think: vi.fn().mockResolvedValue(undefined),
    delay: vi.fn().mockResolvedValue(undefined),
    randomInRange: vi.fn((min) => min),
    gaussian: vi.fn(() => 0),
}));

describe('api/behaviors/recover.js', () => {
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();
        mockPage = {
            url: vi.fn().mockReturnValue('https://example.com/now'),
            goBack: vi.fn().mockResolvedValue(true),
        };
        vi.spyOn(context, 'getPage').mockReturnValue(mockPage);
    });

    describe('urlChanged', () => {
        it('should return true if URL differs', async () => {
            const result = await urlChanged('https://example.com/before');
            expect(result).toBe(true);
        });

        it('should return false if URL is same', async () => {
            const result = await urlChanged('https://example.com/now');
            expect(result).toBe(false);
        });
    });

    describe('goBack', () => {
        it('should call page.goBack', async () => {
            await goBack();
            expect(mockPage.goBack).toHaveBeenCalled();
        });
    });

    describe('recover', () => {
        it('should stay if no previous URL', async () => {
            vi.spyOn(contextState, 'getPreviousUrl').mockReturnValue(null);
            const result = await recover();
            expect(result).toBe(false);
            expect(mockPage.goBack).not.toHaveBeenCalled();
        });

        it('should stay if URL is same as previous', async () => {
            vi.spyOn(contextState, 'getPreviousUrl').mockReturnValue('https://example.com/now');
            const result = await recover();
            expect(result).toBe(false);
            expect(mockPage.goBack).not.toHaveBeenCalled();
        });

        it('should go back if URL differs from previous', async () => {
            vi.spyOn(contextState, 'getPreviousUrl').mockReturnValue(
                'https://example.com/previous'
            );
            const result = await recover();
            expect(result).toBe(true);
            expect(mockPage.goBack).toHaveBeenCalled();
        });
    });
});
