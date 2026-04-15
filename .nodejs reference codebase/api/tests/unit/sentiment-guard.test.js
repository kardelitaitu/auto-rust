/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from 'vitest';
import sentimentGuard from '@api/utils/sentiment-guard.js';

const {
    analyzeSentiment,
    shouldSkipAction,
    getSkipReason,
    getSafeActions,
    formatSentimentReport,
    shouldProcessContent,
} = sentimentGuard;

describe('sentiment-guard', () => {
    describe('analyzeSentiment', () => {
        it('should return neutral result for empty text', () => {
            const result = analyzeSentiment('');
            expect(result.score).toBe(0);
            expect(result.isNegative).toBe(false);
            expect(result.categories).toEqual([]);
        });

        it('should return neutral result for null/undefined', () => {
            expect(analyzeSentiment(null).score).toBe(0);
            expect(analyzeSentiment(undefined).score).toBe(0);
            expect(analyzeSentiment(123).score).toBe(0);
        });

        it('should detect negative keywords in death category', () => {
            const result = analyzeSentiment('My grandfather passed away last week');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(expect.objectContaining({ name: 'death' }));
        });

        it('should detect negative keywords in tragedy category', () => {
            const result = analyzeSentiment('Tragic accident on the highway today');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(expect.objectContaining({ name: 'tragedy' }));
        });

        it('should detect negative keywords in grief category', () => {
            const result = analyzeSentiment('I am so heartbroken and devastated');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(expect.objectContaining({ name: 'grief' }));
        });

        it('should detect negative keywords in scam category', () => {
            const result = analyzeSentiment('My account was hacked and stolen');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(expect.objectContaining({ name: 'scam' }));
        });

        it('should detect negative keywords in controversy category', () => {
            const result = analyzeSentiment('The company is facing serious allegations');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(
                expect.objectContaining({ name: 'controversy' })
            );
        });

        it('should detect negative patterns like RIP', () => {
            const result = analyzeSentiment('RIP my friend');
            expect(result.isNegative).toBe(true);
            expect(result.categories).toContainEqual(expect.objectContaining({ name: 'pattern' }));
        });

        it('should detect "rest in peace" pattern', () => {
            const result = analyzeSentiment('Rest in peace, we will miss you');
            expect(result.isNegative).toBe(true);
        });

        it('should return positive for neutral content', () => {
            const result = analyzeSentiment('Just had a great lunch! Loving this weather');
            expect(result.isNegative).toBe(false);
            expect(result.score).toBeLessThan(0.15);
        });

        it('should set shouldSkipLikes based on threshold', () => {
            const negativeResult = analyzeSentiment('So sad about the tragedy');
            expect(negativeResult.shouldSkipLikes).toBe(true);
        });
    });

    describe('shouldSkipAction', () => {
        it('should return true for like action on negative content', () => {
            const result = shouldSkipAction('Very sad tragedy happened', 'like');
            expect(result).toBe(true);
        });

        it('should return true for retweet action on negative content', () => {
            const result = shouldSkipAction('Passed away suddenly', 'retweet');
            expect(result).toBe(true);
        });

        it('should return true for reply action on negative content', () => {
            const result = shouldSkipAction('Heartbroken about this', 'reply');
            expect(result).toBe(true);
        });

        it('should return true for quote action on negative content', () => {
            const result = shouldSkipAction('RIP friend', 'quote');
            expect(result).toBe(true);
        });

        it('should return true for expand on negative content', () => {
            const result = shouldSkipAction('Devastating news', 'expand');
            expect(result).toBe(true); // allowExpand is always true
        });

        it('should return false for unknown action', () => {
            const result = shouldSkipAction('Bad content', 'unknown');
            expect(result).toBe(false);
        });

        it('should handle action case-insensitively', () => {
            const result = shouldSkipAction('RIP', 'LIKE');
            expect(result).toBe(true);
        });
    });

    describe('getSkipReason', () => {
        it('should return skip reason for likes on negative content', () => {
            const result = getSkipReason('My dog passed away', 'like');
            expect(result.skipped).toBe(true);
            expect(result.reason).toBe('Negative sentiment detected');
            expect(result.categories).toBeDefined();
            expect(result.score).toBeGreaterThan(0);
        });

        it('should return not skipped for positive content', () => {
            const result = getSkipReason('Having a great day!', 'like');
            expect(result.skipped).toBe(false);
            expect(result.reason).toBeNull();
        });
    });

    describe('getSafeActions', () => {
        it('should return all actions allowed for positive content', () => {
            const result = getSafeActions('Love this beautiful weather!');
            expect(result.canLike).toBe(true);
            expect(result.canRetweet).toBe(true);
            expect(result.canReply).toBe(true);
            expect(result.canQuote).toBe(true);
            expect(result.canExpand).toBe(true);
            expect(result.isNegative).toBe(false);
        });

        it('should restrict actions for negative content', () => {
            const result = getSafeActions('Tragedy happened today');
            expect(result.canLike).toBe(false);
            expect(result.canRetweet).toBe(false);
            expect(result.canReply).toBe(false);
            expect(result.canQuote).toBe(false);
            expect(result.isNegative).toBe(true);
        });
    });

    describe('formatSentimentReport', () => {
        it('should format negative content report', () => {
            const report = formatSentimentReport('RIP my friend');
            expect(report).toContain('NEGATIVE');
            expect(report).toContain('score:');
        });

        it('should format positive content report', () => {
            const report = formatSentimentReport('Great day today!');
            expect(report).toContain('Neutral/Positive');
            expect(report).toContain('score:');
        });
    });

    describe('shouldProcessContent', () => {
        it('should allow processing with expand only for negative content by default', () => {
            const result = shouldProcessContent('Very sad tragedy');
            expect(result.allowed).toBe(true);
            expect(result.restrictions.expand).toBe(true);
            expect(result.restrictions.like).toBe(false);
            expect(result.restrictions.retweet).toBe(false);
            expect(result.restrictions.reply).toBe(false);
            expect(result.restrictions.quote).toBe(false);
        });

        it('should block all processing for negative when allowNegativeExpand is false', () => {
            const result = shouldProcessContent('Terrible news', { allowNegativeExpand: false });
            expect(result.allowed).toBe(false);
            expect(result.reason).toBe('Negative content');
        });

        it('should allow all content when positive', () => {
            const result = shouldProcessContent('Having a wonderful time!');
            expect(result.allowed).toBe(true);
            expect(result.restrictions).toBeNull();
        });
    });

    describe('defaults export', () => {
        it('should export thresholds', () => {
            expect(sentimentGuard.defaults.thresholds).toBeDefined();
            expect(sentimentGuard.defaults.thresholds.skipLike).toBe(0.15);
        });

        it('should export keywords', () => {
            expect(sentimentGuard.defaults.keywords).toBeDefined();
            expect(sentimentGuard.defaults.keywords.death).toBeDefined();
        });

        it('should export patterns', () => {
            expect(sentimentGuard.defaults.patterns).toBeDefined();
            expect(sentimentGuard.defaults.patterns.length).toBeGreaterThan(0);
        });
    });
});
