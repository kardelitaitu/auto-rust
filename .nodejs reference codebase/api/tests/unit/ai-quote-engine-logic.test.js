/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { api } from '@api/index.js';
import { createHumanMock, baseSentiment, sampleReplies } from './ai-quote-engine.test-utils.js';

vi.mock('@api/index.js', () => ({
    api: {
        setPage: vi.fn(),
        getPage: vi.fn(),
        wait: vi.fn().mockResolvedValue(undefined),
        click: vi.fn().mockResolvedValue(true),
        type: vi.fn().mockResolvedValue(undefined),
        scroll: { toTop: vi.fn().mockResolvedValue(undefined) },
        visible: vi.fn().mockResolvedValue(true),
        exists: vi.fn().mockResolvedValue(true),
        findElement: vi.fn().mockResolvedValue('#mock-selector'),
        getCurrentUrl: vi.fn().mockResolvedValue('https://x.com/status/1'),
        eval: vi.fn().mockResolvedValue('<div><br></div>'),
        text: vi.fn().mockResolvedValue('https://x.com/status/1'),
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn((name) => ({
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
        debug: vi.fn(),
    })),
}));

vi.mock('@api/utils/math.js', () => ({
    mathUtils: {
        roll: vi.fn(),
        randomInRange: vi
            .fn()
            .mockImplementation((min, max) => Math.floor(Math.random() * (max - min + 1)) + min),
        gaussian: vi.fn().mockReturnValue(0.5),
    },
}));

vi.mock('@api/utils/sentiment-service.js', () => ({
    sentimentService: {
        analyze: vi.fn(),
        analyzeForReplySelection: vi.fn(),
    },
}));

vi.mock('@api/utils/config-service.js', () => ({ config: {} }));
vi.mock('@api/utils/scroll-helper.js', () => ({ scrollRandom: vi.fn() }));

describe('AIQuoteEngine - Core Logic', () => {
    let AIQuoteEngine;
    let mathUtils;
    let sentimentService;
    let engine;

    beforeEach(async () => {
        vi.clearAllMocks();
        ({ default: AIQuoteEngine } = await import('../../agent/ai-quote-engine.js'));
        ({ mathUtils } = await import('@api/utils/math.js'));
        ({ sentimentService } = await import('@api/utils/sentiment-service.js'));
        engine = new AIQuoteEngine(
            { processRequest: vi.fn(), sessionId: 'test' },
            { quoteProbability: 0.5, maxRetries: 1 }
        );
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe('shouldQuote', () => {
        it('skips when probability roll fails', async () => {
            mathUtils.roll.mockReturnValue(false);
            const result = await engine.shouldQuote('hello world', 'user');
            expect(result.decision).toBe('skip');
            expect(result.reason).toBe('probability');
        });

        it('proceeds when probability roll passes', async () => {
            mathUtils.roll.mockReturnValue(true);
            const result = await engine.shouldQuote('hello world', 'user');
            expect(result.decision).toBe('proceed');
            expect(result.reason).toBe('eligible');
        });
    });

    describe('Language Detection', () => {
        it('updates configuration and detects languages', () => {
            engine.updateConfig({ quoteProbability: 0.9, maxRetries: 3 });
            const lang = engine.detectLanguage('hola esto es una prueba');
            const replyLang = engine.detectReplyLanguage([{ text: 'bonjour le monde' }]);
            expect(engine.config.QUOTE_PROBABILITY).toBe(0.9);
            expect(engine.config.MAX_RETRIES).toBe(3);
            expect(lang).toBe('Spanish');
            expect(replyLang).toBe('French');
        });
    });

    describe('Sentiment & Guidance', () => {
        it('rejects negative sentiment content', async () => {
            sentimentService.analyze.mockReturnValue({
                ...baseSentiment,
                isNegative: true,
                score: 0.6,
            });
            const result = await engine.generateQuote('bad content', 'user', {});
            expect(result.success).toBe(false);
            expect(result.reason).toBe('negative_content');
        });

        it('rejects high risk conversations', async () => {
            sentimentService.analyze.mockReturnValue({
                ...baseSentiment,
                composite: { ...baseSentiment.composite, riskLevel: 'high' },
            });
            const result = await engine.generateQuote('risky content', 'user', {});
            expect(result.success).toBe(false);
            expect(result.reason).toBe('high_risk_conversation');
        });

        it('returns guidance based on tone and engagement', () => {
            expect(engine.getToneGuidance('humorous')).toContain('witty');
            expect(engine.getToneGuidance('unknown')).toContain('question');
            expect(engine.getEngagementGuidance('high')).toContain('1-2');
        });

        it('provides length and style guidance', () => {
            const length = engine.getLengthGuidance('question', 0.7);
            const style = engine.getStyleGuidance('humorous', 0.1);
            expect(length).toContain('Be more expressive');
            expect(style).toContain('Witty');
        });
    });

    describe('Quote Cleaning & Extraction', () => {
        it('extracts and cleans quotes', () => {
            const raw = 'Great insight here.';
            const extracted = engine.extractReplyFromResponse(raw);
            expect(extracted).toBe('Great insight here.');

            const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.7); // Avoid lowercase/dot-stripping randomness
            const cleaned = engine.cleanQuote('"Great insight here."');
            randomSpy.mockRestore();
            expect(cleaned).toBe('Great insight here');
        });

        it('validates generic responses as invalid', () => {
            const result = engine.validateQuote('so true for me today');
            expect(result.valid).toBe(false);
            expect(result.reason).toContain('generic_response');
        });
    });

    describe('Quote Generation Flow', () => {
        it('builds enhanced prompt with guidance and replies', () => {
            const prompt = engine.buildEnhancedPrompt(
                'tweet text',
                'author',
                [{ author: 'a', text: 'A longer reply that should be included' }],
                'https://x.com/status/1',
                baseSentiment,
                true,
                'high'
            );
            expect(prompt.text).toContain('TONE GUIDANCE');
            expect(prompt.text).toContain('A longer reply');
        });

        it('generates a quote successfully', async () => {
            sentimentService.analyze.mockReturnValue(baseSentiment);
            sentimentService.analyzeForReplySelection.mockReturnValue({
                strategy: 'mixed',
                distribution: { positive: 1, negative: 0, sarcastic: 0 },
                recommendations: {
                    manualSelection: null,
                    filter: () => true,
                    sort: () => 0,
                    max: 1,
                },
                analyzed: [{ author: 'a', text: 'nice' }],
            });
            engine.agent.processRequest.mockResolvedValue({
                success: true,
                data: { content: 'Great take here.' },
            });
            const result = await engine.generateQuote('tweet text', 'user', {
                replies: [{ author: 'a', text: 'nice' }],
            });
            expect(result.success).toBe(true);
            expect(result.quote.toLowerCase()).toContain('great');
        });

        it('handles LLM failure cases', async () => {
            sentimentService.analyze.mockReturnValue(baseSentiment);
            sentimentService.analyzeForReplySelection.mockReturnValue({
                strategy: 'mixed',
                distribution: { positive: 0, negative: 0, sarcastic: 0 },
                recommendations: {
                    manualSelection: null,
                    filter: () => true,
                    sort: () => 0,
                    max: 1,
                },
                analyzed: [],
            });

            // Empty content
            engine.agent.processRequest.mockResolvedValueOnce({
                success: true,
                data: { content: '' },
            });
            const resultEmpty = await engine.generateQuote('tweet text', 'user', {});
            expect(resultEmpty.success).toBe(false);
            expect(resultEmpty.reason).toContain('llm_empty_content');

            // Request fail
            engine.agent.processRequest.mockResolvedValueOnce({
                success: false,
                error: 'bad_request',
            });
            const resultFail = await engine.generateQuote('tweet text', 'user', {});
            expect(resultFail.success).toBe(false);
        });
    });
});
