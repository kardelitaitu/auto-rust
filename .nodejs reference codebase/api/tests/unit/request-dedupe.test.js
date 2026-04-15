/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { RequestDedupe } from '@api/utils/request-dedupe.js';

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => ({
        debug: vi.fn(),
        info: vi.fn(),
        warn: vi.fn(),
        error: vi.fn(),
    })),
}));

describe('request-dedupe.js', () => {
    let dedupe;

    beforeEach(() => {
        dedupe = new RequestDedupe({
            ttl: 30000,
            maxSize: 100,
            enabled: true,
        });
        vi.clearAllMocks();
    });

    describe('constructor', () => {
        it('should initialize with default options', () => {
            const defaultDedupe = new RequestDedupe();
            expect(defaultDedupe.ttl).toBe(30000);
            expect(defaultDedupe.maxSize).toBe(1000);
            expect(defaultDedupe.enabled).toBe(true);
        });

        it('should accept custom options', () => {
            const customDedupe = new RequestDedupe({
                ttl: 60000,
                maxSize: 500,
                enabled: false,
            });
            expect(customDedupe.ttl).toBe(60000);
            expect(customDedupe.maxSize).toBe(500);
            expect(customDedupe.enabled).toBe(false);
        });
    });

    describe('_generateKey', () => {
        it('should generate consistent keys for same input', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            const key1 = dedupe._generateKey(messages, 'gpt-4', 100, 0.7);
            const key2 = dedupe._generateKey(messages, 'gpt-4', 100, 0.7);
            expect(key1).toBe(key2);
        });

        it('should generate different keys for different messages', () => {
            const messages1 = [{ role: 'user', content: 'Hello' }];
            const messages2 = [{ role: 'user', content: 'Hi' }];
            const key1 = dedupe._generateKey(messages1, 'gpt-4', 100, 0.7);
            const key2 = dedupe._generateKey(messages2, 'gpt-4', 100, 0.7);
            expect(key1).not.toBe(key2);
        });

        it('should generate different keys for different parameters', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            const key1 = dedupe._generateKey(messages, 'gpt-4', 100, 0.7);
            const key2 = dedupe._generateKey(messages, 'gpt-3.5', 100, 0.7);
            expect(key1).not.toBe(key2);
        });
    });

    describe('_hash', () => {
        it('should generate consistent hash for same string', () => {
            const hash1 = dedupe._hash('test');
            const hash2 = dedupe._hash('test');
            expect(hash1).toBe(hash2);
        });

        it('should generate different hash for different strings', () => {
            const hash1 = dedupe._hash('test1');
            const hash2 = dedupe._hash('test2');
            expect(hash1).not.toBe(hash2);
        });
    });

    describe('check', () => {
        it('should return miss when cache is empty', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
            expect(result.key).toBeDefined();
        });

        it('should return hit when cache has matching entry', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', 'Test response', 100, 0.7);
            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(true);
            expect(result.response).toBe('Test response');
        });

        it('should return miss when entry is expired', async () => {
            const shortTtlDedupe = new RequestDedupe({ ttl: 10 });
            const messages = [{ role: 'user', content: 'Hello' }];
            shortTtlDedupe.set(messages, 'gpt-4', 'Test response', 100, 0.7);

            await new Promise((resolve) => setTimeout(resolve, 20));

            const result = shortTtlDedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
            expect(result.reason).toBe('expired');
        });

        it('should return miss when disabled', () => {
            const disabledDedupe = new RequestDedupe({ enabled: false });
            const messages = [{ role: 'user', content: 'Hello' }];
            const result = disabledDedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
            expect(result.reason).toBe('disabled');
        });

        it('should track stats correctly', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(dedupe.stats.misses).toBe(1);
        });
    });

    describe('set', () => {
        it('should cache response', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', 'Test response', 100, 0.7);

            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(true);
            expect(result.response).toBe('Test response');
        });

        it('should not cache empty responses', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', '', 100, 0.7);

            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
        });

        it('should not cache whitespace-only responses', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', '   ', 100, 0.7);

            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
        });

        it('should not cache when disabled', () => {
            const disabledDedupe = new RequestDedupe({ enabled: false });
            const messages = [{ role: 'user', content: 'Hello' }];
            disabledDedupe.set(messages, 'gpt-4', 'Test response', 100, 0.7);

            const result = disabledDedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.hit).toBe(false);
        });

        it('should update cache with new response', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', 'Response 1', 100, 0.7);
            dedupe.set(messages, 'gpt-4', 'Response 2', 100, 0.7);

            const result = dedupe.check(messages, 'gpt-4', 100, 0.7);
            expect(result.response).toBe('Response 2');
        });
    });

    describe('cache eviction', () => {
        it('should evict oldest entry when cache is full', () => {
            const smallCache = new RequestDedupe({ maxSize: 2 });

            smallCache.set([{ role: 'user', content: 'A' }], 'gpt-4', 'Response A');
            smallCache.set([{ role: 'user', content: 'B' }], 'gpt-4', 'Response B');
            smallCache.set([{ role: 'user', content: 'C' }], 'gpt-4', 'Response C');

            const resultA = smallCache.check([{ role: 'user', content: 'A' }], 'gpt-4');
            expect(resultA.hit).toBe(false);
        });
    });

    describe('clear', () => {
        it('should clear all cached entries', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', 'Test response');

            dedupe.clear();

            const result = dedupe.check(messages, 'gpt-4');
            expect(result.hit).toBe(false);
        });
    });

    describe('getStats', () => {
        it('should return statistics', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            const messages2 = [{ role: 'user', content: 'New' }];
            dedupe.set(messages, 'gpt-4', 'Response');
            dedupe.check(messages, 'gpt-4');
            dedupe.check(messages2, 'gpt-4');

            const stats = dedupe.getStats();
            expect(stats.cached).toBe(1);
            expect(stats.hits).toBe(1);
            expect(stats.misses).toBe(1);
        });

        it('should calculate hit rate correctly', () => {
            const dedupe2 = new RequestDedupe({ ttl: 30000 });
            const messages = [{ role: 'user', content: 'Hello' }];
            const messages2 = [{ role: 'user', content: 'World' }];
            dedupe2.set(messages, 'gpt-4', 'Response');
            dedupe2.check(messages, 'gpt-4');
            dedupe2.check(messages2, 'gpt-4');

            const stats = dedupe2.getStats();
            expect(stats.hitRate).toBe('50.0%');
        });
    });

    describe('prune', () => {
        it('should remove expired entries', async () => {
            const shortTtl = new RequestDedupe({ ttl: 10 });
            const messages = [{ role: 'user', content: 'Hello' }];
            shortTtl.set(messages, 'gpt-4', 'Response');

            await new Promise((resolve) => setTimeout(resolve, 20));

            const pruned = shortTtl.prune();
            expect(pruned).toBe(1);
        });

        it('should return 0 if nothing to prune', () => {
            const messages = [{ role: 'user', content: 'Hello' }];
            dedupe.set(messages, 'gpt-4', 'Response');

            const pruned = dedupe.prune();
            expect(pruned).toBe(0);
        });
    });

    describe('enable/disable', () => {
        it('isEnabled should return current state', () => {
            expect(dedupe.isEnabled()).toBe(true);

            dedupe.setEnabled(false);
            expect(dedupe.isEnabled()).toBe(false);

            dedupe.setEnabled(true);
            expect(dedupe.isEnabled()).toBe(true);
        });
    });
});
