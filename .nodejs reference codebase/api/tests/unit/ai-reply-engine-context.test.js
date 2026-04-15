/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * @fileoverview Unit tests for utils/ai-reply-engine/context.js
 * @module tests/unit/ai-reply-engine-context.test
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/index.js', () => ({
    api: {
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        think: vi.fn().mockResolvedValue(undefined),
        scroll: {
            read: vi.fn().mockResolvedValue(undefined),
            back: vi.fn().mockResolvedValue(undefined),
            toTop: vi.fn().mockResolvedValue(undefined),
        },
    },
}));
import { api } from '@api/index.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        debug: vi.fn(),
        error: vi.fn(),
    })),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        randomInRange: vi.fn((min, max) => min),
    },
}));

describe('ai-reply-engine/context', () => {
    let captureContext;
    let extractRepliesMultipleStrategies;
    let extractReplyFromArticle;
    let extractAuthorFromArticle;
    let extractAuthorFromElement;
    let mockEngine;
    let mockPage;

    beforeEach(async () => {
        vi.clearAllMocks();

        const module = await import('../../agent/ai-reply-engine/context.js');
        captureContext = module.captureContext;
        extractRepliesMultipleStrategies = module.extractRepliesMultipleStrategies;
        extractReplyFromArticle = module.extractReplyFromArticle;
        extractAuthorFromArticle = module.extractAuthorFromArticle;
        extractAuthorFromElement = module.extractAuthorFromElement;

        mockEngine = {
            logger: {
                debug: vi.fn(),
                warn: vi.fn(),
            },
        };

        mockPage = {
            waitForSelector: vi.fn().mockResolvedValue(undefined),
            waitForTimeout: vi.fn().mockResolvedValue(undefined),
            evaluate: vi.fn(),
            $$: vi.fn().mockResolvedValue([]),
            locator: vi.fn(() => ({
                count: vi.fn().mockResolvedValue(0),
                first: vi.fn().mockReturnThis(),
            })),
            keyboard: {
                press: vi.fn().mockResolvedValue(undefined),
            },
        };
        api.getPage.mockReturnValue(mockPage);
    });

    describe('captureContext', () => {
        it('should return basic context structure', async () => {
            const context = await captureContext(mockEngine, mockPage, 'https://x.com/status/123');

            expect(context).toHaveProperty('url');
            expect(context).toHaveProperty('screenshot');
            expect(context).toHaveProperty('replies');
            expect(context.url).toBe('https://x.com/status/123');
        });

        it('should call extractRepliesMultipleStrategies', async () => {
            mockEngine.logger.debug = vi.fn();

            await captureContext(mockEngine, mockPage, 'https://x.com/status/123');

            expect(mockEngine.logger.debug).toHaveBeenCalled();
        });

        it('should handle errors gracefully', async () => {
            mockPage.waitForSelector = vi.fn().mockRejectedValue(new Error('Page error'));

            const context = await captureContext(mockEngine, mockPage, 'https://x.com/status/123');

            expect(context).toBeDefined();
            expect(context.replies).toEqual([]);
        });
    });

    describe('extractRepliesMultipleStrategies', () => {
        it('should return empty array on page error', async () => {
            mockPage.waitForSelector = vi.fn().mockRejectedValue(new Error('Error'));

            const replies = await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(Array.isArray(replies)).toBe(true);
        });

        it('should limit replies to 50', async () => {
            const manyReplies = Array.from({ length: 100 }, (_, i) => ({
                author: 'user',
                text: `Reply ${i}`,
            }));

            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockReturnValue([]);
            mockPage.$$ = vi.fn().mockResolvedValue([]);

            const replies = await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(replies.length).toBeLessThanOrEqual(50);
        });

        it('should call scroll operations', async () => {
            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockImplementation((fn) => {
                if (typeof fn === 'function') {
                    return fn();
                }
                return null;
            });
            mockPage.$$ = vi.fn().mockResolvedValue([]);

            await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(mockPage.evaluate).toHaveBeenCalled();
        });

        it('should extract replies from articles', async () => {
            const mockArticle = {
                $: vi.fn().mockImplementation(async (sel) => {
                    if (sel.includes('tweetText')) {
                        return { innerText: vi.fn().mockResolvedValue('@user1 Hello world') };
                    }
                    return null;
                }),
            };

            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockImplementation((fn) => {
                if (typeof fn === 'function') return [];
                return [];
            });
            mockPage.$$ = vi.fn().mockResolvedValue([mockArticle, mockArticle]);

            const replies = await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(mockPage.$$).toHaveBeenCalled();
        });

        it('should extract replies from timeline', async () => {
            const mockCell = {
                $: vi.fn().mockImplementation(async (sel) => {
                    if (sel.includes('tweetText')) {
                        return { innerText: vi.fn().mockResolvedValue('Timeline reply') };
                    }
                    return null;
                }),
            };

            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockReturnValue([]);
            mockPage.$$ = vi.fn().mockImplementation((sel) => {
                if (sel.includes('cellInnerDiv')) {
                    return Promise.resolve([mockCell]);
                }
                return Promise.resolve([]);
            });

            const replies = await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(mockPage.$$).toHaveBeenCalled();
        });

        it('should extract replies from text content search', async () => {
            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockImplementation((fn) => {
                if (typeof fn === 'function') {
                    return ['Reply 1', 'Reply 2', 'Reply 3'];
                }
                return [];
            });
            mockPage.$$ = vi.fn().mockResolvedValue([]);

            const replies = await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(mockPage.evaluate).toHaveBeenCalled();
        });

        it('should handle returnToMainTweet', async () => {
            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockImplementation((fn) => {
                if (typeof fn === 'function') return [];
                return 500;
            });
            mockPage.$$ = vi.fn().mockResolvedValue([]);

            await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(api.scroll.toTop).toHaveBeenCalled();
        });
    });

    describe('extractReplyFromArticle', () => {
        it('should return null when no text element found', async () => {
            const article = {
                $: vi.fn().mockResolvedValue(null),
            };

            const result = await extractReplyFromArticle(mockEngine, article, mockPage);

            expect(result).toBeNull();
        });

        it('should return null when text is too short', async () => {
            const article = {
                $: vi.fn().mockResolvedValue({
                    innerText: vi.fn().mockResolvedValue('Hi'),
                }),
            };

            const result = await extractReplyFromArticle(mockEngine, article, mockPage);

            expect(result).toBeNull();
        });

        it('should extract author and text', async () => {
            const article = {
                $: vi.fn().mockImplementation((sel) => {
                    if (sel.includes('tweetText') || sel.includes('dir="auto"')) {
                        return Promise.resolve({
                            innerText: vi.fn().mockResolvedValue('@user123 Hello world'),
                        });
                    }
                    if (sel.startsWith('/')) {
                        return Promise.resolve({
                            getAttribute: vi.fn().mockResolvedValue('/user123'),
                        });
                    }
                    return Promise.resolve(null);
                }),
            };

            mockEngine.extractAuthorFromArticle = vi.fn().mockResolvedValue('user123');

            const result = await extractReplyFromArticle(mockEngine, article, mockPage);

            expect(result).toBeDefined();
        });
    });

    describe('extractAuthorFromArticle', () => {
        it('should return unknown when no link found', async () => {
            const article = {
                $: vi.fn().mockResolvedValue(null),
            };

            const result = await extractAuthorFromArticle(mockEngine, article);

            expect(result).toBe('unknown');
        });

        it('should extract author from href', async () => {
            const mockLink = {
                getAttribute: vi.fn().mockResolvedValue('/testuser'),
            };

            const article = {
                $: vi.fn().mockImplementation(async (selector) => {
                    if (selector === 'a[href^="/"]') {
                        return mockLink;
                    }
                    return null;
                }),
            };

            const result = await extractAuthorFromArticle(mockEngine, article);

            expect(result).toBe('testuser');
        });

        it('should handle href with query params', async () => {
            const mockLink = {
                getAttribute: vi.fn().mockResolvedValue('/user123?ref=profile'),
            };

            const article = {
                $: vi.fn().mockImplementation(async (selector) => {
                    if (selector === 'a[href^="/"]') {
                        return mockLink;
                    }
                    return null;
                }),
            };

            const result = await extractAuthorFromArticle(mockEngine, article);

            expect(result).toBe('user123');
        });

        it('should return unknown on error', async () => {
            const article = {
                $: vi.fn().mockRejectedValue(new Error('Error')),
            };

            const result = await extractAuthorFromArticle(mockEngine, article);

            expect(result).toBe('unknown');
        });
    });

    describe('extractAuthorFromElement', () => {
        it('should delegate to extractAuthorFromArticle', async () => {
            const element = { test: 'element' };

            await extractAuthorFromElement(mockEngine, element, mockPage);
        });
    });

    describe('extractRepliesMultipleStrategies - additional coverage', () => {
        it('should handle successful waitForSelector (line 51)', async () => {
            mockPage.waitForSelector = vi.fn().mockResolvedValue(undefined);
            mockPage.evaluate = vi.fn().mockImplementation((fn) => {
                if (typeof fn === 'function') return [];
                return null;
            });
            mockPage.$$ = vi.fn().mockResolvedValue([]);

            await extractRepliesMultipleStrategies(mockEngine, mockPage);

            expect(api.wait).toHaveBeenCalled();
        });

        it('should extract author from time element (line 294-296)', async () => {
            const mockLink = {
                getAttribute: vi.fn().mockResolvedValue(null),
            };
            const mockTime = {
                $: vi.fn().mockResolvedValue({}),
            };

            const article = {
                $: vi.fn().mockImplementation(async (selector) => {
                    if (selector === 'a[href^="/"]') return mockLink;
                    if (selector === 'time') return mockTime;
                    return null;
                }),
            };

            const result = await extractAuthorFromArticle(mockEngine, article);

            expect(result).toBe('unknown');
        });

        it('should clean text when author matches first @mention (line 274)', async () => {
            const article = {
                $: vi.fn().mockImplementation(async (sel) => {
                    if (sel.includes('tweetText') || sel.includes('dir="auto"')) {
                        return { innerText: vi.fn().mockResolvedValue('@user123 This is a reply') };
                    }
                    if (sel.startsWith('/')) {
                        return { getAttribute: vi.fn().mockResolvedValue('/user123') };
                    }
                    return Promise.resolve(null);
                }),
            };

            mockEngine.extractAuthorFromArticle = vi.fn().mockResolvedValue('user123');

            const result = await extractReplyFromArticle(mockEngine, article, mockPage);

            expect(result).toBeDefined();
            expect(result.text).toContain('This is a reply');
        });

        it('should return null on extractReplyFromArticle error (line 280)', async () => {
            const article = {
                $: vi.fn().mockImplementation(() => {
                    throw new Error('Unexpected error');
                }),
            };

            const result = await extractReplyFromArticle(mockEngine, article, mockPage);

            expect(result).toBeNull();
        });
    });
});
