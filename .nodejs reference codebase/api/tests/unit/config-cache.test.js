/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocked = vi.hoisted(() => ({
    logger: {
        debug: vi.fn(),
        info: vi.fn(),
    },
}));

vi.mock('@api/core/logger.js', () => ({
    createLogger: vi.fn(() => mocked.logger),
}));

describe('ConfigCache', () => {
    let ConfigCache;
    let configCache;
    let getFromCache;
    let setInCache;
    let clearCache;
    let getCacheStats;

    beforeEach(async () => {
        vi.resetModules();
        vi.clearAllMocks();
        const module = await import('../../utils/config-cache.js');
        ConfigCache = module.ConfigCache;
        configCache = module.configCache;
        getFromCache = module.getFromCache;
        setInCache = module.setInCache;
        clearCache = module.clearCache;
        getCacheStats = module.getCacheStats;
    });

    it('initializes with defaults', () => {
        const cache = new ConfigCache();
        expect(cache.ttl).toBe(300000);
        expect(cache.maxSize).toBe(100);
    });

    it('sets and gets values', () => {
        const cache = new ConfigCache({ ttl: 1000 });
        cache.set('a', { ok: true });
        expect(cache.get('a')).toEqual({ ok: true });
    });

    it('updates existing entries without growing size', () => {
        const cache = new ConfigCache();
        cache.set('a', 1);
        cache.set('a', 2);
        expect(cache.cache.size).toBe(1);
        expect(cache.get('a')).toBe(2);
    });

    it('returns null on miss', () => {
        const cache = new ConfigCache();
        expect(cache.get('missing')).toBeNull();
    });

    it('expires entries by ttl', () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const cache = new ConfigCache({ ttl: 1000 });
        cache.set('a', 1);
        vi.setSystemTime(new Date('2026-02-14T00:00:02Z'));
        expect(cache.get('a')).toBeNull();
        vi.useRealTimers();
    });

    it('evicts least recently used entry', () => {
        vi.useFakeTimers();
        const cache = new ConfigCache({ maxSize: 2 });
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        cache.set('a', 1);
        vi.setSystemTime(new Date('2026-02-14T00:00:01Z'));
        cache.set('b', 2);
        vi.setSystemTime(new Date('2026-02-14T00:00:02Z'));
        cache.get('a');
        vi.setSystemTime(new Date('2026-02-14T00:00:03Z'));
        cache.set('c', 3);
        expect(cache.get('b')).toBeNull();
        expect(cache.get('a')).toBe(1);
        expect(cache.get('c')).toBe(3);
        vi.useRealTimers();
    });

    it('no-ops eviction when cache is empty', () => {
        const cache = new ConfigCache();
        cache.evictLRU();
        expect(cache.evictionCount).toBe(0);
    });

    it('keeps cache when access order is empty', () => {
        const cache = new ConfigCache();
        cache.cache.set('a', { value: 1, timestamp: Date.now() });
        cache.evictLRU();
        expect(cache.cache.size).toBe(1);
    });

    it('deletes entries', () => {
        const cache = new ConfigCache();
        cache.set('a', 1);
        expect(cache.delete('a')).toBe(true);
        expect(cache.delete('a')).toBe(false);
    });

    it('clears entries', () => {
        const cache = new ConfigCache();
        cache.set('a', 1);
        cache.set('b', 2);
        cache.clear();
        expect(cache.cache.size).toBe(0);
    });

    it('checks existence via has', () => {
        const cache = new ConfigCache({ ttl: 1000 });
        cache.set('a', 1);
        expect(cache.has('a')).toBe(true);
    });

    it('returns cache statistics', () => {
        const cache = new ConfigCache();
        cache.set('a', 1);
        cache.get('a');
        cache.get('b');
        const stats = cache.getStats();
        expect(stats.totalRequests).toBe(2);
        expect(stats.hitCount).toBe(1);
        expect(stats.missCount).toBe(1);
    });

    it('returns zero hit rate when no requests', () => {
        const cache = new ConfigCache();
        const stats = cache.getStats();
        expect(stats.hitRate).toBe('0.00%');
    });

    it('returns entries with age metadata', () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const cache = new ConfigCache();
        cache.set('a', 1);
        vi.setSystemTime(new Date('2026-02-14T00:00:10Z'));
        const entries = cache.getEntries();
        expect(entries[0].key).toBe('a');
        expect(entries[0].age).toBe(10000);
        vi.useRealTimers();
    });

    it('sorts entries by age', () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const cache = new ConfigCache();
        cache.set('old', 1);
        vi.setSystemTime(new Date('2026-02-14T00:00:05Z'));
        cache.set('new', 2);
        vi.setSystemTime(new Date('2026-02-14T00:00:10Z'));
        const entries = cache.getEntries();
        expect(entries[0].key).toBe('old');
        expect(entries[1].key).toBe('new');
        vi.useRealTimers();
    });

    it('returns empty entries when cache is empty', () => {
        const cache = new ConfigCache();
        expect(cache.getEntries()).toEqual([]);
    });

    it('cleans expired entries', () => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-02-14T00:00:00Z'));
        const cache = new ConfigCache({ ttl: 1000 });
        cache.set('a', 1);
        vi.setSystemTime(new Date('2026-02-14T00:00:02Z'));
        const cleaned = cache.cleanExpired();
        expect(cleaned).toBe(1);
        vi.useRealTimers();
    });

    it('does not log when no expired entries are cleaned', () => {
        const cache = new ConfigCache({ ttl: 1000 });
        cache.set('a', 1);
        const cleaned = cache.cleanExpired();
        expect(cleaned).toBe(0);
    });

    it('updates ttl and max size', () => {
        const cache = new ConfigCache({ maxSize: 2 });
        cache.updateTTL(5000);
        expect(cache.ttl).toBe(5000);
        cache.set('a', 1);
        cache.set('b', 2);
        cache.set('c', 3);
        cache.updateMaxSize(1);
        expect(cache.cache.size).toBe(1);
    });

    it('estimates memory usage', () => {
        const cache = new ConfigCache();
        cache.set('a', { value: 'x' });
        const usage = cache.getMemoryUsage();
        expect(usage.entries).toBe(1);
    });

    it('uses convenience cache helpers on singleton', () => {
        clearCache();
        setInCache('a', 1);
        expect(getFromCache('a')).toBe(1);
        const stats = getCacheStats();
        expect(stats.size).toBe(1);
        configCache.clear();
    });

    it('tracks misses via singleton helpers', () => {
        clearCache();
        expect(getFromCache('missing')).toBeNull();
        const stats = getCacheStats();
        expect(stats.missCount).toBe(1);
    });

    it('reports entries and memory usage on singleton', () => {
        clearCache();
        setInCache('a', { value: 'x' });
        expect(configCache.getEntries().length).toBe(1);
        const usage = configCache.getMemoryUsage();
        expect(usage.entries).toBe(1);
        clearCache();
    });
});
