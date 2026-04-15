/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { text, attr, visible, count, exists, currentUrl } from '@api/interactions/queries.js';

// Mock dependencies
vi.mock('@api/core/context.js', () => ({
    getPage: vi.fn(),
    isSessionActive: vi.fn().mockReturnValue(true),
}));

vi.mock('@api/utils/locator.js', () => ({
    getLocator: vi.fn(),
}));

import { getPage, isSessionActive } from '@api/core/context.js';
import { getLocator } from '@api/utils/locator.js';

describe('api/interactions/queries.js', () => {
    let mockLocator;
    let mockPage;

    beforeEach(() => {
        vi.clearAllMocks();

        mockLocator = {
            first: vi.fn().mockReturnThis(),
            innerText: vi.fn(),
            getAttribute: vi.fn(),
            isVisible: vi.fn(),
            count: vi.fn(),
        };

        mockPage = {
            url: vi.fn().mockReturnValue('https://x.com/home'),
        };

        getLocator.mockReturnValue(mockLocator);
        getPage.mockReturnValue(mockPage);
        isSessionActive.mockReturnValue(true);
    });

    describe('text', () => {
        it('should extract innerText from element', async () => {
            mockLocator.innerText.mockResolvedValue('Tweet text');

            const result = await text('[data-testid="tweetText"]');

            expect(getLocator).toHaveBeenCalledWith('[data-testid="tweetText"]');
            expect(result).toBe('Tweet text');
        });

        it('should return empty string if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await text('[data-testid="tweetText"]');

            expect(result).toBe('');
            expect(mockLocator.innerText).not.toHaveBeenCalled();
        });

        it('should return empty string on error', async () => {
            mockLocator.innerText.mockRejectedValue(new Error('Not found'));

            const result = await text('[data-testid="tweetText"]');

            expect(result).toBe('');
        });

        it('should work with locator input', async () => {
            const mockLocatorInput = {
                waitFor: vi.fn(),
                first: vi.fn().mockReturnThis(),
                innerText: vi.fn().mockResolvedValue('Locator text'),
            };
            getLocator.mockReturnValue(mockLocatorInput);

            const result = await text(mockLocatorInput);

            expect(result).toBe('Locator text');
        });
    });

    describe('attr', () => {
        it('should extract attribute value', async () => {
            mockLocator.getAttribute.mockResolvedValue('href-value');

            const result = await attr('a', 'href');

            expect(getLocator).toHaveBeenCalledWith('a');
            expect(result).toBe('href-value');
        });

        it('should return null if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await attr('a', 'href');

            expect(result).toBe(null);
        });

        it('should return null on error', async () => {
            mockLocator.getAttribute.mockRejectedValue(new Error('Not found'));

            const result = await attr('a', 'href');

            expect(result).toBe(null);
        });
    });

    describe('visible', () => {
        it('should return true if element is visible', async () => {
            mockLocator.isVisible.mockResolvedValue(true);

            const result = await visible('[data-testid="tweet"]');

            expect(result).toBe(true);
        });

        it('should return false if element is not visible', async () => {
            mockLocator.isVisible.mockResolvedValue(false);

            const result = await visible('[data-testid="tweet"]');

            expect(result).toBe(false);
        });

        it('should return false if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await visible('[data-testid="tweet"]');

            expect(result).toBe(false);
        });

        it('should return false on error', async () => {
            mockLocator.isVisible.mockRejectedValue(new Error('Not found'));

            const result = await visible('[data-testid="tweet"]');

            expect(result).toBe(false);
        });
    });

    describe('count', () => {
        it('should return element count', async () => {
            mockLocator.count.mockResolvedValue(5);

            const result = await count('article');

            expect(result).toBe(5);
        });

        it('should return 0 if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await count('article');

            expect(result).toBe(0);
        });

        it('should return 0 on error', async () => {
            mockLocator.count.mockRejectedValue(new Error('Query failed'));

            const result = await count('article');

            expect(result).toBe(0);
        });
    });

    describe('exists', () => {
        it('should return true if count > 0', async () => {
            mockLocator.count.mockResolvedValue(1);

            const result = await exists('[data-testid="tweet"]');

            expect(result).toBe(true);
        });

        it('should return false if count = 0', async () => {
            mockLocator.count.mockResolvedValue(0);

            const result = await exists('[data-testid="tweet"]');

            expect(result).toBe(false);
        });

        it('should return false if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await exists('[data-testid="tweet"]');

            expect(result).toBe(false);
        });
    });

    describe('currentUrl', () => {
        it('should return current page URL', async () => {
            const result = await currentUrl();

            expect(mockPage.url).toHaveBeenCalled();
            expect(result).toBe('https://x.com/home');
        });

        it('should return empty string if session inactive', async () => {
            isSessionActive.mockReturnValue(false);

            const result = await currentUrl();

            expect(result).toBe('');
        });

        it('should return empty string on error', async () => {
            mockPage.url.mockImplementation(() => {
                throw new Error('URL error');
            });

            const result = await currentUrl();

            expect(result).toBe('');
        });
    });
});
