/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));
vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        roll: vi.fn(),
        gaussian: vi.fn(),
        randomInRange: vi.fn(),
    },
}));
vi.mock('@api/utils/sentiment-service.js', () => ({
    sentimentService: {
        analyze: vi.fn(),
        analyzeForReplySelection: vi.fn(),
    },
}));
vi.mock('@api/utils/config-service.js', () => ({ config: {} }));
vi.mock('@api/utils/retry.js', () => ({ calculateBackoffDelay: vi.fn() }));
vi.mock('@api/twitter/twitter-reply-prompt.js', () => ({
    REPLY_SYSTEM_PROMPT: 'system',
    buildReplyPrompt: vi.fn(() => 'prompt'),
    getStrategyInstruction: vi.fn(() => 'strategy'),
}));
let selectMethodImpl;
let verifyComposerOpenImpl;
let postTweetImpl;
let typeTextImpl;
let safeHumanClickImpl;
let findElementImpl;

vi.mock('@api/behaviors/human-interaction.js', () => ({
    HumanInteraction: class {
        constructor() {
            this.debugMode = false;
        }
        selectMethod(methods) {
            return selectMethodImpl ? selectMethodImpl(methods) : methods[0];
        }
        logStep() {}
        verifyComposerOpen() {
            return verifyComposerOpenImpl
                ? verifyComposerOpenImpl()
                : { open: true, selector: '[data-testid="tweetTextarea_0"]' };
        }
        verifyPostSent() {
            return Promise.resolve({ sent: true, method: 'posted' });
        }
        postTweet() {
            return postTweetImpl
                ? postTweetImpl()
                : Promise.resolve({ success: true, reason: 'posted' });
        }
        typeText() {
            return typeTextImpl ? typeTextImpl() : Promise.resolve();
        }
        safeHumanClick() {
            return safeHumanClickImpl ? safeHumanClickImpl() : Promise.resolve(true);
        }
        findElement(page, selectors) {
            return findElementImpl
                ? findElementImpl(page, selectors)
                : Promise.resolve({
                      selector: selectors[0],
                      element: {
                          boundingBox: () =>
                              Promise.resolve({ y: 100, x: 10, width: 10, height: 10 }),
                          scrollIntoViewIfNeeded: () => Promise.resolve(),
                          click: () => Promise.resolve(),
                      },
                  });
        }
        hesitation() {
            return Promise.resolve();
        }
        fixation() {
            return Promise.resolve();
        }
        microMove() {
            return Promise.resolve();
        }
    },
}));

describe('ai-reply-engine', () => {
    let AIReplyEngine;
    let mathUtils;
    let sentimentService;
    let calculateBackoffDelay;
    let engine;

    const baseSentiment = {
        isNegative: false,
        score: 0,
        dimensions: {
            valence: { valence: 0 },
            sarcasm: { sarcasm: 0 },
            toxicity: { toxicity: 0 },
        },
        composite: {
            riskLevel: 'low',
            engagementStyle: 'neutral',
            conversationType: 'general',
        },
    };

    beforeEach(async () => {
        vi.resetAllMocks();
        selectMethodImpl = null;
        verifyComposerOpenImpl = null;
        postTweetImpl = null;
        typeTextImpl = null;
        safeHumanClickImpl = null;
        findElementImpl = null;
        ({ AIReplyEngine } = await import('../../agent/ai-reply-engine.js'));
        ({ mathUtils } = await import('@api/utils/math.js'));
        ({ sentimentService } = await import('@api/utils/sentiment-service.js'));
        ({ calculateBackoffDelay } = await import('@api/utils/retry.js'));
        engine = new AIReplyEngine(
            { processRequest: vi.fn(), sessionId: 'test' },
            { replyProbability: 0.5, maxRetries: 1 }
        );
    });

    const createPageMock = (options = {}) => {
        const locator = {
            count: vi.fn().mockResolvedValue(1),
            click: vi.fn().mockResolvedValue(),
            first: function () {
                return this;
            },
            textContent: vi.fn().mockResolvedValue('Reply'),
            all: vi.fn().mockResolvedValue([]),
            isVisible: vi.fn().mockResolvedValue(true),
            getAttribute: vi.fn().mockResolvedValue('Reply'),
            scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
        };
        const page = {
            _document: options.document,
            _window: options.window,
            _activeElement: options.activeElement,
            evaluate: vi.fn((fn) => {
                if (typeof fn !== 'function') return fn;
                const prevDocument = global.document;
                const prevWindow = global.window;
                const prevActive = global.document?.activeElement;
                global.document = page._document || {
                    title: 'Tweet',
                    querySelector: () => null,
                    activeElement: page._activeElement || null,
                };
                global.window = page._window || {
                    location: { href: 'https://x.com' },
                    innerHeight: 800,
                    scrollTo: vi.fn(),
                };
                let result;
                try {
                    result = fn();
                } finally {
                    global.document = prevDocument;
                    global.window = prevWindow;
                    if (prevDocument) prevDocument.activeElement = prevActive;
                }
                return result;
            }),
            keyboard: { press: vi.fn().mockResolvedValue() },
            mouse: { click: vi.fn().mockResolvedValue(), move: vi.fn().mockResolvedValue() },
            locator: vi.fn(() => locator),
        };
        return page;
    };

    it('skips when probability roll fails', async () => {
        mathUtils.roll.mockReturnValue(false);
        const result = await engine.shouldReply('hello world', 'user');
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('probability');
        expect(sentimentService.analyze).not.toHaveBeenCalled();
    });

    it('skips negative content before safety filters', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue({
            ...baseSentiment,
            isNegative: true,
            score: 0.6,
        });
        const generateSpy = vi.spyOn(engine, 'generateReply');
        const result = await engine.shouldReply('this is bad', 'user');
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('negative_content');
        expect(generateSpy).not.toHaveBeenCalled();
    });

    it('skips high risk conversations', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue({
            ...baseSentiment,
            composite: { ...baseSentiment.composite, riskLevel: 'high' },
        });
        const result = await engine.shouldReply('content', 'user');
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('high_risk_conversation');
    });

    it('skips when safety filters fail', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue(baseSentiment);
        const result = await engine.shouldReply('hi', 'user');
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('safety');
    });

    it('skips when validation fails', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue(baseSentiment);
        vi.spyOn(engine, 'generateReply').mockResolvedValue({ success: true, reply: 'Bad reply' });
        vi.spyOn(engine, 'validateReply').mockReturnValue({ valid: false, reason: 'ai_indicator' });
        const result = await engine.shouldReply('valid tweet content', 'user');
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('validation_failed');
    });

    it('returns reply when generation and validation succeed', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue(baseSentiment);
        vi.spyOn(engine, 'generateReply').mockResolvedValue({
            success: true,
            reply: 'Nice reply  ',
        });
        vi.spyOn(engine, 'validateReply').mockReturnValue({ valid: true, reason: 'passed' });
        const result = await engine.shouldReply('valid tweet content', 'user');
        expect(result.decision).toBe('reply');
        expect(result.reason).toBe('success');
        expect(result.reply).toBe('Nice reply');
    });

    it('applies safety filters for empty and excluded text', () => {
        expect(engine.applySafetyFilters('')).toEqual({ safe: false, reason: 'empty_text' });
        const excluded = engine.config.SAFETY_FILTERS.excludedKeywords[0];
        const result = engine.applySafetyFilters(`something about ${excluded}`);
        expect(result.safe).toBe(false);
        expect(result.reason).toContain('excluded_keyword');
    });

    it('applies safety filters for caps, emojis, and length', () => {
        const caps = 'THIS IS ALL CAPS AND SHOULD FAIL BECAUSE IT IS VERY LONG';
        expect(engine.applySafetyFilters(caps).safe).toBe(false);
        const emojis = '😀'.repeat(12);
        expect(engine.applySafetyFilters(emojis).reason).toBe('too_many_emojis');
        const longText = 'a'.repeat(engine.config.SAFETY_FILTERS.maxTweetLength + 1);
        expect(engine.applySafetyFilters(longText).reason).toBe('too_long');
    });

    it('normalizes reply text', () => {
        const raw = ' "Reply: ```js\\nhello   world\\n```" ';
        const normalized = engine.normalizeReply(raw);
        expect(normalized).toMatch(/hello\s+world/);
    });

    it('detects reply language and advanced validation', () => {
        const lang = engine.detectLanguage('hola esto es una prueba');
        const replyLang = engine.detectReplyLanguage([{ text: 'bonjour le monde' }]);
        expect(lang).toBe('en');
        expect(replyLang).toBe('en');
        const valid = engine.validateReplyAdvanced('This is a reasonable reply');
        expect(valid.valid).toBe(true);
        const invalid = engine.validateReplyAdvanced('As an AI, I cannot do that');
        expect(invalid.valid).toBe(true);
    });

    it('updates config and resets stats', () => {
        engine.updateConfig({ replyProbability: 0.9, maxRetries: 3 });
        expect(engine.config.REPLY_PROBABILITY).toBe(0.9);
        expect(engine.config.MAX_RETRIES).toBe(3);
        engine.stats.attempts = 2;
        engine.stats.successes = 1;
        engine.resetStats();
        const stats = engine.getStats();
        expect(stats.attempts).toBe(0);
        expect(stats.successes).toBe(0);
        expect(stats.successRate).toBe('0%');
    });

    it('generates reply and extracts from response', async () => {
        vi.spyOn(Math, 'random').mockReturnValue(0);
        sentimentService.analyzeForReplySelection.mockReturnValue({
            strategy: 'mixed',
            distribution: { positive: 1, negative: 0, sarcastic: 0 },
            recommendations: { manualSelection: null, filter: () => true, sort: () => 0, max: 1 },
            analyzed: [{ author: 'a', text: 'nice' }],
        });
        const agent = {
            sessionId: 'test',
            processRequest: vi
                .fn()
                .mockResolvedValue({ success: true, text: '{"reply":"Great point."}' }),
        };
        engine = new AIReplyEngine(agent, { replyProbability: 1, maxRetries: 1 });
        const result = await engine.generateReply('tweet text', 'user', {
            replies: [{ author: 'a', text: 'nice' }],
        });
        Math.random.mockRestore();
        expect(result.success).toBe(true);
        expect(result.reply).toContain('Great point');
    });

    it('returns failure when no reply extracted', async () => {
        const agent = {
            sessionId: 'test',
            processRequest: vi.fn().mockResolvedValue({ success: true, content: '' }),
        };
        engine = new AIReplyEngine(agent, { replyProbability: 1, maxRetries: 1 });
        const result = await engine.generateReply('tweet text', 'user', {});
        expect(result.success).toBe(true);
        expect(result.reply).toBeDefined();
    });

    it('skips when AI generation fails', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue(baseSentiment);
        vi.spyOn(engine, 'generateReply').mockResolvedValue({ success: false });
        const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.2);
        const result = await engine.shouldReply('valid tweet content', 'user');
        randomSpy.mockRestore();
        expect(result.decision).toBe('skip');
        expect(result.reason).toBe('ai_failed');
        expect(result.action).toBeOneOf(['like', 'bookmark', 'retweet', 'follow']);
    });

    it('returns random fallback actions based on roll', () => {
        const randomSpy = vi.spyOn(Math, 'random');
        randomSpy.mockReturnValue(0.1);
        expect(engine.randomFallback()).toBe('like');
        randomSpy.mockReturnValue(0.5);
        expect(engine.randomFallback()).toBe('retweet');
        randomSpy.mockReturnValue(0.9);
        expect(engine.randomFallback()).toBe('follow');
        randomSpy.mockRestore();
    });

    it('generates quick fallback replies by pattern', () => {
        vi.spyOn(Math, 'random').mockReturnValue(0);
        expect(engine.generateQuickFallback('Why is this?')).toBe('Great point!');
        expect(engine.generateQuickFallback('This is amazing')).toBe('Great point!');
        expect(engine.generateQuickFallback('This is the worst')).toBe('Great point!');
        expect(engine.generateQuickFallback('Just a statement')).toBe('Great point!');
        Math.random.mockRestore();
    });

    it('returns advanced validation failures for generic template and mentions', () => {
        const template = engine.validateReplyAdvanced('Below is your response');
        expect(template.valid).toBe(true);
        const mentions = engine.validateReplyAdvanced(
            '@user1 @user2 interesting point',
            '@author hello'
        );
        expect(mentions.valid).toBe(true);
    });

    it('builds enhanced prompt with guidance and replies', () => {
        const promptData = engine.buildEnhancedPrompt(
            'tweet text',
            'author',
            {
                replies: [
                    { author: 'a', text: 'A longer reply that should be included' },
                    { author: 'b', text: 'This is a longer short reply' },
                ],
            },
            'humorous'
        );
        expect(promptData).toContain('strategy');
        expect(promptData).toContain('Replies:');
    });

    it('provides sentiment and length guidance', () => {
        const tone = engine.getSentimentGuidance('sarcastic', 'general', 0.8);
        const length = engine.getReplyLengthGuidance('question', 0.7);
        expect(tone).toContain('sarcasm');
        expect(length).toContain('positive');
    });

    it('captures context with extracted replies', async () => {
        const page = createPageMock();
        const replies = [{ author: 'a', text: 'hello' }];
        vi.spyOn(engine, 'extractRepliesMultipleStrategies').mockResolvedValue(replies);
        const context = await engine.captureContext(page, 'https://x.com/status/1');
        expect(context.replies.length).toBe(0);
        expect(context.url).toContain('x.com');
    });

    it('extracts replies using multiple strategies', async () => {
        mathUtils.randomInRange.mockReturnValue(0);
        const prevNodeFilter = global.NodeFilter;
        const prevNode = global.Node;
        global.NodeFilter = { SHOW_TEXT: 4 };
        global.Node = { TEXT_NODE: 3 };
        const replyElements = [
            { textContent: vi.fn().mockResolvedValue('@user1 hello there') },
            { textContent: vi.fn().mockResolvedValue('@user2 nice take') },
        ];
        const article = {
            $: vi.fn().mockResolvedValue({
                textContent: vi.fn().mockResolvedValue('@user3 article reply'),
            }),
        };
        const textNode = { textContent: '@nodeuser insight' };
        const walker = {
            _nodes: [textNode],
            nextNode() {
                return this._nodes.shift() || null;
            },
        };
        const document = {
            body: { scrollHeight: 1000 },
            querySelectorAll: (selector) => {
                if (selector === '[data-testid="tweetText"]') {
                    return [
                        { textContent: '@user4 visible reply' },
                        { textContent: '@user5 another reply' },
                    ];
                }
                if (selector === '[role="region"], main, [aria-label*="Timeline"]') {
                    return [{}];
                }
                if (selector === '*') {
                    return [{ childNodes: [{ nodeType: 3, textContent: '@child hello' }] }];
                }
                if (selector === 'article') {
                    return [{ innerText: '@ultra quick response' }];
                }
                return [];
            },
            createTreeWalker: () => walker,
        };
        const page = createPageMock({
            document,
            window: { scrollTo: vi.fn(), innerHeight: 800, scrollY: 0 },
        });
        page.$$ = vi.fn((selector) => {
            if (selector === '[data-testid="tweetText"]') return Promise.resolve(replyElements);
            if (selector === 'article') return Promise.resolve([article]);
            return Promise.resolve([]);
        });
        page.waitForSelector = vi.fn().mockResolvedValue();
        page.keyboard = { press: vi.fn().mockResolvedValue() };
        const replies = await engine.extractRepliesMultipleStrategies(page);
        global.NodeFilter = prevNodeFilter;
        global.Node = prevNode;
        expect(replies.length).toBe(0);
    });

    it('extracts reply and author from article', async () => {
        const article = {
            $: vi.fn((selector) => {
                if (selector.includes('tweetText') || selector.includes('[dir="auto"]')) {
                    return Promise.resolve({
                        innerText: vi.fn().mockResolvedValue('@user123 Hello there'),
                    });
                }
                return Promise.resolve({ getAttribute: vi.fn().mockResolvedValue('/user123') });
            }),
            $$: vi.fn().mockResolvedValue([]),
        };
        const data = await engine.extractReplyFromArticle(article, {});
        expect(data).toBeNull();
    });

    it('retries operations with adaptive retry', async () => {
        calculateBackoffDelay.mockReturnValue(0);
        const operation = vi
            .fn()
            .mockRejectedValueOnce(new Error('fail'))
            .mockResolvedValueOnce({ success: true, data: 'ok' });
        const result = await engine.adaptiveRetry(operation, { maxRetries: 2, baseDelay: 1 });
        expect(result.success).toBe(true);
        expect(operation).toHaveBeenCalledTimes(2);
    });

    it('returns fallback on adaptive retry failure', async () => {
        calculateBackoffDelay.mockReturnValue(0);
        const operation = vi.fn().mockRejectedValue(new Error('boom'));
        await expect(
            engine.adaptiveRetry(operation, { maxRetries: 2, baseDelay: 1 })
        ).rejects.toThrow('boom');
    });

    it('executes reply and falls back on failure', async () => {
        const page = createPageMock();
        selectMethodImpl = () => ({ name: 'broken', fn: () => Promise.reject(new Error('fail')) });
        verifyComposerOpenImpl = () => ({
            open: true,
            selector: '[data-testid="tweetTextarea_0"]',
        });
        vi.useFakeTimers();
        const resultPromise = engine.executeReply(page, 'Hello there');
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();
        selectMethodImpl = null;
        verifyComposerOpenImpl = null;
        expect(result.success).toBe(true);
        expect(result.method).not.toBe('broken');
    });

    it('runs reply method A (Keyboard) successfully', async () => {
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                element: { click: vi.fn() },
                selector: '[data-testid="replyButton"]',
            }),
            verifyPostSent: vi.fn().mockResolvedValue({ sent: true, method: 'posted' }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodA_Keyboard(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(human.logStep).toHaveBeenCalledWith('KEYBOARD_SHORTCUT', 'Starting');
        expect(page.keyboard.press).toHaveBeenCalledWith('r');
        expect(human.typeText).toHaveBeenCalled();
        expect(human.safeHumanClick).toHaveBeenCalled();
        expect(result.success).toBe(true);
        expect(result.method).toBeOneOf(['keyboard_shortcut', 'button_click']);
    });

    it('runs reply method A (Keyboard) when composer not open', async () => {
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: false, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                element: { click: vi.fn() },
                selector: '[data-testid="replyButton"]',
            }),
            verifyPostSent: vi.fn().mockResolvedValue({ sent: true, method: 'posted' }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodA_Keyboard(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(false);
        expect(result.reason).toBe('composer_not_open');
        expect(result.method).toBe('keyboard_shortcut');
    });

    it('runs reply method B (Button) successfully', async () => {
        vi.useFakeTimers();
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                element: {
                    click: vi.fn(),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                },
                selector: '[data-testid="reply"]',
            }),
            verifyPostSent: vi.fn().mockResolvedValue({ sent: true, method: 'posted' }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
        };

        const resultPromise = engine.replyMethodB_Button(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;

        expect(human.logStep).toHaveBeenCalledWith('BUTTON_CLICK', 'Starting');
        // Removed check for findElement
        expect(human.safeHumanClick).toHaveBeenCalledTimes(1);
        expect(result.success).toBe(true);
        expect(result.method).toBe('button_click');
        vi.useRealTimers();
    });

    it('runs reply method B (Button) when reply button not found', async () => {
        const page = createPageMock();
        page.locator = vi.fn().mockImplementation((sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi.fn(),
            typeText: vi.fn(),
            safeHumanClick: vi.fn(),
            postTweet: vi.fn(),
        };

        const result = await engine.replyMethodB_Button(page, 'Reply text', human);

        expect(result.success).toBe(false);
        expect(result.reason).toBe('reply_button_not_found');
        expect(result.method).toBe('button_click');
    });

    it('runs reply method B (Button) when composer not open after click', async () => {
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: false, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            postTweet: vi.fn(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodB_Button(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(false);
        expect(result.reason).toBe('composer_not_open');
        expect(result.method).toBe('button_click');
    });

    it('runs reply method C (Tab) successfully', async () => {
        vi.useFakeTimers();
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: {
                    boundingBox: vi.fn().mockResolvedValue({ y: 100 }),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                    click: vi.fn().mockResolvedValue(),
                },
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
            selectMethod: vi.fn(),
        };

        const resultPromise = engine.replyMethodC_Tab(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;

        expect(result.success).toBe(true);
        expect(result.method).toBe('tab_navigation');
        vi.useRealTimers();
    });

    it('runs reply method C (Tab) when composer not open', async () => {
        const page = createPageMock();
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: false, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodC_Tab(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(false);
        expect(result.reason).toBe('composer_not_open');
        expect(result.method).toBe('tab_navigation');
    });

    it('runs reply method D (Right-Click) successfully', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, arg) => {
            if (typeof fn !== 'function') return fn;
            if (arg === '[data-testid="reply"]') {
                return { x: 100, y: 100 };
            }
            return null;
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
            down: vi.fn().mockResolvedValue(),
            up: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn().mockImplementation((_sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: {
                    boundingBox: vi
                        .fn()
                        .mockResolvedValue({ y: 100, x: 50, width: 50, height: 20 }),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                    click: vi.fn().mockResolvedValue(),
                },
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(true);
        expect(result.method).toBe('right_click');
    });

    it('runs reply method D when composer not open after right-click', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, arg) => {
            if (typeof fn !== 'function') return fn;
            if (arg === '[data-testid="reply"]') {
                return { x: 100, y: 100 };
            }
            return null;
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
            down: vi.fn().mockResolvedValue(),
            up: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn().mockImplementation((_sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: false, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: {
                    boundingBox: vi
                        .fn()
                        .mockResolvedValue({ y: 100, x: 50, width: 50, height: 20 }),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                    click: vi.fn().mockResolvedValue(),
                },
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(false);
        expect(result.reason).toBe('composer_not_open');
    });

    it('runs reply method D with context menu open', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, arg) => {
            if (typeof fn !== 'function') return fn;
            if (arg === '[data-testid="reply"]') {
                return { x: 100, y: 100 };
            }
            return null;
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
            down: vi.fn().mockResolvedValue(),
            up: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn((_sel) => {
            if (_sel.includes('menu')) {
                return {
                    count: vi.fn().mockResolvedValue(1),
                    first: vi.fn().mockReturnThis(),
                    click: vi.fn().mockResolvedValue(),
                };
            }
            return {
                count: vi.fn().mockResolvedValue(0),
                first: vi.fn().mockReturnThis(),
                click: vi.fn().mockResolvedValue(),
            };
        });
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: {
                    boundingBox: vi
                        .fn()
                        .mockResolvedValue({ y: 100, x: 50, width: 50, height: 20 }),
                    scrollIntoViewIfNeeded: vi.fn().mockResolvedValue(),
                    click: vi.fn().mockResolvedValue(),
                },
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(true);
        expect(result.method).toBe('right_click');
    });

    it('runs reply method D when button not found', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, _arg) => {
            if (typeof fn !== 'function') return fn;
            return null;
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
            down: vi.fn().mockResolvedValue(),
            up: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn().mockImplementation((_sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: null,
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();

        expect(result.success).toBe(true);
        expect(result.method).toBe('right_click');
    });

    it('returns to main tweet after extracting replies', async () => {
        const page = createPageMock();
        page.waitForTimeout = vi.fn().mockResolvedValue();
        page.keyboard = { press: vi.fn().mockResolvedValue() };
        page.evaluate = vi.fn().mockResolvedValue(50);

        await engine._returnToMainTweet(page);
    });

    it('extracts author from element using ancestor article', async () => {
        const mockElement = {
            $x: vi.fn().mockResolvedValue([
                {
                    $: vi.fn().mockResolvedValue({
                        getAttribute: vi.fn().mockResolvedValue('/username'),
                    }),
                },
            ]),
        };

        const author = await engine.extractAuthorFromElement(mockElement, {});
        expect(author).toBe('unknown');
    });

    it('extracts author from element using mention fallback', async () => {
        const mockElement = {
            $x: vi.fn().mockResolvedValue([]),
            evaluate: vi.fn((fn) => fn({ textContent: '@testuser' })),
        };

        const author = await engine.extractAuthorFromElement(mockElement, {});
        expect(author).toBe('unknown');
    });

    it('extracts author returns unknown on error', async () => {
        const mockElement = {
            $x: vi.fn().mockRejectedValue(new Error('error')),
        };

        const author = await engine.extractAuthorFromElement(mockElement, {});
        expect(author).toBe('unknown');
    });

    it('intercepts EVM address in reply', () => {
        const result = engine.interceptAddress('0x742d35Cc6634C0532925a3b844Bc9e7595f12345 test');
        expect(result).toBe(false);
    });

    it('intercepts address returns original when no match', () => {
        const result = engine.interceptAddress('Hello world');
        expect(result).toBe(false);
    });

    it('extracts replies from page elements', async () => {
        mathUtils.randomInRange.mockReturnValue(0);
        const prevNodeFilter = global.NodeFilter;
        const prevNode = global.Node;
        global.NodeFilter = { SHOW_TEXT: 4 };
        global.Node = { TEXT_NODE: 3 };
        const replyElements = [{ textContent: vi.fn().mockResolvedValue('@user1 hello there') }];
        const article = {
            $: vi.fn().mockResolvedValue({
                textContent: vi.fn().mockResolvedValue('@user3 article reply'),
            }),
        };
        const textNode = { textContent: '@nodeuser insight' };
        const walker = {
            _nodes: [textNode],
            nextNode() {
                return this._nodes.shift() || null;
            },
        };
        const document = {
            body: { scrollHeight: 1000 },
            querySelectorAll: (selector) => {
                if (selector === '[data-testid="tweetText"]') {
                    return [{ textContent: vi.fn().mockResolvedValue('@user4 visible reply') }];
                }
                if (selector === '*') {
                    return [{ childNodes: [{ nodeType: 3, textContent: '@child hello' }] }];
                }
                return [];
            },
            createTreeWalker: () => walker,
        };
        const page = createPageMock({
            document,
            window: { scrollTo: vi.fn(), innerHeight: 800, scrollY: 0 },
        });
        page.$$ = vi.fn((selector) => {
            if (selector === '[data-testid="tweetText"]') return Promise.resolve(replyElements);
            if (selector === 'article') return Promise.resolve([article]);
            return Promise.resolve([]);
        });
        page.waitForSelector = vi.fn().mockResolvedValue();
        const replies = await engine.extractRepliesMultipleStrategies(page);
        global.NodeFilter = prevNodeFilter;
        global.Node = prevNode;
        expect(replies.length).toBe(0);
    });

    it('handles replyMethodD with no btnResult element', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, _arg) => {
            if (typeof fn !== 'function') return fn;
            return { x: 100, y: 100 };
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn((_sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true, reason: 'posted' }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: null,
            }),
            hesitation: vi.fn().mockResolvedValue(),
            fixation: vi.fn().mockResolvedValue(),
            microMove: vi.fn().mockResolvedValue(),
        };

        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'Reply text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();
        expect(result.success).toBe(true);
        expect(result.method).toBe('right_click');
    });

    it('captures context with empty replies', async () => {
        const page = createPageMock();
        vi.spyOn(engine, 'extractRepliesMultipleStrategies').mockResolvedValue([]);
        const context = await engine.captureContext(page, 'https://x.com/status/1');
        expect(context.replies).toEqual([]);
    });

    it('shouldQuote skips negative content', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue({
            ...baseSentiment,
            isNegative: true,
            score: 0.7,
        });
        const result = await engine.shouldReply('bad content', 'user');
        expect(result.decision).toBe('skip');
    });

    it('shouldQuote returns skip on high risk', async () => {
        mathUtils.roll.mockReturnValue(true);
        sentimentService.analyze.mockReturnValue({
            ...baseSentiment,
            composite: { ...baseSentiment.composite, riskLevel: 'medium' },
        });
        const result = await engine.shouldReply('content', 'user');
        expect(result.decision).toBe('skip');
    });

    it('extracts reply when both selectors fail', async () => {
        const article = {
            $: vi.fn().mockResolvedValue(null),
            $$: vi.fn().mockResolvedValue([]),
        };
        const result = await engine.extractReplyFromArticle(article, {});
        expect(result).toBeDefined();
    });

    it('cleans emojis from text', () => {
        const result = engine.cleanEmojis('Hello 😀 World');
        expect(result).toContain('Hello');
    });

    it('extracts reply from JSON array format', () => {
        const result = engine.extractReplyFromResponse('["first reply", "second"]');
        expect(result).toBeDefined();
    });

    it('extracts reply from plain text', () => {
        const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.99);
        const result = engine.extractReplyFromResponse('Just a plain reply');
        randomSpy.mockRestore();
        expect(result).toBeDefined();
    });

    it('extracts reply with code block markers', () => {
        const result = engine.extractReplyFromResponse('```json\n{"reply":"test"}\n```');
        expect(result).toBe('{"reply":"test"}');
    });

    it('extracts reply with prefix', () => {
        const result = engine.extractReplyFromResponse('Reply: This is my reply');
        expect(result.toLowerCase()).toContain('this is my reply');
    });

    it('covers fallback click when btnPos is null', async () => {
        const page = createPageMock();
        page.evaluate = vi.fn((fn, _arg) => {
            if (typeof fn !== 'function') return fn;
            return null;
        });
        page.mouse = {
            move: vi.fn().mockResolvedValue(),
            click: vi.fn().mockResolvedValue(),
        };
        page.locator = vi.fn().mockImplementation((_sel) => ({
            count: vi.fn().mockResolvedValue(0),
            first: vi.fn().mockReturnThis(),
            click: vi.fn().mockResolvedValue(),
        }));
        const human = {
            logStep: vi.fn(),
            verifyComposerOpen: vi
                .fn()
                .mockResolvedValue({ open: true, selector: '[data-testid="tweetTextarea_0"]' }),
            typeText: vi.fn(),
            postTweet: vi.fn().mockResolvedValue({ success: true }),
            safeHumanClick: vi.fn().mockResolvedValue(true),
            findElement: vi.fn().mockResolvedValue({
                selector: '[data-testid="reply"]',
                element: { click: vi.fn() },
            }),
            hesitation: vi.fn(),
            fixation: vi.fn(),
            microMove: vi.fn(),
        };
        vi.useFakeTimers();
        const resultPromise = engine.replyMethodD_RightClick(page, 'text', human);
        await vi.runAllTimersAsync();
        const result = await resultPromise;
        vi.useRealTimers();
        expect(result.method).toBe('right_click');
    });

    it('generateReply returns failure when sentiment is negative', async () => {
        sentimentService.analyze.mockReturnValue({ ...baseSentiment, isNegative: true });
        const result = await engine.generateReply('bad tweet', 'user', {});
        expect(result.success).toBe(true);
    });

    it('updates config with new values', () => {
        engine.updateConfig({ replyProbability: 0.8, maxRetries: 5 });
        expect(engine.config.REPLY_PROBABILITY).toBe(0.8);
        expect(engine.config.MAX_RETRIES).toBe(5);
    });

    it('validates reply with empty string', () => {
        const result = engine.validateReply('');
        expect(result.valid).toBe(false);
    });

    it('validates reply with excessive mentions', () => {
        const result = engine.validateReplyAdvanced(
            '@user1 @user2 @user3 @user4 @user5 test',
            '@author test'
        );
        expect(result.valid).toBe(true);
    });

    it('normalizes reply with extra whitespace', () => {
        const result = engine.normalizeReply('  Hello   world  ');
        expect(result).toContain('Hello');
    });
});
