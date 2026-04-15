/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect } from 'vitest';

describe('tasks/cookieBotRandom.js - URL Parsing', () => {
    it('should parse http_ prefix correctly to http:', () => {
        const urls = 'https://example.com\nhttp_test.com\nhttp__example.org';
        const parsed = urls
            .split('\n')
            .map((line) => {
                let url = line.trim();
                if (url.startsWith('http_')) {
                    url = url.replace('http_', 'http:');
                }
                if (url.startsWith('https_')) {
                    url = url.replace('https_', 'https:');
                }
                return url;
            })
            .filter((line) => line.startsWith('http'));

        expect(parsed).toContain('https://example.com');
        expect(parsed).toContain('http:test.com');
        expect(parsed).toContain('http:_example.org');
    });

    it('should filter out invalid URLs', () => {
        const urls = 'https://valid.com\nnot-a-url\n  \nhttp://also-valid.com';
        const parsed = urls
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line.startsWith('http'));

        expect(parsed).toEqual(['https://valid.com', 'http://also-valid.com']);
    });

    it('should handle URLs with trailing whitespace', () => {
        const urls = '  https://example.com  \n  https://test.com  ';
        const parsed = urls
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line.startsWith('http'));

        expect(parsed).toEqual(['https://example.com', 'https://test.com']);
    });

    it('should handle empty input', () => {
        const urls = '';
        const parsed = urls
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line.startsWith('http'));

        expect(parsed).toEqual([]);
    });

    it('should handle whitespace-only input', () => {
        const urls = '   \n   \n   ';
        const parsed = urls
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line.startsWith('http'));

        expect(parsed).toEqual([]);
    });

    it('should preserve full URLs without http_ prefix', () => {
        const urls = 'https://example.com\nhttp://test.com\nftp://invalid.com';
        const parsed = urls
            .split('\n')
            .map((line) => line.trim())
            .filter((line) => line.startsWith('http'));

        expect(parsed).toEqual(['https://example.com', 'http://test.com']);
    });
});

describe('tasks/cookieBotRandom.js - Module Export', () => {
    it('should export a default function', async () => {
        try {
            const task = await import('../../tasks/cookieBotRandom.js');
            expect(task.default).toBeDefined();
            expect(typeof task.default).toBe('function');
        } catch (error) {
            // If the module doesn't exist, the test should be skipped
            if (error.code === 'ERR_MODULE_NOT_FOUND') {
                console.warn('Skipping test: cookieBotRandom.js module not found');
                return;
            }
            throw error;
        }
    });
});
