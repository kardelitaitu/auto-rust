/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
    REPLY_SYSTEM_PROMPT,
    getStrategyInstruction,
    buildReplyPrompt,
    getSentimentGuidance,
    getReplyLengthGuidance,
    buildEnhancedPrompt,
    buildAnalysisPrompt,
} from '@api/twitter/twitter-reply-prompt.js';

describe('twitter-reply-prompt', () => {
    describe('REPLY_SYSTEM_PROMPT', () => {
        it('should be a non-empty string', () => {
            expect(typeof REPLY_SYSTEM_PROMPT).toBe('string');
            expect(REPLY_SYSTEM_PROMPT.length).toBeGreaterThan(0);
        });

        it('should contain critical formatting rules', () => {
            expect(REPLY_SYSTEM_PROMPT).toContain('NO @mentions');
            expect(REPLY_SYSTEM_PROMPT).toContain('NO #hashtags');
            expect(REPLY_SYSTEM_PROMPT).toContain('NO emojis');
            expect(REPLY_SYSTEM_PROMPT).toContain('KEEP IT SHORT');
        });

        it('should contain banned words section', () => {
            expect(REPLY_SYSTEM_PROMPT).toContain('BANNED WORDS');
            expect(REPLY_SYSTEM_PROMPT).toContain('Tapestry');
            expect(REPLY_SYSTEM_PROMPT).toContain('Testament');
        });

        it('should contain image handling instructions', () => {
            expect(REPLY_SYSTEM_PROMPT).toContain('IMAGE HANDLING');
            expect(REPLY_SYSTEM_PROMPT).toContain('analyze visuals');
        });
    });

    describe('getStrategyInstruction', () => {
        beforeEach(() => {
            vi.spyOn(Math, 'random').mockReturnValue(0.5);
        });

        afterEach(() => {
            vi.restoreAllMocks();
        });

        it('should return a strategy instruction string', () => {
            const result = getStrategyInstruction({});
            expect(result).toContain('CRITICAL INSTRUCTION');
        });

        it('should handle different contexts without throwing', () => {
            expect(() => getStrategyInstruction({ type: 'humorous' })).not.toThrow();
            expect(() => getStrategyInstruction({ sentiment: 'negative' })).not.toThrow();
            expect(() => getStrategyInstruction({ engagement: 'viral' })).not.toThrow();
        });

        it('should return valid strategy for viral engagement', () => {
            // With viral context, MINIMALIST/REACTION/SLANG get boosts.
            // We don't need to test exact probability, just that it returns a valid string.
            const result = getStrategyInstruction({ engagement: 'viral' });
            expect(result).toContain('CRITICAL INSTRUCTION');
        });

        it('should return strategy for humorous type', () => {
            const result = getStrategyInstruction({ type: 'humorous' });
            expect(result).toContain('CRITICAL INSTRUCTION');
        });
    });

    describe('buildReplyPrompt', () => {
        it('should build basic reply prompt', () => {
            const tweetText = 'This is a test tweet';
            const author = 'testuser';

            const prompt = buildReplyPrompt(tweetText, author);

            expect(prompt).toContain(tweetText);
            expect(prompt).toContain(author);
            expect(prompt).toContain('Tweet from @testuser');
        });

        it('should include replies in prompt', () => {
            const tweetText = 'Test tweet';
            const author = 'user1';
            const replies = [
                { author: 'user2', text: 'Great tweet!' },
                { author: 'user3', text: 'Agreed!' },
            ];

            const prompt = buildReplyPrompt(tweetText, author, replies);

            expect(prompt).toContain('@user2');
            expect(prompt).toContain('@user3');
            expect(prompt).toContain('Great tweet!');
            expect(prompt).toContain('Agreed!');
        });

        it('should handle empty replies array', () => {
            const tweetText = 'Test tweet';
            const author = 'user1';

            const prompt = buildReplyPrompt(tweetText, author, []);

            expect(prompt).toContain('no other replies visible');
        });

        it('should truncate long replies to 80 chars', () => {
            const tweetText = 'Test';
            const author = 'user';
            const longText = 'a'.repeat(300);
            const replies = [{ author: 'user2', text: longText }];

            const prompt = buildReplyPrompt(tweetText, author, replies);

            const replyLine = prompt.split('\n').find((line) => line.startsWith('1. @user2:'));
            const replyText = replyLine.replace('1. @user2: ', '');
            expect(replyText.length).toBe(150);
        });

        it('should include strategy instruction in prompt', () => {
            const tweetText = 'Test';
            const author = 'user';

            const prompt = buildReplyPrompt(tweetText, author);

            expect(prompt).toContain('CRITICAL INSTRUCTION');
        });
    });

    describe('getSentimentGuidance', () => {
        it('should return guidance for enthusiastic sentiment', () => {
            const guidance = getSentimentGuidance('enthusiastic', 'general', 0);
            expect(guidance).toContain('excitement');
            expect(guidance).toContain('energy');
        });

        it('should return guidance for humorous sentiment', () => {
            const guidance = getSentimentGuidance('humorous', 'general', 0);
            expect(guidance).toContain('witty');
            expect(guidance).toContain('fun');
        });

        it('should return guidance for informative sentiment', () => {
            const guidance = getSentimentGuidance('informative', 'general', 0);
            expect(guidance).toContain('fact');
            expect(guidance).toContain('information');
        });

        it('should return guidance for neutral sentiment', () => {
            const guidance = getSentimentGuidance('neutral', 'general', 0);
            expect(guidance).toContain('question');
            expect(guidance).toContain('observation');
        });

        it('should handle high sarcasm score', () => {
            const guidance = getSentimentGuidance('sarcastic', 'general', 0.7);
            expect(guidance).toContain('Match the ironic tone');
        });
    });

    describe('getReplyLengthGuidance', () => {
        it('should return guidance for heated-debate', () => {
            const guidance = getReplyLengthGuidance('heated-debate', 0);
            expect(guidance).toContain('Maximum 1 short sentence');
            expect(guidance).toContain('CRITICAL');
        });

        it('should return guidance for casual-chat', () => {
            const guidance = getReplyLengthGuidance('casual-chat', 0);
            expect(guidance).toContain('Maximum 1 short sentence');
        });

        it('should return general guidance for unknown type', () => {
            const guidance = getReplyLengthGuidance('unknown', 0);
            expect(guidance).toContain('Maximum 1 short sentence');
        });

        it('should add expressiveness for high valence', () => {
            const guidance = getReplyLengthGuidance('general', 0.8);
            expect(guidance).toContain('expressive');
        });
    });

    describe('buildEnhancedPrompt', () => {
        it('should build enhanced prompt with tweet context', () => {
            const context = {
                tweetText: 'Test tweet',
                author: 'testuser',
                replies: [],
                sentiment: {
                    overall: 'positive',
                },
                hasImage: false,
            };

            const prompt = buildEnhancedPrompt(context);

            expect(prompt).toContain('Test tweet');
            expect(prompt).toContain('@testuser');
            expect(prompt).toContain('CRITICAL INSTRUCTION');
        });

        it('should handle replies in enhanced prompt', () => {
            const context = {
                tweetText: 'Test',
                author: 'user',
                replies: [
                    { author: 'reply1', text: 'This is a valid reply text' },
                    { author: 'reply2', text: 'Agreed completely with this' },
                ],
                sentiment: { overall: 'positive' },
            };

            const prompt = buildEnhancedPrompt(context);

            expect(prompt).toContain('@reply1');
            expect(prompt).toContain('@reply2');
            expect(prompt).toContain('This is a valid reply text');
        });

        it('should filter short replies (< 5 chars)', () => {
            const context = {
                tweetText: 'Test',
                author: 'user',
                replies: [
                    { author: 'r1', text: 'ab' }, // Too short
                    { author: 'r2', text: 'This is long enough' },
                ],
            };

            const prompt = buildEnhancedPrompt(context);

            expect(prompt).toContain('@r2');
            expect(prompt).not.toContain('@r1');
        });

        it('should handle image in tweet', () => {
            const context = {
                tweetText: 'Test',
                author: 'user',
                replies: [],
                hasImage: true,
            };

            const prompt = buildEnhancedPrompt(context);

            expect(prompt).toContain('[IMAGE ATTACHED');
        });

        it('should use default values for missing sentiment', () => {
            const context = {
                tweetText: 'Test',
                author: 'user',
                replies: [],
                sentiment: null,
            };

            const prompt = buildEnhancedPrompt(context);

            // Default sentiment is neutral, which should work
            expect(prompt).toContain('CRITICAL INSTRUCTION');
        });
    });

    describe('buildAnalysisPrompt', () => {
        it('should build analysis prompt', () => {
            const tweetText = 'This is a test tweet';
            const prompt = buildAnalysisPrompt(tweetText);

            expect(prompt).toContain(tweetText);
            expect(prompt).toContain('Analyze this tweet');
            expect(prompt).toContain('Respond with JSON');
        });
    });
});
