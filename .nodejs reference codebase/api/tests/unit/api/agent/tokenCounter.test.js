/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from 'vitest';
import {
    estimateTokens,
    estimateMessageTokens,
    estimateConversationTokens,
} from '@api/agent/tokenCounter.js';

describe('api/agent/tokenCounter.js', () => {
    describe('estimateTokens', () => {
        it('should return 0 for empty string', () => {
            expect(estimateTokens('')).toBe(0);
        });

        it('should return 0 for null', () => {
            expect(estimateTokens(null)).toBe(0);
        });

        it('should return 0 for undefined', () => {
            expect(estimateTokens(undefined)).toBe(0);
        });

        it('should return 0 for non-string values', () => {
            expect(estimateTokens(123)).toBe(0);
            expect(estimateTokens({})).toBe(0);
            expect(estimateTokens([])).toBe(0);
            expect(estimateTokens(true)).toBe(0);
        });

        it('should return Math.ceil(text.length / 4) for regular text', () => {
            expect(estimateTokens('test')).toBe(1);
            expect(estimateTokens('hello world')).toBe(3);
            expect(estimateTokens('a')).toBe(1);
            expect(estimateTokens('ab')).toBe(1);
            expect(estimateTokens('abc')).toBe(1);
            expect(estimateTokens('abcd')).toBe(1);
            expect(estimateTokens('abcde')).toBe(2);
        });
    });

    describe('estimateMessageTokens', () => {
        it('should return 0 for null', () => {
            expect(estimateMessageTokens(null)).toBe(0);
        });

        it('should return 0 for undefined', () => {
            expect(estimateMessageTokens(undefined)).toBe(0);
        });

        it('should handle message with role and string content', () => {
            const message = { role: 'user', content: 'hello' };
            expect(estimateMessageTokens(message)).toBe(3);
        });

        it('should handle message with role and array content (with text parts)', () => {
            const message = {
                role: 'assistant',
                content: [
                    { type: 'text', text: 'Hello' },
                    { type: 'text', text: ' world' },
                ],
            };
            expect(estimateMessageTokens(message)).toBe(7);
        });

        it('should handle message with no content', () => {
            const message = { role: 'user' };
            expect(estimateMessageTokens(message)).toBe(1);
        });

        it('should ignore non-text parts in array content', () => {
            const message = {
                role: 'user',
                content: [
                    { type: 'text', text: 'hello' },
                    { type: 'image', url: 'http://example.com/img.png' },
                    { type: 'text', text: 'world' },
                ],
            };
            expect(estimateMessageTokens(message)).toBe(5);
        });
    });

    describe('estimateConversationTokens', () => {
        it('should return 0 for null', () => {
            expect(estimateConversationTokens(null)).toBe(0);
        });

        it('should return 0 for undefined', () => {
            expect(estimateConversationTokens(undefined)).toBe(0);
        });

        it('should return 0 for empty array', () => {
            expect(estimateConversationTokens([])).toBe(0);
        });

        it('should return sum of message tokens for array of messages', () => {
            const messages = [
                { role: 'user', content: 'hi' },
                { role: 'assistant', content: 'hello there' },
            ];
            expect(estimateConversationTokens(messages)).toBe(8);
        });

        it('should handle array with mixed content types', () => {
            const messages = [
                { role: 'system', content: 'You are helpful' },
                { role: 'user', content: 'Hello' },
                {
                    role: 'assistant',
                    content: [{ type: 'text', text: 'Hi there' }],
                },
            ];
            expect(estimateConversationTokens(messages)).toBe(14);
        });
    });
});
